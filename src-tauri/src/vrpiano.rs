use base64::Engine;
use midly::{MetaMessage, MidiMessage, Smf, Timing, TrackEventKind};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex, OnceLock,
};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{Emitter, Manager};

mod midi_backend;
use midi_backend::{MidiDevice, MidiOutputBackend, MidiOutputState};

const NOTE_HOLD_MS: u64 = 28;
const SPEED_STEP: f64 = 0.1;
const MAX_MIDI_DOWNLOAD_BYTES: u64 = 32 * 1024 * 1024;
const MIDISHOW_LOGIN_WINDOW_LABEL: &str = "midishow-login";
const MIDISHOW_LOGIN_URL: &str = "https://www.midishow.com/user/account/login";
const MIDISHOW_ACCOUNT_URL: &str = "https://www.midishow.com/user/account";
const MIDISHOW_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";
const MIDISHOW_LOGIN_TIMEOUT_MS: u64 = 75_000;
/// When the login window is shown for the user to solve a Cloudflare / captcha
/// challenge manually, give them a much longer window than the automatic fill.
const MIDISHOW_LOGIN_CONFIRM_TIMEOUT_MS: u64 = 300_000;
/// 自动填充迟迟没进展（页面被 Cloudflare 卡住、加载过慢）时，把窗口弹出来让用户
/// 手动完成，而不是干等 75s 才超时。
const MIDISHOW_LOGIN_FALLBACK_MS: u64 = 20_000;
/// Keep an online search responsive when the proxy or Cloudflare stalls.
const MIDISHOW_SEARCH_HTTP_TIMEOUT_SECS: u64 = 5;
const MIDISHOW_SEARCH_CLI_TIMEOUT_SECS: u64 = 6;
const MIDISHOW_LOGIN_TITLE_PREFIX: &str = "VRCDOG_MIDISHOW:";

/// Regex patterns compiled once and reused across all calls.
mod regex_patterns {
    use regex::Regex;
    use std::sync::OnceLock;

    pub fn midi_id_from_url() -> &'static Regex {
        static RE: OnceLock<Regex> = OnceLock::new();
        RE.get_or_init(|| Regex::new(r#"/(?:en/)?midi/(\d+)\.html(?:[?#][^"'\s<>]*)?"#).unwrap())
    }

    pub fn extract_number() -> &'static Regex {
        static RE: OnceLock<Regex> = OnceLock::new();
        RE.get_or_init(|| Regex::new(r"(\d+)").unwrap())
    }

    pub fn data_key() -> &'static Regex {
        static RE: OnceLock<Regex> = OnceLock::new();
        RE.get_or_init(|| Regex::new(r#"data-key\s*=\s*["'](\d+)["']"#).unwrap())
    }

    pub fn csrf_meta() -> &'static Regex {
        static RE: OnceLock<Regex> = OnceLock::new();
        RE.get_or_init(|| {
            Regex::new(r#"(?is)<meta[^>]+name=["']csrf-token["'][^>]+content=["']([^"']+)["']"#)
                .unwrap()
        })
    }

    pub fn html_tag() -> &'static Regex {
        static RE: OnceLock<Regex> = OnceLock::new();
        RE.get_or_init(|| Regex::new(r"(?is)<[^>]+>").unwrap())
    }

    pub fn clean_text() -> &'static Regex {
        static RE: OnceLock<Regex> = OnceLock::new();
        RE.get_or_init(|| {
            Regex::new(
                r"\s*[-|·]\s*|上传于|下载|评分|\d+\.\d+\s*\(|\d+(?:\.\d+)?\s*(?:KB|MB)|\bGM\d*\b",
            )
            .unwrap()
        })
    }
}

/// Resolve the VRPiano source project path.
/// Priority:
/// 1. `VRCDOG_VRPIANO_PATH` environment variable
/// 2. `VRPIANO_PROJECT_PATH` environment variable
/// 3. `<app_resource_dir>/src-python` (bundled with the app)
/// 4. `<app_resource_dir>/VRPiano-auto-play` (optional external bundle)
/// 5. `<current_dir>/src-python`
/// 6. `<current_dir>/VRPiano-auto-play` (optional external checkout)
fn resolve_vrpiano_project_path(app: Option<&tauri::AppHandle>) -> PathBuf {
    if let Some(found) = find_vrpiano_project_root(app) {
        return found;
    }
    // Fallback: try to find midishow.py relative to the executable
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            let candidates = [
                exe_dir.join("src-python"),
                exe_dir.join("VRPiano-auto-play").join("src-python"),
            ];
            for candidate in &candidates {
                if candidate.join("midishow.py").exists() {
                    return candidate.parent().unwrap_or(exe_dir).to_path_buf();
                }
            }
        }
    }
    // Last resort: current directory
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// Search candidate locations for the VRPiano project root (a directory that
/// contains the Midishow Python module). Returns the root directory so that
/// both `midishow.py` and `midishow-downloader/` are reachable as children.
fn find_vrpiano_project_root(app: Option<&tauri::AppHandle>) -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();

    // Env vars first
    for key in ["VRCDOG_VRPIANO_PATH", "VRPIANO_PROJECT_PATH"] {
        if let Ok(path) = std::env::var(key) {
            let p = PathBuf::from(path);
            if p.exists() {
                candidates.push(p);
            }
        }
    }

    // App resource directory (bundled modules). Tauri rewrites a leading
    // `../` in resource paths to `_up_`, so the packaged src-python folder is
    // normally located at `$RESOURCE/_up_/src-python`.
    if let Some(app_handle) = app {
        if let Ok(resource_dir) = app_handle.path().resource_dir() {
            candidates.push(resource_dir.clone());
            candidates.push(resource_dir.join("VRPiano-auto-play"));
            candidates.push(resource_dir.join("src-python"));
            candidates.push(resource_dir.join("_up_"));
            candidates.push(resource_dir.join("_up_").join("src-python"));
        }
    }

    // Current working directory and common siblings
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.clone());
        candidates.push(cwd.join("VRPiano-auto-play"));
        candidates.push(cwd.join("src-python"));
        if let Some(parent) = cwd.parent() {
            candidates.push(parent.join("VRPiano-auto-play"));
        }
    }

    // Try relative to executable
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            candidates.push(exe_dir.join("VRPiano-auto-play"));
            candidates.push(exe_dir.join("src-python"));
        }
    }

    for candidate in candidates {
        if candidate.join("midishow.py").exists() {
            return Some(candidate);
        }
        if candidate.join("src-python").join("midishow.py").exists() {
            return Some(candidate);
        }
    }
    None
}

#[cfg(target_os = "windows")]
static HOTKEY_CONTEXT: OnceLock<GlobalHotkeyContext> = OnceLock::new();
#[cfg(target_os = "windows")]
static HOTKEY_HOOK_STARTED: AtomicBool = AtomicBool::new(false);
#[cfg(target_os = "windows")]
struct HotkeyHook(windows::Win32::UI::WindowsAndMessaging::HHOOK);
// HHOOK wraps a raw pointer and is !Sync; we only touch it from the hook thread
// and the stop path behind a Mutex, so it is safe to share across threads.
#[cfg(target_os = "windows")]
unsafe impl Send for HotkeyHook {}
#[cfg(target_os = "windows")]
unsafe impl Sync for HotkeyHook {}
#[cfg(target_os = "windows")]
static HOTKEY_HOOK: OnceLock<std::sync::Mutex<Option<HotkeyHook>>> = OnceLock::new();
#[cfg(target_os = "windows")]
static HOTKEY_THREAD_ID: OnceLock<std::sync::Mutex<Option<u32>>> = OnceLock::new();
#[cfg(target_os = "windows")]
fn init_hook_mutex() -> std::sync::Mutex<Option<HotkeyHook>> {
    std::sync::Mutex::new(None)
}
#[cfg(target_os = "windows")]
fn init_tid_mutex() -> std::sync::Mutex<Option<u32>> {
    std::sync::Mutex::new(None)
}

#[cfg(target_os = "windows")]
#[derive(Clone)]
struct GlobalHotkeyContext {
    app: tauri::AppHandle,
    state: Arc<Mutex<VrpianoRuntime>>,
    midi_backend: Arc<Mutex<MidiOutputBackend>>,
    recorder: Arc<Mutex<MidiRecorder>>,
}

#[derive(Clone, Serialize)]
pub struct VrpianoSong {
    id: String,
    name: String,
    path: String,
    size: u64,
    modified_ms: u64,
    /// 绝对路径，指向与 MIDI 同目录的封面图（`<basename>.cover.{jpg,png,webp}`）。
    /// 仅当从 Midishow 下载且封面成功落盘后才会被填充；
    /// `list_local_songs` 也会在探测到该文件时自动填上。
    cover_path: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct VrpianoOnlineSong {
    id: u64,
    title: String,
    artist: String,
    page_url: String,
    /// 从 Midishow 搜索结果中提取到的封面图 URL（绝对 http(s) URL），
    /// 仅指向 midishow.com / midishowstatic.com 静态域，避免被滥用为通用加载器。
    cover_url: Option<String>,
}

#[derive(Clone, Serialize)]
pub struct VrpianoMidishowAccount {
    username: String,
    login_type: String,
}

#[derive(Clone, Serialize)]
pub struct VrpianoMidiData {
    name: String,
    data: String,
}

#[derive(Clone, Serialize, Deserialize)]
struct StoredMidishowAccount {
    username: String,
    #[serde(default, skip_serializing)]
    password: String,
    #[serde(default)]
    cookie: String,
}

#[derive(Clone, Serialize)]
pub struct VrpianoStatus {
    running: bool,
    paused: bool,
    song_name: String,
    song_path: String,
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
    last_hotkey: String,
    last_hotkey_at_ms: u64,
    midi_connected: bool,
    midi_device_name: Option<String>,
    recording: bool,
    recorded_midi_path: Option<String>,
    channels: [ChannelState; 16],
    voice_listening: bool,
    tts_enabled: bool,
    last_transcription: String,
    vrchat_osc_enabled: bool,
    vrchat_osc_host: String,
    vrchat_osc_port: u16,
    vrchat_osc_running: bool,
    vrchat_osc_last_error: String,
    vrchat_osc_connected: bool,
}

#[derive(Clone, Serialize, Deserialize, Copy)]
pub struct ChannelState {
    pub muted: bool,
    pub solo: bool,
    pub volume: u8,
}

impl Default for ChannelState {
    fn default() -> Self {
        Self {
            muted: false,
            solo: false,
            volume: 127,
        }
    }
}

#[derive(Clone, Deserialize)]
pub struct VrpianoStartRequest {
    song_path: String,
    delay_secs: u64,
    speed: f64,
    #[serde(default)]
    output_mode: String,
    midi_output_device: Option<String>,
}

#[derive(Clone, Deserialize)]
pub struct VrchatOscStartRequest {
    song_path: String,
    delay_secs: u64,
    speed: f64,
    host: String,
    port: u16,
    #[serde(default = "default_osc_mode")]
    mode: String,
    #[serde(default = "default_osc_avatar_prefix")]
    avatar_prefix: String,
}

fn default_osc_mode() -> String { "piano".to_string() }
fn default_osc_avatar_prefix() -> String { "/avatar/parameters/note".to_string() }

#[derive(Clone, Deserialize)]
pub struct VrpianoHotkeyConfig {
    enabled: bool,
    song_path: String,
    delay_secs: u64,
    speed: f64,
    #[serde(default)]
    output_mode: String,
    #[serde(default)]
    osc_host: String,
    #[serde(default = "default_osc_port")]
    osc_port: u16,
}

fn default_osc_port() -> u16 {
    9000
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
    /// 可选的封面图 URL，由前端从 Midishow 搜索结果中透传过来；
    /// 后端仅在 URL 通过 `is_midishow_cover_url` 校验时才会下载并落盘。
    cover_url: Option<String>,
}

#[derive(Clone, Deserialize)]
pub struct VrpianoMidishowLoginRequest {
    account: String,
    password: String,
}

#[derive(Clone, Serialize)]
pub struct VrpianoMidishowLoginStatus {
    state: String,
    message: String,
    username: Option<String>,
}

#[derive(Clone, Default)]
struct MidishowLoginRuntime {
    started_at_ms: u64,
    state: String,
    message: String,
    /// 用户输入的账号（用于在不再做 HTTP 验证的情况下作为 username 持久化）
    account: String,
    /// 最近一次 `on_page_load` 看到的 URL（用于判断登录页是否已跳转走）
    last_url: String,
}

fn midishow_login_runtime() -> &'static Mutex<MidishowLoginRuntime> {
    static RUNTIME: OnceLock<Mutex<MidishowLoginRuntime>> = OnceLock::new();
    RUNTIME.get_or_init(|| Mutex::new(MidishowLoginRuntime::default()))
}

#[derive(Clone)]
pub struct VrpianoState {
    inner: Arc<Mutex<VrpianoRuntime>>,
    midi_backend: Arc<Mutex<MidiOutputBackend>>,
    recorder: Arc<Mutex<MidiRecorder>>,
}

struct VrpianoRuntime {
    stop: Option<Arc<AtomicBool>>,
    paused: Arc<AtomicBool>,
    speed: Arc<Mutex<f64>>,
    hotkeys_enabled: bool,
    hotkey_song_path: String,
    hotkey_delay_secs: u64,
    vrchat_osc_enabled: bool,
    vrchat_osc_host: String,
    vrchat_osc_port: u16,
    vrchat_osc_mode: String,
    vrchat_osc_avatar_prefix: String,
    /// Global transposition in semitones (clamped -24..24). Drum channel (9) is excluded.
    transpose: i8,
    /// Per-channel routing to the VRChat piano. Channels disabled here are skipped.
    piano_channels: [bool; 16],
    /// Active playlist (song paths) for auto-advance between tracks.
    playlist: Vec<String>,
    /// Playlist mode: sequential | random | one | repeat_all | stop_at_song_end | stop_at_end.
    play_mode: String,
    /// Index of the currently playing song within `playlist`.
    current_index: usize,
    /// Which playback engine started the current session ("osc" | "midi").
    active_engine: String,
    /// Stop flag for the standalone OSC heartbeat (set when disconnecting).
    osc_heartbeat_stop: Option<Arc<AtomicBool>>,
    status: VrpianoStatus,
}

struct MidiRecorder {
    recording: bool,
    events: Vec<MidiRecordEvent>,
    start_time_ms: u64,
    output_path: Option<String>,
}

#[derive(Clone, Serialize)]
struct MidiRecordEvent {
    at_ms: u64,
    #[serde(rename = "type")]
    event_type: String,
    channel: u8,
    data: Vec<u8>,
}

const MIDI_PPQ: u16 = 480;
const MIDI_TEMPO_US: u32 = 500_000;

impl MidiRecorder {
    fn new() -> Self {
        Self {
            recording: false,
            events: Vec::new(),
            start_time_ms: 0,
            output_path: None,
        }
    }

    fn start(&mut self, output_path: String) {
        self.recording = true;
        self.events.clear();
        self.start_time_ms = std::time::SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        self.output_path = Some(output_path);
    }

    fn stop(&mut self) -> Option<String> {
        self.recording = false;
        let output_path = self.output_path.clone()?;
        let events = std::mem::take(&mut self.events);
        if events.is_empty() {
            return Some(output_path);
        }
        if let Err(e) = write_midi_recording(Path::new(&output_path), &events) {
            eprintln!("Failed to write MIDI recording: {e}");
        }
        Some(output_path)
    }

    fn record(&mut self, at_ms: u64, event_type: &str, channel: u8, data: &[u8]) {
        if self.recording {
            self.events.push(MidiRecordEvent {
                at_ms: at_ms - self.start_time_ms,
                event_type: event_type.to_string(),
                channel,
                data: data.to_vec(),
            });
        }
    }

    fn is_recording(&self) -> bool {
        self.recording
    }
}

/// Trim leading silence (shift first event to t=0) and drop trailing events that
/// occur after the last note on/off, so recorded .mid files start cleanly.
fn trim_recording_silence(events: &mut Vec<MidiRecordEvent>) {
    if events.is_empty() {
        return;
    }
    let first = events.iter().map(|e| e.at_ms).min().unwrap_or(0);
    if first > 0 {
        for e in events.iter_mut() {
            e.at_ms = e.at_ms.saturating_sub(first);
        }
    }
    if let Some(last_note) = events
        .iter()
        .filter(|e| e.event_type == "note_on" || e.event_type == "note_off")
        .map(|e| e.at_ms)
        .max()
    {
        events.retain(|e| e.at_ms <= last_note + 1);
    }
}

fn write_midi_recording(path: &Path, events: &[MidiRecordEvent]) -> Result<(), String> {
    let mut events: Vec<MidiRecordEvent> = events.to_vec();
    trim_recording_silence(&mut events);
    let mut file = fs::File::create(path).map_err(|e| format!("Failed to create MIDI file: {e}"))?;

    let mut sorted: Vec<_> = events.iter().collect();
    sorted.sort_by_key(|e| e.at_ms);

    let mut track_bytes: Vec<u8> = Vec::new();
    let mut last_tick: u64 = 0;

    for event in sorted {
        let time_us = event.at_ms.saturating_mul(1000);
        let tick = ((time_us as u128 * MIDI_PPQ as u128) / MIDI_TEMPO_US as u128) as u64;
        let delta = tick.saturating_sub(last_tick);
        last_tick = tick;

        write_variable_length(&mut track_bytes, delta);

        match event.event_type.as_str() {
            "note_on" => {
                if event.data.len() >= 2 {
                    track_bytes.push(0x90 | (event.channel & 0x0F));
                    track_bytes.push(event.data[0] & 0x7F);
                    track_bytes.push(event.data[1] & 0x7F);
                }
            }
            "note_off" => {
                if event.data.len() >= 2 {
                    track_bytes.push(0x80 | (event.channel & 0x0F));
                    track_bytes.push(event.data[0] & 0x7F);
                    track_bytes.push(event.data[1] & 0x7F);
                }
            }
            "program_change" => {
                if event.data.len() >= 1 {
                    track_bytes.push(0xC0 | (event.channel & 0x0F));
                    track_bytes.push(event.data[0] & 0x7F);
                }
            }
            "control_change" => {
                if event.data.len() >= 2 {
                    track_bytes.push(0xB0 | (event.channel & 0x0F));
                    track_bytes.push(event.data[0] & 0x7F);
                    track_bytes.push(event.data[1] & 0x7F);
                }
            }
            _ => {}
        }
    }

    write_variable_length(&mut track_bytes, 0);
    track_bytes.push(0xFF);
    track_bytes.push(0x2F);
    track_bytes.push(0x00);

    let header = build_smf_header(1, MIDI_PPQ);
    file.write_all(&header).map_err(|e| format!("Failed to write MIDI header: {e}"))?;
    write_chunk(&mut file, b"MTrk", &track_bytes)
        .map_err(|e| format!("Failed to write MIDI track: {e}"))?;
    Ok(())
}

