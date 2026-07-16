use base64::Engine;
use midly::{MetaMessage, MidiMessage, Smf, Timing, TrackEventKind};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex, OnceLock,
};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{Emitter, Manager};

const SOURCE_PROJECT_PATH: &str = r"C:\Users\27457\Desktop\VRD\VRPiano-auto-play";
const NOTE_HOLD_MS: u64 = 28;
const SPEED_STEP: f64 = 0.1;

#[cfg(target_os = "windows")]
static HOTKEY_CONTEXT: OnceLock<GlobalHotkeyContext> = OnceLock::new();
#[cfg(target_os = "windows")]
static HOTKEY_HOOK_STARTED: AtomicBool = AtomicBool::new(false);

#[cfg(target_os = "windows")]
#[derive(Clone)]
struct GlobalHotkeyContext {
    app: tauri::AppHandle,
    state: Arc<Mutex<VrpianoRuntime>>,
}

#[derive(Clone, Serialize)]
pub struct VrpianoSong {
    id: String,
    name: String,
    path: String,
    size: u64,
    modified_ms: u64,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct VrpianoOnlineSong {
    id: u64,
    title: String,
    artist: String,
    page_url: String,
}

#[derive(Clone, Serialize)]
pub struct VrpianoMidishowAccount {
    username: String,
}

#[derive(Clone, Serialize)]
pub struct VrpianoMidiData {
    name: String,
    data: String,
}

#[derive(Clone, Serialize, Deserialize)]
struct StoredMidishowAccount {
    username: String,
    password: String,
}

#[derive(Clone, Serialize)]
pub struct VrpianoStatus {
    running: bool,
    song_name: String,
    progress: f64,
    played_notes: usize,
    total_notes: usize,
    duration_ms: u64,
    elapsed_ms: u64,
    last_event: String,
    last_error: String,
    songs_dir: String,
    speed: f64,
    hotkeys_enabled: bool,
    hotkeys_available: bool,
}

#[derive(Clone, Deserialize)]
pub struct VrpianoStartRequest {
    song_path: String,
    delay_secs: u64,
    speed: f64,
}

#[derive(Clone, Deserialize)]
pub struct VrpianoHotkeyConfig {
    enabled: bool,
    song_path: String,
    delay_secs: u64,
    speed: f64,
}

#[derive(Clone, Deserialize)]
pub struct VrpianoRenameRequest {
    song_path: String,
    new_name: String,
    overwrite: bool,
}

#[derive(Clone, Deserialize)]
pub struct VrpianoDownloadRequest {
    url: String,
    filename: Option<String>,
}

#[derive(Clone, Deserialize)]
pub struct VrpianoMidishowDownloadRequest {
    midi_id: u64,
    title: Option<String>,
    preview: bool,
}

#[derive(Clone, Deserialize)]
pub struct VrpianoMidishowLoginRequest {
    username: String,
    password: String,
}

#[derive(Clone)]
pub struct VrpianoState {
    inner: Arc<Mutex<VrpianoRuntime>>,
}

struct VrpianoRuntime {
    stop: Option<Arc<AtomicBool>>,
    speed: Arc<Mutex<f64>>,
    hotkeys_enabled: bool,
    hotkey_song_path: String,
    hotkey_delay_secs: u64,
    status: VrpianoStatus,
}

#[derive(Clone)]
struct PlayEvent {
    at_ms: u64,
    note: u8,
    vk: u16,
}

impl Default for VrpianoState {
    fn default() -> Self {
        Self {
            inner: Arc::new(Mutex::new(VrpianoRuntime {
                stop: None,
                speed: Arc::new(Mutex::new(1.0)),
                hotkeys_enabled: false,
                hotkey_song_path: String::new(),
                hotkey_delay_secs: 5,
                status: VrpianoStatus {
                    running: false,
                    song_name: String::new(),
                    progress: 0.0,
                    played_notes: 0,
                    total_notes: 0,
                    duration_ms: 0,
                    elapsed_ms: 0,
                    last_event: "VRPiano ready".to_string(),
                    last_error: String::new(),
                    songs_dir: String::new(),
                    speed: 1.0,
                    hotkeys_enabled: false,
                    hotkeys_available: cfg!(target_os = "windows"),
                },
            })),
        }
    }
}

#[tauri::command]
pub async fn vrpiano_init(app: tauri::AppHandle) -> Result<VrpianoStatus, String> {
    let songs_dir = ensure_songs_dir(&app)?;
    import_seed_songs(&songs_dir)?;
    Ok(status_with_dir(&app, "VRPiano ready")?)
}

#[tauri::command]
pub async fn vrpiano_list_songs(app: tauri::AppHandle) -> Result<Vec<VrpianoSong>, String> {
    let songs_dir = ensure_songs_dir(&app)?;
    let mut songs = Vec::new();
    for entry in fs::read_dir(&songs_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if !is_midi_file(&path) {
            continue;
        }
        songs.push(song_from_path(&path)?);
    }
    songs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(songs)
}

#[tauri::command]
pub async fn vrpiano_import_song(
    app: tauri::AppHandle,
    source_path: String,
) -> Result<VrpianoSong, String> {
    let source = PathBuf::from(source_path);
    if !source.exists() || !source.is_file() {
        return Err("MIDI file does not exist".to_string());
    }
    if !is_midi_file(&source) {
        return Err("Only .mid and .midi files are supported".to_string());
    }

    let songs_dir = ensure_songs_dir(&app)?;
    let filename = source
        .file_name()
        .and_then(|name| name.to_str())
        .map(sanitize_filename)
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "imported.mid".to_string());
    let target = unique_path(&songs_dir.join(filename));
    fs::copy(&source, &target).map_err(|e| format!("Failed to import MIDI: {e}"))?;
    Ok(song_from_path(&target)?)
}

#[tauri::command]
pub async fn vrpiano_rename_song(
    app: tauri::AppHandle,
    request: VrpianoRenameRequest,
) -> Result<VrpianoSong, String> {
    let songs_dir = ensure_songs_dir(&app)?;
    let source = resolve_song_path(&songs_dir, &request.song_path)?;
    let mut filename = sanitize_filename(request.new_name.trim());
    if filename.is_empty() {
        return Err("Please enter a valid song name".to_string());
    }
    if !filename.to_lowercase().ends_with(".mid") && !filename.to_lowercase().ends_with(".midi") {
        filename.push_str(".mid");
    }

    let target = songs_dir.join(filename);
    if target == source {
        return song_from_path(&source);
    }
    if target.exists() {
        if !request.overwrite {
            return Err("A song with this name already exists".to_string());
        }
        fs::remove_file(&target).map_err(|e| format!("Failed to overwrite existing song: {e}"))?;
    }
    fs::rename(&source, &target).map_err(|e| format!("Failed to rename song: {e}"))?;
    Ok(song_from_path(&target)?)
}

