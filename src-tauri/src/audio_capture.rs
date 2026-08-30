use crate::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{Emitter, Manager};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioDevice {
    pub id: String,
    pub index: i32,
    pub name: String,
    pub source: String,
    pub is_default: bool,
    pub sample_rate: u32,
    pub channels: u16,
}

#[derive(Debug, Clone, Serialize)]
pub struct AudioCaptureStatus {
    pub source: String,
    pub running: bool,
}

#[derive(Clone)]
struct AudioProcessHandle {
    id: u64,
    control_tx: mpsc::Sender<String>,
    stop_tx: mpsc::Sender<()>,
    running: Arc<AtomicBool>,
    stopping: Arc<AtomicBool>,
}

pub struct AudioCaptureState {
    processes: Arc<Mutex<HashMap<String, AudioProcessHandle>>>,
    next_id: AtomicU64,
}

impl AudioCaptureState {
    pub fn new() -> Self {
        Self {
            processes: Arc::new(Mutex::new(HashMap::new())),
            next_id: AtomicU64::new(1),
        }
    }
}

impl Default for AudioCaptureState {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for AudioCaptureState {
    fn drop(&mut self) {
        if let Ok(processes) = self.processes.lock() {
            for process in processes.values() {
                process.stopping.store(true, Ordering::SeqCst);
                let _ = process.control_tx.send("stop".into());
                let _ = process.stop_tx.send(());
            }
        }
    }
}

fn existing_file(candidates: impl IntoIterator<Item = PathBuf>) -> Option<PathBuf> {
    candidates.into_iter().find(|candidate| candidate.is_file())
}

/// Locate the bundled ASR correction dictionaries directory that ships next to
/// the audio worker script (``<script_dir>/dictionaries``).
fn resolve_dict_dir(script: &std::path::Path) -> Option<String> {
    let candidate = script.parent()?.join("dictionaries");
    if candidate.is_dir() {
        candidate.to_str().map(|value| value.to_string())
    } else {
        None
    }
}

fn resolve_silero_model(app: &tauri::AppHandle) -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(app_data) = app.path().app_data_dir() { candidates.push(app_data.join("models").join("silero_vad.onnx")); }
    if let Ok(resource_dir) = app.path().resource_dir() {
        candidates.push(resource_dir.join("models").join("silero_vad.onnx"));
        candidates.push(resource_dir.join("silero_vad.onnx"));
    }
    existing_file(candidates)
}

fn resolve_worker_paths(app: &tauri::AppHandle) -> AppResult<(PathBuf, PathBuf)> {
    let mut runtime_candidates = Vec::new();
    let mut script_candidates = Vec::new();

    if let Ok(path) = std::env::var("VRCDOG_PYTHON_RUNTIME") {
        runtime_candidates.push(PathBuf::from(path).join("python.exe"));
    }
    if let Ok(path) = std::env::var("VRCDOG_AUDIO_SCRIPT") {
        script_candidates.push(PathBuf::from(path));
    }

    if let Ok(resource_dir) = app.path().resource_dir() {
        runtime_candidates.extend([
            resource_dir.join("python-runtime").join("python.exe"),
            resource_dir
                .join("resources")
                .join("python-runtime")
                .join("python.exe"),
        ]);
        script_candidates.extend([
            resource_dir
                .join("_up_")
                .join("src-python")
                .join("vrcdog_audio.py"),
            resource_dir.join("src-python").join("vrcdog_audio.py"),
            resource_dir.join("vrcdog_audio.py"),
        ]);
    }

    if let Ok(cwd) = std::env::current_dir() {
        runtime_candidates.extend([
            cwd.join("src-tauri")
                .join("resources")
                .join("python-runtime")
                .join("python.exe"),
            cwd.join("resources")
                .join("python-runtime")
                .join("python.exe"),
        ]);
        script_candidates.extend([
            cwd.join("src-python").join("vrcdog_audio.py"),
            cwd.join("..").join("src-python").join("vrcdog_audio.py"),
        ]);
    }

    if let Ok(executable) = std::env::current_exe() {
        if let Some(exe_dir) = executable.parent() {
            runtime_candidates.extend([
                exe_dir.join("python-runtime").join("python.exe"),
                exe_dir
                    .join("resources")
                    .join("python-runtime")
                    .join("python.exe"),
            ]);
            script_candidates.extend([
                exe_dir.join("src-python").join("vrcdog_audio.py"),
                exe_dir
                    .join("_up_")
                    .join("src-python")
                    .join("vrcdog_audio.py"),
            ]);
        }
    }

    let runtime = existing_file(runtime_candidates).ok_or_else(|| AppError::from(
        "Embedded Python audio runtime was not found. Run `node scripts/prepare-python-runtime.mjs` before packaging.",
    ))?;
    let script = existing_file(script_candidates)
        .ok_or_else(|| AppError::from("Bundled vrcdog_audio.py was not found"))?;
    Ok((runtime, script))
}