fn build_smf_header(ntrks: u16, division: u16) -> Vec<u8> {
    let mut header = Vec::with_capacity(14);
    header.extend_from_slice(b"MThd");
    header.extend_from_slice(&6u32.to_be_bytes());
    header.extend_from_slice(&0u16.to_be_bytes());
    header.extend_from_slice(&ntrks.to_be_bytes());
    header.extend_from_slice(&division.to_be_bytes());
    header
}

fn write_chunk(file: &mut fs::File, tag: &[u8; 4], data: &[u8]) -> Result<(), String> {
    file.write_all(tag).map_err(|e| e.to_string())?;
    file.write_all(&(data.len() as u32).to_be_bytes())
        .map_err(|e| e.to_string())?;
    file.write_all(data).map_err(|e| e.to_string())
}

fn write_variable_length(buffer: &mut Vec<u8>, mut value: u64) {
    let mut bytes = Vec::new();
    bytes.push((value & 0x7F) as u8);
    value >>= 7;
    while value > 0 {
        bytes.push(((value & 0x7F) | 0x80) as u8);
        value >>= 7;
    }
    buffer.extend(bytes.into_iter().rev());
}

#[derive(Clone)]
struct PlayEvent {
    at_ms: u64,
    note: u8,
    vk: u16,
}

#[derive(Clone)]
struct MidiPlayEvent {
    at_ms: u64,
    note: u8,
    velocity: u8,
    channel: u8,
    is_note_on: bool,
    /// Control change events (e.g. sustain pedal CC64, all-notes-off CC123) are carried
    /// separately so they are not mistaken for note events.
    control_change: Option<(u8, u8)>,
}

impl Default for VrpianoState {
    fn default() -> Self {
        Self {
            inner: Arc::new(Mutex::new(VrpianoRuntime {
                stop: None,
                paused: Arc::new(AtomicBool::new(false)),
                speed: Arc::new(Mutex::new(1.0)),
                hotkeys_enabled: false,
                hotkey_song_path: String::new(),
                hotkey_delay_secs: 0,
                vrchat_osc_enabled: false,
                vrchat_osc_host: String::new(),
                vrchat_osc_port: 9000,
                vrchat_osc_mode: default_osc_mode(),
                vrchat_osc_avatar_prefix: default_osc_avatar_prefix(),
                transpose: 0,
                piano_channels: [true; 16],
                playlist: Vec::new(),
                play_mode: "sequential".to_string(),
                current_index: 0,
                active_engine: String::new(),
                osc_heartbeat_stop: None,
                status: VrpianoStatus {
                    running: false,
                    paused: false,
                    song_name: String::new(),
                    song_path: String::new(),
                    progress: 0.0,
                    played_notes: 0,
                    total_notes: 0,
                    duration_ms: 0,
                    elapsed_ms: 0,
                    last_event: String::new(),
                    last_error: String::new(),
                    songs_dir: String::new(),
                    speed: 1.0,
                    hotkeys_enabled: false,
                    hotkeys_available: false,
                    last_hotkey: String::new(),
                    last_hotkey_at_ms: 0,
                    midi_connected: false,
                    midi_device_name: None,
                    recording: false,
                    recorded_midi_path: None,
                    channels: [ChannelState::default(); 16],
                    voice_listening: false,
                    tts_enabled: false,
                    last_transcription: String::new(),
                    vrchat_osc_enabled: false,
                    vrchat_osc_host: String::new(),
                    vrchat_osc_port: 9000,
                vrchat_osc_running: false,
                vrchat_osc_last_error: String::new(),
                vrchat_osc_connected: false,
            },
            })),
            midi_backend: Arc::new(Mutex::new(MidiOutputBackend::new())),
            recorder: Arc::new(Mutex::new(MidiRecorder::new())),
        }
    }
}

#[tauri::command]
pub async fn vrpiano_init(app: tauri::AppHandle) -> Result<VrpianoStatus, String> {
    let songs_dir = ensure_songs_dir(&app)?;
    let project_path = resolve_vrpiano_project_path(Some(&app));
    import_seed_songs(&project_path, &songs_dir)?;
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

    if is_midishow_input(url) && !looks_like_direct_midi_url(url) {
        let midi_id = extract_midishow_id(url)?;
        return download_midishow_to_library(
            &app,
            &songs_dir,
            midi_id,
            request.filename,
            None,
            false,
        )
        .await;
    }

    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("URL must start with http:// or https://".to_string());
    }

    let (midi, suggested_filename) = download_direct_midi(url).await?;
    let filename = request
        .filename
        .map(|name| sanitize_filename(&name))
        .filter(|name| !name.is_empty())
        .or(suggested_filename)
        .unwrap_or_else(|| filename_from_url(url));
    let target = unique_path(&songs_dir.join(ensure_midi_extension(filename)));
    write_midi_file(&target, &midi)?;
    Ok(song_from_path(&target)?)
}

/// Download a public MIDI URL with bounded redirects, size limits, and an
/// actual Standard MIDI validation step before anything reaches the library.
async fn download_direct_midi(url: &str) -> Result<(Vec<u8>, Option<String>), String> {
    let response = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36")
        .redirect(reqwest::redirect::Policy::limited(10))
        .timeout(Duration::from_secs(45))
        .build()
        .map_err(|e| format!("Failed to create MIDI download client: {e}"))?
        .get(url)
        .header(
            reqwest::header::ACCEPT,
            "audio/midi,audio/x-midi,application/octet-stream;q=0.9,*/*;q=0.8",
        )
        .send()
        .await
        .map_err(|e| format!("Failed to download MIDI: {e}"))?;

    let status = response.status();
    if is_midishow_challenge_response(status, response.headers(), "") {
        return Err("MidiShow requires an interactive browser verification. Open the official page in a browser and use its download action, or paste a public .mid/.midi direct link here.".to_string());
    }
    if !status.is_success() {
        return Err(format!("Download failed with HTTP {status}"));
    }
    if let Some(length) = response.content_length() {
        if length > MAX_MIDI_DOWNLOAD_BYTES {
            return Err(format!(
                "MIDI download is too large ({length} bytes; limit is {MAX_MIDI_DOWNLOAD_BYTES} bytes)"
            ));
        }
    }

    let suggested_filename = response
        .headers()
        .get(reqwest::header::CONTENT_DISPOSITION)
        .and_then(|value| value.to_str().ok())
        .and_then(filename_from_content_disposition);
    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read MIDI download: {e}"))?
        .to_vec();
    if bytes.len() as u64 > MAX_MIDI_DOWNLOAD_BYTES {
        return Err(format!(
            "MIDI download is too large (limit is {MAX_MIDI_DOWNLOAD_BYTES} bytes)"
        ));
    }

    Ok((validate_midi_bytes(bytes)?, suggested_filename))
}

#[tauri::command]
pub async fn vrpiano_search_midishow(
    app: tauri::AppHandle,
    keyword: String,
    max_results: Option<usize>,
) -> Result<Vec<VrpianoOnlineSong>, String> {
    let keyword = keyword.trim();
    if keyword.is_empty() {
        return Err("Please enter a search keyword".to_string());
    }
    let limit = max_results.unwrap_or(30).clamp(1, 50);
    let project_path = resolve_vrpiano_project_path(Some(&app));
    search_midishow(&app, &project_path, keyword, limit).await
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

    download_midishow_to_library(
        &app,
        &songs_dir,
        request.midi_id,
        request.title,
        request.cover_url.as_deref(),
        false,
    )
    .await
    .map(Some)
}

#[tauri::command]
pub async fn vrpiano_midishow_preview_data(
    app: tauri::AppHandle,
    request: VrpianoMidishowDownloadRequest,
) -> Result<VrpianoMidiData, String> {
    let data = download_midishow_bytes(&app, request.midi_id).await?;
    let project_path = resolve_vrpiano_project_path(Some(&app));
    let title = request
        .title
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| {
            midishow_title(&project_path, request.midi_id)
                .unwrap_or_else(|| format!("MIDI_{}", request.midi_id))
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
        // Password-only records were created by the old login flow. Do not
        // expose them as active sessions; the user must complete browser login.
        .filter(|account| !account.cookie.trim().is_empty())
        .map(|account| VrpianoMidishowAccount {
            login_type: "browser-cookie".to_string(),
            username: account.username,
        })
        .collect())
}

#[tauri::command]
pub async fn vrpiano_midishow_login(
    app: tauri::AppHandle,
    request: VrpianoMidishowLoginRequest,
) -> Result<VrpianoMidishowLoginStatus, String> {
    let account = request.account.trim().to_string();
    if account.is_empty() || request.password.is_empty() {
        return Err("请输入 Midishow 账号和密码".to_string());
    }

    if let Some(window) = app.get_webview_window(MIDISHOW_LOGIN_WINDOW_LABEL) {
        let _ = window.close();
    }

    let account_js = serde_json::to_string(&account).map_err(|_| "无法准备登录信息".to_string())?;
    let password_js =
        serde_json::to_string(&request.password).map_err(|_| "无法准备登录信息".to_string())?;
    let login_script = Arc::new(Mutex::new(Some(midishow_login_script(
        &account_js,
        &password_js,
    ))));
    drop(password_js);
    drop(request.password);

    {
        let mut runtime = midishow_login_runtime()
            .lock()
            .map_err(|_| "暂时无法开始登录，请稍后重试".to_string())?;
        runtime.started_at_ms = current_time_ms();
        runtime.state = "opening".to_string();
        runtime.message = "正在准备登录".to_string();
        runtime.account = account.clone();
        runtime.last_url.clear();
    }

    let page_script = Arc::clone(&login_script);
    let monitor_script = midishow_login_monitor_script();
    tauri::WebviewWindowBuilder::new(
        &app,
        MIDISHOW_LOGIN_WINDOW_LABEL,
        tauri::WebviewUrl::External(
            MIDISHOW_LOGIN_URL
                .parse::<tauri::Url>()
                .map_err(|_| "登录地址不可用".to_string())?,
        ),
    )
    .title("登录 Midishow")
    .user_agent(MIDISHOW_USER_AGENT)
    .inner_size(980.0, 760.0)
    .resizable(true)
    // 立即显示登录窗口：原先 `.visible(false)` + 20s 兜底会让用户看不到任何反馈，
    // 主观感受"打开浏览器和登录都好慢"。改为可见后用户立刻看到页面正在加载，
    // 自动填充脚本仍在后台跑；如果 Cloudflare 拦截了自动填充，用户可继续手动操作。
    .visible(true)
    .focused(true)
    .on_page_load(move |window, payload| {
        if payload.event() == tauri::webview::PageLoadEvent::Finished {
            // 记录最近一次加载的 URL，用于判断登录页是否跳转走（登录成功信号）
            if let Ok(mut runtime) = midishow_login_runtime().lock() {
                runtime.last_url = payload.url().as_str().to_string();
            }
            if is_midishow_login_url(payload.url().as_str()) {
                let script = page_script.lock().ok().and_then(|mut value| value.take());
                if let Some(script) = script {
                    let _ = window.eval(&script);
                } else {
                    let _ = window.eval(&monitor_script);
                }
            } else {
                let _ = window.eval(&monitor_script);
            }
        }
    })
    .build()
    .map_err(|_| "暂时无法开始登录，请稍后重试".to_string())?;

    let status = VrpianoMidishowLoginStatus {
        state: "waiting".to_string(),
        message: "正在自动登录".to_string(),
        username: None,
    };
    update_midishow_login_runtime(&status);
    let _ = app.emit("vrpiano_midishow_login_status", status.clone());
    Ok(status)
}

#[tauri::command]
pub async fn vrpiano_midishow_login_status(
    app: tauri::AppHandle,
) -> Result<VrpianoMidishowLoginStatus, String> {
    let Some(window) = app.get_webview_window(MIDISHOW_LOGIN_WINDOW_LABEL) else {
        let current = current_midishow_login_status();
        if matches!(
            current.state.as_str(),
            "opening" | "waiting" | "needs_confirmation"
        ) {
            return Ok(finish_midishow_login(
                &app,
                "failed",
                "登录窗口已关闭，请重新尝试",
            ));
        }
        return Ok(current);
    };

    let elapsed = current_time_ms().saturating_sub(midishow_login_started_at_ms());
    let current_state = current_midishow_login_status().state;
    // A human solving a Cloudflare / captcha challenge needs far more than the
    // automatic-fill window. Keep waiting (window is already visible) instead of
    // cutting them off with a misleading "登录等待时间过长".
    let timeout_ms = if current_state == "needs_confirmation" {
        MIDISHOW_LOGIN_CONFIRM_TIMEOUT_MS
    } else {
        MIDISHOW_LOGIN_TIMEOUT_MS
    };
    if elapsed >= timeout_ms {
        let _ = window.close();
        if current_state == "needs_confirmation" {
            return Ok(finish_midishow_login(
                &app,
                "failed",
                "登录确认超时，请重新尝试",
            ));
        }
        return Ok(finish_midishow_login(
            &app,
            "failed",
            "登录等待时间过长，请重新尝试",
        ));
    }

    let runtime = midishow_login_runtime()
        .lock()
        .map(|runtime| (runtime.last_url.clone(), runtime.account.clone()))
        .unwrap_or_default();
    let (last_url, auto_account) = runtime;
    let navigated_away = !last_url.is_empty() && !is_midishow_login_url(&last_url);
    if navigated_away && !auto_account.is_empty() {
        // 仅在页面已经跳离登录页之后才读 cookie：
        // 之前每次 500ms 轮询都调用 `read_midishow_browser_cookie`，
        // 那是一次 WebView2 COM 调用，比较耗时；现在按需触发，主观上"卡顿感"会明显减少。
        let cookie = read_midishow_browser_cookie(window.clone()).await?;
        if !cookie.is_empty() {
            // 不再发起会触发 Cloudflare 拦截的 HTTP 校验请求（该请求从中国网络常被拦，
            // 会误报「暂时无法确认登录状态」）。改为用「网页已离开登录页 + 存在 cookie」
            // 作为登录成功判据：登录表单提交成功后页面会跳走、cookie 被写入。
            persist_midishow_session(&app, &auto_account, cookie)?;
            let _ = window.close();
            let status = VrpianoMidishowLoginStatus {
                state: "signed_in".to_string(),
                message: format!("已登录 {auto_account}"),
                username: Some(auto_account),
            };
            update_midishow_login_runtime(&status);
            let _ = app.emit("vrpiano_midishow_login_status", status.clone());
            return Ok(status);
        }
    }
    if elapsed >= MIDISHOW_LOGIN_FALLBACK_MS && current_state == "waiting" {
        let already_visible = window.is_visible().unwrap_or(false);
        if !already_visible {
            let _ = window.show();
            let _ = window.set_focus();
        }
        return Ok(finish_midishow_login(
            &app,
            "needs_confirmation",
            "自动登录未完成，请在弹出的登录窗口中手动完成登录",
        ));
    }

    let title = window.title().unwrap_or_default();
    if let Some(signal) = parse_midishow_login_title(&title) {
        match signal {
            MidishowLoginSignal::CredentialsRejected => {
                let _ = window.close();
                return Ok(finish_midishow_login(
                    &app,
                    "failed",
                    "账号或密码不正确，请检查后重试",
                ));
            }
            MidishowLoginSignal::NeedsConfirmation => {
                let already_visible = window.is_visible().unwrap_or(false);
                if !already_visible {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
                let current = current_midishow_login_status();
                if current.state == "needs_confirmation" {
                    return Ok(current);
                }
                return Ok(finish_midishow_login(
                    &app,
                    "needs_confirmation",
                    "请在登录窗口完成确认，完成后会自动继续",
                ));
            }
            MidishowLoginSignal::FormMissing => {
                let _ = window.close();
                return Ok(finish_midishow_login(
                    &app,
                    "failed",
                    "暂时无法完成登录，请稍后重试",
                ));
            }
            MidishowLoginSignal::Submitted | MidishowLoginSignal::Ready => {}
        }
    }

    // Cloudflare overwrites document.title with "Just a moment…" on its own page,
    // which would otherwise make the next poll parse to None and flip the state
    // back to "waiting" (re-enabling the 75s timeout). Keep an already-acknowledged
    // confirmation sticky while the window is visible so the user isn't cut off.
    let current = current_midishow_login_status();
    if current.state == "needs_confirmation" && window.is_visible().unwrap_or(false) {
        return Ok(current);
    }

    let status = VrpianoMidishowLoginStatus {
        state: "waiting".to_string(),
        message: "正在自动登录".to_string(),
        username: None,
    };
    update_midishow_login_runtime(&status);
    Ok(status)
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
        {
            let mut runtime = state
                .inner
                .lock()
                .map_err(|_| "VRPiano state lock poisoned".to_string())?;
            runtime.active_engine = match request.output_mode.trim().to_ascii_lowercase().as_str() {
                "midi" | "midi_device" => "midi",
                "osc" | "vrchat_osc" => "osc",
                _ => "keyboard",
            }.to_string();
            if runtime.playlist.is_empty() {
                runtime.playlist = vec![request.song_path.trim().to_string()];
                runtime.current_index = 0;
            }
        }
    start_playback(app, state.inner.clone(), state.midi_backend.clone(), state.recorder.clone(), request)
    }
}

#[tauri::command]
pub async fn vrpiano_stop(
    app: tauri::AppHandle,
    state: tauri::State<'_, VrpianoState>,
) -> Result<VrpianoStatus, String> {
    stop_playback(app, state.inner.clone())
}

// ==================== OSC config persistence ====================

fn save_vrpiano_osc_config(app: &tauri::AppHandle, host: &str, port: u16) {
    if let Ok(dir) = app.path().app_config_dir() {
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("vrpiano_osc.json");
        let data = serde_json::json!({ "host": host, "port": port });
        if let Ok(text) = serde_json::to_string_pretty(&data) {
            let _ = std::fs::write(&path, text);
        }
    }
}

fn load_vrpiano_osc_config(app: &tauri::AppHandle) -> Option<(String, u16)> {
    let dir = app.path().app_config_dir().ok()?;
    let path = dir.join("vrpiano_osc.json");
    let content = std::fs::read_to_string(path).ok()?;
    let v: serde_json::Value = serde_json::from_str(&content).ok()?;
    let host = v.get("host")?.as_str()?.to_string();
    let port = v.get("port")?.as_u64()? as u16;
    if host.is_empty() {
        None
    } else {
        Some((host, port))
    }
}

// ==================== OSC heartbeat / connection monitor ====================

fn spawn_osc_heartbeat(app: tauri::AppHandle, state: Arc<Mutex<VrpianoRuntime>>, stop: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        loop {
            if stop.load(Ordering::SeqCst) {
                break;
            }
            let connected = crate::osc::system_snapshot(false).vrc_running;
            update_runtime(&state, |s| s.vrchat_osc_connected = connected);
            emit_status(&app, &state);
            thread::sleep(Duration::from_millis(1500));
        }
    });
}

// ==================== Playlist auto-advance ====================

/// Lightweight pseudo-random index pick (avoids pulling in the `rand` crate here).
fn random_index(n: usize) -> usize {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    (nanos as usize) % n.max(1)
}