#[tauri::command]
pub async fn vrpiano_delete_song(app: tauri::AppHandle, song_path: String) -> Result<(), String> {
    let songs_dir = ensure_songs_dir(&app)?;
    let source = resolve_song_path(&songs_dir, &song_path)?;
    fs::remove_file(&source).map_err(|e| format!("Failed to delete song: {e}"))?;
    Ok(())
}

#[tauri::command]
pub async fn vrpiano_preview_song(app: tauri::AppHandle, song_path: String) -> Result<(), String> {
    let songs_dir = ensure_songs_dir(&app)?;
    let source = resolve_song_path(&songs_dir, &song_path)?;
    open_file(&source)
}

#[tauri::command]
pub async fn vrpiano_read_song_data(
    app: tauri::AppHandle,
    song_path: String,
) -> Result<VrpianoMidiData, String> {
    let songs_dir = ensure_songs_dir(&app)?;
    let source = resolve_song_path(&songs_dir, &song_path)?;
    let data = fs::read(&source).map_err(|e| format!("Failed to read MIDI: {e}"))?;
    let data = validate_midi_bytes(data)?;
    let name = source
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("Untitled")
        .to_string();
    Ok(VrpianoMidiData {
        name,
        data: base64::engine::general_purpose::STANDARD.encode(data),
    })
}

#[tauri::command]
pub async fn vrpiano_download_url(
    app: tauri::AppHandle,
    request: VrpianoDownloadRequest,
) -> Result<VrpianoSong, String> {
    let songs_dir = ensure_songs_dir(&app)?;
    let url = request.url.trim();
    if url.is_empty() {
        return Err("Please enter a MIDI URL or Midishow ID".to_string());
    }

    if is_midishow_input(url) {
        let midi_id = extract_midishow_id(url)?;
        return download_midishow_to_library(&app, &songs_dir, midi_id, request.filename, false)
            .await;
    }

    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("URL must start with http:// or https://".to_string());
    }

    let response = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) VRPiano/1.0")
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Failed to download MIDI: {e}"))?;
    if !response.status().is_success() {
        return Err(format!("Download failed with HTTP {}", response.status()));
    }
    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read download: {e}"))?
        .to_vec();
    let midi = validate_midi_bytes(bytes)?;
    let filename = request
        .filename
        .map(|name| sanitize_filename(&name))
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| filename_from_url(url));
    let target = unique_path(&songs_dir.join(ensure_midi_extension(filename)));
    write_midi_file(&target, &midi)?;
    Ok(song_from_path(&target)?)
}

#[tauri::command]
pub async fn vrpiano_search_midishow(
    keyword: String,
    max_results: Option<usize>,
) -> Result<Vec<VrpianoOnlineSong>, String> {
    let keyword = keyword.trim();
    if keyword.is_empty() {
        return Err("Please enter a search keyword".to_string());
    }
    let limit = max_results.unwrap_or(30).clamp(1, 50);
    search_midishow(keyword, limit).await
}

#[tauri::command]
pub async fn vrpiano_download_midishow(
    app: tauri::AppHandle,
    request: VrpianoMidishowDownloadRequest,
) -> Result<Option<VrpianoSong>, String> {
    let songs_dir = ensure_songs_dir(&app)?;
    if request.preview {
        let preview_dir = app
            .path()
            .app_cache_dir()
            .map_err(|e| e.to_string())?
            .join("vrpiano")
            .join("previews");
        fs::create_dir_all(&preview_dir)
            .map_err(|e| format!("Failed to create preview folder: {e}"))?;
        let title = request
            .title
            .clone()
            .map(|name| sanitize_filename(&name))
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| format!("MIDI_{}", request.midi_id));
        let path = unique_path(&preview_dir.join(ensure_midi_extension(title)));
        let data = download_midishow_bytes(&app, request.midi_id).await?;
        write_midi_file(&path, &data)?;
        open_file(&path)?;
        return Ok(None);
    }

    download_midishow_to_library(&app, &songs_dir, request.midi_id, request.title, false)
        .await
        .map(Some)
}

#[tauri::command]
pub async fn vrpiano_midishow_preview_data(
    app: tauri::AppHandle,
    request: VrpianoMidishowDownloadRequest,
) -> Result<VrpianoMidiData, String> {
    let data = download_midishow_bytes(&app, request.midi_id).await?;
    let title = request
        .title
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| {
            midishow_title(request.midi_id).unwrap_or_else(|| format!("MIDI_{}", request.midi_id))
        });
    Ok(VrpianoMidiData {
        name: title,
        data: base64::engine::general_purpose::STANDARD.encode(data),
    })
}

#[tauri::command]
pub async fn vrpiano_midishow_accounts(
    app: tauri::AppHandle,
) -> Result<Vec<VrpianoMidishowAccount>, String> {
    Ok(load_midishow_accounts(&app)?
        .into_iter()
        .map(|account| VrpianoMidishowAccount {
            username: account.username,
        })
        .collect())
}

#[tauri::command]
pub async fn vrpiano_midishow_login(
    app: tauri::AppHandle,
    request: VrpianoMidishowLoginRequest,
) -> Result<Vec<VrpianoMidishowAccount>, String> {
    let username = request.username.trim().to_string();
    if username.is_empty() || request.password.is_empty() {
        return Err("Please enter a Midishow username and password".to_string());
    }
    verify_midishow_login(&username, &request.password)?;
    let mut accounts = load_midishow_accounts(&app)?;
    if let Some(account) = accounts
        .iter_mut()
        .find(|account| account.username == username)
    {
        account.password = request.password;
    } else {
        accounts.push(StoredMidishowAccount {
            username,
            password: request.password,
        });
    }
    save_midishow_accounts(&app, &accounts)?;
    vrpiano_midishow_accounts(app).await
}

#[tauri::command]
pub async fn vrpiano_midishow_remove_account(
    app: tauri::AppHandle,
    username: String,
) -> Result<Vec<VrpianoMidishowAccount>, String> {
    let target = username.trim();
    let accounts = load_midishow_accounts(&app)?
        .into_iter()
        .filter(|account| account.username != target)
        .collect::<Vec<_>>();
    save_midishow_accounts(&app, &accounts)?;
    vrpiano_midishow_accounts(app).await
}