fn configure_command(command: &mut Command) {
    command.env("PYTHONNOUSERSITE", "1").env("PYTHONUTF8", "1");
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
}

fn emit_audio_event(app: &tauri::AppHandle, source: &str, mut payload: Value) {
    if let Some(object) = payload.as_object_mut() {
        object.insert("source".into(), Value::String(source.to_string()));
    }
    let _ = app.emit("audio-capture-event", payload);
}

#[tauri::command]
pub fn vrct_get_audio_devices(app: tauri::AppHandle) -> AppResult<Vec<AudioDevice>> {
    let (runtime, script) = resolve_worker_paths(&app)?;
    let mut command = Command::new(runtime);
    command.arg(script).arg("--list-devices");
    configure_command(&mut command);
    let output = command
        .output()
        .map_err(|error| AppError::from(format!("Unable to enumerate audio devices: {error}")))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines().rev() {
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if value.get("type").and_then(Value::as_str) == Some("devices") {
            return serde_json::from_value(value.get("devices").cloned().unwrap_or_default())
                .map_err(|error| {
                    AppError::from(format!("Invalid audio device response: {error}"))
                });
        }
        if value.get("type").and_then(Value::as_str) == Some("error") {
            return Err(AppError::from(
                value
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("Audio device enumeration failed"),
            ));
        }
    }
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    Err(AppError::from(if stderr.is_empty() {
        "Audio device worker returned no device list".to_string()
    } else {
        stderr
    }))
}