fn maybe_advance_playlist(
    app: tauri::AppHandle,
    state: Arc<Mutex<VrpianoRuntime>>,
    engine: &str,
    midi_backend: Option<Arc<Mutex<MidiOutputBackend>>>,
    recorder: Option<Arc<Mutex<MidiRecorder>>>,
) {
    let next = {
        let mut runtime = match state.lock() {
            Ok(r) => r,
            Err(_) => return,
        };
        if runtime.playlist.is_empty() {
            return;
        }
        let n = runtime.playlist.len();
        let cur = runtime.current_index;
        let mode = runtime.play_mode.clone();
        let next_idx = match mode.as_str() {
            "one" => cur,
            "repeat_all" | "sequential" => (cur + 1) % n,
            "random" => {
                if n <= 1 {
                    0
                } else {
                    random_index(n)
                }
            }
            "stop_at_song_end" | "stop_at_end" => {
                if cur + 1 >= n {
                    return;
                }
                cur + 1
            }
            _ => (cur + 1) % n,
        };
        runtime.current_index = next_idx;
        let speed = runtime.speed.lock().map(|s| *s).unwrap_or(1.0);
        (runtime.playlist[next_idx].clone(), speed)
    };
    let (path, speed) = next;
        // APS-NoteCast style short gap between songs
        thread::sleep(Duration::from_millis(400));
        match engine {
            "osc" => {
                let _ = begin_vrchat_osc(&app, &state, &path, 1, speed);
            }
            "midi" => {
                if let (Some(mb), Some(rec)) = (midi_backend, recorder) {
                    let device_id = mb
                        .lock()
                        .ok()
                        .and_then(|backend| backend.state().lock().ok().and_then(|status| status.device_id.clone()));
                    let req = VrpianoStartRequest {
                        song_path: path.to_string(),
                        delay_secs: 1,
                        speed,
                        output_mode: "midi".into(),
                        midi_output_device: device_id,
                    };
                    let _ = start_playback(app.clone(), state.clone(), mb, rec, req);
                }
            }
            _ => {}
        }
}

// ==================== Begin playback (shared by command + auto-advance) ====================

fn begin_vrchat_osc(
    app: &tauri::AppHandle,
    state: &Arc<Mutex<VrpianoRuntime>>,
    song_path: &str,
    delay_secs: u64,
    speed: f64,
) -> Result<(), String> {
    let path = PathBuf::from(song_path.trim());
    if !path.exists() || !path.is_file() || !is_midi_file(&path) {
        return Err("Please choose a valid MIDI file from the VRPiano library".to_string());
    }
    let (events, duration_ms) = parse_midi_for_output(&path)?;
    if events.is_empty() {
        return Err("This MIDI has no notes that can be mapped to VRPiano keys".to_string());
    }
    let song_name = path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("Untitled")
        .to_string();
    let stop_flag = Arc::new(AtomicBool::new(false));
    let pause_flag;
    let (host, port, osc_mode, avatar_prefix) = {
        let mut runtime = state
            .lock()
            .map_err(|_| "VRPiano state lock poisoned".to_string())?;
        if runtime.status.running {
            return Err("VRPiano is already playing".to_string());
        }
        runtime.stop = Some(stop_flag.clone());
        runtime.paused.store(false, Ordering::SeqCst);
        pause_flag = runtime.paused.clone();
        if let Ok(mut s) = runtime.speed.lock() {
            *s = speed;
        }
        // VRChat's local OSC input defaults to 127.0.0.1:9000; fall back to it
        // when the user hasn't configured a custom host.
        let host = if runtime.vrchat_osc_host.trim().is_empty() {
            "127.0.0.1".to_string()
        } else {
            runtime.vrchat_osc_host.clone()
        };
        let port = runtime.vrchat_osc_port;
        let osc_mode = runtime.vrchat_osc_mode.clone();
        let avatar_prefix = runtime.vrchat_osc_avatar_prefix.clone();
        runtime.status = VrpianoStatus {
            running: true,
            paused: false,
            song_name: song_name.clone(),
            song_path: path.to_string_lossy().to_string(),
            progress: 0.0,
            played_notes: 0,
            total_notes: events.len(),
            duration_ms,
            elapsed_ms: 0,
            last_event: format!("VRChat OSC Starting after {}s delay", delay_secs),
            last_error: String::new(),
            songs_dir: runtime.status.songs_dir.clone(),
            speed,
            hotkeys_enabled: runtime.hotkeys_enabled,
            hotkeys_available: cfg!(target_os = "windows"),
            last_hotkey: runtime.status.last_hotkey.clone(),
            last_hotkey_at_ms: runtime.status.last_hotkey_at_ms,
            midi_connected: runtime.status.midi_connected,
            midi_device_name: runtime.status.midi_device_name.clone(),
            recording: false,
            recorded_midi_path: None,
            channels: runtime.status.channels.clone(),
            voice_listening: false,
            tts_enabled: false,
            last_transcription: String::new(),
            vrchat_osc_enabled: true,
            vrchat_osc_host: host.clone(),
            vrchat_osc_port: port,
            vrchat_osc_running: true,
            vrchat_osc_last_error: String::new(),
            vrchat_osc_connected: runtime.status.vrchat_osc_connected,
        };
        (host, port, osc_mode, avatar_prefix)
    };
    let app_handle = app.clone();
    let state_arc = state.clone();
    spawn_osc_heartbeat(app_handle.clone(), state_arc.clone(), stop_flag.clone());
    std::thread::spawn(move || {
        run_vrchat_osc_playback(
            app_handle,
            state_arc,
            song_name,
            events,
            duration_ms,
            delay_secs,
            host,
            port,
            osc_mode,
            avatar_prefix,
            stop_flag,
            pause_flag,
        );
    });
    Ok(())
}

#[tauri::command]
pub async fn vrpiano_start_vrchat_osc(
    app: tauri::AppHandle,
    state: tauri::State<'_, VrpianoState>,
    request: VrchatOscStartRequest,
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
        let host = request.host.trim().to_string();
        if host.is_empty() || request.port == 0 {
            return Err("OSC target must use a valid host and port (1-65535)".to_string());
        }
        {
            let mut runtime = state
                .inner
                .lock()
                .map_err(|_| "VRPiano state lock poisoned".to_string())?;
            runtime.vrchat_osc_enabled = true;
            runtime.vrchat_osc_host = host.clone();
            runtime.vrchat_osc_port = request.port;
            runtime.vrchat_osc_mode = if request.mode.trim().eq_ignore_ascii_case("avatar") {
                "avatar".to_string()
            } else {
                "piano".to_string()
            };
            runtime.vrchat_osc_avatar_prefix = request.avatar_prefix.trim().trim_end_matches('/').to_string();
            runtime.active_engine = "osc".to_string();
            if runtime.playlist.is_empty() {
                runtime.playlist = vec![request.song_path.trim().to_string()];
                runtime.current_index = 0;
            }
        }
        save_vrpiano_osc_config(&app, &host, request.port);
        begin_vrchat_osc(&app, &state.inner, &request.song_path, request.delay_secs, speed)?;
        let status = status_snapshot(&app, &state.inner)?;
        Ok(status)
    }
}

#[tauri::command]
pub async fn vrpiano_test_osc_note(
    host: String,
    port: u16,
    mode: Option<String>,
    avatar_prefix: Option<String>,
    note: Option<u8>,
) -> Result<(), String> {
    use crate::osc::{osc_send_message_multi, OscArgument};
    let note = note.unwrap_or(60).min(127);
    let mode = mode.unwrap_or_else(default_osc_mode);
    let prefix = avatar_prefix.unwrap_or_else(default_osc_avatar_prefix);
    let address = osc_note_address(&mode, &prefix, note);
    let pressed = vec![OscArgument {
        value_type: "float".to_string(),
        value: serde_json::json!(0.8_f64),
    }];
    osc_send_message_multi(host.clone(), port, address.clone(), pressed).map_err(|e| e.message)?;
    std::thread::sleep(Duration::from_millis(180));
    let released = vec![OscArgument {
        value_type: "float".to_string(),
        value: serde_json::json!(0.0_f64),
    }];
    osc_send_message_multi(host, port, address, released).map_err(|e| e.message)
}

#[tauri::command]
pub async fn vrpiano_toggle_pause(
    app: tauri::AppHandle,
    state: tauri::State<'_, VrpianoState>,
) -> Result<VrpianoStatus, String> {
    toggle_playback_pause(app, state.inner.clone())
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
pub async fn vrpiano_list_midi_devices() -> Result<Vec<MidiDevice>, String> {
    Ok(MidiOutputBackend::list_usb_devices())
}

#[tauri::command]
pub async fn vrpiano_connect_midi_device(
    _app: tauri::AppHandle,
    state: tauri::State<'_, VrpianoState>,
    device_id: String,
) -> Result<MidiOutputState, String> {
    let mut backend = state.midi_backend.lock().unwrap();
    backend.connect_usb(&device_id)?;
    drop(backend);
    let backend = state.midi_backend.lock().unwrap();
    Ok(backend.state().lock().unwrap().clone())
}

#[tauri::command]
pub async fn vrpiano_disconnect_midi_device(
    state: tauri::State<'_, VrpianoState>,
) -> Result<MidiOutputState, String> {
    let mut backend = state.midi_backend.lock().unwrap();
    backend.disconnect();
    Ok(backend.state().lock().unwrap().clone())
}

#[tauri::command]
pub async fn vrpiano_get_midi_output_state(
    state: tauri::State<'_, VrpianoState>,
) -> Result<MidiOutputState, String> {
    let backend = state.midi_backend.lock().unwrap();
    Ok(backend.state().lock().unwrap().clone())
}

#[tauri::command]
pub async fn vrpiano_set_hotkeys(
    app: tauri::AppHandle,
    state: tauri::State<'_, VrpianoState>,
    config: VrpianoHotkeyConfig,
) -> Result<VrpianoStatus, String> {
    set_hotkeys(app, state.inner.clone(), state.midi_backend.clone(), state.recorder.clone(), config)
}

fn start_playback(
    app: tauri::AppHandle,
    state: Arc<Mutex<VrpianoRuntime>>,
    midi_backend: Arc<Mutex<MidiOutputBackend>>,
    recorder: Arc<Mutex<MidiRecorder>>,
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
        let output_mode = match request.output_mode.trim().to_ascii_lowercase().as_str() {
            "keyboard" | "pc_keyboard" | "send_input" => "keyboard",
            "midi" | "midi_device" => "midi",
            "osc" | "vrchat_osc" => "osc",
            _ if request.midi_output_device.is_some() => "midi",
            _ => "keyboard",
        };
        if output_mode == "osc" {
            return Err("请使用 VRChat OSC 专用启动命令进入 OSC 直连模式".to_string());
        }
        if output_mode == "midi" {
            let device_id = request
                .midi_output_device
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| "MIDI 直连模式必须选择输出设备".to_string())?;
            let backend = midi_backend
                .lock()
                .map_err(|_| "MIDI 输出状态不可用".to_string())?;
            let midi_status = backend.state();
            let midi_state = midi_status
                .lock()
                .map_err(|_| "MIDI 输出状态不可用".to_string())?;
            if !midi_state.connected
                || midi_state.device_id.as_deref() != Some(device_id)
            {
                return Err("MIDI 直连模式需要先连接所选 MIDI 输出设备".to_string());
            }
        }
        let (keyboard_events, _midi_events, duration_ms, total_notes) = if output_mode == "midi" {
            let (midi_events, duration_ms) = parse_midi_for_output(&song_path)?;
            if midi_events.is_empty() { return Err("This MIDI has no playable events".to_string()); }
            let total_notes = midi_events.len();
            (Vec::new(), midi_events, duration_ms, total_notes)
        } else {
            let (keyboard_events, duration_ms) = parse_midi_events(&song_path)?;
            if keyboard_events.is_empty() { return Err("This MIDI has no notes that can be mapped to VRPiano keys".to_string()); }
            let total_notes = keyboard_events.len();
            (keyboard_events, Vec::new(), duration_ms, total_notes)
        };
        if total_notes == 0 {
            return Err("This MIDI has no notes that can be mapped to VRPiano keys".to_string());
        }

        let song_name = song_path
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or("Untitled")
            .to_string();

        let stop_flag = Arc::new(AtomicBool::new(false));
        let pause_flag;
        {
            let mut runtime = state
                .lock()
                .map_err(|_| "VRPiano state lock poisoned".to_string())?;
            if runtime.status.running {
                return Err("VRPiano is already playing".to_string());
            }
            runtime.stop = Some(stop_flag.clone());
            runtime.paused.store(false, Ordering::SeqCst);
            pause_flag = runtime.paused.clone();
            if let Ok(mut current) = runtime.speed.lock() {
                *current = speed;
            }
            runtime.status = VrpianoStatus {
                running: true,
                paused: false,
                song_name: song_name.clone(),
                song_path: song_path.to_string_lossy().to_string(),
                progress: 0.0,
                played_notes: 0,
                total_notes,
                duration_ms,
                elapsed_ms: 0,
                last_event: format!("Starting after {}s delay", request.delay_secs),
                last_error: String::new(),
                songs_dir: ensure_songs_dir(&app)?.to_string_lossy().to_string(),
                speed,
                hotkeys_enabled: runtime.hotkeys_enabled,
                hotkeys_available: cfg!(target_os = "windows"),
                last_hotkey: runtime.status.last_hotkey.clone(),
                last_hotkey_at_ms: runtime.status.last_hotkey_at_ms,
                midi_connected: runtime.status.midi_connected,
                midi_device_name: runtime.status.midi_device_name.clone(),
                recording: false,
                recorded_midi_path: None,
                channels: runtime.status.channels.clone(),
                voice_listening: false,
                tts_enabled: false,
                last_transcription: String::new(),
                vrchat_osc_enabled: false,
                vrchat_osc_host: String::new(),
                vrchat_osc_port: 9000,
                vrchat_osc_running: false,
                vrchat_osc_last_error: String::new(),
                vrchat_osc_connected: false,
            };
        }

        emit_status(&app, &state);
        let app_handle = app.clone();
        let state_inner = state.clone();
        let midi_backend = midi_backend.clone();

        if output_mode == "midi" {
            let (midi_events, _) = parse_midi_for_output(&song_path)?;
            let recorder = recorder.clone();
            thread::spawn(move || {
                run_midi_playback(
                    app_handle,
                    state_inner,
                    midi_backend,
                    recorder,
                    stop_flag,
                    pause_flag,
                    song_name,
                midi_events,
                    duration_ms,
                    request.delay_secs,
                );
            });
                return status_snapshot(&app, &state);
        }

        thread::spawn(move || {
            run_playback(
                app_handle,
                state_inner,
                stop_flag,
                pause_flag,
                song_name,
                keyboard_events,
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
            runtime.paused.store(false, Ordering::SeqCst);
            runtime.status.paused = false;
            runtime.status.last_event = "Stopping playback".to_string();
        } else {
            runtime.status.last_event = "Playback already stopped".to_string();
        }
    }
    emit_status(&app, &state);
    status_snapshot(&app, &state)
}

fn clear_playback_if_current(state: &Arc<Mutex<VrpianoRuntime>>, stop: &Arc<AtomicBool>) {
    if let Ok(mut runtime) = state.lock() {
        if runtime.stop.as_ref().is_some_and(|current| Arc::ptr_eq(current, stop)) {
            runtime.paused.store(false, Ordering::SeqCst);
            runtime.stop = None;
        }
    }
}

fn toggle_playback_pause(
    app: tauri::AppHandle,
    state: Arc<Mutex<VrpianoRuntime>>,
) -> Result<VrpianoStatus, String> {
    {
        let mut runtime = state
            .lock()
            .map_err(|_| "VRPiano state lock poisoned".to_string())?;
        if !runtime.status.running {
            runtime.status.paused = false;
            runtime.status.last_event = "Playback is not running".to_string();
        } else {
            let paused = !runtime.paused.load(Ordering::SeqCst);
            runtime.paused.store(paused, Ordering::SeqCst);
            runtime.status.paused = paused;
            runtime.status.last_event = if paused {
                "Playback paused".to_string()
            } else {
                "Playback resumed".to_string()
            };
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
    midi_backend: Arc<Mutex<MidiOutputBackend>>,
    recorder: Arc<Mutex<MidiRecorder>>,
    config: VrpianoHotkeyConfig,
) -> Result<VrpianoStatus, String> {
    #[cfg(target_os = "windows")]
    if config.enabled {
        start_global_hotkey_hook(app.clone(), state.clone(), midi_backend.clone(), recorder.clone())?;
    } else {
        stop_global_hotkey_hook();
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
        let output_mode = config.output_mode.trim().to_ascii_lowercase();
        runtime.active_engine = match output_mode.as_str() {
            "midi" | "midi_device" => "midi",
            "osc" | "vrchat_osc" => "osc",
            _ => "keyboard",
        }.to_string();
        runtime.vrchat_osc_enabled = output_mode == "osc" || output_mode == "vrchat_osc";
        if !config.osc_host.trim().is_empty() {
            runtime.vrchat_osc_host = config.osc_host.trim().to_string();
        }
        runtime.vrchat_osc_port = config.osc_port.clamp(1, 65535);
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
    midi_backend: Arc<Mutex<MidiOutputBackend>>,
    recorder: Arc<Mutex<MidiRecorder>>,
) -> Result<(), String> {
    let _ = HOTKEY_CONTEXT.set(GlobalHotkeyContext { app, state, midi_backend, recorder });

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
            use windows::Win32::System::Threading::GetCurrentThreadId;
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
                Ok(hook) => {
                    *HOTKEY_HOOK.get_or_init(init_hook_mutex).lock().unwrap() =
                        Some(HotkeyHook(hook));
                    *HOTKEY_THREAD_ID.get_or_init(init_tid_mutex).lock().unwrap() =
                        Some(unsafe { GetCurrentThreadId() });
                    let _ = tx.send(Ok(()));
                    let mut msg = MSG::default();
                    while unsafe { GetMessageW(&mut msg, HWND::default(), 0, 0) }.as_bool() {}
                    // Loop exited on WM_QUIT — release the stored hook handle.
                    *HOTKEY_HOOK.get_or_init(init_hook_mutex).lock().unwrap() = None;
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

/// Tear down the low-level keyboard hook and its message-pump thread. Called when
/// global hotkeys are disabled so the hook is actually removed (not just silenced
/// via `hotkeys_enabled`) — otherwise it intercepts every keystroke for the whole
/// process lifetime.
#[cfg(target_os = "windows")]
fn stop_global_hotkey_hook() {
    use windows::Win32::Foundation::{LPARAM, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        PostThreadMessageW, UnhookWindowsHookEx, WM_QUIT,
    };

    if HOTKEY_HOOK_STARTED
        .compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return;
    }
    // Signal the message-pump thread to exit (GetMessageW returns 0 on WM_QUIT).
    if let Some(tid) = *HOTKEY_THREAD_ID.get_or_init(init_tid_mutex).lock().unwrap() {
        unsafe {
            let _ = PostThreadMessageW(tid, WM_QUIT, WPARAM(0), LPARAM(0));
        }
    }
    // Remove the hook so it no longer intercepts keystrokes.
    if let Some(HotkeyHook(hook)) =
        HOTKEY_HOOK.get_or_init(init_hook_mutex).lock().unwrap().take()
    {
        unsafe {
            let _ = UnhookWindowsHookEx(hook);
        }
    }
    *HOTKEY_THREAD_ID.get_or_init(init_tid_mutex).lock().unwrap() = None;
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
    thread::spawn(move || {
        match vk {
            112 => {
                let (running, song_path, delay_secs, output_mode) = match context.state.lock() {
                    Ok(runtime) => (
                        runtime.status.running,
                        runtime.hotkey_song_path.clone(),
                        runtime.hotkey_delay_secs,
                        runtime.active_engine.clone(),
                    ),
                    Err(_) => return,
                };
                if running {
                    let _ = toggle_playback_pause(context.app.clone(), context.state.clone());
                } else if output_mode == "osc" {
                    let _ = begin_vrchat_osc(&context.app, &context.state, &song_path, delay_secs, current_speed(&context.state));
                } else {
                    let request = VrpianoStartRequest {
                        song_path,
                        delay_secs,
                        speed: current_speed(&context.state),
                        output_mode: output_mode.clone(),
                        midi_output_device: if output_mode == "midi" { context.midi_backend.lock().ok().and_then(|backend| backend.state().lock().ok().and_then(|status| status.device_id.clone())) } else { None },
                    };
                    let _ = start_playback(context.app.clone(), context.state.clone(), context.midi_backend.clone(), context.recorder.clone(), request);
                }
            }
            113 => {
                let (running, song_path, delay_secs, output_mode) = match context.state.lock() {
                    Ok(runtime) => (
                        runtime.status.running,
                        runtime.status.song_path.clone(),
                        runtime.hotkey_delay_secs,
                        runtime.active_engine.clone(),
                    ),
                    Err(_) => return,
                };
                if !song_path.is_empty() {
                if running {
                        let _ = stop_playback(context.app.clone(), context.state.clone());
                        for _ in 0..50 {
                            let stopped = context
                                .state
                                .lock()
                                .map(|runtime| !runtime.status.running)
                                .unwrap_or(true);
                            if stopped {
                                break;
                            }
                            thread::sleep(Duration::from_millis(20));
                        }
                    }

                    let stopped = context
                        .state
                        .lock()
                        .map(|runtime| !runtime.status.running)
                        .unwrap_or(false);
                    if stopped {
                        if output_mode == "osc" {
                            let _ = begin_vrchat_osc(&context.app, &context.state, &song_path, delay_secs, current_speed(&context.state));
                        } else {
                            let request = VrpianoStartRequest {
                            song_path,
                            delay_secs,
                            speed: current_speed(&context.state),
                            output_mode: output_mode.clone(),
                            midi_output_device: if output_mode == "midi" { context.midi_backend.lock().ok().and_then(|backend| backend.state().lock().ok().and_then(|status| status.device_id.clone())) } else { None },
                            };
                            let _ = start_playback(context.app.clone(), context.state.clone(), context.midi_backend.clone(), context.recorder.clone(), request);
                        }
                    }
                }
            }
            114 => {
                let next = current_speed(&context.state) + SPEED_STEP;
                let _ = set_playback_speed(context.app.clone(), context.state.clone(), next);
            }
            115 => {
                let next = current_speed(&context.state) - SPEED_STEP;
                let _ = set_playback_speed(context.app.clone(), context.state.clone(), next);
            }
            116 => {
                let _ = set_playback_speed(context.app.clone(), context.state.clone(), 1.0);
            }
            _ => {}
        }
        record_hotkey(&context.app, &context.state, vk);
    });
}

#[cfg(target_os = "windows")]
fn record_hotkey(app: &tauri::AppHandle, state: &Arc<Mutex<VrpianoRuntime>>, vk: u32) {
    update_runtime(state, |status| {
        status.last_hotkey = format!("F{}", vk.saturating_sub(111));
        status.last_hotkey_at_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
    });
    emit_status(app, state);
}

#[cfg(target_os = "windows")]
fn run_playback(
    app: tauri::AppHandle,
    state: Arc<Mutex<VrpianoRuntime>>,
    stop: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
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
            sleep_unscaled_interruptible(1_000, &stop, &paused);
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
            sleep_scaled_interruptible(wait_ms, &stop, &paused, &state);
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
            release_all(&active_keys);
            active_keys.clear();
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
        status.paused = false;
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
    clear_playback_if_current(&state, &stop);
    emit_status(&app, &state);
}

#[cfg(target_os = "windows")]
fn run_midi_playback(
    app: tauri::AppHandle,
    state: Arc<Mutex<VrpianoRuntime>>,
    midi_backend: Arc<Mutex<MidiOutputBackend>>,
    recorder: Arc<Mutex<MidiRecorder>>,
    stop: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
    song_name: String,
    events: Vec<MidiPlayEvent>,
    duration_ms: u64,
    delay_secs: u64,
) {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        for remaining in (1..=delay_secs).rev() {
            if stop.load(Ordering::SeqCst) {
                return;
            }
            update_runtime(&state, |status| {
                status.last_event = format!("MIDI Starting in {remaining}s");
            });
            emit_status_with_midi(&app, &state, &midi_backend);
            sleep_unscaled_interruptible(1_000, &stop, &paused);
        }

        let mut active_notes: HashSet<(u8, u8)> = HashSet::new();
        let mut last_at = 0_u64;
        let mut played = 0_usize;
        let mut index = 0_usize;

        while index < events.len() {
            if stop.load(Ordering::SeqCst) {
                break;
            }

            let at_ms = events[index].at_ms;
            let wait_ms = at_ms.saturating_sub(last_at);
            sleep_scaled_interruptible(wait_ms, &stop, &paused, &state);
            if stop.load(Ordering::SeqCst) {
                break;
            }

            let backend = midi_backend.lock().unwrap();
            if !backend.state().lock().unwrap().connected {
                drop(backend);
                update_runtime(&state, |status| {
                    status.last_error = "MIDI device disconnected".to_string();
                    status.running = false;
                });
                emit_status_with_midi(&app, &state, &midi_backend);
                return;
            }

            let transpose = current_transpose(&state);
                let mut notes_to_send = Vec::new();
                let mut notes_to_stop = Vec::new();
            let mut controls_to_send = Vec::new();
            while index < events.len() && events[index].at_ms == at_ms {
                let ev = &events[index];
                if let Some((cc, value)) = ev.control_change {
                    controls_to_send.push((ev.channel, cc, value));
                    played += 1;
                    index += 1;
                    continue;
                }
                let channel_state = get_channel_state(&state, ev.channel);
                let routed = is_channel_routed(&state, ev.channel);
                let solo_active = is_solo_active(&state);
                let sent_note = apply_transpose(ev.note, ev.channel, transpose);
                if ev.is_note_on {
                    let should_play = routed && !channel_state.muted && (!solo_active || channel_state.solo);
                    if should_play {
                        notes_to_send.push((sent_note, ev.velocity, ev.channel));
                    }
                    active_notes.insert((sent_note, ev.channel));
                } else {
                    active_notes.remove(&(sent_note, ev.channel));
                    notes_to_stop.push((sent_note, ev.channel));
                }
                if recorder.lock().unwrap().is_recording() {
                    let mut rec = recorder.lock().unwrap();
                    rec.record(at_ms, if ev.is_note_on { "note_on" } else { "note_off" }, ev.channel, &[ev.note, ev.velocity]);
                }
                played += 1;
                index += 1;
            }

            for (note, velocity, channel) in notes_to_send {
                let adjusted_velocity = adjust_velocity(velocity, get_channel_state(&state, channel).volume);
                if let Err(e) = backend.send_note_on(note, adjusted_velocity, channel) {
                    drop(backend);
                    update_runtime(&state, |status| {
                        status.last_error = format!("MIDI error: {e}");
                        status.running = false;
                    });
                    emit_status_with_midi(&app, &state, &midi_backend);
                    return;
                }
            }
            for (channel, cc, value) in controls_to_send {
                let _ = backend.send_control_change(channel, cc, value);
            }
            for (note, channel) in notes_to_stop {
                let _ = backend.send_note_off(note, channel);
            }

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
                status.last_event = format!("MIDI Playing {} at {:.2}x", song_name, status.speed);
            });
            emit_status_with_midi(&app, &state, &midi_backend);
            last_at = at_ms;
        }

        let backend = midi_backend.lock().unwrap();
        let _ = backend.send_panic();
    }));

    if result.is_err() {
        update_runtime(&state, |status| {
            status.last_error = "MIDI playback crashed unexpectedly".to_string();
        });
    }

    update_runtime(&state, |status| {
        status.running = false;
        status.paused = false;
        status.progress = if stop.load(Ordering::SeqCst) {
            status.progress
        } else {
            1.0
        };
        status.last_event = if stop.load(Ordering::SeqCst) {
            "MIDI Playback stopped".to_string()
        } else {
            "MIDI Playback finished".to_string()
        };
    });
    if !stop.load(Ordering::SeqCst) {
        maybe_advance_playlist(app.clone(), state.clone(), "midi", Some(midi_backend.clone()), Some(recorder.clone()));
    }
    clear_playback_if_current(&state, &stop);
    emit_status(&app, &state);
}