#[tauri::command]
pub async fn vrpiano_open_songs_dir(app: tauri::AppHandle) -> Result<(), String> {
    let songs_dir = ensure_songs_dir(&app)?;
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(songs_dir)
            .spawn()
            .map_err(|e| format!("Failed to open songs folder: {e}"))?;
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::process::Command::new("xdg-open")
            .arg(songs_dir)
            .spawn()
            .map_err(|e| format!("Failed to open songs folder: {e}"))?;
    }
    Ok(())
}

#[tauri::command]
pub async fn vrpiano_get_status(
    app: tauri::AppHandle,
    state: tauri::State<'_, VrpianoState>,
) -> Result<VrpianoStatus, String> {
    status_snapshot(&app, &state.inner)
}

#[tauri::command]
pub async fn vrpiano_start(
    app: tauri::AppHandle,
    state: tauri::State<'_, VrpianoState>,
    request: VrpianoStartRequest,
) -> Result<VrpianoStatus, String> {
    #[cfg(not(target_os = "windows"))]
    {
        let _ = request;
        let _ = app;
        let _ = state;
        return Err("VRPiano playback is currently supported on Windows only".to_string());
    }

    #[cfg(target_os = "windows")]
    {
        start_playback(app, state.inner.clone(), request)
    }
}

#[tauri::command]
pub async fn vrpiano_stop(
    app: tauri::AppHandle,
    state: tauri::State<'_, VrpianoState>,
) -> Result<VrpianoStatus, String> {
    stop_playback(app, state.inner.clone())
}

#[tauri::command]
pub async fn vrpiano_set_speed(
    app: tauri::AppHandle,
    state: tauri::State<'_, VrpianoState>,
    speed: f64,
) -> Result<VrpianoStatus, String> {
    set_playback_speed(app, state.inner.clone(), speed)
}

#[tauri::command]
pub async fn vrpiano_set_hotkeys(
    app: tauri::AppHandle,
    state: tauri::State<'_, VrpianoState>,
    config: VrpianoHotkeyConfig,
) -> Result<VrpianoStatus, String> {
    set_hotkeys(app, state.inner.clone(), config)
}

fn start_playback(
    app: tauri::AppHandle,
    state: Arc<Mutex<VrpianoRuntime>>,
    request: VrpianoStartRequest,
) -> Result<VrpianoStatus, String> {
    #[cfg(not(target_os = "windows"))]
    {
        let _ = request;
        let _ = app;
        let _ = state;
        return Err("VRPiano playback is currently supported on Windows only".to_string());
    }

    #[cfg(target_os = "windows")]
    {
        let song_path = PathBuf::from(request.song_path.trim());
        if !song_path.exists() || !song_path.is_file() || !is_midi_file(&song_path) {
            return Err("Please choose a valid MIDI file from the VRPiano library".to_string());
        }

        let speed = normalize_speed(request.speed);
        let (events, duration_ms) = parse_midi_events(&song_path)?;
        if events.is_empty() {
            return Err("This MIDI has no notes that can be mapped to VRPiano keys".to_string());
        }

        let song_name = song_path
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or("Untitled")
            .to_string();

        let stop_flag = Arc::new(AtomicBool::new(false));
        {
            let mut runtime = state
                .lock()
                .map_err(|_| "VRPiano state lock poisoned".to_string())?;
            if runtime.status.running {
                return Err("VRPiano is already playing".to_string());
            }
            runtime.stop = Some(stop_flag.clone());
            if let Ok(mut current) = runtime.speed.lock() {
                *current = speed;
            }
            runtime.status = VrpianoStatus {
                running: true,
                song_name: song_name.clone(),
                progress: 0.0,
                played_notes: 0,
                total_notes: events.len(),
                duration_ms,
                elapsed_ms: 0,
                last_event: format!("Starting after {}s delay", request.delay_secs),
                last_error: String::new(),
                songs_dir: ensure_songs_dir(&app)?.to_string_lossy().to_string(),
                speed,
                hotkeys_enabled: runtime.hotkeys_enabled,
                hotkeys_available: cfg!(target_os = "windows"),
            };
        }

        emit_status(&app, &state);
        let app_handle = app.clone();
        let state_inner = state.clone();
        thread::spawn(move || {
            run_playback(
                app_handle,
                state_inner,
                stop_flag,
                song_name,
                events,
                duration_ms,
                request.delay_secs,
            );
        });

        status_snapshot(&app, &state)
    }
}

fn stop_playback(
    app: tauri::AppHandle,
    state: Arc<Mutex<VrpianoRuntime>>,
) -> Result<VrpianoStatus, String> {
    {
        let mut runtime = state
            .lock()
            .map_err(|_| "VRPiano state lock poisoned".to_string())?;
        if let Some(stop) = &runtime.stop {
            stop.store(true, Ordering::SeqCst);
            runtime.status.last_event = "Stopping playback".to_string();
        } else {
            runtime.status.last_event = "Playback already stopped".to_string();
        }
    }
    emit_status(&app, &state);
    status_snapshot(&app, &state)
}

fn set_playback_speed(
    app: tauri::AppHandle,
    state: Arc<Mutex<VrpianoRuntime>>,
    speed: f64,
) -> Result<VrpianoStatus, String> {
    let next_speed = normalize_speed(speed);
    {
        let mut runtime = state
            .lock()
            .map_err(|_| "VRPiano state lock poisoned".to_string())?;
        if let Ok(mut current) = runtime.speed.lock() {
            *current = next_speed;
        }
        runtime.status.speed = next_speed;
        runtime.status.last_event = format!("Playback speed {:.2}x", next_speed);
    }
    emit_status(&app, &state);
    status_snapshot(&app, &state)
}

fn set_hotkeys(
    app: tauri::AppHandle,
    state: Arc<Mutex<VrpianoRuntime>>,
    config: VrpianoHotkeyConfig,
) -> Result<VrpianoStatus, String> {
    #[cfg(target_os = "windows")]
    if config.enabled {
        start_global_hotkey_hook(app.clone(), state.clone())?;
    }

    #[cfg(not(target_os = "windows"))]
    if config.enabled {
        return Err("Global hotkeys are currently supported on Windows only".to_string());
    }

    let speed = normalize_speed(config.speed);
    {
        let mut runtime = state
            .lock()
            .map_err(|_| "VRPiano state lock poisoned".to_string())?;
        runtime.hotkeys_enabled = config.enabled;
        runtime.hotkey_song_path = config.song_path.trim().to_string();
        runtime.hotkey_delay_secs = config.delay_secs.min(60);
        if let Ok(mut current) = runtime.speed.lock() {
            *current = speed;
        }
        runtime.status.hotkeys_enabled = config.enabled;
        runtime.status.hotkeys_available = cfg!(target_os = "windows");
        runtime.status.speed = speed;
        runtime.status.last_event = if config.enabled {
            "Global VRPiano hotkeys enabled".to_string()
        } else {
            "Global VRPiano hotkeys disabled".to_string()
        };
    }
    emit_status(&app, &state);
    status_snapshot(&app, &state)
}