#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub fn vrct_start_audio_capture(
    app: tauri::AppHandle,
    state: tauri::State<'_, AudioCaptureState>,
    source: String,
    source_lang: String,
    engine: String,
    device_index: Option<i32>,
    energy_threshold: Option<u32>,
    dynamic_energy_threshold: Option<bool>,
    phrase_time_limit: Option<f32>,
    whisper_model: Option<String>,
    vad_type: Option<String>,
    vad_aggressiveness: Option<u32>,
    denoise_strength: Option<f32>,
    correction_enabled: Option<bool>,
    min_segment_s: Option<f32>,
    max_segment_s: Option<f32>,
    partial_interval: Option<f32>,
    capture_mode: Option<String>,
    target_process: Option<String>,
    self_suppress_seconds: Option<f32>,
    realtime_provider: Option<String>,
    realtime_config: Option<serde_json::Value>,
    sherpa_config: Option<serde_json::Value>,
) -> AppResult<()> {
    if !matches!(source.as_str(), "mic" | "speaker") {
        return Err(AppError::from("Audio source must be `mic` or `speaker`"));
    }
    if !matches!(engine.as_str(), "cloud" | "local" | "whisper" | "sensevoice" | "sherpa" | "tencent_realtime" | "aliyun_realtime") {
        return Err(AppError::from(
            "STT engine must be `cloud`, `local`, `whisper`, `sensevoice`, `sherpa`, `tencent_realtime` or `aliyun_realtime`",
        ));
    }

    let mut processes = state
        .processes
        .lock()
        .map_err(|_| AppError::from("Audio process state is unavailable"))?;
    if let Some(existing) = processes.get(&source) {
        if existing.running.load(Ordering::SeqCst) {
            return Ok(());
        }
        processes.remove(&source);
    }

    let (runtime, script) = resolve_worker_paths(&app)?;
    let mut command = Command::new(runtime);
    command
        .arg(&script)
        .arg("--source")
        .arg(&source)
        .arg("--source-lang")
        .arg(&source_lang)
        .arg("--engine")
        .arg(&engine)
        .arg("--energy-threshold")
        .arg(energy_threshold.unwrap_or(0).to_string())
        .arg(if dynamic_energy_threshold.unwrap_or(true) {
            "--dynamic-energy-threshold"
        } else {
            "--no-dynamic-energy-threshold"
        })
        .arg("--phrase-time-limit")
        .arg(
            phrase_time_limit
                .unwrap_or(10.0)
                .clamp(2.0, 30.0)
                .to_string(),
        )
        .arg("--whisper-model")
        .arg(whisper_model.unwrap_or_else(|| "tiny".into()))
        .arg("--vad-type")
        .arg(vad_type.clone().unwrap_or_else(|| "webrtc".into()))
        .arg("--silero-model")
        .arg(resolve_silero_model(&app).map(|path| path.to_string_lossy().into_owned()).unwrap_or_default())
        .arg("--vad-aggressiveness")
        .arg(vad_aggressiveness.unwrap_or(2).to_string())
        .arg("--denoise-strength")
        .arg(denoise_strength.unwrap_or(0.0).to_string())
        .arg("--min-segment-s")
        .arg(min_segment_s.unwrap_or(0.45).to_string())
        .arg("--max-segment-s")
        .arg(max_segment_s.unwrap_or(8.0).to_string())
        .arg("--partial-interval")
        .arg(partial_interval.unwrap_or(1.2).to_string())
        .arg("--capture-mode")
        .arg(capture_mode.clone().unwrap_or_else(|| "loopback".into()))
        .arg("--target-process")
        .arg(target_process.clone().unwrap_or_else(|| "VRChat.exe".into()))
        .arg("--self-suppress-seconds")
        .arg(self_suppress_seconds.unwrap_or(0.0).to_string());
    if correction_enabled.unwrap_or(false) {
        command.arg("--correction-enabled");
        if let Some(dict_dir) = resolve_dict_dir(&script) {
            command.arg("--correction-dict-dir").arg(dict_dir);
        }
    }
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(index) = device_index {
        command.arg("--device-index").arg(index.to_string());
    }
    if let Ok(cache_dir) = app.path().app_cache_dir() {
        let model_cache = cache_dir.join("whisper");
        let _ = std::fs::create_dir_all(&model_cache);
        command.env("HF_HOME", model_cache);
    }
    if let Some(provider) = realtime_provider.filter(|value| !value.trim().is_empty()) {
        command.env("VRCDOG_REALTIME_PROVIDER", provider);
    }
    if let Some(config) = realtime_config {
        if let Ok(serialized) = serde_json::to_string(&config) {
            command.env("VRCDOG_REALTIME_CONFIG", serialized);
        }
    }
    if let Some(config) = sherpa_config {
        if let Some(object) = config.as_object() {
            for (key, env_key) in [("tokens", "VRCDOG_SHERPA_TOKENS"), ("encoder", "VRCDOG_SHERPA_ENCODER"), ("decoder", "VRCDOG_SHERPA_DECODER"), ("joiner", "VRCDOG_SHERPA_JOINER")] {
                if let Some(value) = object.get(key).and_then(serde_json::Value::as_str) {
                    command.env(env_key, value);
                }
            }
        }
    }
    configure_command(&mut command);

    let mut child = command
        .spawn()
        .map_err(|error| AppError::from(format!("Unable to start audio capture: {error}")))?;
    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| AppError::from("Unable to control audio worker"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| AppError::from("Unable to read audio worker output"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| AppError::from("Unable to read audio worker diagnostics"))?;

    let id = state.next_id.fetch_add(1, Ordering::Relaxed);
    let (control_tx, control_rx) = mpsc::channel::<String>();
    let (stop_tx, stop_rx) = mpsc::channel::<()>();
    let running = Arc::new(AtomicBool::new(true));
    let stopping = Arc::new(AtomicBool::new(false));
    processes.insert(
        source.clone(),
        AudioProcessHandle {
            id,
            control_tx: control_tx.clone(),
            stop_tx,
            running: running.clone(),
            stopping: stopping.clone(),
        },
    );
    drop(processes);

    std::thread::spawn(move || {
        let mut stdin = stdin;
        while let Ok(command) = control_rx.recv() {
            if writeln!(stdin, "{command}").is_err() || stdin.flush().is_err() {
                break;
            }
            if command == "stop" {
                break;
            }
        }
    });

    let stdout_app = app.clone();
    let stdout_source = source.clone();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if let Ok(payload) = serde_json::from_str::<Value>(&line) {
                emit_audio_event(&stdout_app, &stdout_source, payload);
            }
        }
    });

    let stderr_app = app.clone();
    let stderr_source = source.clone();
    std::thread::spawn(move || {
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            let _ = stderr_app.emit(
                "audio-capture-log",
                json!({ "source": stderr_source, "message": line }),
            );
        }
    });

    let watcher_app = app.clone();
    let watcher_source = source.clone();
    let all_processes = state.processes.clone();
    std::thread::spawn(move || {
        let mut stop_requested_at: Option<Instant> = None;
        let exit_status = loop {
            if stop_rx.try_recv().is_ok() && stop_requested_at.is_none() {
                stop_requested_at = Some(Instant::now());
            }
            match child.try_wait() {
                Ok(Some(status)) => break Some(status),
                Ok(None) => {}
                Err(_) => break None,
            }
            if stop_requested_at.is_some_and(|started| started.elapsed() >= Duration::from_secs(1))
            {
                let _ = child.kill();
                break child.wait().ok();
            }
            std::thread::sleep(Duration::from_millis(50));
        };
        running.store(false, Ordering::SeqCst);
        if let Ok(mut map) = all_processes.lock() {
            if map
                .get(&watcher_source)
                .is_some_and(|handle| handle.id == id)
            {
                map.remove(&watcher_source);
            }
        }
        let expected = stopping.load(Ordering::SeqCst);
        emit_audio_event(
            &watcher_app,
            &watcher_source,
            json!({
                "type": "status",
                "message": "stopped",
                "expected": expected,
                "exit_code": exit_status.and_then(|status| status.code()),
            }),
        );
    });

    Ok(())
}