fn run_vrchat_osc_playback(
    app: tauri::AppHandle,
    state: Arc<Mutex<VrpianoRuntime>>,
    song_name: String,
    events: Vec<MidiPlayEvent>,
    duration_ms: u64,
    delay_secs: u64,
    host: String,
    port: u16,
    osc_mode: String,
    avatar_prefix: String,
    stop: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
) {
    use crate::osc::{osc_send_message_multi, OscArgument};

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        for remaining in (1..=delay_secs).rev() {
            if stop.load(Ordering::SeqCst) {
                return;
            }
            update_runtime(&state, |status| {
                status.last_event = format!("VRChat OSC Starting in {remaining}s");
            });
            emit_status(&app, &state);
            sleep_unscaled_interruptible(1_000, &stop, &paused);
        }

        let mut active_notes: HashSet<(u8, u8)> = HashSet::new();
        let mut last_at = 0_u64;
        let mut played = 0_usize;
        let mut index = 0_usize;

        while index < events.len() {
            if stop.load(Ordering::SeqCst) {
                break;
            }

            let at_ms = events[index].at_ms;
            let wait_ms = at_ms.saturating_sub(last_at);
            sleep_scaled_interruptible(wait_ms, &stop, &paused, &state);
            if stop.load(Ordering::SeqCst) {
                break;
            }

            let transpose = current_transpose(&state);
            let mut notes_to_send = Vec::new();
            let mut notes_to_release = Vec::new();
            let mut pitches_pressed_at_timestamp = HashSet::new();
            while index < events.len() && events[index].at_ms == at_ms {
                let ev = &events[index];
                if let Some((cc, _value)) = ev.control_change {
                    // VRChat's native keyboard piano has no sustain OSC parameter,
                    // so CC64 is ignored. All-notes-off (CC123/CC120) still releases
                    // every held key to avoid stuck notes (粘键).
                    if cc == 123 || cc == 120 {
                        send_osc_all_notes_off(&host, port, &active_notes, &osc_mode, &avatar_prefix);
                        active_notes.clear();
                    }
                    played += 1;
                    index += 1;
                    continue;
                }
                let channel_state = get_channel_state(&state, ev.channel);
                let routed = is_channel_routed(&state, ev.channel);
                let solo_active = is_solo_active(&state);
                let sent_note = apply_transpose(ev.note, ev.channel, transpose);
                if ev.is_note_on {
                    let should_play = routed
                        && !channel_state.muted
                        && (!solo_active || channel_state.solo);
                    if should_play {
                        let adjusted = adjust_velocity(ev.velocity, channel_state.volume);
                        notes_to_send.push((sent_note, adjusted, ev.channel));
                    }
                    pitches_pressed_at_timestamp.insert(sent_note);
                    active_notes.insert((sent_note, ev.channel));
                } else {
                    active_notes.remove(&(sent_note, ev.channel));
                    // The PianoKeys OSC address has no MIDI channel. Release a
                    // pitch only after all source channels have released it.
                    if !active_notes.iter().any(|(note, _)| *note == sent_note)
                        && !pitches_pressed_at_timestamp.contains(&sent_note)
                    {
                        notes_to_release.push((sent_note, ev.channel));
                    }
                }
                played += 1;
                index += 1;
            }

            // A NoteOff and NoteOn may share a timestamp. Release old pitches
            // first, but never release a pitch that is re-pressed in this group.
            for (note, _channel) in notes_to_release {
                let args = vec![OscArgument {
                    value_type: "float".to_string(),
                    value: serde_json::json!(0.0_f64),
                }];
                let address = osc_note_address(&osc_mode, &avatar_prefix, note);
                if let Err(e) = osc_send_message_multi(host.clone(), port, address, args) {
                    update_runtime(&state, |status| {
                        status.last_error = format!("VRChat OSC error: {}", e.message);
                        status.running = false;
                    });
                    send_osc_all_notes_off(&host, port, &active_notes, &osc_mode, &avatar_prefix);
                    emit_status(&app, &state);
                    return;
                }
            }

            for (note, velocity, _channel) in notes_to_send {
                // VRChat's native keyboard piano listens on /PianoKeys/<midi note>
                // with a float 0..1 press value (velocity / 127), exactly like
                // VRChat_MIDI_Player. Note-off (0.0) is sent by send_osc_all_notes_off.
                let args = vec![OscArgument {
                    value_type: "float".to_string(),
                    value: serde_json::json!((velocity as f64) / 127.0),
                }];
                let address = osc_note_address(&osc_mode, &avatar_prefix, note);
                if let Err(e) = osc_send_message_multi(host.clone(), port, address, args) {
                    update_runtime(&state, |status| {
                        status.last_error = format!("VRChat OSC error: {}", e.message);
                        status.running = false;
                    });
                    send_osc_all_notes_off(&host, port, &active_notes, &osc_mode, &avatar_prefix);
                    emit_status(&app, &state);
                    return;
                }
            }
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
                status.last_event = format!("VRChat OSC Playing {} at {:.2}x", song_name, status.speed);
            });
            emit_status(&app, &state);
            last_at = at_ms;
        }
        send_osc_all_notes_off(&host, port, &active_notes, &osc_mode, &avatar_prefix);
    }));

    if result.is_err() {
        update_runtime(&state, |status| {
            status.last_error = "VRChat OSC playback crashed unexpectedly".to_string();
        });
    }

    update_runtime(&state, |status| {
        status.running = false;
        status.paused = false;
        status.vrchat_osc_running = false;
        status.progress = if stop.load(Ordering::SeqCst) {
            status.progress
        } else {
            1.0
        };
        status.last_event = if stop.load(Ordering::SeqCst) {
            "VRChat OSC Playback stopped".to_string()
        } else {
            "VRChat OSC Playback finished".to_string()
        };
    });
    if !stop.load(Ordering::SeqCst) {
        stop.store(true, Ordering::SeqCst);
        maybe_advance_playlist(app.clone(), state.clone(), "osc", None, None);
    }
    clear_playback_if_current(&state, &stop);
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