#[cfg(target_os = "windows")]
fn start_global_hotkey_hook(
    app: tauri::AppHandle,
    state: Arc<Mutex<VrpianoRuntime>>,
) -> Result<(), String> {
    let _ = HOTKEY_CONTEXT.set(GlobalHotkeyContext { app, state });

    if HOTKEY_HOOK_STARTED
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Ok(());
    }

    let (tx, rx) = mpsc::channel();
    thread::Builder::new()
        .name("vrpiano-global-hotkeys".to_string())
        .spawn(move || {
            use windows::Win32::Foundation::{HINSTANCE, HWND};
            use windows::Win32::UI::WindowsAndMessaging::{
                GetMessageW, SetWindowsHookExW, MSG, WH_KEYBOARD_LL,
            };

            let hook = unsafe {
                SetWindowsHookExW(
                    WH_KEYBOARD_LL,
                    Some(vrpiano_keyboard_proc),
                    HINSTANCE::default(),
                    0,
                )
            };
            match hook {
                Ok(_hook) => {
                    let _ = tx.send(Ok(()));
                    let mut msg = MSG::default();
                    while unsafe { GetMessageW(&mut msg, HWND::default(), 0, 0) }.as_bool() {}
                }
                Err(err) => {
                    HOTKEY_HOOK_STARTED.store(false, Ordering::SeqCst);
                    let _ = tx.send(Err(format!(
                        "Failed to install VRPiano global hotkeys: {err}"
                    )));
                }
            }
        })
        .map_err(|e| format!("Failed to start VRPiano hotkey thread: {e}"))?;

    rx.recv_timeout(Duration::from_secs(2))
        .map_err(|_| "Timed out while enabling VRPiano global hotkeys".to_string())?
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn vrpiano_keyboard_proc(
    ncode: i32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
) -> windows::Win32::Foundation::LRESULT {
    use windows::Win32::Foundation::LRESULT;
    use windows::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, KBDLLHOOKSTRUCT, LLKHF_INJECTED, LLKHF_UP, WM_KEYDOWN, WM_SYSKEYDOWN,
    };

    if ncode < 0 {
        return unsafe { CallNextHookEx(None, ncode, wparam, lparam) };
    }

    let message = wparam.0 as u32;
    if message != WM_KEYDOWN && message != WM_SYSKEYDOWN {
        return unsafe { CallNextHookEx(None, ncode, wparam, lparam) };
    }

    let key = unsafe { *(lparam.0 as *const KBDLLHOOKSTRUCT) };
    if key.flags.contains(LLKHF_INJECTED) || key.flags.contains(LLKHF_UP) {
        return unsafe { CallNextHookEx(None, ncode, wparam, lparam) };
    }

    let vk = key.vkCode;
    if !(112..=116).contains(&vk) {
        return unsafe { CallNextHookEx(None, ncode, wparam, lparam) };
    }

    let Some(context) = HOTKEY_CONTEXT.get().cloned() else {
        return unsafe { CallNextHookEx(None, ncode, wparam, lparam) };
    };

    let enabled = context
        .state
        .lock()
        .map(|runtime| runtime.hotkeys_enabled)
        .unwrap_or(false);
    if !enabled {
        return unsafe { CallNextHookEx(None, ncode, wparam, lparam) };
    }

    dispatch_hotkey(context, vk);
    LRESULT(1)
}

#[cfg(target_os = "windows")]
fn dispatch_hotkey(context: GlobalHotkeyContext, vk: u32) {
    thread::spawn(move || match vk {
        112 => {
            let request = match context.state.lock() {
                Ok(runtime) => VrpianoStartRequest {
                    song_path: runtime.hotkey_song_path.clone(),
                    delay_secs: runtime.hotkey_delay_secs,
                    speed: current_speed(&context.state),
                },
                Err(_) => return,
            };
            let _ = start_playback(context.app, context.state, request);
        }
        113 => {
            let _ = stop_playback(context.app, context.state);
        }
        114 => {
            let next = current_speed(&context.state) + SPEED_STEP;
            let _ = set_playback_speed(context.app, context.state, next);
        }
        115 => {
            let next = current_speed(&context.state) - SPEED_STEP;
            let _ = set_playback_speed(context.app, context.state, next);
        }
        116 => {
            let _ = set_playback_speed(context.app, context.state, 1.0);
        }
        _ => {}
    });
}

#[cfg(target_os = "windows")]
fn run_playback(
    app: tauri::AppHandle,
    state: Arc<Mutex<VrpianoRuntime>>,
    stop: Arc<AtomicBool>,
    song_name: String,
    events: Vec<PlayEvent>,
    duration_ms: u64,
    delay_secs: u64,
) {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        for remaining in (1..=delay_secs).rev() {
            if stop.load(Ordering::SeqCst) {
                return;
            }
            update_runtime(&state, |status| {
                status.last_event = format!("Starting in {remaining}s");
            });
            emit_status(&app, &state);
            thread::sleep(Duration::from_secs(1));
        }

        let mut active_keys = HashSet::new();
        let mut last_at = 0_u64;
        let mut played = 0_usize;
        let mut index = 0_usize;

        while index < events.len() {
            if stop.load(Ordering::SeqCst) {
                break;
            }

            let at_ms = events[index].at_ms;
            let wait_ms = at_ms.saturating_sub(last_at);
            sleep_scaled_interruptible(wait_ms, &stop, &state);
            if stop.load(Ordering::SeqCst) {
                break;
            }

            release_all(&active_keys);
            active_keys.clear();

            while index < events.len() && events[index].at_ms == at_ms {
                send_key(events[index].vk, false);
                active_keys.insert(events[index].vk);
                played += 1;
                index += 1;
            }

            thread::sleep(Duration::from_millis(NOTE_HOLD_MS));
            let playback_speed = current_speed(&state);
            update_runtime(&state, |status| {
                status.elapsed_ms = at_ms;
                status.played_notes = played;
                status.progress = if duration_ms == 0 {
                    1.0
                } else {
                    (at_ms as f64 / duration_ms as f64).clamp(0.0, 1.0)
                };
                status.speed = playback_speed;
                status.last_event = format!("Playing {} at {:.2}x", song_name, status.speed);
            });
            emit_status(&app, &state);
            last_at = at_ms;
        }

        release_all(&active_keys);
    }));

    if result.is_err() {
        update_runtime(&state, |status| {
            status.last_error = "Playback crashed unexpectedly".to_string();
        });
    }

    update_runtime(&state, |status| {
        status.running = false;
        status.progress = if stop.load(Ordering::SeqCst) {
            status.progress
        } else {
            1.0
        };
        status.last_event = if stop.load(Ordering::SeqCst) {
            "Playback stopped".to_string()
        } else {
            "Playback finished".to_string()
        };
    });
    if let Ok(mut runtime) = state.lock() {
        runtime.stop = None;
    }
    emit_status(&app, &state);
}