#[tauri::command]
pub fn vrct_set_audio_capture_paused(
    state: tauri::State<'_, AudioCaptureState>,
    source: String,
    paused: bool,
) -> AppResult<()> {
    let processes = state
        .processes
        .lock()
        .map_err(|_| AppError::from("Audio process state is unavailable"))?;
    if let Some(process) = processes.get(&source) {
        process
            .control_tx
            .send(if paused { "pause" } else { "resume" }.into())
            .map_err(|_| AppError::from("Audio worker is no longer running"))?;
    }
    Ok(())
}

#[tauri::command]
pub fn vrct_stop_audio_capture(
    state: tauri::State<'_, AudioCaptureState>,
    source: String,
) -> AppResult<()> {
    let processes = state
        .processes
        .lock()
        .map_err(|_| AppError::from("Audio process state is unavailable"))?;
    if let Some(process) = processes.get(&source) {
        process.stopping.store(true, Ordering::SeqCst);
        let _ = process.control_tx.send("stop".into());
        let _ = process.stop_tx.send(());
    }
    Ok(())
}

#[tauri::command]
pub fn vrct_get_audio_capture_status(
    state: tauri::State<'_, AudioCaptureState>,
) -> AppResult<Vec<AudioCaptureStatus>> {
    let processes = state
        .processes
        .lock()
        .map_err(|_| AppError::from("Audio process state is unavailable"))?;
    Ok(["mic", "speaker"]
        .into_iter()
        .map(|source| AudioCaptureStatus {
            source: source.to_string(),
            running: processes
                .get(source)
                .is_some_and(|process| process.running.load(Ordering::SeqCst)),
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::existing_file;
    use std::path::PathBuf;

    #[test]
    fn existing_file_selects_the_first_real_file() {
        let current = std::env::current_exe().expect("current executable");
        let selected = existing_file([PathBuf::from("missing-file"), current.clone()]);
        assert_eq!(selected, Some(current));
    }
}