fn parse_midi_for_output(path: &Path) -> Result<(Vec<MidiPlayEvent>, u64), String> {
    let bytes = fs::read(path).map_err(|e| format!("Failed to read MIDI: {e}"))?;
    let smf = Smf::parse(&bytes).map_err(|e| format!("Invalid MIDI file: {e}"))?;
    let ticks_per_beat = match smf.header.timing {
        Timing::Metrical(ticks) => u64::from(ticks.as_int()),
        Timing::Timecode(_, _) => {
            return Err("SMPTE timecode MIDI files are not supported yet".to_string())
        }
    };

    let tempo_map = collect_tempo_map(&smf);
    let mut grouped: BTreeMap<u64, Vec<MidiPlayEvent>> = BTreeMap::new();

    for track in &smf.tracks {
        let mut tick = 0_u64;
        for event in track {
            tick = tick.saturating_add(u64::from(event.delta.as_int()));
            if let TrackEventKind::Midi { channel, message } = event.kind {
                match message {
                    MidiMessage::NoteOn { key, vel } => {
                        let note = key.as_int();
                        let micros = tick_to_micros(tick, &tempo_map, ticks_per_beat);
                        let at_ms = (micros as f64 / 1000.0).round().max(0.0) as u64;
                        grouped
                            .entry(at_ms)
                            .or_default()
                            .push(MidiPlayEvent {
                                at_ms,
                                note,
                                velocity: vel.as_int(),
                                channel: channel.as_int(),
                                is_note_on: vel.as_int() != 0,
                                control_change: None,
                            });
                    }
                    MidiMessage::NoteOff { key, .. } => {
                        let note = key.as_int();
                        let micros = tick_to_micros(tick, &tempo_map, ticks_per_beat);
                        let at_ms = (micros as f64 / 1000.0).round().max(0.0) as u64;
                        grouped
                            .entry(at_ms)
                            .or_default()
                            .push(MidiPlayEvent {
                                at_ms,
                                note,
                                velocity: 0,
                                channel: channel.as_int(),
                                is_note_on: false,
                                control_change: None,
                            });
                    }
                    MidiMessage::ProgramChange { program, .. } => {
                        let micros = tick_to_micros(tick, &tempo_map, ticks_per_beat);
                        let at_ms = (micros as f64 / 1000.0).round().max(0.0) as u64;
                        grouped
                            .entry(at_ms)
                            .or_default()
                            .push(MidiPlayEvent {
                                at_ms,
                                note: program.as_int(),
                                velocity: 0,
                                channel: channel.as_int(),
                                is_note_on: false,
                                control_change: None,
                            });
                    }
                    MidiMessage::Controller { controller, value } => {
                        let micros = tick_to_micros(tick, &tempo_map, ticks_per_beat);
                        let at_ms = (micros as f64 / 1000.0).round().max(0.0) as u64;
                        grouped
                            .entry(at_ms)
                            .or_default()
                            .push(MidiPlayEvent {
                                at_ms,
                                note: 0,
                                velocity: value.as_int(),
                                channel: channel.as_int(),
                                is_note_on: false,
                                control_change: Some((controller.as_int(), value.as_int())),
                            });
                    }
                    _ => {}
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
    paused: &AtomicBool,
    state: &Arc<Mutex<VrpianoRuntime>>,
) {
    let mut remaining = music_ms as f64;
    while remaining > 0.0 && !stop.load(Ordering::SeqCst) {
        if paused.load(Ordering::SeqCst) {
            thread::sleep(Duration::from_millis(20));
            continue;
        }
        let speed = current_speed(state).max(0.25);
        let real_chunk = (remaining / speed).ceil().clamp(5.0, 20.0) as u64;
        thread::sleep(Duration::from_millis(real_chunk));
        remaining -= real_chunk as f64 * speed;
    }
}

fn sleep_unscaled_interruptible(millis: u64, stop: &AtomicBool, paused: &AtomicBool) {
    let mut remaining = millis;
    while remaining > 0 && !stop.load(Ordering::SeqCst) {
        if paused.load(Ordering::SeqCst) {
            thread::sleep(Duration::from_millis(20));
            continue;
        }
        let chunk = remaining.min(20);
        thread::sleep(Duration::from_millis(chunk));
        remaining -= chunk;
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

fn get_channel_state(state: &Arc<Mutex<VrpianoRuntime>>, channel: u8) -> ChannelState {
    state
        .lock()
        .ok()
        .map(|runtime| runtime.status.channels[channel as usize].clone())
        .unwrap_or_default()
}

fn is_solo_active(state: &Arc<Mutex<VrpianoRuntime>>) -> bool {
    state
        .lock()
        .ok()
        .map(|runtime| runtime.status.channels.iter().any(|c| c.solo))
        .unwrap_or(false)
}

fn adjust_velocity(velocity: u8, channel_volume: u8) -> u8 {
    let scaled = (velocity as u16 * channel_volume as u16) / 127;
    scaled.min(127).max(0) as u8
}

/// Apply global transposition. The drum channel (index 9) is never transposed so
/// percussion stays in place, matching APS-NoteCast behaviour.
fn apply_transpose(note: u8, channel: u8, transpose: i8) -> u8 {
    if channel == 9 {
        return note;
    }
    let shifted = note as i16 + transpose as i16;
    shifted.clamp(0, 127) as u8
}

fn current_transpose(state: &Arc<Mutex<VrpianoRuntime>>) -> i8 {
    state.lock().ok().map(|runtime| runtime.transpose).unwrap_or(0)
}

fn is_channel_routed(state: &Arc<Mutex<VrpianoRuntime>>, channel: u8) -> bool {
    state
        .lock()
        .ok()
        .map(|runtime| runtime.piano_channels[channel as usize])
        .unwrap_or(true)
}

/// Release every currently-held note on VRChat's native keyboard piano so it
/// never gets stuck holding keys (prevents "粘键").
fn osc_note_address(mode: &str, avatar_prefix: &str, note: u8) -> String {
    if mode.eq_ignore_ascii_case("avatar") {
        format!("{}/{}", avatar_prefix.trim_end_matches('/'), note)
    } else {
        format!("/PianoKeys/{}", note)
    }
}

fn send_osc_all_notes_off(
    host: &str,
    port: u16,
    notes: &HashSet<(u8, u8)>,
    mode: &str,
    avatar_prefix: &str,
) {
    use crate::osc::{osc_send_message_multi, OscArgument};

    for (note, _channel) in notes {
        let args = vec![OscArgument {
            value_type: "float".to_string(),
            value: serde_json::json!(0.0_f64),
        }];
        let _ = osc_send_message_multi(host.to_string(), port, osc_note_address(mode, avatar_prefix, *note), args);
    }
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
        status.paused = runtime.paused.load(Ordering::SeqCst) && status.running;
        let _ = app.emit("vrpiano_status", status);
    }
}

fn emit_status_with_midi(
    app: &tauri::AppHandle,
    state: &Arc<Mutex<VrpianoRuntime>>,
    midi_backend: &Arc<Mutex<MidiOutputBackend>>,
) {
    if let Ok(runtime) = state.lock() {
        let mut status = runtime.status.clone();
        status.speed = runtime
            .speed
            .lock()
            .map(|speed| *speed)
            .unwrap_or(status.speed);
        status.hotkeys_enabled = runtime.hotkeys_enabled;
        status.hotkeys_available = cfg!(target_os = "windows");
        status.paused = runtime.paused.load(Ordering::SeqCst) && status.running;
        if let Ok(backend) = midi_backend.lock() {
            let state_arc = backend.state();
            let midi_state = state_arc.lock().unwrap();
            status.midi_connected = midi_state.connected;
            status.midi_device_name = midi_state.device_name.clone();
        }
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
        status.paused = runtime.paused.load(Ordering::SeqCst) && status.running;
    }
    Ok(status)
}

fn status_with_dir(app: &tauri::AppHandle, event: &str) -> Result<VrpianoStatus, String> {
    Ok(VrpianoStatus {
        running: false,
        paused: false,
        song_name: String::new(),
        song_path: String::new(),
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
        last_hotkey: String::new(),
        last_hotkey_at_ms: 0,
        midi_connected: false,
        midi_device_name: None,
        recording: false,
        recorded_midi_path: None,
        channels: [ChannelState::default(); 16],
        voice_listening: false,
        tts_enabled: false,
        last_transcription: String::new(),
        vrchat_osc_enabled: false,
        vrchat_osc_host: String::new(),
        vrchat_osc_port: 9000,
        vrchat_osc_running: false,
        vrchat_osc_last_error: String::new(),
        vrchat_osc_connected: false,
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

fn filename_from_content_disposition(value: &str) -> Option<String> {
    value.split(';').find_map(|part| {
        let (name, filename) = part.trim().split_once('=')?;
        if !name.trim().eq_ignore_ascii_case("filename") {
            return None;
        }
        let filename = filename.trim().trim_matches(['"', '\'']);
        let filename = sanitize_filename(filename);
        (!filename.is_empty()).then_some(filename)
    })
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

fn looks_like_direct_midi_url(input: &str) -> bool {
    let path = input
        .split(['?', '#'])
        .next()
        .unwrap_or(input)
        .to_ascii_lowercase();
    path.ends_with(".mid") || path.ends_with(".midi")
}

fn extract_midishow_id(input: &str) -> Result<u64, String> {
    regex_patterns::extract_number()
        .captures(input)
        .and_then(|captures| captures.get(1))
        .and_then(|id| id.as_str().parse::<u64>().ok())
        .ok_or_else(|| "Invalid Midishow ID or URL".to_string())
}

async fn search_midishow(
    app: &tauri::AppHandle,
    project_path: &Path,
    keyword: &str,
    limit: usize,
) -> Result<Vec<VrpianoOnlineSong>, String> {
    // The CLI creates a new cookie jar for every request, so it cannot use the
    // account or browser Cookie that the user saved in VRPiano. Search through
    // the application session first and only keep the CLI as a public fallback.
    let account = default_midishow_account(app)?;
    match tokio::time::timeout(
        Duration::from_secs(MIDISHOW_SEARCH_HTTP_TIMEOUT_SECS),
        search_midishow_http(keyword, limit, account.as_ref()),
    )
    .await
    {
        Ok(Ok(results)) => Ok(results),
        Ok(Err(request_error)) => match run_midishow_cli_json_with_timeout(
            project_path,
            &["search", keyword],
            Duration::from_secs(MIDISHOW_SEARCH_CLI_TIMEOUT_SECS),
        ) {
            Ok(value) => parse_midishow_results(value, limit),
            Err(cli_error) => Err(format!("{request_error}; CLI fallback failed: {cli_error}")),
        },
        Err(_) => {
            Err("Midishow 搜索超时，请检查代理连接，或点击右侧按钮在浏览器打开官方搜索".to_string())
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
        let cover_url = item
            .get("cover_url")
            .and_then(|value| value.as_str())
            .map(str::to_string)
            .filter(|value| is_midishow_cover_url(value));
        results.push(VrpianoOnlineSong {
            id,
            title,
            artist,
            page_url,
            cover_url,
        });
        if results.len() >= limit {
            break;
        }
    }
    Ok(results)
}

async fn search_midishow_http(
    keyword: &str,
    limit: usize,
    account: Option<&StoredMidishowAccount>,
) -> Result<Vec<VrpianoOnlineSong>, String> {
    let client = midishow_client()?;
    let cookie = account
        .map(|account| account.cookie.trim())
        .filter(|cookie| !cookie.is_empty());
    if account.is_some() && cookie.is_none() {
        return Err("Midishow 登录状态已失效，请重新登录".to_string());
    }

    let response = with_midishow_cookie(
        client
            .get("https://www.midishow.com/search/result")
            .query(&[("q", keyword), ("page", "1"), ("per-page", "50")])
            .header(reqwest::header::ACCEPT, "text/html,application/xhtml+xml")
            .header(reqwest::header::ACCEPT_LANGUAGE, "zh-CN,zh;q=0.9,en;q=0.8")
            .header(reqwest::header::REFERER, "https://www.midishow.com/"),
        cookie,
    )
    .send()
    .await
    .map_err(|_| "暂时无法搜索 Midishow 曲库，请稍后重试".to_string())?;
    let status = response.status();
    let html = response
        .text()
        .await
        .map_err(|_| "暂时无法读取 Midishow 搜索结果，请稍后重试".to_string())?;
    if is_cloudflare_challenge(&html) || !status.is_success() {
        return Err("当前搜索未完成，请重新登录后再试".to_string());
    }

    let mut results = Vec::new();
    let mut seen = HashSet::new();

    // Strategy 1: Parse data-key attributes for structured results.
    for captures in regex_patterns::data_key().captures_iter(&html) {
        let Some(id) = captures
            .get(1)
            .and_then(|value| value.as_str().parse::<u64>().ok())
        else {
            continue;
        };
        if !seen.insert(id) {
            continue;
        }
        // Try to extract title from nearby HTML context
        let title = extract_search_title_from_html(&html, id);
        let artist = extract_search_artist_from_html(&html, id);
        let cover_url = extract_search_cover_from_html(&html, id);
        results.push(VrpianoOnlineSong {
            id,
            title,
            artist,
            page_url: format!("https://www.midishow.com/en/midi/{id}.html"),
            cover_url,
        });
        if results.len() >= limit {
            break;
        }
    }

    // Strategy 2: Fallback to URL extraction.
    if results.is_empty() {
        let re = regex_patterns::midi_id_from_url();
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
                cover_url: None,
            });
            if results.len() >= limit {
                break;
            }
        }
    }

    Ok(results)
}

/// Try to extract a MIDI title from HTML context around a given ID.
#[allow(dead_code, unused_variables)]
fn extract_title_from_html(html: &str, midi_id: u64) -> String {
    // Look for patterns like: <a href="/en/midi/12345.html">Title</a>
    let link_pattern = format!(r#"/en/midi/{}\.html"[^>]*>([^<]+)<"#, midi_id);
    if let Ok(re) = regex::Regex::new(&link_pattern) {
        if let Some(caps) = re.captures(html) {
            if let Some(title_match) = caps.get(1) {
                let title = clean_midishow_text(title_match.as_str());
                if !title.is_empty() && title != format!("MIDI #{midi_id}") {
                    return title;
                }
            }
        }
    }
    // Fallback: look for data-key附近的内容
    let id_str = midi_id.to_string();
    if let Some(pos) = html.find(&format!("data-key=\"{}\"", midi_id)) {
        // Search forward for link text
        let window = &html[pos..std::cmp::min(pos + 2000, html.len())];
        if let Some(link_start) = window.find('>') {
            let after_link = &window[link_start + 1..];
            if let Some(link_end) = after_link.find('<') {
                let text = after_link[..link_end].trim();
                if !text.is_empty() && text.len() > 2 && text.len() < 200 {
                    return clean_midishow_text(text);
                }
            }
        }
    }
    format!("MIDI #{midi_id}")
}

/// Try to extract artist from HTML context around a given ID.
#[allow(dead_code)]
fn extract_artist_from_html(html: &str, midi_id: u64) -> String {
    let id_str = midi_id.to_string();
    if let Some(pos) = html.find(&format!("data-key=\"{}\"", id_str)) {
        // Look for artist/author class patterns near the MIDI entry
        let window = &html[pos..std::cmp::min(pos + 3000, html.len())];
        for pattern in &["artist", "author", "uploader"] {
            if let Some(class_pos) = window.find(pattern) {
                let after_class = &window[class_pos..];
                if let Some(tag_end) = after_class.find('>') {
                    let content = &after_class[tag_end + 1..];
                    if let Some(text_end) = content.find('<') {
                        let text = content[..text_end].trim();
                        if !text.is_empty() && text.len() > 1 && text.len() < 100 {
                            return clean_midishow_text(text);
                        }
                    }
                }
            }
        }
    }
    String::new()
}

/// 从 html 中 `pos` 处开始截取最多 `span` 字节的窗口切片。
/// `pos` 来自 `str::find`，是合法字符边界；但 `pos + span` 是字节偏移，
/// 可能落在多字节 UTF-8 字符（如中文）中间，直接使用 `&html[pos..end]`
/// 会触发 `byte index is not a char boundary` panic。这里把结束索引
/// 向下取整到最近的字符边界，避免 panic。
fn safe_html_window(html: &str, pos: usize, span: usize) -> &str {
    let end = (pos + span).min(html.len());
    let end = html.floor_char_boundary(end);
    &html[pos..end]
}

fn extract_search_title_from_html(html: &str, midi_id: u64) -> String {
    let patterns = [
        format!(
            r#"(?is)<a[^>]+href=["'][^"']*/(?:en/)?midi/{}\.html[^"']*["'][^>]*>(.*?)</a>"#,
            midi_id
        ),
        format!(
            r#"(?is)<a[^>]+href=["'][^"']*/midi/{}[^"']*["'][^>]*>(.*?)</a>"#,
            midi_id
        ),
    ];

    for pattern in patterns {
        if let Ok(re) = regex::Regex::new(&pattern) {
            if let Some(text) = re
                .captures(html)
                .and_then(|captures| captures.get(1))
                .map(|value| clean_midishow_text(&strip_html(value.as_str())))
                .filter(|value| !value.is_empty())
            {
                return text;
            }
        }
    }

    let id_attr = format!(r#"data-key="{}""#, midi_id);
    if let Some(pos) = html.find(&id_attr) {
        let window = safe_html_window(html, pos, 3500);
        for pattern in [
            r#"(?is)<a[^>]+href=["'][^"']*(?:en/)?midi/\d+\.html[^"']*["'][^>]*>(.*?)</a>"#,
            r#"(?is)<h[1-6][^>]*>(.*?)</h[1-6]>"#,
        ] {
            if let Ok(re) = regex::Regex::new(pattern) {
                if let Some(text) = re
                    .captures(window)
                    .and_then(|captures| captures.get(1))
                    .map(|value| clean_midishow_text(&strip_html(value.as_str())))
                    .filter(|value| !value.is_empty())
                {
                    return text;
                }
            }
        }
    }

    format!("MIDI #{midi_id}")
}

fn extract_search_artist_from_html(html: &str, midi_id: u64) -> String {
    let id_attr = format!(r#"data-key="{}""#, midi_id);
    let Some(pos) = html.find(&id_attr) else {
        return String::new();
    };
    let window = safe_html_window(html, pos, 3500);
    let Ok(re) = regex::Regex::new(
        r#"(?is)<[^>]+class=["'][^"']*(?:artist|author|uploader)[^"']*["'][^>]*>(.*?)</[^>]+>"#,
    ) else {
        return String::new();
    };

    for captures in re.captures_iter(window) {
        if let Some(value) = captures.get(1) {
            let artist = clean_midishow_text(&strip_html(value.as_str()));
            if !artist.is_empty() && artist.len() < 100 {
                return artist;
            }
        }
    }

    String::new()
}

/// 从搜索结果 HTML 中提取与 `midi_id` 对应的封面图 URL。
///
/// Midishow 的搜索列表卡片常用以下结构之一：
///   * `<a data-key="{id}" href="..."><img data-src="..."></a>`
///   * `<img data-key="{id}" src="..." class="thumb">`
///   * `<div class="..." data-key="{id}"><img src="..."></div>`
///
/// 我们先抓 `data-key="{id}"` 所在的标签本身，再向后再扫一段窗口，
/// 依次尝试 `data-src` → `data-original` → `src`，找到的第一个
/// 经过 `is_midishow_cover_url` 校验的 URL 即返回。
fn extract_search_cover_from_html(html: &str, midi_id: u64) -> Option<String> {
    let id_attr = format!(r#"data-key="{}""#, midi_id);
    let pos = html.find(&id_attr)?;
    let window = safe_html_window(html, pos, 4000);

    // 1) 直接抓 data-key 所在标签上的 data-src / data-original / src
    let tag_pattern = format!(
        r#"(?is)<[a-zA-Z]+\b[^>]*data-key=["']{0}["'][^>]*>"#,
        midi_id
    );
    if let Ok(re) = regex::Regex::new(&tag_pattern) {
        if let Some(tag_match) = re.find(window) {
            let tag = tag_match.as_str();
            for attr in ["data-src", "data-original", "src"] {
                if let Some(url) = extract_html_attr(tag, attr) {
                    let url = decode_html_entities(&url);
                    if is_midishow_cover_url(&url) {
                        return Some(url);
                    }
                }
            }
        }
    }

    // 2) 在 data-key 之后的窗口里找第一个合法的 <img> URL。
    //    优先 lazy-load 属性（data-src / data-original），其次 src。
    for attr in ["data-src", "data-original", "src"] {
        let pattern = format!(
            r#"(?is)<img[^>]+{attr}=["']([^"']+)["'][^>]*>"#,
            attr = regex::escape(attr)
        );
        let Ok(re) = regex::Regex::new(&pattern) else {
            continue;
        };
        if let Some(captures) = re.captures(window) {
            if let Some(value) = captures.get(1) {
                let url = decode_html_entities(value.as_str());
                if is_midishow_cover_url(&url) {
                    return Some(url);
                }
            }
        }
    }

    None
}

/// 仅允许 midishow / midishowstatic 域名的图片 URL，避免被滥用为通用加载器。
/// 优先要求 URL 自带图片后缀，否则要求 host 命中可信静态域。
fn is_midishow_cover_url(url: &str) -> bool {
    let lower = url.trim().to_ascii_lowercase();
    if !(lower.starts_with("http://") || lower.starts_with("https://")) {
        return false;
    }
    if lower.contains(".jpg")
        || lower.contains(".jpeg")
        || lower.contains(".png")
        || lower.contains(".webp")
        || lower.contains("/images/")
        || lower.contains("/uploads/")
        || lower.contains("/thumbs/")
    {
        return true;
    }
    lower.contains("midishow.com") || lower.contains("midishowstatic.com")
}

fn strip_html(value: &str) -> String {
    let without_tags = regex_patterns::html_tag().replace_all(value, " ");
    decode_html_entities(&without_tags)
}

fn decode_html_entities(value: &str) -> String {
    value
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
}

/// Normalize a proxy host:port pair into a full http(s):// URL.
fn normalize_proxy(value: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        return String::new();
    }
    if value.starts_with("http://") || value.starts_with("https://") {
        value.to_string()
    } else {
        format!("http://{value}")
    }
}

/// Parse a Windows `ProxyServer` string (e.g. `127.0.0.1:7890` or
/// `http=127.0.0.1:7890;https=127.0.0.1:7890;socks=127.0.0.1:7891`) into the
/// HTTPS proxy URL we want to use.
#[cfg(windows)]
fn parse_proxy_server(server: &str) -> Option<String> {
    let server = server.trim();
    if server.is_empty() {
        return None;
    }
    if server.contains('=') {
        let mut http_proxy = None;
        for part in server.split(';') {
            let mut it = part.splitn(2, '=');
            let (proto, value) = match (it.next(), it.next()) {
                (Some(p), Some(v)) => (p.trim().to_ascii_lowercase(), v.trim()),
                _ => continue,
            };
            if proto == "https" {
                return Some(normalize_proxy(value));
            }
            if proto == "http" {
                http_proxy = Some(normalize_proxy(value));
            }
        }
        // No explicit https entry: fall back to the http one.
        http_proxy
    } else {
        Some(normalize_proxy(server))
    }
}

/// Read the Windows system proxy (Internet Settings / WinHTTP), which is what
/// WebView2 uses for Midishow login. reqwest and node do NOT read this
/// automatically, so we surface it here. Returns `(https_proxy_url, bypass_list)`.
#[cfg(windows)]
fn read_system_proxy() -> Option<(String, Vec<String>)> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu
        .open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings")
        .ok()?;
    let enabled: u32 = key.get_value("ProxyEnable").ok()?;
    if enabled == 0 {
        return None;
    }
    let server: String = key.get_value("ProxyServer").ok()?;
    if server.trim().is_empty() {
        return None;
    }
    let override_list: String = key.get_value("ProxyOverride").unwrap_or_default();
    let bypass: Vec<String> = override_list
        .split(';')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    Some((server, bypass))
}

#[cfg(windows)]
fn system_proxy() -> Option<(String, Vec<String>)> {
    let (server, bypass) = read_system_proxy()?;
    parse_proxy_server(&server).map(|https| (https, bypass))
}

#[cfg(not(windows))]
fn system_proxy() -> Option<(String, Vec<String>)> {
    // On non-Windows platforms, honor explicit proxy env vars (reqwest's
    // default behavior); we only add bypass plumbing for consistency.
    std::env::var("HTTPS_PROXY")
        .or_else(|_| std::env::var("https_proxy"))
        .or_else(|_| std::env::var("HTTP_PROXY"))
        .or_else(|_| std::env::var("http_proxy"))
        .ok()
        .map(|url| (normalize_proxy(&url), Vec::new()))
}

/// Whether a destination host matches a Windows proxy-override pattern
/// (`localhost`, `127.*`, `192.168.*`, `*.example.com`, `<local>`).
fn proxy_should_bypass(pattern: &str, host: &str) -> bool {
    let pat = pattern.trim().to_ascii_lowercase();
    let host = host.to_ascii_lowercase();
    if pat == "<local>" {
        return !host.contains('.') || host.ends_with(".local") || host == "localhost";
    }
    if let Some(suffix) = pat.strip_prefix("*.") {
        return host == suffix || host.ends_with(&format!(".{suffix}"));
    }
    if let Some(prefix) = pat.strip_suffix(".*") {
        return host.starts_with(prefix);
    }
    pat == host
}

fn midishow_client() -> Result<reqwest::Client, String> {
    let mut builder = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36")
        .cookie_store(true)
        .redirect(reqwest::redirect::Policy::limited(10))
        .timeout(Duration::from_secs(35));

    // Midishow login uses the WebView2 network stack (which honors the Windows
    // system proxy), but reqwest does not. Without this, every search/download
    // request to midishow.com hangs until timeout behind a proxy.
    if let Some((proxy_url, bypass)) = system_proxy() {
        builder = builder.proxy(reqwest::Proxy::custom(move |url| {
            let host = url.host_str().unwrap_or("");
            if bypass.iter().any(|b| proxy_should_bypass(b, host)) {
                None
            } else {
                Some(proxy_url.clone())
            }
        }));
    }

    builder
        .build()
        .map_err(|e| format!("Failed to create Midishow client: {e}"))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MidishowLoginSignal {
    Ready,
    Submitted,
    CredentialsRejected,
    NeedsConfirmation,
    FormMissing,
}

fn current_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn is_midishow_login_url(url: &str) -> bool {
    url.starts_with("https://www.midishow.com/") && url.contains("/user/account/login")
}

fn update_midishow_login_runtime(status: &VrpianoMidishowLoginStatus) {
    if let Ok(mut runtime) = midishow_login_runtime().lock() {
        runtime.state.clone_from(&status.state);
        runtime.message.clone_from(&status.message);
    }
}

fn current_midishow_login_status() -> VrpianoMidishowLoginStatus {
    let runtime = midishow_login_runtime().lock().ok();
    let state = runtime
        .as_ref()
        .map(|value| value.state.clone())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "idle".to_string());
    let message = runtime
        .as_ref()
        .map(|value| value.message.clone())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "等待登录".to_string());
    VrpianoMidishowLoginStatus {
        state,
        message,
        username: None,
    }
}

fn midishow_login_started_at_ms() -> u64 {
    midishow_login_runtime()
        .lock()
        .map(|runtime| runtime.started_at_ms)
        .unwrap_or_default()
}

fn finish_midishow_login(
    app: &tauri::AppHandle,
    state: &str,
    message: &str,
) -> VrpianoMidishowLoginStatus {
    let status = VrpianoMidishowLoginStatus {
        state: state.to_string(),
        message: message.to_string(),
        username: None,
    };
    update_midishow_login_runtime(&status);
    let _ = app.emit("vrpiano_midishow_login_status", status.clone());
    status
}

fn parse_midishow_login_title(title: &str) -> Option<MidishowLoginSignal> {
    let signal = title.strip_prefix(MIDISHOW_LOGIN_TITLE_PREFIX)?;
    match signal.trim().to_ascii_lowercase().as_str() {
        "ready" => Some(MidishowLoginSignal::Ready),
        "submitted" => Some(MidishowLoginSignal::Submitted),
        "credentials_rejected" => Some(MidishowLoginSignal::CredentialsRejected),
        "needs_confirmation" => Some(MidishowLoginSignal::NeedsConfirmation),
        "form_missing" => Some(MidishowLoginSignal::FormMissing),
        _ => None,
    }
}

fn midishow_login_monitor_script() -> String {
    format!(
        r#"(() => {{
  if (window.__vrcdogMidishowLoginMonitorStarted) return;
  window.__vrcdogMidishowLoginMonitorStarted = true;
  const signal = (value) => {{ document.title = '{MIDISHOW_LOGIN_TITLE_PREFIX}' + value; }};
  const text = () => ((document.body?.innerText || '') + ' ' + (document.title || '')).replace(/\s+/g, ' ').trim();
  const hasAny = (source, patterns) => patterns.some((pattern) => source.includes(pattern));
  const inspect = () => {{
    const pageText = text();
    if (hasAny(pageText, ['用户名或密码错误', '账号或密码错误', '密码错误', '账户不存在', '用户不存在', '登录信息有误'])) {{
      signal('credentials_rejected');
      return true;
    }}
    if (hasAny(pageText, ['验证码', '滑动验证', '安全确认', '人机验证', '请完成验证'])) {{
      signal('needs_confirmation');
      return true;
    }}
    // Cloudflare / DDoS challenge shown after a navigation (e.g. the login POST
    // bounces to an interstitial). Surface it for manual completion.
    if (hasAny(pageText, ['Just a moment', 'Verify you are human', 'Checking your browser', 'cf-chl', 'cf-mitigated', 'cdn-cgi', 'Enable JavaScript', 'Attention Required', 'DDoS'])) {{
      signal('needs_confirmation');
      return true;
    }}
    return false;
  }};
  if (inspect()) return;
  const observer = new MutationObserver(() => {{
    if (inspect()) observer.disconnect();
  }});
  if (document.body) observer.observe(document.body, {{ childList: true, subtree: true, characterData: true }});
  window.setTimeout(() => observer.disconnect(), 75000);
}})();"#
    )
}

fn midishow_login_script(account_js: &str, password_js: &str) -> String {
    format!(
        r#"(() => {{
  if (window.__vrcdogMidishowLoginStarted) return;
  window.__vrcdogMidishowLoginStarted = true;
  const account = {account_js};
  let password = {password_js};
  let attempts = 0;
  let submitted = false;
  const signal = (value) => {{ document.title = '{MIDISHOW_LOGIN_TITLE_PREFIX}' + value; }};
  const text = () => ((document.body?.innerText || '') + ' ' + (document.title || '')).replace(/\s+/g, ' ').trim();
  const hasAny = (source, patterns) => patterns.some((pattern) => source.includes(pattern));
  const findVisible = (selectors) => selectors
    .map((selector) => document.querySelector(selector))
    .find((element) => element && element instanceof HTMLInputElement && !element.disabled && element.offsetParent !== null);
  const fill = (element, value) => {{
    if (!element) return false;
    const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')?.set;
    if (setter) setter.call(element, value); else element.value = value;
    element.dispatchEvent(new Event('input', {{ bubbles: true }}));
    element.dispatchEvent(new Event('change', {{ bubbles: true }}));
    return true;
  }};
  signal('ready');
  // Wall-clock budget for the auto-fill loop. Independent of the polling interval
  // so the form_missing timeout doesn't shrink if we tune the interval down.
  const startMs = Date.now();
  const FORM_MISSING_BUDGET_MS = 20000;
  const timer = window.setInterval(() => {{
    attempts += 1;
    const pageText = text();
    if (hasAny(pageText, ['用户名或密码错误', '账号或密码错误', '密码错误', '账户不存在', '用户不存在', '登录信息有误'])) {{
      password = '';
      window.clearInterval(timer);
      signal('credentials_rejected');
      return;
    }}
    if (hasAny(pageText, ['验证码', '滑动验证', '安全确认', '人机验证', '请完成验证'])) {{
      window.clearInterval(timer);
      signal('needs_confirmation');
      return;
    }}
    // Cloudflare / DDoS protection challenge. After submitting, midishow.com may
    // bounce to a "Just a moment…" interstitial. We cannot solve it for the user,
    // so surface the window and let them complete it manually instead of timing out.
    if (hasAny(pageText, ['Just a moment', 'Verify you are human', 'Checking your browser', 'cf-chl', 'cf-mitigated', 'cdn-cgi', 'Enable JavaScript', 'Attention Required', 'DDoS'])) {{
      window.clearInterval(timer);
      signal('needs_confirmation');
      // 用户在弹出的窗口里解开 CF 后，登录表单会重新出现。这里挂一个 MutationObserver
      // 监听表单出现并自动回填+提交，这样用户不用手动再输一遍账号密码。
      const cfObserver = new MutationObserver(() => {{
        const acc = findVisible([
          'input[autocomplete="username"]',
          'input[name="username"]',
          'input[name="email"]',
          'input[type="email"]',
          'input[name="login"]',
          'input[type="text"]'
        ]);
        const pw = findVisible([
          'input[autocomplete="current-password"]',
          'input[name="password"]',
          'input[type="password"]'
        ]);
        if (acc && pw) {{
          cfObserver.disconnect();
          fill(acc, account);
          fill(pw, password);
          const form = pw.form || acc.form || document.querySelector('form');
          const submit = form?.querySelector('button[type="submit"], input[type="submit"]');
          signal('submitted');
          window.setTimeout(() => {{
            if (submit instanceof HTMLElement) submit.click();
            else if (form instanceof HTMLFormElement) form.requestSubmit();
            password = '';
          }}, 80);
        }}
      }});
      if (document.body) cfObserver.observe(document.body, {{ childList: true, subtree: true, characterData: true }});
      window.setTimeout(() => cfObserver.disconnect(), MIDISHOW_LOGIN_CONFIRM_TIMEOUT_MS);
      return;
    }}
    if (!location.pathname.includes('/user/account/login')) {{
      password = '';
      window.clearInterval(timer);
      return;
    }}
    if (submitted) return;
    const accountInput = findVisible([
      'input[autocomplete="username"]',
      'input[name="username"]',
      'input[name="email"]',
      'input[type="email"]',
      'input[name="login"]',
      'input[type="text"]'
    ]);
    const passwordInput = findVisible([
      'input[autocomplete="current-password"]',
      'input[name="password"]',
      'input[type="password"]'
    ]);
    if (!fill(accountInput, account) || !fill(passwordInput, password)) {{
      if (Date.now() - startMs >= FORM_MISSING_BUDGET_MS) {{
        password = '';
        window.clearInterval(timer);
        signal('form_missing');
      }}
      return;
    }}
    const form = passwordInput.form || accountInput.form || document.querySelector('form');
    const submit = form?.querySelector('button[type="submit"], input[type="submit"]');
    submitted = true;
    signal('submitted');
    window.setTimeout(() => {{
      if (submit instanceof HTMLElement) submit.click();
      else if (form instanceof HTMLFormElement) form.requestSubmit();
      password = '';
    }}, 80);
  }}, 100);
}})();"#
    )
}

async fn read_midishow_browser_cookie(window: tauri::WebviewWindow) -> Result<String, String> {
    let cookies = tauri::async_runtime::spawn_blocking(move || {
        let url = MIDISHOW_ACCOUNT_URL
            .parse::<tauri::Url>()
            .map_err(|_| "登录地址不可用".to_string())?;
        window
            .cookies_for_url(url)
            .map_err(|_| "暂时无法读取登录状态".to_string())
    })
    .await
    .map_err(|_| "暂时无法读取登录状态".to_string())??;

    let raw = cookies
        .iter()
        .map(|cookie| format!("{}={}", cookie.name(), cookie.value()))
        .collect::<Vec<_>>()
        .join("; ");
    normalize_midishow_cookie_header(&raw).map_err(|_| "暂时无法读取登录状态".to_string())
}

#[allow(dead_code)]
async fn inspect_midishow_session(cookie: &str) -> Result<Option<String>, String> {
    let client = midishow_client().map_err(|_| "暂时无法确认登录状态".to_string())?;
    let response = with_midishow_cookie(
        client
            .get(MIDISHOW_ACCOUNT_URL)
            .header(reqwest::header::ACCEPT, "text/html,application/xhtml+xml")
            .header(reqwest::header::REFERER, MIDISHOW_LOGIN_URL),
        Some(cookie),
    )
    .send()
    .await
    .map_err(|_| "暂时无法确认登录状态，请稍后重试".to_string())?;
    let final_url = response.url().as_str().to_string();
    let body = response.text().await.unwrap_or_default();
    if !midishow_login_succeeded(&final_url, &body) {
        return Ok(None);
    }
    Ok(extract_midishow_username(&body))
}

#[allow(dead_code)]
fn extract_midishow_username(body: &str) -> Option<String> {
    let patterns = [
        r#"(?is)<meta[^>]+name=["'](?:username|user-name)["'][^>]+content=["']([^"']+)["']"#,
        r#"(?is)<meta[^>]+content=["']([^"']+)["'][^>]+name=["'](?:username|user-name)["']"#,
        r#"(?is)data-(?:username|user-name)=["']([^"']+)["']"#,
        r#"(?is)class=["'][^"']*(?:username|user-name|nickname)[^"']*["'][^>]*>\s*([^<]{1,80})\s*<"#,
        r#"(?is)<input[^>]+name=["']username["'][^>]+value=["']([^"']+)["']"#,
        r#"(?is)<input[^>]+value=["']([^"']+)["'][^>]+name=["']username["']"#,
    ];
    patterns.into_iter().find_map(|pattern| {
        regex::Regex::new(pattern)
            .ok()?
            .captures(body)?
            .get(1)
            .map(|value| html_unescape(value.as_str()).trim().to_string())
            .filter(|value| !value.is_empty())
    })
}

#[allow(dead_code)]
fn html_unescape(value: &str) -> String {
    value
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
}

fn persist_midishow_session(
    app: &tauri::AppHandle,
    username: &str,
    cookie: String,
) -> Result<(), String> {
    let mut accounts = load_midishow_accounts(app)?;
    for account in &mut accounts {
        account.password.clear();
    }
    if let Some(account) = accounts
        .iter_mut()
        .find(|account| account.username == username)
    {
        account.cookie = cookie;
    } else {
        accounts.push(StoredMidishowAccount {
            username: username.to_string(),
            password: String::new(),
            cookie,
        });
    }
    save_midishow_accounts(app, &accounts)
}

#[allow(dead_code)]
fn midishow_login_succeeded(final_url: &str, body: &str) -> bool {
    let url = final_url.to_lowercase();
    let body = body.to_lowercase();
    (!url.contains("/user/account/login")
        && (url.contains("/user") || body.contains("/user/account/logout")))
        || body.contains("logout")
}

fn normalize_midishow_cookie_header(raw: &str) -> Result<String, String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Ok(String::new());
    }

    if raw.starts_with('[') {
        if let Ok(values) = serde_json::from_str::<Vec<String>>(raw) {
            return normalize_midishow_cookie_header(&values.join("; "));
        }
    }

    let mut cookies = Vec::new();
    let mut seen = HashSet::new();
    for line in raw.replace('\r', "\n").split('\n') {
        let mut text = line.trim();
        if text.is_empty() {
            continue;
        }
        if let Some((name, value)) = text.split_once(':') {
            if name.eq_ignore_ascii_case("cookie") || name.eq_ignore_ascii_case("set-cookie") {
                text = value.trim();
            }
        }
        for part in text.split(';') {
            let pair = part.trim();
            if pair.is_empty() || !pair.contains('=') {
                continue;
            }
            let Some((name, _)) = pair.split_once('=') else {
                continue;
            };
            let name = name.trim();
            if name.is_empty()
                || matches!(
                    name.to_ascii_lowercase().as_str(),
                    "path" | "expires" | "max-age" | "domain" | "samesite" | "secure" | "httponly"
                )
            {
                continue;
            }
            if seen.insert(name.to_ascii_lowercase()) {
                cookies.push(pair.to_string());
            }
        }
    }

    if cookies.is_empty() {
        Err("Midishow Cookie did not contain any name=value pairs".to_string())
    } else {
        Ok(cookies.join("; "))
    }
}

