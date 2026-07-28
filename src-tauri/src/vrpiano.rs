use base64::Engine;
use midly::{MetaMessage, MidiMessage, Smf, Timing, TrackEventKind};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
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

const NOTE_HOLD_MS: u64 = 28;
const SPEED_STEP: f64 = 0.1;
const MAX_MIDI_DOWNLOAD_BYTES: u64 = 32 * 1024 * 1024;

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

    // App resource directory (bundled modules)
    if let Some(app_handle) = app {
        if let Ok(resource_dir) = app_handle.path().resource_dir() {
            candidates.push(resource_dir.clone());
            candidates.push(resource_dir.join("VRPiano-auto-play"));
            candidates.push(resource_dir.join("src-python"));
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
    #[serde(default)]
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
    cookie: Option<String>,
}

#[derive(Clone)]
pub struct VrpianoState {
    inner: Arc<Mutex<VrpianoRuntime>>,
}

struct VrpianoRuntime {
    stop: Option<Arc<AtomicBool>>,
    paused: Arc<AtomicBool>,
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
                paused: Arc::new(AtomicBool::new(false)),
                speed: Arc::new(Mutex::new(1.0)),
                hotkeys_enabled: false,
                hotkey_song_path: String::new(),
                hotkey_delay_secs: 5,
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
                    last_event: "VRPiano ready".to_string(),
                    last_error: String::new(),
                    songs_dir: String::new(),
                    speed: 1.0,
                    hotkeys_enabled: false,
                    hotkeys_available: cfg!(target_os = "windows"),
                    last_hotkey: String::new(),
                    last_hotkey_at_ms: 0,
                },
            })),
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
        return download_midishow_to_library(&app, &songs_dir, midi_id, request.filename, false)
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
        .map(|account| VrpianoMidishowAccount {
            login_type: if account.cookie.trim().is_empty() {
                "password".to_string()
            } else {
                "cookie".to_string()
            },
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
    let password = request.password.trim().to_string();
    let cookie = request.cookie.unwrap_or_default();
    let cookie = normalize_midishow_cookie_header(&cookie)?;
    if username.is_empty() || (password.is_empty() && cookie.is_empty()) {
        return Err("Please enter a Midishow username plus password or Cookie".to_string());
    }
    let project_path = resolve_vrpiano_project_path(Some(&app));
    let project_path_str = project_path.to_string_lossy();
    if cookie.is_empty() {
        if let Err(native_error) = verify_midishow_login_direct(&username, &password).await {
            verify_midishow_login(&project_path_str, &username, &password).map_err(
                |python_error| {
                    format!(
                        "Midishow login failed: {native_error}; Python fallback failed: {python_error}"
                    )
                },
            )?;
        }
    } else {
        verify_midishow_cookie_direct(&cookie).await?;
    }
    let mut accounts = load_midishow_accounts(&app)?;
    if let Some(account) = accounts
        .iter_mut()
        .find(|account| account.username == username)
    {
        account.password = password;
        account.cookie = cookie;
    } else {
        accounts.push(StoredMidishowAccount {
            username,
            password,
            cookie,
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
                total_notes: events.len(),
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
                pause_flag,
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
    thread::spawn(move || {
        match vk {
            112 => {
                let (song_path, delay_secs) = match context.state.lock() {
                    Ok(runtime) => (runtime.hotkey_song_path.clone(), runtime.hotkey_delay_secs),
                    Err(_) => return,
                };
                let request = VrpianoStartRequest {
                    song_path,
                    delay_secs,
                    speed: current_speed(&context.state),
                };
                let _ = start_playback(context.app.clone(), context.state.clone(), request);
            }
            113 => {
                let _ = stop_playback(context.app.clone(), context.state.clone());
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
    if let Ok(mut runtime) = state.lock() {
        runtime.paused.store(false, Ordering::SeqCst);
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
    match search_midishow_http(keyword, limit, account.as_ref()).await {
        Ok(results) => Ok(results),
        Err(request_error) => match run_midishow_cli_json(project_path, &["search", keyword]) {
            Ok(value) => parse_midishow_results(value, limit),
            Err(cli_error) => Err(format!(
                "{request_error}; CLI fallback also failed: {cli_error}"
            )),
        },
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

async fn search_midishow_http(
    keyword: &str,
    limit: usize,
    account: Option<&StoredMidishowAccount>,
) -> Result<Vec<VrpianoOnlineSong>, String> {
    let client = midishow_client()?;
    let cookie = account
        .map(|account| account.cookie.trim())
        .filter(|cookie| !cookie.is_empty());
    if let Some(account) = account.filter(|_| cookie.is_none()) {
        login_midishow_direct(&client, &account.username, &account.password).await?;
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
    .map_err(|e| format!("Midishow search failed: {e}"))?;
    let status = response.status();
    let html = response
        .text()
        .await
        .map_err(|e| format!("Failed to read Midishow response: {e}"))?;
    if is_cloudflare_challenge(&html) {
        return Err("Midishow requires browser JavaScript/cookies before search. Open Midishow in a browser first, then use Cookie login in VRPiano.".to_string());
    }
    if !status.is_success() {
        return Err(format!("Midishow search returned HTTP {status}"));
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
        results.push(VrpianoOnlineSong {
            id,
            title,
            artist,
            page_url: format!("https://www.midishow.com/en/midi/{id}.html"),
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
        let window = &html[pos..html.len().min(pos + 3500)];
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
    let window = &html[pos..html.len().min(pos + 3500)];
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

fn midishow_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36")
        .cookie_store(true)
        .redirect(reqwest::redirect::Policy::limited(10))
        .timeout(Duration::from_secs(35))
        .build()
        .map_err(|e| format!("Failed to create Midishow client: {e}"))
}

async fn verify_midishow_login_direct(username: &str, password: &str) -> Result<(), String> {
    let client = midishow_client()?;
    login_midishow_direct(&client, username, password).await
}

async fn verify_midishow_cookie_direct(cookie: &str) -> Result<(), String> {
    let client = midishow_client()?;
    let response = with_midishow_cookie(
        client
            .get("https://www.midishow.com/en/user/account")
            .header(reqwest::header::ACCEPT, "text/html,application/xhtml+xml")
            .header(
                reqwest::header::REFERER,
                "https://www.midishow.com/en/user/account/login",
            ),
        Some(cookie),
    )
    .send()
    .await
    .map_err(|e| format!("Failed to verify Midishow Cookie: {e}"))?;
    let status = response.status();
    let final_url = response.url().as_str().to_string();
    let body = response.text().await.unwrap_or_default();
    if is_cloudflare_challenge(&body) {
        return Err("Midishow requires browser JavaScript/cookies. Please paste a fresh Cookie from a browser session that has already opened Midishow.".to_string());
    }
    if status.is_success() && midishow_login_succeeded(&final_url, &body) {
        Ok(())
    } else {
        Err("Midishow Cookie is invalid or expired".to_string())
    }
}

async fn login_midishow_direct(
    client: &reqwest::Client,
    username: &str,
    password: &str,
) -> Result<(), String> {
    let login_urls = [
        "https://www.midishow.com/en/user/account/login",
        "https://www.midishow.com/user/account/login",
    ];
    let mut last_error = String::from("Midishow login failed");

    for login_url in login_urls {
        match try_midishow_login_url(client, login_url, username, password).await {
            Ok(()) => return Ok(()),
            Err(error) => last_error = error,
        }
    }

    Err(last_error)
}

async fn try_midishow_login_url(
    client: &reqwest::Client,
    login_url: &str,
    username: &str,
    password: &str,
) -> Result<(), String> {
    let page = client
        .get(login_url)
        .header(reqwest::header::ACCEPT, "text/html,application/xhtml+xml")
        .header(reqwest::header::REFERER, login_url)
        .send()
        .await
        .map_err(|e| format!("Failed to open Midishow login page: {e}"))?;
    if !page.status().is_success() {
        return Err(format!(
            "Midishow login page returned HTTP {}",
            page.status()
        ));
    }
    let page_html = page
        .text()
        .await
        .map_err(|e| format!("Failed to read Midishow login page: {e}"))?;
    if is_cloudflare_challenge(&page_html) {
        return Err("Midishow requires browser JavaScript/cookies. Please use Cookie login after opening Midishow in a browser.".to_string());
    }
    let csrf = extract_csrf_token(&page_html)
        .ok_or_else(|| "Midishow login page did not include a CSRF token".to_string())?;

    let form = [
        ("_csrf", csrf.as_str()),
        ("LoginForm[identity]", username),
        ("LoginForm[password]", password),
        ("login-button", ""),
    ];
    let response = client
        .post(login_url)
        .header(
            reqwest::header::CONTENT_TYPE,
            "application/x-www-form-urlencoded",
        )
        .header(reqwest::header::ORIGIN, "https://www.midishow.com")
        .header(reqwest::header::REFERER, login_url)
        .form(&form)
        .send()
        .await
        .map_err(|e| format!("Midishow login request failed: {e}"))?;
    let status = response.status();
    let final_url = response.url().as_str().to_string();
    let body = response.text().await.unwrap_or_default();

    if status.is_success() && midishow_login_succeeded(&final_url, &body) {
        return Ok(());
    }

    Err(extract_midishow_error(&body).unwrap_or_else(|| {
        if status.as_u16() == 403 {
            "Midishow rejected the login request (HTTP 403)".to_string()
        } else {
            format!("Midishow login was not accepted (HTTP {status})")
        }
    }))
}

fn midishow_login_succeeded(final_url: &str, body: &str) -> bool {
    let url = final_url.to_lowercase();
    let body = body.to_lowercase();
    (!url.contains("/user/account/login")
        && (url.contains("/user") || body.contains("/user/account/logout")))
        || body.contains("logout")
}

fn extract_midishow_error(html: &str) -> Option<String> {
    let re = regex::Regex::new(
        r#"(?is)<[^>]+class=["'][^"']*(?:help-block|error|alert)[^"']*["'][^>]*>(.*?)</[^>]+>"#,
    )
    .ok()?;
    let error = re
        .captures_iter(html)
        .filter_map(|captures| captures.get(1))
        .map(|value| clean_midishow_text(&strip_html(value.as_str())))
        .find(|value| !value.is_empty());
    error
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
    if let Some(account) = account.filter(|_| cookie.is_none()) {
        login_midishow_direct(&client, &account.username, &account.password).await?;
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
    song_from_path(&target)
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
    let max_retries = 2;

    for attempt in 0..=max_retries {
        if let Some(account) = default_midishow_account(app)? {
            match download_midishow_direct(midi_id, Some(&account)).await {
                Ok((data, title)) => return Ok((validate_midi_bytes(data)?, title)),
                Err(account_error) => {
                    match download_midishow_with_python(
                        &project_path.to_string_lossy(),
                        midi_id,
                        &account,
                    ) {
                        Ok((data, title)) => return Ok((validate_midi_bytes(data)?, title)),
                        Err(python_error) => {
                            if attempt < max_retries {
                                if attempt == 0 {
                                    clear_midishow_cookie_cache(&project_path);
                                }
                                continue;
                            }
                            let direct_public_result =
                                download_midishow_direct(midi_id, None).await;
                            if let Ok((data, title)) = direct_public_result {
                                return Ok((validate_midi_bytes(data)?, title));
                            }
                            let cli_result =
                                download_midishow_file_with_cli(&project_path, midi_id);
                            return cli_result.map_err(|cli_error| {
                                format!("Midishow account download failed: {account_error}; Python fallback failed: {python_error}; public/CLI fallback failed: {cli_error}")
                            });
                        }
                    }
                }
            }
        } else {
            return download_midishow_direct(midi_id, None)
                .await
                .or_else(|direct_error| {
                    download_midishow_file_with_cli(&project_path, midi_id).map_err(|cli_error| {
                        format!("{direct_error}; CLI fallback failed: {cli_error}")
                    })
                });
        }
    }
    Err("Midishow download failed after retries".to_string())
}

/// Clear the Midishow cookie cache file to force re-login on next attempt.
fn clear_midishow_cookie_cache(project_path: &Path) {
    let cookie_path = project_path
        .join("src-python")
        .join(".midishow_cookies.json");
    if cookie_path.exists() {
        let _ = fs::remove_file(&cookie_path);
    }
    // Also check the project root
    let cookie_path_root = project_path.join(".midishow_cookies.json");
    if cookie_path_root.exists() {
        let _ = fs::remove_file(&cookie_path_root);
    }
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

fn verify_midishow_login(project_path: &str, username: &str, password: &str) -> Result<(), String> {
    let script = r#"
import json, sys, os
project = os.path.realpath(sys.argv[1])
src_python = os.path.join(project, "src-python")
# Add both project root and src-python to path (src-python first for midishow.py)
sys.path.insert(0, src_python)
sys.path.insert(0, project)
# Also try relative to this script's location if running from a bundle
try:
    exe_dir = os.path.dirname(os.path.realpath(sys.executable))
    for rel in [".", "src-python", "../src-python", "VRPiano-auto-play/src-python"]:
        p = os.path.normpath(os.path.join(exe_dir, rel))
        if os.path.isdir(p) and p not in sys.path:
            sys.path.insert(0, p)
except Exception:
    pass
try:
    from midishow import login_midi
    ok = login_midi(sys.argv[2], sys.argv[3])
    print(json.dumps({"success": bool(ok), "error": ""}, ensure_ascii=False))
except Exception as e:
    print(json.dumps({"success": False, "error": str(e)}, ensure_ascii=False))
"#;
    let value = run_python_json(
        script,
        &[project_path, username, password],
        Duration::from_secs(45),
    )?;
    let success = value
        .get("success")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    if success {
        Ok(())
    } else {
        let error_msg = value
            .get("error")
            .and_then(|value| value.as_str())
            .filter(|s| !s.is_empty())
            .unwrap_or("Midishow login failed");
        Err(error_msg.to_string())
    }
}

fn download_midishow_with_python(
    project_path: &str,
    midi_id: u64,
    account: &StoredMidishowAccount,
) -> Result<(Vec<u8>, String), String> {
    let url = format!("https://www.midishow.com/en/midi/{midi_id}.html");
    let script = r#"
import base64, json, sys, os
project = os.path.realpath(sys.argv[1])
src_python = os.path.join(project, "src-python")
sys.path.insert(0, src_python)
sys.path.insert(0, project)
try:
    exe_dir = os.path.dirname(os.path.realpath(sys.executable))
    for rel in [".", "src-python", "../src-python", "VRPiano-auto-play/src-python"]:
        p = os.path.normpath(os.path.join(exe_dir, rel))
        if os.path.isdir(p) and p not in sys.path:
            sys.path.insert(0, p)
except Exception:
    pass
try:
    from midishow_api import get_account_manager, download_midi_url
    mgr = get_account_manager()
    mgr.add_account(sys.argv[2], sys.argv[3])
    data, title = download_midi_url(sys.argv[4], sys.argv[2])
    print(json.dumps({
        "title": title or "",
        "data": base64.b64encode(data).decode("ascii"),
        "error": ""
    }, ensure_ascii=False))
except Exception as e:
    print(json.dumps({
        "title": "",
        "data": "",
        "error": str(e)
    }, ensure_ascii=False))
"#;
    let value = run_python_json(
        script,
        &[project_path, &account.username, &account.password, &url],
        Duration::from_secs(60),
    )?;
    // Check for error from Python
    if let Some(error) = value.get("error").and_then(|value| value.as_str()) {
        if !error.is_empty() {
            return Err(error.to_string());
        }
    }
    let encoded = value
        .get("data")
        .and_then(|value| value.as_str())
        .ok_or_else(|| "Python Midishow downloader did not return MIDI data".to_string())?;
    if encoded.is_empty() {
        return Err("Python Midishow downloader returned empty data".to_string());
    }
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
    // Collect all possible Python executable paths to try
    let mut candidates: Vec<String> = Vec::new();
    candidates.extend(["python", "py", "python3"].iter().map(|s| s.to_string()));

    // On Windows, also check common Python installation paths
    #[cfg(target_os = "windows")]
    {
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            let base = Path::new(&local_app_data).join("Programs").join("Python");
            if let Ok(entries) = std::fs::read_dir(&base) {
                for entry in entries.flatten() {
                    let py_path = entry.path().join("python.exe");
                    if py_path.exists() {
                        candidates.push(py_path.to_string_lossy().to_string());
                    }
                }
            }
        }
        if let Ok(program_files) = std::env::var("ProgramFiles") {
            let base = Path::new(&program_files).join("Python");
            if let Ok(entries) = std::fs::read_dir(&base) {
                for entry in entries.flatten() {
                    let py_path = entry.path().join("python.exe");
                    if py_path.exists() {
                        candidates.push(py_path.to_string_lossy().to_string());
                    }
                }
            }
        }
        // Also try the Microsoft Store Python path
        let store_path =
            Path::new(r"C:\Program Files\WindowsApps\PythonSoftwareFoundation.Python*");
        if let Ok(entries) = std::fs::read_dir(store_path.parent().unwrap_or(Path::new("C:\\"))) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with("PythonSoftwareFoundation.Python") {
                    let py_path = entry.path().join("python.exe");
                    if py_path.exists() {
                        candidates.push(py_path.to_string_lossy().to_string());
                    }
                }
            }
        }
    }

    // Track whether every candidate failed to even start (Python missing) vs
    // Python ran but the script/import failed (module missing, etc.).
    let mut started_any = false;
    let mut last_error = String::from("No Python interpreter candidate succeeded");
    for executable in &candidates {
        let mut process_args = vec!["-c", script];
        process_args.extend_from_slice(args);
        match run_process_json(executable, &process_args, timeout) {
            Ok(value) => return Ok(value),
            Err(error) => {
                if error.contains("Failed to start") {
                    last_error = format!("Python 不可用（无法启动 {executable}）");
                } else {
                    started_any = true;
                    last_error = error;
                }
            }
        }
    }
    if started_any {
        Err(format!(
            "Midishow Python 桥接失败：{last_error}。请确认 Midishow 模块已随程序打包（src-python/midishow.py）。"
        ))
    } else {
        Err(format!(
            "Python 不可用或 Midishow Python 桥接失败：{last_error}。请从 https://python.org 安装 Python 并确保其在 PATH 中。"
        ))
    }
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
    parse_json_from_process_stdout(&output.stdout)
        .map_err(|e| format!("Failed to parse {executable} JSON response: {e}"))
}

fn midishow_title(project_path: &Path, midi_id: u64) -> Option<String> {
    let value = run_midishow_cli_json(project_path, &["info", &midi_id.to_string()]).ok()?;
    value
        .get("title")
        .and_then(|value| value.as_str())
        .map(sanitize_filename)
        .filter(|value| !value.is_empty())
}

fn run_midishow_cli_json(project_path: &Path, args: &[&str]) -> Result<serde_json::Value, String> {
    let cli = find_midishow_cli(project_path)
        .ok_or_else(|| format!("Midishow CLI not found near: {}", project_path.display()))?;
    let mut child = std::process::Command::new("node")
        .arg(&cli)
        .args(args)
        .current_dir(cli.parent().unwrap_or(project_path))
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
            if candidate.exists() {
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

#[cfg(test)]
mod vrpiano_download_tests {
    use super::{filename_from_content_disposition, looks_like_direct_midi_url};

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
}