#[cfg(target_os = "windows")]
fn send_key(vk: u16, key_up: bool) {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP,
        VIRTUAL_KEY,
    };

    let flags = if key_up {
        KEYEVENTF_KEYUP
    } else {
        KEYBD_EVENT_FLAGS(0)
    };
    let input = INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(vk),
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    unsafe {
        SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
    }
}

fn parse_midi_events(path: &Path) -> Result<(Vec<PlayEvent>, u64), String> {
    let bytes = fs::read(path).map_err(|e| format!("Failed to read MIDI: {e}"))?;
    let smf = Smf::parse(&bytes).map_err(|e| format!("Invalid MIDI file: {e}"))?;
    let ticks_per_beat = match smf.header.timing {
        Timing::Metrical(ticks) => u64::from(ticks.as_int()),
        Timing::Timecode(_, _) => {
            return Err("SMPTE timecode MIDI files are not supported yet".to_string())
        }
    };

    let tempo_map = collect_tempo_map(&smf);
    let mut grouped: BTreeMap<u64, Vec<PlayEvent>> = BTreeMap::new();

    for track in &smf.tracks {
        let mut tick = 0_u64;
        for event in track {
            tick = tick.saturating_add(u64::from(event.delta.as_int()));
            if let TrackEventKind::Midi {
                message: MidiMessage::NoteOn { key, vel },
                ..
            } = event.kind
            {
                if vel.as_int() == 0 {
                    continue;
                }
                let note = key.as_int();
                if let Some(vk) = note_to_vk(note) {
                    let micros = tick_to_micros(tick, &tempo_map, ticks_per_beat);
                    let at_ms = (micros as f64 / 1000.0).round().max(0.0) as u64;
                    grouped
                        .entry(at_ms)
                        .or_default()
                        .push(PlayEvent { at_ms, note, vk });
                }
            }
        }
    }

    let mut events = Vec::new();
    for (_at, mut group) in grouped {
        group.sort_by_key(|event| event.note);
        events.extend(group);
    }
    let duration_ms = events.last().map(|event| event.at_ms).unwrap_or(0);
    Ok((events, duration_ms))
}

fn collect_tempo_map(smf: &Smf<'_>) -> Vec<(u64, u64)> {
    let mut tempos = vec![(0, 500_000)];
    for track in &smf.tracks {
        let mut tick = 0_u64;
        for event in track {
            tick = tick.saturating_add(u64::from(event.delta.as_int()));
            if let TrackEventKind::Meta(MetaMessage::Tempo(tempo)) = event.kind {
                tempos.push((tick, u64::from(tempo.as_int())));
            }
        }
    }
    tempos.sort_by_key(|(tick, _)| *tick);
    tempos.dedup_by_key(|(tick, _)| *tick);
    tempos
}

fn tick_to_micros(tick: u64, tempo_map: &[(u64, u64)], ticks_per_beat: u64) -> u64 {
    let mut micros = 0_u64;
    let mut last_tick = 0_u64;
    let mut tempo = 500_000_u64;

    for &(tempo_tick, next_tempo) in tempo_map {
        if tempo_tick == 0 {
            tempo = next_tempo;
            continue;
        }
        if tempo_tick > tick {
            break;
        }
        micros = micros.saturating_add(
            tempo_tick.saturating_sub(last_tick).saturating_mul(tempo) / ticks_per_beat.max(1),
        );
        last_tick = tempo_tick;
        tempo = next_tempo;
    }

    micros.saturating_add(
        tick.saturating_sub(last_tick).saturating_mul(tempo) / ticks_per_beat.max(1),
    )
}

fn note_to_vk(note: u8) -> Option<u16> {
    let key = match note {
        36 => "z",
        37 => ",",
        38 => "x",
        39 => ".",
        40 => "c",
        41 => "v",
        42 => "/",
        43 => "b",
        44 => "b0",
        45 => "n",
        46 => "b.",
        47 => "m",
        48 => "a",
        49 => "k",
        50 => "s",
        51 => "l",
        52 => "d",
        53 => "f",
        54 => ";",
        55 => "g",
        56 => "b2",
        57 => "h",
        58 => "b3",
        59 => "j",
        60 => "q",
        61 => "i",
        62 => "w",
        63 => "o",
        64 => "e",
        65 => "r",
        66 => "p",
        67 => "t",
        68 => "b5",
        69 => "y",
        70 => "b6",
        71 => "u",
        72 => "1",
        73 => "8",
        74 => "2",
        75 => "9",
        76 => "3",
        77 => "4",
        78 => "0",
        79 => "5",
        80 => "b8",
        81 => "6",
        82 => "b9",
        83 => "7",
        84 => "F1",
        85 => "F8",
        86 => "F2",
        87 => "F9",
        88 => "F3",
        89 => "F4",
        90 => "F10",
        91 => "F5",
        92 => "b/",
        93 => "F6",
        94 => "b*",
        95 => "F7",
        _ => return None,
    };
    key_to_vk(key)
}

fn key_to_vk(key: &str) -> Option<u16> {
    Some(match key {
        "0" => 48,
        "1" => 49,
        "2" => 50,
        "3" => 51,
        "4" => 52,
        "5" => 53,
        "6" => 54,
        "7" => 55,
        "8" => 56,
        "9" => 57,
        "a" => 65,
        "b" => 66,
        "c" => 67,
        "d" => 68,
        "e" => 69,
        "f" => 70,
        "g" => 71,
        "h" => 72,
        "i" => 73,
        "j" => 74,
        "k" => 75,
        "l" => 76,
        "m" => 77,
        "n" => 78,
        "o" => 79,
        "p" => 80,
        "q" => 81,
        "r" => 82,
        "s" => 83,
        "t" => 84,
        "u" => 85,
        "v" => 86,
        "w" => 87,
        "x" => 88,
        "y" => 89,
        "z" => 90,
        "," => 188,
        "." => 190,
        "/" => 191,
        ";" => 186,
        "F1" => 112,
        "F2" => 113,
        "F3" => 114,
        "F4" => 115,
        "F5" => 116,
        "F6" => 117,
        "F7" => 118,
        "F8" => 119,
        "F9" => 120,
        "F10" => 121,
        "b0" => 96,
        "b." => 110,
        "b2" => 98,
        "b3" => 99,
        "b5" => 101,
        "b6" => 102,
        "b8" => 104,
        "b9" => 105,
        "b/" => 111,
        "b*" => 106,
        _ => return None,
    })
}