fn with_midishow_cookie(
    request: reqwest::RequestBuilder,
    cookie: Option<&str>,
) -> reqwest::RequestBuilder {
    match cookie.map(str::trim).filter(|value| !value.is_empty()) {
        Some(cookie) => request.header(reqwest::header::COOKIE, cookie),
        None => request,
    }
}

fn is_cloudflare_challenge(html: &str) -> bool {
    html.contains("challenge-error-text")
        || html.contains("__cf_chl")
        || html.contains("Enable JavaScript and cookies to continue")
}

fn is_midishow_challenge_response(
    status: reqwest::StatusCode,
    headers: &reqwest::header::HeaderMap,
    body: &str,
) -> bool {
    status.as_u16() == 403
        && (headers
            .get("cf-mitigated")
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.eq_ignore_ascii_case("challenge"))
            || is_cloudflare_challenge(body))
}

async fn download_midishow_direct(
    midi_id: u64,
    account: Option<&StoredMidishowAccount>,
) -> Result<(Vec<u8>, String), String> {
    let client = midishow_client()?;
    let cookie = account
        .map(|account| account.cookie.trim())
        .filter(|cookie| !cookie.is_empty());
    if account.is_some() && cookie.is_none() {
        return Err("Midishow 登录状态已失效，请重新登录".to_string());
    }

    let page_url = format!("https://www.midishow.com/en/midi/{midi_id}.html");
    let page = with_midishow_cookie(
        client
            .get(&page_url)
            .header(reqwest::header::ACCEPT, "text/html,application/xhtml+xml")
            .header(reqwest::header::REFERER, "https://www.midishow.com/"),
        cookie,
    )
    .send()
    .await
    .map_err(|e| format!("Failed to open Midishow page: {e}"))?;
    let page_status = page.status();
    let page_headers = page.headers().clone();
    let html = page
        .text()
        .await
        .map_err(|e| format!("Failed to read Midishow page: {e}"))?;
    if is_midishow_challenge_response(page_status, &page_headers, &html) {
        return Err("MidiShow requires an interactive browser verification. Open the official page in a browser and use its download action, or paste a public .mid/.midi direct link here.".to_string());
    }
    if !page_status.is_success() {
        return Err(format!("Midishow page returned HTTP {page_status}"));
    }
    let title = extract_midishow_page_title(&html, midi_id);
    let csrf = extract_csrf_token(&html)
        .ok_or_else(|| "Midishow page did not include a CSRF token".to_string())?;
    let fake_midi_url = extract_midishow_data_mid(&html, midi_id)
        .ok_or_else(|| "Midishow page did not include a MIDI data link".to_string())?;

    let new_file_url = format!("https://www.midishow.com/midi/new-file?id={midi_id}");
    let response1 = with_midishow_cookie(
        client
            .post(&new_file_url)
            .header(
                reqwest::header::CONTENT_TYPE,
                "application/x-www-form-urlencoded; charset=UTF-8",
            )
            .header(reqwest::header::ORIGIN, "https://www.midishow.com")
            .header(reqwest::header::REFERER, &page_url)
            .header("X-CSRF-Token", csrf.as_str())
            .header("X-Requested-With", "XMLHttpRequest")
            .form(&[("id", midi_id.to_string())]),
        cookie,
    )
    .send()
    .await
    .map_err(|e| format!("Midishow new-file request failed: {e}"))?;
    if is_midishow_challenge_response(response1.status(), response1.headers(), "") {
        return Err("MidiShow requires an interactive browser verification. Open the official page in a browser and use its download action, or paste a public .mid/.midi direct link here.".to_string());
    }
    if !response1.status().is_success() {
        return Err(format!(
            "Midishow new-file returned HTTP {}",
            response1.status()
        ));
    }
    let etag = response1
        .headers()
        .get(reqwest::header::ETAG)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_string();
    let response1_text = response1
        .text()
        .await
        .map_err(|e| format!("Failed to read Midishow new-file response: {e}"))?;
    if response1_text.len() < 56 {
        return Err("Midishow new-file response was too short".to_string());
    }

    let real_url = fake_midi_url
        .replacen("tokeno#:@!", "token", 1)
        .replace("https://www.midishow.com", "https://s.midishow.net")
        .replace(".mid?", ".js?");
    let response2_text = with_midishow_cookie(
        client
            .get(&real_url)
            .header(reqwest::header::REFERER, &page_url)
            .header("X-CSRF-Token", csrf.as_str())
            .header("X-Requested-With", "XMLHttpRequest"),
        cookie,
    )
    .send()
    .await
    .map_err(|e| format!("Failed to fetch Midishow MIDI payload: {e}"))?
    .text()
    .await
    .map_err(|e| format!("Failed to read Midishow MIDI payload: {e}"))?;
    if response2_text.len() < 6 {
        return Err("Midishow MIDI payload was too short".to_string());
    }

    let charset = format!(
        "{}{}",
        hex_to_string(&etag),
        ascii_slice(&response1_text, 56, response1_text.len())?
    );
    let mut midi = Vec::new();
    midi.extend(decode_midishow_base64(
        ascii_slice(&response1_text, 28, 56)?,
        &charset,
    )?);
    midi.extend(decode_midishow_base64(
        ascii_slice(&response2_text, 3, response2_text.len().saturating_sub(3))?,
        &charset,
    )?);
    midi.extend(decode_midishow_base64(
        ascii_slice(&response1_text, 0, 28)?,
        &charset,
    )?);
    Ok((midi, title))
}

fn extract_csrf_token(html: &str) -> Option<String> {
    if let Some(value) = regex_patterns::csrf_meta()
        .captures(html)
        .and_then(|captures| captures.get(1))
        .map(|value| value.as_str().to_string())
    {
        return Some(value);
    }
    let reversed =
        regex::Regex::new(r#"(?is)<meta[^>]+content=["']([^"']+)["'][^>]+name=["']csrf-token["']"#)
            .ok()?;
    reversed
        .captures(html)
        .and_then(|captures| captures.get(1))
        .map(|value| value.as_str().to_string())
}

fn extract_midishow_data_mid(html: &str, midi_id: u64) -> Option<String> {
    let pattern = format!(r#"(?is)<[^>]+data-id=["']{}["'][^>]*>"#, midi_id);
    let re = regex::Regex::new(&pattern).ok()?;
    for tag in re.find_iter(html) {
        if let Some(value) = extract_html_attr(tag.as_str(), "data-mid") {
            return Some(value);
        }
    }
    None
}

fn extract_html_attr(tag: &str, attr: &str) -> Option<String> {
    let pattern = format!(r#"(?is)\b{}\s*=\s*["']([^"']+)["']"#, regex::escape(attr));
    let re = regex::Regex::new(&pattern).ok()?;
    re.captures(tag)
        .and_then(|captures| captures.get(1))
        .map(|value| decode_html_entities(value.as_str()))
}

fn extract_midishow_page_title(html: &str, midi_id: u64) -> String {
    for pattern in [
        r#"(?is)<div[^>]+class=["'][^"']*ms-player-container[^"']*["'][^>]*>.*?<h1[^>]*>(.*?)</h1>"#,
        r#"(?is)<h1[^>]*>(.*?)</h1>"#,
        r#"(?is)<title[^>]*>(.*?)</title>"#,
    ] {
        if let Ok(re) = regex::Regex::new(pattern) {
            if let Some(title) = re
                .captures(html)
                .and_then(|captures| captures.get(1))
                .map(|value| clean_midishow_text(&strip_html(value.as_str())))
                .filter(|value| !value.is_empty())
            {
                return title.trim_end_matches(" - MidiShow").trim().to_string();
            }
        }
    }
    format!("MIDI_{midi_id}")
}

fn hex_to_string(value: &str) -> String {
    let hex = value
        .chars()
        .filter(|ch| ch.is_ascii_hexdigit())
        .collect::<String>();
    let mut result = String::new();
    let bytes = hex.as_bytes();
    for index in (0..bytes.len()).step_by(2) {
        if index + 2 > bytes.len() {
            break;
        }
        let Ok(pair) = std::str::from_utf8(&bytes[index..index + 2]) else {
            break;
        };
        if pair == "00" {
            break;
        }
        if let Ok(value) = u8::from_str_radix(pair, 16) {
            result.push(value as char);
        }
    }
    result
}

fn ascii_slice(value: &str, start: usize, end: usize) -> Result<&str, String> {
    value
        .get(start..end)
        .ok_or_else(|| "Midishow response contained unexpected non-ASCII data".to_string())
}

fn decode_midishow_base64(encoded: &str, charset: &str) -> Result<Vec<u8>, String> {
    let standard = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/=";
    let mut map = HashMap::new();
    for (custom, standard) in charset.chars().zip(standard.chars()) {
        map.entry(custom).or_insert(standard);
    }

    let mut translated = String::with_capacity(encoded.len());
    for ch in encoded.chars().filter(|ch| !ch.is_whitespace()) {
        let Some(standard_ch) = map.get(&ch) else {
            return Err("Midishow response used an unknown encoding character".to_string());
        };
        translated.push(*standard_ch);
    }

    base64::engine::general_purpose::STANDARD
        .decode(translated)
        .map_err(|e| format!("Failed to decode Midishow MIDI chunk: {e}"))
}

async fn download_midishow_to_library(
    app: &tauri::AppHandle,
    songs_dir: &Path,
    midi_id: u64,
    title: Option<String>,
    cover_url: Option<&str>,
    _overwrite: bool,
) -> Result<VrpianoSong, String> {
    let (data, downloaded_title) = download_midishow_file(app, midi_id).await?;
    let title = title
        .map(|value| sanitize_filename(&value))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| sanitize_filename(&downloaded_title))
        .trim()
        .to_string();
    let title = if title.is_empty() {
        format!("MIDI_{midi_id}")
    } else {
        title
    };
    let target = unique_path(&songs_dir.join(ensure_midi_extension(title)));
    write_midi_file(&target, &data)?;

    // 封面下载：失败不回退——MIDI 已写入，封面只是锦上添花，
    // 让 `song_from_path` 在下次 `list_local_songs` 时重新探测即可。
    if let Some(cover_url) = cover_url.filter(|url| is_midishow_cover_url(url)) {
        if let Err(error) = download_cover_for_song(&target, cover_url).await {
            let _ = app; // suppress unused
            eprintln!("[vrpiano] cover download failed for {midi_id}: {error}");
        }
    }

    song_from_path(&target)
}

/// 把封面图下载到 MIDI 同目录，命名为 `<basename>.cover.{jpg|png|webp}`。
/// - 仅允许通过 `is_midishow_cover_url` 的 URL；
/// - 单文件大小上限 5 MiB；
/// - 若目标文件已存在则跳过，避免覆盖用户手动设置的图标。
async fn download_cover_for_song(midi_path: &Path, cover_url: &str) -> Result<(), String> {
    let stem = midi_path
        .file_stem()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "MIDI path has no usable filename".to_string())?;
    let parent = midi_path.parent().unwrap_or_else(|| Path::new("."));

    let client = midishow_client()?;
    let response = client
        .get(cover_url)
        .header(reqwest::header::REFERER, "https://www.midishow.com/")
        .send()
        .await
        .map_err(|e| format!("Failed to fetch cover: {e}"))?;
    let status = response.status();
    if is_midishow_challenge_response(status, response.headers(), "") {
        return Err("Cover host requires interactive browser verification".to_string());
    }
    if !status.is_success() {
        return Err(format!("Cover returned HTTP {status}"));
    }

    // 根据 Content-Type 推断后缀，缺失时退化为 URL 后缀，再缺失用 jpg。
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_ascii_lowercase();
    let ext = if content_type.contains("png") {
        "png"
    } else if content_type.contains("webp") {
        "webp"
    } else if content_type.contains("jpeg") || content_type.contains("jpg") {
        "jpg"
    } else {
        let lower = cover_url.to_ascii_lowercase();
        if lower.ends_with(".png") {
            "png"
        } else if lower.ends_with(".webp") {
            "webp"
        } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
            "jpg"
        } else {
            "jpg"
        }
    };

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read cover body: {e}"))?;
    const COVER_MAX_BYTES: u64 = 5 * 1024 * 1024;
    if bytes.len() as u64 > COVER_MAX_BYTES {
        return Err(format!(
            "Cover too large ({} bytes, max {})",
            bytes.len(),
            COVER_MAX_BYTES
        ));
    }

    let cover_path = parent.join(format!("{stem}.cover.{ext}"));
    if cover_path.exists() {
        return Ok(());
    }
    fs::write(&cover_path, &bytes).map_err(|e| format!("Failed to save cover: {e}"))?;
    Ok(())
}

async fn download_midishow_bytes(app: &tauri::AppHandle, midi_id: u64) -> Result<Vec<u8>, String> {
    download_midishow_file(app, midi_id)
        .await
        .map(|(data, _title)| data)
}

async fn download_midishow_file(
    app: &tauri::AppHandle,
    midi_id: u64,
) -> Result<(Vec<u8>, String), String> {
    let project_path = resolve_vrpiano_project_path(Some(app));

    if let Some(account) = default_midishow_account(app)? {
        match download_midishow_direct(midi_id, Some(&account)).await {
            Ok((data, title)) => return Ok((validate_midi_bytes(data)?, title)),
            Err(account_error) => {
                // Do not retry with the legacy password/Python flow. Midishow's
                // browser challenge rejects those non-browser login requests and
                // can turn a clear session-expired error into repeated HTTP 403s.
                if let Ok((data, title)) = download_midishow_direct(midi_id, None).await {
                    return Ok((validate_midi_bytes(data)?, title));
                }
                return download_midishow_file_with_cli(&project_path, midi_id).map_err(
                    |cli_error| {
                        format!(
                            "Midishow browser session download failed: {account_error}; public/CLI fallback failed: {cli_error}"
                        )
                    },
                );
            }
        }
    }

    download_midishow_direct(midi_id, None)
        .await
        .or_else(|direct_error| {
            download_midishow_file_with_cli(&project_path, midi_id)
                .map_err(|cli_error| format!("{direct_error}; CLI fallback failed: {cli_error}"))
        })
}

fn download_midishow_file_with_cli(
    project_path: &Path,
    midi_id: u64,
) -> Result<(Vec<u8>, String), String> {
    let value = run_midishow_cli_json(project_path, &["download", &midi_id.to_string()])?;
    let encoded = value
        .get("data")
        .and_then(|value| value.as_str())
        .ok_or_else(|| "Midishow download did not return MIDI data".to_string())?;
    let data = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|e| format!("Failed to decode Midishow MIDI: {e}"))?;
    let title = midishow_title(project_path, midi_id).unwrap_or_else(|| format!("MIDI_{midi_id}"));
    Ok((validate_midi_bytes(data)?, title))
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
    let safe_accounts = accounts
        .iter()
        .map(|account| StoredMidishowAccount {
            username: account.username.clone(),
            password: String::new(),
            cookie: account.cookie.clone(),
        })
        .collect::<Vec<_>>();
    let text = serde_json::to_string_pretty(&safe_accounts).map_err(|e| e.to_string())?;
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
    Ok(load_midishow_accounts(app)?
        .into_iter()
        .find(|account| !account.cookie.trim().is_empty()))
}

fn midishow_title(project_path: &Path, midi_id: u64) -> Option<String> {
    let value = run_midishow_cli_json(project_path, &["info", &midi_id.to_string()]).ok()?;
    value
        .get("title")
        .and_then(|value| value.as_str())
        .map(sanitize_filename)
        .filter(|value| !value.is_empty())
}

/// Resolve `node` to a concrete `node.exe` on PATH. Preferring the `.exe`
/// directly (instead of relying on CreateProcessW's `.cmd` shim discovery)
/// avoids a command-line re-parse that can mangle the script path into a bare
/// drive root like `C:` (causing Node's `EISDIR` crash).
fn resolve_node_exe() -> PathBuf {
    if let Ok(path) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path) {
            let candidate = dir.join("node.exe");
            if candidate.is_file() {
                return candidate;
            }
        }
    }
    PathBuf::from("node")
}

fn run_midishow_cli_json(project_path: &Path, args: &[&str]) -> Result<serde_json::Value, String> {
    run_midishow_cli_json_with_timeout(project_path, args, Duration::from_secs(45))
}