fn sleep_scaled_interruptible(
    music_ms: u64,
    stop: &AtomicBool,
    state: &Arc<Mutex<VrpianoRuntime>>,
) {
    let mut remaining = music_ms as f64;
    while remaining > 0.0 && !stop.load(Ordering::SeqCst) {
        let speed = current_speed(state).max(0.25);
        let real_chunk = (remaining / speed).ceil().clamp(5.0, 20.0) as u64;
        thread::sleep(Duration::from_millis(real_chunk));
        remaining -= real_chunk as f64 * speed;
    }
}

fn normalize_speed(speed: f64) -> f64 {
    if speed.is_finite() {
        (speed.clamp(0.25, 3.0) * 100.0).round() / 100.0
    } else {
        1.0
    }
}

fn current_speed(state: &Arc<Mutex<VrpianoRuntime>>) -> f64 {
    state
        .lock()
        .ok()
        .and_then(|runtime| runtime.speed.lock().ok().map(|speed| *speed))
        .unwrap_or(1.0)
}

#[cfg(target_os = "windows")]
fn release_all(keys: &HashSet<u16>) {
    for &vk in keys {
        send_key(vk, true);
    }
}

fn update_runtime(state: &Arc<Mutex<VrpianoRuntime>>, update: impl FnOnce(&mut VrpianoStatus)) {
    if let Ok(mut runtime) = state.lock() {
        update(&mut runtime.status);
    }
}

fn emit_status(app: &tauri::AppHandle, state: &Arc<Mutex<VrpianoRuntime>>) {
    if let Ok(runtime) = state.lock() {
        let mut status = runtime.status.clone();
        status.speed = runtime
            .speed
            .lock()
            .map(|speed| *speed)
            .unwrap_or(status.speed);
        status.hotkeys_enabled = runtime.hotkeys_enabled;
        status.hotkeys_available = cfg!(target_os = "windows");
        let _ = app.emit("vrpiano_status", status);
    }
}

fn status_snapshot(
    app: &tauri::AppHandle,
    state: &Arc<Mutex<VrpianoRuntime>>,
) -> Result<VrpianoStatus, String> {
    let mut status = state
        .lock()
        .map_err(|_| "VRPiano state lock poisoned".to_string())?
        .status
        .clone();
    status.songs_dir = ensure_songs_dir(app)?.to_string_lossy().to_string();
    status.speed = current_speed(state);
    if let Ok(runtime) = state.lock() {
        status.hotkeys_enabled = runtime.hotkeys_enabled;
        status.hotkeys_available = cfg!(target_os = "windows");
    }
    Ok(status)
}

fn status_with_dir(app: &tauri::AppHandle, event: &str) -> Result<VrpianoStatus, String> {
    Ok(VrpianoStatus {
        running: false,
        song_name: String::new(),
        progress: 0.0,
        played_notes: 0,
        total_notes: 0,
        duration_ms: 0,
        elapsed_ms: 0,
        last_event: event.to_string(),
        last_error: String::new(),
        songs_dir: ensure_songs_dir(app)?.to_string_lossy().to_string(),
        speed: 1.0,
        hotkeys_enabled: false,
        hotkeys_available: cfg!(target_os = "windows"),
    })
}

fn resolve_song_path(songs_dir: &Path, song_path: &str) -> Result<PathBuf, String> {
    let candidate = PathBuf::from(song_path.trim());
    if !candidate.exists() || !candidate.is_file() || !is_midi_file(&candidate) {
        return Err("Please choose a valid MIDI song".to_string());
    }
    let songs_root = songs_dir
        .canonicalize()
        .map_err(|e| format!("Failed to resolve songs folder: {e}"))?;
    let canonical = candidate
        .canonicalize()
        .map_err(|e| format!("Failed to resolve song path: {e}"))?;
    if !canonical.starts_with(&songs_root) {
        return Err("Song is outside the VRPiano library".to_string());
    }
    Ok(canonical)
}

fn ensure_midi_extension(mut filename: String) -> String {
    let lower = filename.to_lowercase();
    if !lower.ends_with(".mid") && !lower.ends_with(".midi") {
        filename.push_str(".mid");
    }
    filename
}

fn filename_from_url(url: &str) -> String {
    let base = url
        .split('?')
        .next()
        .and_then(|value| value.rsplit('/').next())
        .map(sanitize_filename)
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "downloaded.mid".to_string());
    ensure_midi_extension(base)
}

fn validate_midi_bytes(data: Vec<u8>) -> Result<Vec<u8>, String> {
    if data.is_empty() {
        return Err("Download result is empty".to_string());
    }
    let head = String::from_utf8_lossy(&data[..data.len().min(512)]).to_lowercase();
    let trimmed = head.trim_start();
    if trimmed.starts_with("<!doctype")
        || trimmed.starts_with("<html")
        || trimmed.starts_with("<?xml")
        || head.contains("<html")
    {
        return Err("Download result is a web page, not a MIDI file".to_string());
    }
    if data.starts_with(b"MThd") {
        return Ok(data);
    }
    if let Some(pos) = data[..data.len().min(128)]
        .windows(4)
        .position(|chunk| chunk == b"MThd")
    {
        return Ok(data[pos..].to_vec());
    }
    Err("File header is not MThd, so it cannot be read as MIDI".to_string())
}

fn write_midi_file(path: &Path, data: &[u8]) -> Result<(), String> {
    let valid = validate_midi_bytes(data.to_vec())?;
    let mut file = fs::File::create(path).map_err(|e| format!("Failed to save MIDI: {e}"))?;
    file.write_all(&valid)
        .map_err(|e| format!("Failed to write MIDI: {e}"))?;
    Ok(())
}

fn is_midishow_input(input: &str) -> bool {
    input.contains("midishow.com") || input.chars().all(|ch| ch.is_ascii_digit())
}

fn extract_midishow_id(input: &str) -> Result<u64, String> {
    let re = regex::Regex::new(r"(\d+)").map_err(|e| e.to_string())?;
    re.captures(input)
        .and_then(|captures| captures.get(1))
        .and_then(|id| id.as_str().parse::<u64>().ok())
        .ok_or_else(|| "Invalid Midishow ID or URL".to_string())
}

async fn search_midishow(keyword: &str, limit: usize) -> Result<Vec<VrpianoOnlineSong>, String> {
    match run_midishow_cli_json(&["search", keyword]) {
        Ok(value) => parse_midishow_results(value, limit),
        Err(cli_error) => {
            let fallback = fallback_midishow_search(keyword, limit).await;
            fallback.map_err(|fallback_error| {
                format!("{fallback_error}; CLI fallback also failed: {cli_error}")
            })
        }
    }
}

fn parse_midishow_results(
    value: serde_json::Value,
    limit: usize,
) -> Result<Vec<VrpianoOnlineSong>, String> {
    let items = value
        .as_array()
        .ok_or_else(|| "Midishow search returned an unexpected response".to_string())?;
    let mut results = Vec::new();
    let mut seen = HashSet::new();
    for item in items {
        let id = item.get("id").and_then(|value| value.as_u64()).unwrap_or(0);
        if id == 0 || !seen.insert(id) {
            continue;
        }
        let title = item
            .get("title")
            .and_then(|value| value.as_str())
            .map(clean_midishow_text)
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| format!("MIDI #{id}"));
        let artist = item
            .get("artist")
            .and_then(|value| value.as_str())
            .map(clean_midishow_text)
            .unwrap_or_default();
        let page_url = item
            .get("page_url")
            .and_then(|value| value.as_str())
            .map(str::to_string)
            .unwrap_or_else(|| format!("https://www.midishow.com/en/midi/{id}.html"));
        results.push(VrpianoOnlineSong {
            id,
            title,
            artist,
            page_url,
        });
        if results.len() >= limit {
            break;
        }
    }
    Ok(results)
}

async fn fallback_midishow_search(
    keyword: &str,
    limit: usize,
) -> Result<Vec<VrpianoOnlineSong>, String> {
    let html = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) VRPiano/1.0")
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|e| e.to_string())?
        .get("https://www.midishow.com/search/result")
        .query(&[("q", keyword), ("page", "1"), ("per-page", "50")])
        .send()
        .await
        .map_err(|e| format!("Midishow search failed: {e}"))?
        .text()
        .await
        .map_err(|e| format!("Failed to read Midishow response: {e}"))?;

    let re = regex::Regex::new(r#"/en/midi/(\d+)\.html"#).map_err(|e| e.to_string())?;
    let mut results = Vec::new();
    let mut seen = HashSet::new();
    for captures in re.captures_iter(&html) {
        let Some(id) = captures
            .get(1)
            .and_then(|value| value.as_str().parse::<u64>().ok())
        else {
            continue;
        };
        if !seen.insert(id) {
            continue;
        }
        results.push(VrpianoOnlineSong {
            id,
            title: format!("MIDI #{id}"),
            artist: String::new(),
            page_url: format!("https://www.midishow.com/en/midi/{id}.html"),
        });
        if results.len() >= limit {
            break;
        }
    }
    Ok(results)
}

async fn download_midishow_to_library(
    app: &tauri::AppHandle,
    songs_dir: &Path,
    midi_id: u64,
    title: Option<String>,
    _overwrite: bool,
) -> Result<VrpianoSong, String> {
    let data = download_midishow_bytes(app, midi_id).await?;
    let title = title
        .map(|value| sanitize_filename(&value))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| midishow_title(midi_id).unwrap_or_else(|| format!("MIDI_{midi_id}")));
    let target = unique_path(&songs_dir.join(ensure_midi_extension(title)));
    write_midi_file(&target, &data)?;
    song_from_path(&target)
}

async fn download_midishow_bytes(app: &tauri::AppHandle, midi_id: u64) -> Result<Vec<u8>, String> {
    if let Some(account) = default_midishow_account(app)? {
        match download_midishow_with_python(midi_id, &account) {
            Ok((data, _title)) => return validate_midi_bytes(data),
            Err(account_error) => {
                let cli_result = download_midishow_bytes_with_cli(midi_id);
                return cli_result.map_err(|cli_error| {
                    format!("Midishow account download failed: {account_error}; CLI fallback failed: {cli_error}")
                });
            }
        }
    }
    download_midishow_bytes_with_cli(midi_id)
}

fn download_midishow_bytes_with_cli(midi_id: u64) -> Result<Vec<u8>, String> {
    let value = run_midishow_cli_json(&["download", &midi_id.to_string()])?;
    let encoded = value
        .get("data")
        .and_then(|value| value.as_str())
        .ok_or_else(|| "Midishow download did not return MIDI data".to_string())?;
    let data = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|e| format!("Failed to decode Midishow MIDI: {e}"))?;
    validate_midi_bytes(data)
}

fn load_midishow_accounts(app: &tauri::AppHandle) -> Result<Vec<StoredMidishowAccount>, String> {
    let path = midishow_accounts_path(app)?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let text =
        fs::read_to_string(&path).map_err(|e| format!("Failed to read Midishow accounts: {e}"))?;
    serde_json::from_str(&text).map_err(|e| format!("Failed to parse Midishow accounts: {e}"))
}

fn save_midishow_accounts(
    app: &tauri::AppHandle,
    accounts: &[StoredMidishowAccount],
) -> Result<(), String> {
    let path = midishow_accounts_path(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create account folder: {e}"))?;
    }
    let text = serde_json::to_string_pretty(accounts).map_err(|e| e.to_string())?;
    fs::write(path, text).map_err(|e| format!("Failed to save Midishow accounts: {e}"))
}

fn midishow_accounts_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("vrpiano")
        .join("midishow_accounts.json"))
}

fn default_midishow_account(
    app: &tauri::AppHandle,
) -> Result<Option<StoredMidishowAccount>, String> {
    Ok(load_midishow_accounts(app)?.into_iter().next())
}

fn verify_midishow_login(username: &str, password: &str) -> Result<(), String> {
    let script = r#"
import json, sys
sys.path.insert(0, sys.argv[1])
from midishow import login_midi
ok = login_midi(sys.argv[2], sys.argv[3])
print(json.dumps({"success": bool(ok)}, ensure_ascii=False))
"#;
    let value = run_python_json(
        script,
        &[SOURCE_PROJECT_PATH, username, password],
        Duration::from_secs(45),
    )?;
    if value
        .get("success")
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
    {
        Ok(())
    } else {
        Err("Midishow login failed".to_string())
    }
}

fn download_midishow_with_python(
    midi_id: u64,
    account: &StoredMidishowAccount,
) -> Result<(Vec<u8>, String), String> {
    let url = format!("https://www.midishow.com/en/midi/{midi_id}.html");
    let script = r#"
import base64, json, sys
sys.path.insert(0, sys.argv[1])
from midishow_api import get_account_manager, download_midi_url
mgr = get_account_manager()
mgr.add_account(sys.argv[2], sys.argv[3])
data, title = download_midi_url(sys.argv[4], sys.argv[2])
print(json.dumps({
    "title": title or "",
    "data": base64.b64encode(data).decode("ascii")
}, ensure_ascii=False))
"#;
    let value = run_python_json(
        script,
        &[
            SOURCE_PROJECT_PATH,
            &account.username,
            &account.password,
            &url,
        ],
        Duration::from_secs(60),
    )?;
    let encoded = value
        .get("data")
        .and_then(|value| value.as_str())
        .ok_or_else(|| "Python Midishow downloader did not return MIDI data".to_string())?;
    let title = value
        .get("title")
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .to_string();
    let data = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|e| format!("Failed to decode Python Midishow MIDI: {e}"))?;
    Ok((data, title))
}