fn run_midishow_cli_json_with_timeout(
    project_path: &Path,
    args: &[&str],
    timeout: Duration,
) -> Result<serde_json::Value, String> {
    let cli = find_midishow_cli(project_path)
        .ok_or_else(|| format!("Midishow CLI not found near: {}", project_path.display()))?;
    // Canonicalize to a fully-qualified, drive-absolute path so Windows Node
    // never receives a drive-relative path (e.g. `C:`) that triggers EISDIR.
    let cli = std::fs::canonicalize(&cli).unwrap_or(cli);
    if !cli.is_file() {
        return Err(format!("Midishow CLI is not a file: {}", cli.display()));
    }
    let node = resolve_node_exe();
    let mut command = std::process::Command::new(&node);
    command
        .arg(&cli)
        .args(args)
        .current_dir(cli.parent().unwrap_or(project_path))
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    // The node CLI (axios) does not inherit the Windows system proxy, so pass
    // it explicitly via env vars, mirroring what the HTTP client now does.
    if let Some((proxy_url, _)) = system_proxy() {
        command
            .env("HTTPS_PROXY", &proxy_url)
            .env("HTTP_PROXY", &proxy_url)
            .env("https_proxy", &proxy_url)
            .env("http_proxy", &proxy_url);
    }
    let mut child = command
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
        if started.elapsed() > timeout {
            let _ = child.kill();
            let _ = child.wait();
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
    parse_json_from_process_stdout(&output.stdout)
        .map_err(|e| format!("Failed to parse Midishow CLI response: {e}"))
}

fn parse_json_from_process_stdout(output: &[u8]) -> Result<serde_json::Value, serde_json::Error> {
    if let Ok(value) = serde_json::from_slice(output) {
        return Ok(value);
    }

    let text = String::from_utf8_lossy(output);
    for line in text.lines().rev() {
        let trimmed = line.trim();
        if trimmed.starts_with('{') || trimmed.starts_with('[') {
            if let Ok(value) = serde_json::from_str(trimmed) {
                return Ok(value);
            }
        }
    }

    serde_json::from_str(text.trim())
}

fn find_midishow_cli(project_path: &Path) -> Option<PathBuf> {
    let mut roots = vec![project_path.to_path_buf()];
    if let Ok(cwd) = std::env::current_dir() {
        roots.push(cwd.clone());
        roots.push(cwd.join("src-python"));
        roots.push(cwd.join("VRPiano-auto-play"));
    }
    if let Some(parent) = project_path.parent() {
        roots.push(parent.to_path_buf());
        roots.push(parent.join("VRPiano-auto-play"));
    }

    for root in roots {
        for candidate in [
            root.join("midishow-downloader").join("dist").join("cli.js"),
            root.join("src-python")
                .join("midishow-downloader")
                .join("dist")
                .join("cli.js"),
        ] {
            // `is_file()` (not just `exists()`) avoids picking a directory that
            // happens to share the prefix, which would make Node resolve the
            // script path to a drive root and crash with EISDIR.
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    None
}

fn clean_midishow_text(value: &str) -> String {
    let text = decode_html_entities(value)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let mut cleaned = regex_patterns::clean_text()
        .split(&text)
        .next()
        .unwrap_or(&text)
        .trim()
        .to_string();
    for marker in ["上传人", "下载", "评分", "Artist:", "Author:", "Uploader:"] {
        if let Some((before, _)) = cleaned.split_once(marker) {
            cleaned = before.trim().to_string();
        }
    }
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

fn import_seed_songs(project_path: &Path, songs_dir: &Path) -> Result<(), String> {
    for source in seed_song_dirs(project_path) {
        if !source.exists() {
            continue;
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
    }
    Ok(())
}

fn seed_song_dirs(project_path: &Path) -> Vec<PathBuf> {
    let mut dirs = vec![
        project_path.join("songs"),
        project_path.join("src-python").join("songs"),
    ];
    if let Ok(cwd) = std::env::current_dir() {
        dirs.push(cwd.join("songs"));
        dirs.push(cwd.join("src-python").join("songs"));
        dirs.push(cwd.join("VRPiano-auto-play").join("songs"));
    }
    if let Some(parent) = project_path.parent() {
        dirs.push(parent.join("VRPiano-auto-play").join("songs"));
    }
    dirs
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
    let cover_path = find_local_cover_path(path);
    Ok(VrpianoSong {
        id: path.to_string_lossy().to_string(),
        name,
        path: path.to_string_lossy().to_string(),
        size: metadata.len(),
        modified_ms,
        cover_path,
    })
}

/// 探测与 MIDI 同名的 `<stem>.cover.{jpg,jpeg,png,webp}` 封面文件，
/// 命中即返回绝对路径。优先级：jpg > jpeg > png > webp。
fn find_local_cover_path(midi_path: &Path) -> Option<String> {
    let stem = midi_path.file_stem().and_then(|stem| stem.to_str())?;
    let parent = midi_path.parent()?;
    for ext in ["jpg", "jpeg", "png", "webp"] {
        let candidate = parent.join(format!("{stem}.cover.{ext}"));
        if candidate.exists() {
            return Some(candidate.to_string_lossy().to_string());
        }
    }
    None
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

// ==================== Recording Commands ====================

#[tauri::command]
pub async fn vrpiano_start_recording(
    app: tauri::AppHandle,
    state: tauri::State<'_, VrpianoState>,
) -> Result<String, String> {
    let songs_dir = ensure_songs_dir(&app)?;
    let timestamp = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let filename = format!("recording_{}.mid", timestamp);
    let output_path = songs_dir.join(&filename);
    let recorder = state.recorder.lock().unwrap();
    let mut recorder = recorder;
    recorder.start(output_path.to_string_lossy().to_string());
    drop(recorder);
    update_status(&state, |status| {
        status.recording = true;
        status.recorded_midi_path = Some(output_path.to_string_lossy().to_string());
    });
    Ok(output_path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn vrpiano_stop_recording(
    state: tauri::State<'_, VrpianoState>,
) -> Result<Option<String>, String> {
    let recorder = state.recorder.lock().unwrap();
    let mut recorder = recorder;
    let output_path = recorder.stop();
    drop(recorder);
    update_status(&state, |status| {
        status.recording = false;
    });
    Ok(output_path)
}

#[tauri::command]
pub async fn vrpiano_get_recording_status(
    state: tauri::State<'_, VrpianoState>,
) -> Result<VrpianoRecordingStatus, String> {
    let recorder = state.recorder.lock().unwrap();
    Ok(VrpianoRecordingStatus {
        recording: recorder.is_recording(),
        recorded_midi_path: recorder.output_path.clone(),
    })
}

#[derive(Clone, Serialize)]
pub struct VrpianoRecordingStatus {
    pub recording: bool,
    pub recorded_midi_path: Option<String>,
}

// ==================== Channel Control Commands ====================

#[tauri::command]
pub async fn vrpiano_get_channel_states(
    state: tauri::State<'_, VrpianoState>,
) -> Result<[ChannelState; 16], String> {
    let runtime = state.inner.lock().unwrap();
    Ok(runtime.status.channels)
}

#[tauri::command]
pub async fn vrpiano_set_channel_mute(
    state: tauri::State<'_, VrpianoState>,
    channel: u8,
    muted: bool,
) -> Result<(), String> {
    if channel >= 16 {
        return Err("Channel must be 0-15".to_string());
    }
    update_status(&state, |status| {
        status.channels[channel as usize].muted = muted;
    });
    Ok(())
}

#[tauri::command]
pub async fn vrpiano_set_channel_solo(
    state: tauri::State<'_, VrpianoState>,
    channel: u8,
    solo: bool,
) -> Result<(), String> {
    if channel >= 16 {
        return Err("Channel must be 0-15".to_string());
    }
    update_status(&state, |status| {
        status.channels[channel as usize].solo = solo;
    });
    Ok(())
}

#[tauri::command]
pub async fn vrpiano_set_channel_volume(
    state: tauri::State<'_, VrpianoState>,
    channel: u8,
    volume: u8,
) -> Result<(), String> {
    if channel >= 16 {
        return Err("Channel must be 0-15".to_string());
    }
    update_status(&state, |status| {
        status.channels[channel as usize].volume = volume.min(127);
    });
    Ok(())
}

#[tauri::command]
pub async fn vrpiano_set_transpose(
    app: tauri::AppHandle,
    state: tauri::State<'_, VrpianoState>,
    transpose: i8,
) -> Result<VrpianoStatus, String> {
    if let Ok(mut runtime) = state.inner.lock() {
        runtime.transpose = transpose.clamp(-24, 24);
    }
    let status = status_snapshot(&app, &state.inner)?;
    Ok(status)
}

#[tauri::command]
pub async fn vrpiano_set_channel_routed(
    app: tauri::AppHandle,
    state: tauri::State<'_, VrpianoState>,
    channel: u8,
    routed: bool,
) -> Result<VrpianoStatus, String> {
    if channel >= 16 {
        return Err("Channel must be 0-15".to_string());
    }
    if let Ok(mut runtime) = state.inner.lock() {
        runtime.piano_channels[channel as usize] = routed;
    }
    let status = status_snapshot(&app, &state.inner)?;
    Ok(status)
}

#[tauri::command]
pub async fn vrpiano_set_playlist(
    app: tauri::AppHandle,
    state: tauri::State<'_, VrpianoState>,
    songs: Vec<String>,
) -> Result<VrpianoStatus, String> {
    if let Ok(mut runtime) = state.inner.lock() {
        runtime.playlist = songs;
        runtime.current_index = 0;
    }
    let status = status_snapshot(&app, &state.inner)?;
    Ok(status)
}

#[tauri::command]
pub async fn vrpiano_set_play_mode(
    app: tauri::AppHandle,
    state: tauri::State<'_, VrpianoState>,
    mode: String,
) -> Result<VrpianoStatus, String> {
    let allowed = [
        "sequential",
        "random",
        "one",
        "repeat_all",
        "stop_at_song_end",
        "stop_at_end",
    ];
    if !allowed.contains(&mode.as_str()) {
        return Err(format!("Unknown play mode: {mode}"));
    }
    if let Ok(mut runtime) = state.inner.lock() {
        runtime.play_mode = mode;
    }
    let status = status_snapshot(&app, &state.inner)?;
    Ok(status)
}

#[tauri::command]
pub async fn vrpiano_save_osc_config(
    app: tauri::AppHandle,
    host: String,
    port: u16,
) -> Result<(), String> {
    save_vrpiano_osc_config(&app, &host, port);
    Ok(())
}

#[tauri::command]
pub async fn vrpiano_load_osc_config(
    app: tauri::AppHandle,
) -> Result<Option<serde_json::Value>, String> {
    Ok(load_vrpiano_osc_config(&app).map(|(host, port)| {
        serde_json::json!({ "host": host, "port": port })
    }))
}

#[tauri::command]
pub async fn vrpiano_connect_vrchat_osc(
    app: tauri::AppHandle,
    state: tauri::State<'_, VrpianoState>,
    host: String,
    port: u16,
) -> Result<VrpianoStatus, String> {
    let stop_flag = Arc::new(AtomicBool::new(false));
    {
        let mut runtime = state
            .inner
            .lock()
            .map_err(|_| "VRPiano state lock poisoned".to_string())?;
        runtime.vrchat_osc_enabled = true;
        runtime.vrchat_osc_host = host.clone();
        runtime.vrchat_osc_port = port;
        runtime.status.vrchat_osc_running = true;
        runtime.osc_heartbeat_stop = Some(stop_flag.clone());
    }
    save_vrpiano_osc_config(&app, &host, port);
    spawn_osc_heartbeat(app.clone(), state.inner.clone(), stop_flag);
    let status = status_snapshot(&app, &state.inner)?;
    Ok(status)
}

#[tauri::command]
pub async fn vrpiano_disconnect_vrchat_osc(
    app: tauri::AppHandle,
    state: tauri::State<'_, VrpianoState>,
) -> Result<VrpianoStatus, String> {
    {
        let mut runtime = state
            .inner
            .lock()
            .map_err(|_| "VRPiano state lock poisoned".to_string())?;
        runtime.status.vrchat_osc_running = false;
        runtime.status.vrchat_osc_connected = false;
        if let Some(flag) = runtime.osc_heartbeat_stop.take() {
            flag.store(true, Ordering::SeqCst);
        }
    }
    let status = status_snapshot(&app, &state.inner)?;
    Ok(status)
}

#[tauri::command]
pub async fn vrpiano_record_external_event(
    state: tauri::State<'_, VrpianoState>,
    at_ms: u64,
    kind: String,
    channel: u8,
    data: Vec<u8>,
) -> Result<(), String> {
    let mut recorder = state.recorder.lock().unwrap();
    if recorder.is_recording() {
        recorder.record(at_ms, &kind, channel, &data);
    }
    Ok(())
}

// ==================== Voice Control Commands ====================

#[tauri::command]
pub async fn vrpiano_set_voice_control_enabled(
    state: tauri::State<'_, VrpianoState>,
    enabled: bool,
) -> Result<(), String> {
    update_status(&state, |status| {
        status.voice_listening = enabled;
        status.last_event = if enabled {
            format!("Voice control enabled")
        } else {
            format!("Voice control disabled")
        };
    });
    Ok(())
}

#[tauri::command]
pub async fn vrpiano_set_tts_enabled(
    state: tauri::State<'_, VrpianoState>,
    enabled: bool,
) -> Result<(), String> {
    update_status(&state, |status| {
        status.tts_enabled = enabled;
        status.last_event = if enabled {
            format!("TTS singing enabled")
        } else {
            format!("TTS singing disabled")
        };
    });
    Ok(())
}

// ==================== Helper ====================

fn update_status<F>(state: &tauri::State<'_, VrpianoState>, f: F)
where
    F: FnOnce(&mut VrpianoStatus),
{
    if let Ok(mut runtime) = state.inner.lock() {
        f(&mut runtime.status);
    }
}

// ==================== Python Runtime Bridge ====================

fn resolve_python_runtime(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let mut candidates = Vec::new();

    if let Ok(path) = std::env::var("VRCDOG_PYTHON_RUNTIME") {
        candidates.push(PathBuf::from(path).join("python.exe"));
    }

    if let Ok(resource_dir) = app.path().resource_dir() {
        candidates.extend([
            resource_dir.join("python-runtime").join("python.exe"),
            resource_dir.join("resources").join("python-runtime").join("python.exe"),
        ]);
    }

    if let Ok(cwd) = std::env::current_dir() {
        candidates.extend([
            cwd.join("src-tauri").join("resources").join("python-runtime").join("python.exe"),
            cwd.join("resources").join("python-runtime").join("python.exe"),
        ]);
    }

    if let Ok(executable) = std::env::current_exe() {
        if let Some(exe_dir) = executable.parent() {
            candidates.extend([
                exe_dir.join("python-runtime").join("python.exe"),
                exe_dir.join("resources").join("python-runtime").join("python.exe"),
            ]);
        }
    }

    candidates
        .into_iter()
        .find(|p| p.is_file())
        .ok_or_else(|| "Embedded Python runtime not found. Run `node scripts/prepare-python-runtime.mjs` before packaging.".to_string())
}

fn resolve_bridge_script(app: &tauri::AppHandle, name: &str) -> Result<PathBuf, String> {
    let mut candidates = Vec::new();

    if let Ok(resource_dir) = app.path().resource_dir() {
        candidates.extend([
            resource_dir.join("_up_").join("src-python").join(name),
            resource_dir.join("src-python").join(name),
            resource_dir.join(name),
        ]);
    }

    if let Ok(cwd) = std::env::current_dir() {
        candidates.extend([
            cwd.join("src-tauri").join("src-python").join(name),
            cwd.join("src-python").join(name),
        ]);
    }

    if let Ok(executable) = std::env::current_exe() {
        if let Some(exe_dir) = executable.parent() {
            candidates.extend([
                exe_dir.join("src-python").join(name),
                exe_dir.join("_up_").join("src-python").join(name),
            ]);
        }
    }

    candidates
        .into_iter()
        .find(|p| p.is_file())
        .ok_or_else(|| format!("Bridge script not found: {name}"))
}

fn run_python_bridge(app: &tauri::AppHandle, script: &str, args: &[&str]) -> Result<String, String> {
    let python = resolve_python_runtime(app)?;
    let script_path = resolve_bridge_script(app, script)?;

    let mut command = Command::new(&python);
    command
        .arg(&script_path)
        .args(args)
        .env("PYTHONNOUSERSITE", "1")
        .env("PYTHONUTF8", "1")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }

    let output = command
        .output()
        .map_err(|e| format!("Failed to run Python bridge: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Python bridge failed: {stderr}"));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.trim().to_string())
}

// ==================== ASR Commands ====================

#[tauri::command]
pub async fn vrpiano_start_voice_listening(
    _app: tauri::AppHandle,
    state: tauri::State<'_, VrpianoState>,
) -> Result<String, String> {
    update_status(&state, |status| {
        status.last_event = "Voice listening started".to_string();
    });
    Ok("Voice listening started".to_string())
}

#[tauri::command]
pub async fn vrpiano_stop_voice_listening(
    state: tauri::State<'_, VrpianoState>,
) -> Result<String, String> {
    update_status(&state, |status| {
        status.last_event = "Voice listening stopped".to_string();
    });
    Ok("Voice listening stopped".to_string())
}

#[tauri::command]
pub async fn vrpiano_transcribe_audio(
    app: tauri::AppHandle,
    audio_path: String,
) -> Result<VrpianoTranscriptionResult, String> {
    let output = run_python_bridge(&app, "vrcdog_asr.py", &[&audio_path])?;
    let parsed: serde_json::Value = serde_json::from_str(&output)
        .map_err(|e| format!("Failed to parse ASR result: {e}"))?;

    if let Some(error) = parsed.get("error") {
        return Err(error.as_str().unwrap_or("ASR error").to_string());
    }

    let text = parsed.get("text").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let language = parsed.get("language").and_then(|v| v.as_str()).unwrap_or("zh").to_string();

    Ok(VrpianoTranscriptionResult {
        text,
        language,
        confidence: 1.0,
    })
}

#[derive(Clone, Serialize)]
pub struct VrpianoTranscriptionResult {
    pub text: String,
    pub language: String,
    pub confidence: f64,
}

// ==================== TTS Commands ====================

#[tauri::command]
pub async fn vrpiano_synthesize_speech(
    app: tauri::AppHandle,
    text: String,
    voice: Option<String>,
    rate: Option<f64>,
    volume: Option<f64>,
) -> Result<VrpianoSynthesisResult, String> {
    let voice = voice.unwrap_or_else(|| "zh-CN-XiaoxiaoNeural".to_string());
    let rate = rate.unwrap_or(1.0);
    let volume = volume.unwrap_or(1.0);

    let rate_percent = ((rate - 1.0) * 100.0).round() as i32;
    let rate_str = format!("{:+}%", rate_percent);
    let volume_percent = ((volume - 1.0) * 100.0).round() as i32;
    let volume_str = format!("{:+}%", volume_percent);

    let songs_dir = ensure_songs_dir(&app)?;
    let timestamp = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let output_path = songs_dir.join(format!("tts_{}.mp3", timestamp));
    let output_path_str = output_path.to_string_lossy().into_owned();

    let args: Vec<&str> = vec![
        "--text", &text,
        "--voice", &voice,
        "--rate", &rate_str,
        "--volume", &volume_str,
        "--output", &output_path_str,
    ];

    let output = run_python_bridge(&app, "vrcdog_tts.py", &args)?;
    let parsed: serde_json::Value = serde_json::from_str(&output)
        .map_err(|e| format!("Failed to parse TTS result: {e}"))?;

    if let Some(error) = parsed.get("error") {
        return Err(error.as_str().unwrap_or("TTS error").to_string());
    }

    let output_path_str = parsed.get("output_path")
        .and_then(|v| v.as_str())
        .unwrap_or(&output_path.to_string_lossy().to_string())
        .to_string();

    Ok(VrpianoSynthesisResult {
        output_path: output_path_str,
        voice,
        text,
    })
}

#[derive(Clone, Serialize)]
pub struct VrpianoSynthesisResult {
    pub output_path: String,
    pub voice: String,
    pub text: String,
}

#[cfg(test)]
mod vrpiano_download_tests {
    use super::{
        extract_midishow_username, filename_from_content_disposition, is_midishow_login_url,
        looks_like_direct_midi_url, midishow_login_monitor_script, midishow_login_script,
        parse_midishow_login_title, MidishowLoginSignal, MIDISHOW_LOGIN_URL,
    };

    #[test]
    fn identifies_public_midi_links_without_matching_page_urls() {
        assert!(looks_like_direct_midi_url(
            "https://example.com/music/song.mid?token=abc"
        ));
        assert!(looks_like_direct_midi_url(
            "https://example.com/music/song.MIDI#download"
        ));
        assert!(!looks_like_direct_midi_url(
            "https://www.midishow.com/en/midi/70804.html"
        ));
    }

    #[test]
    fn extracts_safe_filename_from_download_header() {
        assert_eq!(
            filename_from_content_disposition("attachment; filename=concert.mid"),
            Some("concert.mid".to_string())
        );
        assert_eq!(
            filename_from_content_disposition("attachment; filename=../../unsafe.mid"),
            Some("_.._unsafe.mid".to_string())
        );
    }

    #[test]
    fn extracts_real_midishow_username_without_fallback() {
        assert_eq!(
            extract_midishow_username(
                r#"<div class="profile"><span class="nickname">小搭&amp;老板</span></div>"#,
            ),
            Some("小搭&老板".to_string())
        );
        assert_eq!(extract_midishow_username("<main>账户中心</main>"), None);
    }

    #[test]
    fn login_script_uses_encoded_values_and_password_field() {
        let account = serde_json::to_string("user@example.com").unwrap();
        let password = serde_json::to_string("p\"ass").unwrap();
        let script = midishow_login_script(&account, &password);
        assert!(script.contains("input[autocomplete=\"username\"]"));
        assert!(script.contains("input[type=\"password\"]"));
        assert!(script.contains("user@example.com"));
        assert!(script.contains("p\\\"ass"));
        assert!(script.contains("credentials_rejected"));
        assert!(script.contains("needs_confirmation"));
        assert!(script.contains("password = ''"));
    }

    #[test]
    fn login_monitor_detects_failures_without_credentials() {
        let script = midishow_login_monitor_script();
        assert!(script.contains("credentials_rejected"));
        assert!(script.contains("needs_confirmation"));
        assert!(!script.contains("password"));
        assert!(!script.contains("account"));
    }

    #[test]
    fn recognizes_midishow_login_signals_without_exposing_page_text() {
        assert_eq!(
            parse_midishow_login_title("VRCDOG_MIDISHOW:credentials_rejected"),
            Some(MidishowLoginSignal::CredentialsRejected)
        );
        assert_eq!(
            parse_midishow_login_title("VRCDOG_MIDISHOW:needs_confirmation"),
            Some(MidishowLoginSignal::NeedsConfirmation)
        );
        assert_eq!(parse_midishow_login_title("普通页面"), None);
    }

    #[test]
    fn accepts_only_midishow_login_url_for_script_injection() {
        assert!(is_midishow_login_url(MIDISHOW_LOGIN_URL));
        assert!(!is_midishow_login_url(
            "https://example.com/user/account/login"
        ));
        assert!(!is_midishow_login_url(
            "https://www.midishow.com.evil.test/user/account/login"
        ));
    }
}