fn run_python_json(
    script: &str,
    args: &[&str],
    timeout: Duration,
) -> Result<serde_json::Value, String> {
    let mut last_error = String::new();
    for executable in ["python", "py"] {
        let mut process_args = vec!["-c", script];
        process_args.extend_from_slice(args);
        match run_process_json(executable, &process_args, timeout) {
            Ok(value) => return Ok(value),
            Err(error) => last_error = error,
        }
    }
    Err(format!(
        "Python unavailable or Midishow Python bridge failed: {last_error}"
    ))
}

fn run_process_json(
    executable: &str,
    args: &[&str],
    timeout: Duration,
) -> Result<serde_json::Value, String> {
    let mut child = std::process::Command::new(executable)
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to start {executable}: {e}"))?;
    let started = std::time::Instant::now();
    loop {
        if child
            .try_wait()
            .map_err(|e| format!("Failed to wait for {executable}: {e}"))?
            .is_some()
        {
            break;
        }
        if started.elapsed() > timeout {
            let _ = child.kill();
            return Err(format!("{executable} request timed out"));
        }
        thread::sleep(Duration::from_millis(80));
    }
    let output = child
        .wait_with_output()
        .map_err(|e| format!("Failed to read {executable} output: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            format!("{executable} exited with {}", output.status)
        } else {
            stderr
        });
    }
    serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("Failed to parse {executable} JSON response: {e}"))
}

fn midishow_title(midi_id: u64) -> Option<String> {
    let value = run_midishow_cli_json(&["info", &midi_id.to_string()]).ok()?;
    value
        .get("title")
        .and_then(|value| value.as_str())
        .map(sanitize_filename)
        .filter(|value| !value.is_empty())
}

fn run_midishow_cli_json(args: &[&str]) -> Result<serde_json::Value, String> {
    let cli = Path::new(SOURCE_PROJECT_PATH)
        .join("midishow-downloader")
        .join("dist")
        .join("cli.js");
    if !cli.exists() {
        return Err(format!("Midishow CLI not found: {}", cli.display()));
    }
    let mut child = std::process::Command::new("node")
        .arg(&cli)
        .args(args)
        .current_dir(
            cli.parent()
                .unwrap_or_else(|| Path::new(SOURCE_PROJECT_PATH)),
        )
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to run Midishow CLI. Please install Node.js: {e}"))?;
    let started = std::time::Instant::now();
    loop {
        if child
            .try_wait()
            .map_err(|e| format!("Failed to wait for Midishow CLI: {e}"))?
            .is_some()
        {
            break;
        }
        if started.elapsed() > Duration::from_secs(45) {
            let _ = child.kill();
            return Err("Midishow request timed out".to_string());
        }
        thread::sleep(Duration::from_millis(80));
    }
    let output = child
        .wait_with_output()
        .map_err(|e| format!("Failed to read Midishow CLI output: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(stderr.trim()) {
            if let Some(error) = value.get("error").and_then(|value| value.as_str()) {
                return Err(error.to_string());
            }
        }
        return Err(stderr.trim().to_string());
    }
    serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("Failed to parse Midishow CLI response: {e}"))
}

fn clean_midishow_text(value: &str) -> String {
    let text = value.split_whitespace().collect::<Vec<_>>().join(" ");
    let re = regex::Regex::new(
        r"\s*[-|·]\s*|上传于|下载|评分|\d+\.\d+\s*\(|\d+(?:\.\d+)?\s*(?:KB|MB)|\bGM\d*\b",
    )
    .ok();
    let cleaned = re
        .as_ref()
        .and_then(|re| re.split(&text).next())
        .unwrap_or(&text)
        .trim();
    cleaned.chars().take(120).collect()
}

fn open_file(path: &Path) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", ""])
            .arg(path)
            .spawn()
            .map_err(|e| format!("Failed to open MIDI preview: {e}"))?;
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::process::Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map_err(|e| format!("Failed to open MIDI preview: {e}"))?;
    }
    Ok(())
}

fn ensure_songs_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("vrpiano")
        .join("songs");
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create songs folder: {e}"))?;
    Ok(dir)
}

fn import_seed_songs(songs_dir: &Path) -> Result<(), String> {
    let source = Path::new(SOURCE_PROJECT_PATH).join("songs");
    if !source.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(source).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if !is_midi_file(&path) {
            continue;
        }
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let target = songs_dir.join(sanitize_filename(name));
        let should_copy = match (fs::metadata(&path), fs::metadata(&target)) {
            (Ok(source_meta), Ok(target_meta)) => {
                source_meta.len() != target_meta.len()
                    || source_meta.modified().ok() > target_meta.modified().ok()
            }
            (Ok(_), Err(_)) => true,
            _ => false,
        };
        if should_copy {
            let _ = fs::copy(&path, target);
        }
    }
    Ok(())
}

fn song_from_path(path: &Path) -> Result<VrpianoSong, String> {
    let metadata = fs::metadata(path).map_err(|e| e.to_string())?;
    let modified_ms = metadata
        .modified()
        .unwrap_or(SystemTime::UNIX_EPOCH)
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    let name = path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("Untitled")
        .to_string();
    Ok(VrpianoSong {
        id: path.to_string_lossy().to_string(),
        name,
        path: path.to_string_lossy().to_string(),
        size: metadata.len(),
        modified_ms,
    })
}

fn is_midi_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| matches!(ext.to_lowercase().as_str(), "mid" | "midi"))
        .unwrap_or(false)
}

fn sanitize_filename(input: &str) -> String {
    input
        .chars()
        .map(|ch| match ch {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            ch if ch.is_control() => '_',
            ch => ch,
        })
        .collect::<String>()
        .trim_matches([' ', '.'])
        .chars()
        .take(140)
        .collect()
}

fn unique_path(path: &Path) -> PathBuf {
    if !path.exists() {
        return path.to_path_buf();
    }
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("song");
    let ext = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("mid");
    for index in 1..1000 {
        let candidate = parent.join(format!("{stem}_{index}.{ext}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    parent.join(format!("{stem}_copy.{ext}"))
}
