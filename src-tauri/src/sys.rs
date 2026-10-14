use crate::AppResult;
use base64::Engine;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use tauri::{Emitter, Manager};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GptSovitsSynthesisRequest {
    pub base_url: String,
    pub text: String,
    pub text_language: String,
    pub sovits_weights: Option<String>,
    pub gpt_weights: Option<String>,
    pub reference_audio: Option<String>,
    pub prompt_text: Option<String>,
    pub prompt_language: Option<String>,
}

fn non_empty(value: &Option<String>) -> Option<&str> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

async fn set_gpt_sovits_weight(
    client: &reqwest::Client,
    base_url: &str,
    endpoint: &str,
    weights_path: &str,
) -> Result<(), String> {
    let response = client
        .get(format!("{base_url}/{endpoint}"))
        .query(&[("weights_path", weights_path)])
        .send()
        .await
        .map_err(|error| format!("{endpoint} request failed: {error}"))?;

    if response.status().is_success() {
        return Ok(());
    }

    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    Err(format!("{endpoint} returned HTTP {status}: {body}"))
}

async fn gpt_sovits_audio_response(
    response: reqwest::Response,
) -> Result<(String, Vec<u8>), String> {
    let status = response.status();
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("audio/wav")
        .split(';')
        .next()
        .unwrap_or("audio/wav")
        .to_string();
    let bytes = response
        .bytes()
        .await
        .map_err(|error| format!("Failed to read TTS audio: {error}"))?;

    if !status.is_success() {
        return Err(format!(
            "GPT-SoVITS returned HTTP {status}: {}",
            String::from_utf8_lossy(&bytes)
        ));
    }
    if bytes.is_empty() {
        return Err("GPT-SoVITS returned empty audio".to_string());
    }

    Ok((content_type, bytes.to_vec()))
}

#[tauri::command]
pub async fn sys_gpt_sovits_synthesize(
    request: GptSovitsSynthesisRequest,
) -> Result<String, String> {
    let base_url = request.base_url.trim().trim_end_matches('/');
    if !(base_url.starts_with("http://") || base_url.starts_with("https://")) {
        return Err("GPT-SoVITS URL must start with http:// or https://".to_string());
    }
    if request.text.trim().is_empty() {
        return Err("TTS text cannot be empty".to_string());
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(90))
        .build()
        .map_err(|error| format!("Failed to create TTS client: {error}"))?;

    if let Some(weights_path) = non_empty(&request.sovits_weights) {
        set_gpt_sovits_weight(&client, base_url, "set_sovits_weights", weights_path).await?;
    }
    if let Some(weights_path) = non_empty(&request.gpt_weights) {
        set_gpt_sovits_weight(&client, base_url, "set_gpt_weights", weights_path).await?;
    }

    let prompt_language = non_empty(&request.prompt_language)
        .unwrap_or(&request.text_language)
        .to_string();
    let mut payload = serde_json::json!({
        "text": request.text.trim(),
        "text_lang": request.text_language.clone(),
        "prompt_lang": prompt_language,
        "text_split_method": "cut5",
        "batch_size": 1,
        "media_type": "wav",
        "streaming_mode": false
    });
    if let Some(reference_audio) = non_empty(&request.reference_audio) {
        payload["ref_audio_path"] = serde_json::Value::String(reference_audio.to_string());
    }
    if let Some(prompt_text) = non_empty(&request.prompt_text) {
        payload["prompt_text"] = serde_json::Value::String(prompt_text.to_string());
    }

    let post_result = client
        .post(format!("{base_url}/tts"))
        .json(&payload)
        .send()
        .await;

    let audio = match post_result {
        Ok(response) if response.status().is_success() => gpt_sovits_audio_response(response).await,
        Ok(response) => {
            let post_status = response.status();
            let post_error = response.text().await.unwrap_or_default();
            let mut query = vec![
                ("text", request.text.trim().to_string()),
                ("text_language", request.text_language.clone()),
                ("prompt_language", prompt_language),
            ];
            if let Some(reference_audio) = non_empty(&request.reference_audio) {
                query.push(("refer_wav_path", reference_audio.to_string()));
            }
            if let Some(prompt_text) = non_empty(&request.prompt_text) {
                query.push(("prompt_text", prompt_text.to_string()));
            }

            let fallback = client
                .get(base_url)
                .query(&query)
                .send()
                .await
                .map_err(|error| {
                    format!(
                        "GPT-SoVITS /tts returned HTTP {post_status}: {post_error}; legacy API request failed: {error}"
                    )
                })?;
            gpt_sovits_audio_response(fallback).await
        }
        Err(post_error) => {
            let mut query = vec![
                ("text", request.text.trim().to_string()),
                ("text_language", request.text_language.clone()),
                ("prompt_language", prompt_language),
            ];
            if let Some(reference_audio) = non_empty(&request.reference_audio) {
                query.push(("refer_wav_path", reference_audio.to_string()));
            }
            if let Some(prompt_text) = non_empty(&request.prompt_text) {
                query.push(("prompt_text", prompt_text.to_string()));
            }

            let fallback = client
                .get(base_url)
                .query(&query)
                .send()
                .await
                .map_err(|error| {
                    format!(
                        "GPT-SoVITS /tts request failed: {post_error}; legacy API request failed: {error}"
                    )
                })?;
            gpt_sovits_audio_response(fallback).await
        }
    }?;

    let encoded = base64::engine::general_purpose::STANDARD.encode(audio.1);
    Ok(format!("data:{};base64,{}", audio.0, encoded))
}

#[tauri::command]
pub async fn sys_clear_vrchat_cache() -> AppResult<u64> {
    let mut cache_dir =
        dirs::data_local_dir().ok_or_else(|| "Could not find LocalAppData".to_string())?;

    cache_dir.pop(); // Up from Local to AppData
    cache_dir.push("LocalLow");
    cache_dir.push("VRChat");
    cache_dir.push("vrchat");

    let mut total_deleted: u64 = 0;

    // HTTPCache
    let mut http_cache = cache_dir.clone();
    http_cache.push("HTTPCache");
    if http_cache.exists() {
        if let Ok(size) = get_dir_size(&http_cache) {
            if fs::remove_dir_all(&http_cache).is_ok() {
                total_deleted += size;
            }
        }
    }

    // Cache-WindowsPlayer
    let mut win_cache = cache_dir.clone();
    win_cache.push("Cache-WindowsPlayer");
    if win_cache.exists() {
        if let Ok(size) = get_dir_size(&win_cache) {
            if fs::remove_dir_all(&win_cache).is_ok() {
                total_deleted += size;
            }
        }
    }

    Ok(total_deleted)
}

fn get_dir_size(path: &PathBuf) -> std::io::Result<u64> {
    let mut size = 0;
    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                size += get_dir_size(&path)?;
            } else {
                size += entry.metadata()?.len();
            }
        }
    }
    Ok(size)
}

/// 可执行/脚本类扩展名。这些文件一旦落盘即可能被执行，因此只允许写入应用自己的
/// 数据目录或用户临时目录，防止「导出文件」能力被当作持久化 / 自启动原语使用。
const SCRIPT_LIKE_EXT: &[&str] = &[
    "exe", "com", "scr", "pif", "dll", "sys", "ocx", "cpl", "drv", "msi", "msp", "mst", "bat",
    "cmd", "ps1", "psm1", "psd1", "vbs", "vbe", "js", "jse", "wsf", "wsh", "hta", "reg", "lnk",
    "url", "inf", "jar", "scf", "application",
];

/// 禁止写入的系统目录（对目录做 canonicalize 后按前缀匹配）。
fn protected_write_roots() -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = Vec::new();
    for key in [
        "SystemRoot",
        "WINDIR",
        "ProgramFiles",
        "ProgramFiles(x86)",
        "ProgramW6432",
        "ProgramData",
    ] {
        if let Ok(value) = std::env::var(key) {
            if let Ok(canonical) = std::fs::canonicalize(&value) {
                roots.push(canonical);
            }
        }
    }
    // 当前用户的「启动」文件夹：写入这里等于配置开机自启
    if let Some(data) = dirs::data_dir() {
        roots.push(
            data.join("Microsoft")
                .join("Windows")
                .join("Start Menu")
                .join("Programs")
                .join("Startup"),
        );
    }
    roots
}

/// 目标目录落在受保护的系统目录内则拒绝。
fn ensure_not_protected_dir(dir: &std::path::Path) -> Result<(), String> {
    for root in protected_write_roots() {
        if dir.starts_with(&root) {
            return Err(format!(
                "拒绝写入受保护的系统目录: {}",
                dir.display()
            ));
        }
    }
    Ok(())
}

/// 校验由前端提供的写入目标路径。
///
/// 修复点：`sys_save_text_file` / `sys_save_binary_file` 原先直接 `fs::write(&path, ..)`，
/// 前端（或任何注入到 WebView 的脚本）可以把任意内容写到任意可写路径——包括往
/// 「启动」文件夹丢一个 `.bat`、覆盖其它用户文件、或改写应用自身的安装文件。
/// 现在统一要求：绝对路径、不含 `..`、父目录真实存在、不落在受保护系统目录，
/// 且可执行/脚本类扩展名只能写进应用数据目录或临时目录。
fn validate_write_target(app: &tauri::AppHandle, raw_path: &str) -> Result<PathBuf, String> {
    let trimmed = raw_path.trim();
    if trimmed.is_empty() {
        return Err("写入路径为空".to_string());
    }
    let candidate = PathBuf::from(trimmed);
    if !candidate.is_absolute() {
        return Err("写入路径必须是绝对路径".to_string());
    }
    if candidate
        .components()
        .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return Err("写入路径不允许包含 `..`".to_string());
    }
    let parent = candidate
        .parent()
        .filter(|dir| !dir.as_os_str().is_empty())
        .ok_or_else(|| "写入路径缺少父目录".to_string())?;
    let parent_canonical = parent
        .canonicalize()
        .map_err(|error| format!("目标目录不可用: {error}"))?;
    ensure_not_protected_dir(&parent_canonical)?;

    // 目标已存在时跟随符号链接再判定一次，避免用软链接绕过上面的目录检查
    if let Ok(existing) = candidate.canonicalize() {
        if let Some(existing_parent) = existing.parent() {
            ensure_not_protected_dir(existing_parent)?;
        }
    }

    let ext = candidate
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if SCRIPT_LIKE_EXT.contains(&ext.as_str()) {
        let app_dir_ok = app
            .path()
            .app_data_dir()
            .ok()
            .and_then(|dir| std::fs::canonicalize(dir).ok())
            .map(|dir| parent_canonical.starts_with(&dir))
            .unwrap_or(false);
        let temp_ok = std::fs::canonicalize(std::env::temp_dir())
            .map(|dir| parent_canonical.starts_with(&dir))
            .unwrap_or(false);
        if !app_dir_ok && !temp_ok {
            return Err(format!(
                "出于安全考虑，拒绝把可执行/脚本文件写入该目录: {}",
                parent_canonical.display()
            ));
        }
    }

    Ok(candidate)
}

#[tauri::command]
pub async fn sys_save_text_file(
    app: tauri::AppHandle,
    path: String,
    content: String,
) -> Result<(), String> {
    let target = validate_write_target(&app, &path)?;
    std::fs::write(&target, content).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn sys_read_text_file(
    _app: tauri::AppHandle,
    path: String,
) -> Result<String, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("File path is empty".to_string());
    }
    let p = std::path::PathBuf::from(trimmed);
    if !p.is_file() {
        return Err("Target file does not exist".to_string());
    }
    let meta = std::fs::metadata(&p).map_err(|e| e.to_string())?;
    if meta.len() > 50 * 1024 * 1024 {
        return Err("File size exceeds 50MB limit".to_string());
    }
    std::fs::read_to_string(&p).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn sys_save_binary_file(
    app: tauri::AppHandle,
    path: String,
    content: Vec<u8>,
) -> Result<(), String> {
    let target = validate_write_target(&app, &path)?;
    std::fs::write(&target, content).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn sys_check_steamvr() -> crate::AppResult<bool> {
    use sysinfo::System;
    let sys = System::new_all();
    // Detect all major VR runtime processes:
    // SteamVR, Meta/Oculus Link, Pico Streaming, HTC Vive, Virtual Desktop, ALVR
    let vr_processes: &[&str] = &[
        // SteamVR
        "vrserver.exe",
        "vrmonitor.exe",
        "vrdashboard.exe",
        "vrcompositor.exe",
        // Meta / Oculus (Quest Link / Air Link)
        "ovrserver_x64.exe",
        "oculusclient.exe",
        "ovrserviceprocess.exe",
        "oculus_runtime.exe",
        "oculusdash.exe",
        // Pico (Streaming Assistant / Business Streaming)
        "pico_streaming_assistant.exe",
        "pico connect.exe",
        "picolink.exe",
        // HTC Vive (Vive Console / VIVEPORT)
        "htcvrs.exe",
        "viveport.exe",
        "viveconsole.exe",
        // Virtual Desktop (cross-device)
        "virtual desktop.server.exe",
        "virtualdesktop.server.exe",
        // ALVR (open source VR streaming)
        "alvr_dashboard.exe",
        "alvr_server.exe",
    ];
    for process in sys.processes().values() {
        let name = process.name().to_string_lossy().to_lowercase();
        if vr_processes.iter().any(|&vr| name == vr) {
            return Ok(true);
        }
    }
    Ok(false)
}

#[tauri::command]
pub async fn sys_set_autostart(enable: bool) -> crate::AppResult<()> {
    #[cfg(target_os = "windows")]
    {
        use winreg::enums::*;
        use winreg::RegKey;

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let path = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";

        let (key, _) = hkcu.create_subkey(path).map_err(|e| e.to_string())?;

        let app_name = "VrcDog";

        if enable {
            let exe_path = std::env::current_exe().map_err(|e| e.to_string())?;

            let mut exe_str = exe_path.to_string_lossy().to_string();
            if !exe_str.starts_with('"') {
                exe_str = format!("\"{}\"", exe_str);
            }

            key.set_value(app_name, &exe_str)
                .map_err(|e| e.to_string())?;
        } else {
            let _ = key.delete_value(app_name);
        }

        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err("Auto-start is only supported on Windows".to_string())
    }
}

#[tauri::command]
pub async fn sys_register_url_scheme(enable: bool) -> crate::AppResult<()> {
    #[cfg(target_os = "windows")]
    {
        use winreg::enums::*;
        use winreg::RegKey;

        let hkcr = RegKey::predef(HKEY_CLASSES_ROOT);
        let scheme = "vrcdog";

        if enable {
            let (key, _) = hkcr.create_subkey(scheme).map_err(|e| e.to_string())?;
            key.set_value("", &format!("URL:{} Protocol", scheme))
                .map_err(|e| e.to_string())?;
            key.set_value("URL Protocol", &"")
                .map_err(|e| e.to_string())?;

            let (icon_key, _) = key
                .create_subkey("DefaultIcon")
                .map_err(|e| e.to_string())?;
            let exe_path = std::env::current_exe().map_err(|e| e.to_string())?;
            icon_key
                .set_value("", &format!("{},1", exe_path.to_string_lossy()))
                .map_err(|e| e.to_string())?;

            let (cmd_key, _) = key
                .create_subkey("shell\\open\\command")
                .map_err(|e| e.to_string())?;
            cmd_key
                .set_value("", &format!("\"{}\" \"%1\"", exe_path.to_string_lossy()))
                .map_err(|e| e.to_string())?;
        } else {
            let _ = hkcr.delete_subkey_all(scheme);
        }

        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err("URL scheme is only supported on Windows".to_string())
    }
}

#[tauri::command]
pub async fn sys_get_launch_args() -> Result<Vec<String>, String> {
    Ok(std::env::args().collect())
}

#[derive(serde::Serialize)]
pub struct ClientServerConfig {
    pub server_url: String,
    pub config_path: String,
}

fn client_server_config_path(app_handle: &tauri::AppHandle) -> Result<PathBuf, String> {
    app_handle
        .path()
        .app_data_dir()
        .map(|dir| dir.join("client-server.json"))
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn sys_get_client_server_config(
    app_handle: tauri::AppHandle,
) -> Result<ClientServerConfig, String> {
    let path = client_server_config_path(&app_handle)?;
    let mut server_url = "http://127.0.0.1:11451".to_string();
    if let Ok(content) = tokio::fs::read_to_string(&path).await {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(saved_url) = value.get("server_url").and_then(|value| value.as_str()) {
                if !saved_url.trim().is_empty() {
                    server_url = saved_url.trim().to_string();
                }
            }
        }
    } else {
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|error| error.to_string())?;
        }
        let content = serde_json::to_string_pretty(&serde_json::json!({
            "server_url": server_url
        }))
        .map_err(|error| error.to_string())?;
        tokio::fs::write(&path, content)
            .await
            .map_err(|error| error.to_string())?;
    }
    Ok(ClientServerConfig {
        server_url,
        config_path: path.to_string_lossy().to_string(),
    })
}

#[tauri::command]
pub async fn sys_save_client_server_config(
    app_handle: tauri::AppHandle,
    server_url: String,
) -> Result<ClientServerConfig, String> {
    let server_url = server_url.trim().trim_end_matches('/').to_string();
    if server_url.is_empty() {
        return Err("Server URL cannot be empty".to_string());
    }
    let path = client_server_config_path(&app_handle)?;
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|error| error.to_string())?;
    }
    let content = serde_json::to_string_pretty(&serde_json::json!({
        "server_url": server_url
    }))
    .map_err(|error| error.to_string())?;
    tokio::fs::write(&path, content)
        .await
        .map_err(|error| error.to_string())?;
    Ok(ClientServerConfig {
        server_url,
        config_path: path.to_string_lossy().to_string(),
    })
}

#[tauri::command]
pub async fn sys_open_dir(app_handle: tauri::AppHandle, target: String) -> Result<(), String> {
    let mut path = PathBuf::new();

    match target.as_str() {
        "logs" => {
            if let Some(mut local_low) = dirs::data_local_dir() {
                local_low.pop();
                local_low.push("LocalLow");
                local_low.push("VRChat");
                local_low.push("vrchat");
                path = local_low;
            }
        }
        "cache" => {
            if let Some(mut local_low) = dirs::data_local_dir() {
                local_low.pop();
                local_low.push("LocalLow");
                local_low.push("VRChat");
                local_low.push("vrchat");
                local_low.push("HTTPCache");
                path = local_low;
            }
        }
        "screenshots" => {
            let mut custom_path = None;
            if let Some(mut config_path) = dirs::data_local_dir() {
                config_path.pop();
                config_path.push("LocalLow");
                config_path.push("VRChat");
                config_path.push("VRChat");
                config_path.push("config.json");
                if config_path.exists() {
                    if let Ok(content) = std::fs::read_to_string(&config_path) {
                        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&content) {
                            if let Some(p) =
                                parsed.get("pictureOutputFolder").and_then(|v| v.as_str())
                            {
                                let cp = std::path::PathBuf::from(p);
                                if cp.exists() {
                                    custom_path = Some(cp);
                                }
                            }
                        }
                    }
                }
            }
            if let Some(cp) = custom_path {
                path = cp;
            } else if let Some(mut pic_dir) = dirs::picture_dir() {
                pic_dir.push("VRChat");
                path = pic_dir;
            }
        }
        "crash_reports" => {
            if let Some(mut local) = dirs::data_local_dir() {
                local.push("Temp");
                local.push("VRChat");
                path = local;
            }
        }
        "client_config" => {
            path = app_handle
                .path()
                .app_data_dir()
                .map_err(|error| error.to_string())?;
            std::fs::create_dir_all(&path).map_err(|error| error.to_string())?;
        }
        // 兼容历史枚举之外的用法：BilidownView 下载任务的"打开文件夹"按钮
        // 直接传下载目录的绝对路径。若 target 本身就是一个存在的目录，
        // 视为文件系统路径直接打开，而不是报"未知的目录目标"。
        _ => {
            let candidate = std::path::Path::new(&target);
            if candidate.is_absolute() && candidate.is_dir() {
                path = candidate.to_path_buf();
            } else {
                return Err("未知的目录目标".to_string());
            }
        }
    }

    if path.exists() {
        #[cfg(target_os = "windows")]
        {
            std::process::Command::new("explorer")
                .arg(path)
                .spawn()
                .map_err(|e| format!("无法打开目录: {}", e))?;
        }
        #[cfg(target_os = "linux")]
        {
            std::process::Command::new("xdg-open")
                .arg(path)
                .spawn()
                .map_err(|e| format!("无法打开目录: {}", e))?;
        }
        #[cfg(target_os = "macos")]
        {
            std::process::Command::new("open")
                .arg(path)
                .spawn()
                .map_err(|e| format!("无法打开目录: {}", e))?;
        }
        Ok(())
    } else {
        Err("目录不存在".to_string())
    }
}

#[tauri::command]
pub async fn sys_register_steamvr_autostart() -> Result<(), String> {
    // Generate vrmanifest file
    let exe_path = std::env::current_exe().map_err(|e| e.to_string())?;
    // 可执行文件位于盘根（`D:\VrcDog.exe`）时 `parent()` 是 None，路径含非 UTF-8 字符时
    // `to_str()` 是 None——原来的两处 `unwrap()` 会让这个命令直接 panic，
    // 用户只会看到「点了 SteamVR 自启没反应」。
    let dir = exe_path.parent().ok_or("Cannot resolve application directory")?;
    let manifest_path = dir.join("vrcdog_manifest.vrmanifest");
    let manifest_path_str = manifest_path
        .to_str()
        .ok_or("Application path contains non-UTF-8 characters")?
        .to_string();

    let manifest_content = serde_json::json!({
        "source": "VrcDog",
        "applications": [{
            "app_key": "vrcdog.hyperengine.overlay",
            "launch_type": "binary",
            "binary_path_windows": exe_path.to_string_lossy().to_string(),
            "is_dashboard_overlay": true,
            "strings": {
                "en_us": {
                    "name": "VrcDog Overlay"
                }
            }
        }]
    });
    let manifest_json =
        serde_json::to_string_pretty(&manifest_content).map_err(|e| e.to_string())?;

    // 写文件和调用 `vrpathreg.exe` 都是阻塞操作（后者要同步等待外部进程退出），
    // 在 `async fn` 里直接做会占死一个 tokio worker，拖慢同一运行时上的其它 IPC 命令。
    let write_target = manifest_path.clone();
    tauri::async_runtime::spawn_blocking(move || std::fs::write(&write_target, manifest_json))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| format!("Cannot write vrmanifest: {e}"))?;

    // Call vrpathreg to add the manifest
    let vrpathreg =
        "C:\\Program Files (x86)\\Steam\\steamapps\\common\\SteamVR\\bin\\win64\\vrpathreg.exe";
    if std::path::Path::new(vrpathreg).exists() {
        let arg = manifest_path_str.clone();
        tauri::async_runtime::spawn_blocking(move || {
            std::process::Command::new(vrpathreg).arg("addapp").arg(arg).output()
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| format!("vrpathreg failed: {e}"))?;
        Ok(())
    } else {
        Err("Could not find SteamVR vrpathreg.exe".to_string())
    }
}

#[tauri::command]
pub async fn sys_open_steamvr_bindings() -> Result<(), String> {
    // Open localhost SteamVR binding dashboard in default browser
    let url = "http://localhost:8998/dashboard/controllerbinding.html";
    std::process::Command::new("explorer")
        .arg(url)
        .spawn()
        .map_err(|e| format!("Failed to open bindings: {}", e))?;
    Ok(())
}

#[cfg_attr(not(test), allow(dead_code))]
fn validate_external_url(url: &str) -> Result<(), String> {
    let normalized = url.trim().to_ascii_lowercase();
    if normalized.starts_with("https://") || normalized.starts_with("http://") {
        Ok(())
    } else {
        Err("Only HTTP and HTTPS links can be opened".to_string())
    }
}

#[tauri::command]
pub async fn sys_open_url(app: tauri::AppHandle, url: String) -> Result<(), String> {
    validate_external_url(&url)?;
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_url(&url, None::<&str>)
        .map_err(|error| format!("Failed to open the system browser: {error}"))
}

#[cfg(test)]
mod external_url_tests {
    use super::validate_external_url;

    #[test]
    fn accepts_http_links_and_rejects_other_schemes() {
        assert!(validate_external_url("https://www.midishow.com/user/account/signup").is_ok());
        assert!(validate_external_url("http://127.0.0.1:1420").is_ok());
        assert!(validate_external_url("file:///C:/Windows/System32").is_err());
        assert!(validate_external_url("javascript:alert(1)").is_err());
    }
}

#[tauri::command]
pub async fn sys_get_vrc_screenshot_dir() -> Result<String, String> {
    if let Some(mut config_path) = dirs::data_local_dir() {
        config_path.pop();
        config_path.push("LocalLow");
        config_path.push("VRChat");
        config_path.push("VRChat");
        config_path.push("config.json");
        if config_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&config_path) {
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(p) = parsed.get("pictureOutputFolder").and_then(|v| v.as_str()) {
                        return Ok(p.to_string());
                    }
                }
            }
        }
    }

    if let Some(mut pic_dir) = dirs::picture_dir() {
        pic_dir.push("VRChat");
        return Ok(pic_dir.to_string_lossy().to_string());
    }

    Ok("".to_string())
}

#[tauri::command]
pub async fn sys_set_vrc_screenshot_dir(path: String) -> Result<(), String> {
    if let Some(mut config_path) = dirs::data_local_dir() {
        config_path.pop();
        config_path.push("LocalLow");
        config_path.push("VRChat");
        config_path.push("VRChat");

        if !config_path.exists() {
            std::fs::create_dir_all(&config_path)
                .map_err(|e| format!("无法创建VRChat配置目录: {}", e))?;
        }

        config_path.push("config.json");

        let mut config_json = serde_json::json!({});
        if config_path.exists() {
            let content =
                std::fs::read_to_string(&config_path).unwrap_or_else(|_| "{}".to_string());
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&content) {
                if parsed.is_object() {
                    config_json = parsed;
                }
            }
        }

        if let Some(obj) = config_json.as_object_mut() {
            obj.insert("pictureOutputFolder".to_string(), serde_json::json!(path));
        }

        std::fs::write(
            &config_path,
            serde_json::to_string_pretty(&config_json).unwrap(),
        )
        .map_err(|e| format!("无法写入 config.json: {}", e))?;

        Ok(())
    } else {
        Err("找不到 Local AppData 目录".to_string())
    }
}

#[tauri::command]
pub async fn sys_get_vrc_config() -> Result<String, String> {
    if let Some(mut config_path) = dirs::data_local_dir() {
        config_path.pop();
        config_path.push("LocalLow");
        config_path.push("VRChat");
        config_path.push("VRChat");
        config_path.push("config.json");

        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path).map_err(|e| e.to_string())?;
            return Ok(content);
        } else {
            return Ok("{}".to_string());
        }
    }
    Err("Cannot find AppData directory".to_string())
}

#[tauri::command]
pub async fn sys_save_vrc_config(content: String) -> Result<(), String> {
    if let Some(mut config_path) = dirs::data_local_dir() {
        config_path.pop();
        config_path.push("LocalLow");
        config_path.push("VRChat");
        config_path.push("VRChat");

        if !config_path.exists() {
            std::fs::create_dir_all(&config_path)
                .map_err(|e| format!("Cannot create VRC directory: {}", e))?;
        }

        config_path.push("config.json");

        std::fs::write(&config_path, content)
            .map_err(|e| format!("Cannot write config.json: {}", e))?;
        Ok(())
    } else {
        Err("Cannot find AppData directory".to_string())
    }
}

#[tauri::command]
pub async fn sys_backup_database(
    app: tauri::AppHandle,
    state: tauri::State<'_, crate::db::DbState>,
    dest_path: String,
) -> Result<(), String> {
    // dest_path 由前端提供，必须经过与普通写文件相同的校验，
    // 否则 `fs::copy` 会变成一个「用数据库内容覆盖任意文件」的原语。
    let target = validate_write_target(&app, &dest_path)?;
    if let Ok(app_dir) = app.path().app_data_dir() {
        let database_path = app_dir.join("vrcdog.db");
        let legacy_path = app_dir.join("livehime.db");
        let source_path = if database_path.exists() {
            database_path
        } else {
            legacy_path
        };
        if source_path.exists() {
            // 库以 WAL 模式打开，未 checkpoint 时最近的写入全部留在 `vrcdog.db-wal` 里。
            // 只 `fs::copy` 主文件会得到一个静默残缺的备份（最近的活动记录/备注/收藏缺失），
            // 必须先把 WAL 落盘并截断，再拷贝。
            state
                .checkpoint_wal()
                .map_err(|e| format!("Backup aborted: {e}"))?;
            std::fs::copy(&source_path, &target).map_err(|e| format!("Backup failed: {}", e))?;
            return Ok(());
        }
        return Err("Database file not found".to_string());
    }
    Err("Cannot resolve app data directory".to_string())
}

#[tauri::command]
pub async fn sys_restore_database(
    app: tauri::AppHandle,
    state: tauri::State<'_, crate::db::DbState>,
    src_path: String,
) -> Result<(), String> {
    let trimmed = src_path.trim();
    if trimmed.is_empty() {
        return Err("Restore source path is empty".to_string());
    }
    let source = PathBuf::from(trimmed);
    if !source.is_absolute() || !source.is_file() {
        return Err("Restore source must be an existing absolute file path".to_string());
    }
    // 只接受 .db / .sqlite / 无扩展名的数据库文件，且不得来自受保护的系统目录
    let ext = source
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if !ext.is_empty() && !matches!(ext.as_str(), "db" | "sqlite" | "sqlite3" | "bak") {
        return Err("Restore source must be a database file (.db/.sqlite)".to_string());
    }
    if let Some(parent) = source.parent() {
        if let Ok(parent_canonical) = parent.canonicalize() {
            ensure_not_protected_dir(&parent_canonical)?;
        }
    }
    if let Ok(app_dir) = app.path().app_data_dir() {
        let database_path = app_dir.join("vrcdog.db");
        if !database_path.exists() {
            return Err("Database file not found".to_string());
        }
        // 走 SQLite Online Backup API 而不是 `fs::copy`：直接覆盖在用的主库文件会留下
        // 与主库不匹配的残留 -wal/-shm，重启后要么丢弃 WAL 内容、要么复活被覆盖前的旧数据。
        state
            .restore_from(&source)
            .map_err(|e| format!("Restore failed: {e}"))?;
        // 导入完成后把 WAL 落盘截断，保证紧接着的进程重启看到的是一致的主库文件。
        state
            .checkpoint_wal()
            .map_err(|e| format!("Restore applied but {e}"))?;
        return Ok(());
    }
    Err("Cannot resolve app data directory".to_string())
}

/// 敏感字符串的落盘目录与文件路径。
fn secure_storage_paths(
    app: &tauri::AppHandle,
    key: &str,
) -> Result<(std::path::PathBuf, std::path::PathBuf), String> {
    let app_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("无法获取应用数据目录: {e}"))?;
    let secure_dir = app_dir.join("secure");
    let file_path = secure_dir.join(format!("{}.txt", sanitize_filename(key)));
    Ok((secure_dir, file_path))
}

/// S4：敏感字符串落盘前先加密，绝不明文写入。
///
/// - Windows：DPAPI（CryptProtectData），密文绑定当前 Windows 用户账户，
///   把文件复制到别的机器或换个账户登录都无法解密。
/// - 其它平台：ChaCha20-Poly1305 + 安装级随机主密钥（弱于 DPAPI，但同样不是明文）。
#[cfg(target_os = "windows")]
fn protect_bytes(_secure_dir: &std::path::Path, plaintext: &[u8]) -> Result<Vec<u8>, String> {
    use windows::Win32::Security::Cryptography::{
        CryptProtectData, CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN,
    };
    if plaintext.is_empty() {
        return Ok(Vec::new());
    }
    unsafe {
        let mut input = CRYPT_INTEGER_BLOB {
            cbData: plaintext.len() as u32,
            pbData: plaintext.as_ptr() as *mut u8,
        };
        let mut output = CRYPT_INTEGER_BLOB::default();
        let ok = CryptProtectData(
            &mut input,
            None,
            None,
            None,
            None,
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        );
        if let Err(error) = ok {
            return Err(format!("DPAPI 加密失败: {error}"));
        }
        let slice = std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
        windows::Win32::Foundation::LocalFree(windows::Win32::Foundation::HLOCAL(
            output.pbData as *mut std::ffi::c_void,
        ));
        Ok(slice)
    }
}

#[cfg(target_os = "windows")]
fn unprotect_bytes(_secure_dir: &std::path::Path, blob: &[u8]) -> Result<Vec<u8>, String> {
    use windows::Win32::Security::Cryptography::{
        CryptUnprotectData, CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN,
    };
    if blob.is_empty() {
        return Ok(Vec::new());
    }
    unsafe {
        let mut input = CRYPT_INTEGER_BLOB {
            cbData: blob.len() as u32,
            pbData: blob.as_ptr() as *mut u8,
        };
        let mut output = CRYPT_INTEGER_BLOB::default();
        let ok = CryptUnprotectData(
            &mut input,
            None,
            None,
            None,
            None,
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        );
        if let Err(_) = ok {
            return Err("DPAPI 解密失败：数据可能已损坏或不属于当前用户".to_string());
        }
        let slice = std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
        windows::Win32::Foundation::LocalFree(windows::Win32::Foundation::HLOCAL(
            output.pbData as *mut std::ffi::c_void,
        ));
        Ok(slice)
    }
}

#[cfg(not(target_os = "windows"))]
fn local_master_key(secure_dir: &std::path::Path) -> Result<[u8; 32], String> {
    use sha2::{Digest, Sha256};
    let path = secure_dir.join(".localkey");
    let material = match std::fs::read(&path) {
        Ok(existing) if !existing.is_empty() => existing,
        _ => {
            let random = format!("{}{}", uuid::Uuid::new_v4(), uuid::Uuid::new_v4());
            std::fs::write(&path, random.as_bytes())
                .map_err(|e| format!("写入本机主密钥失败: {e}"))?;
            random.into_bytes()
        }
    };
    let mut key = [0u8; 32];
    key.copy_from_slice(&Sha256::digest(&material));
    Ok(key)
}

#[cfg(not(target_os = "windows"))]
fn protect_bytes(secure_dir: &std::path::Path, plaintext: &[u8]) -> Result<Vec<u8>, String> {
    use chacha20poly1305::{
        aead::{Aead, KeyInit},
        ChaCha20Poly1305, Nonce,
    };
    let key = local_master_key(secure_dir)?;
    let cipher = ChaCha20Poly1305::new_from_slice(&key).map_err(|e| format!("{e}"))?;
    let uuid_bytes = uuid::Uuid::new_v4().as_bytes().to_owned();
    let nonce = Nonce::from(<[u8; 12]>::try_from(&uuid_bytes[..12]).unwrap_or([0u8; 12]));
    let ciphertext = cipher
        .encrypt(&nonce, plaintext)
        .map_err(|e| format!("本机加密失败: {e}"))?;
    let mut out = nonce.to_vec();
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

#[cfg(not(target_os = "windows"))]
fn unprotect_bytes(secure_dir: &std::path::Path, blob: &[u8]) -> Result<Vec<u8>, String> {
    use chacha20poly1305::{
        aead::{Aead, KeyInit},
        ChaCha20Poly1305, Nonce,
    };
    if blob.len() < 12 {
        return Err("密文长度不足".to_string());
    }
    let key = local_master_key(secure_dir)?;
    let cipher = ChaCha20Poly1305::new_from_slice(&key).map_err(|e| format!("{e}"))?;
    let nonce = Nonce::from(<[u8; 12]>::try_from(&blob[..12]).unwrap_or([0u8; 12]));
    cipher
        .decrypt(&nonce, &blob[12..])
        .map_err(|_| "本机解密失败：数据可能已损坏".to_string())
}

/// 将敏感字符串加密后存储到应用数据目录（替代 localStorage，降低 XSS 窃取风险）
#[tauri::command]
pub async fn sys_store_secure_string(
    app: tauri::AppHandle,
    key: String,
    value: String,
) -> Result<(), String> {
    let (secure_dir, file_path) = secure_storage_paths(&app, &key)?;
    std::fs::create_dir_all(&secure_dir).map_err(|e| format!("创建安全目录失败: {e}"))?;
    let blob = protect_bytes(&secure_dir, value.as_bytes())?;
    std::fs::write(&file_path, blob).map_err(|e| format!("写入安全存储失败: {e}"))?;
    Ok(())
}

/// 从应用数据目录读取并解密敏感字符串。
/// 若遇到升级前遗留的明文文件，读出后立即改写为加密格式。
#[tauri::command]
pub async fn sys_load_secure_string(
    app: tauri::AppHandle,
    key: String,
) -> Result<Option<String>, String> {
    let (secure_dir, file_path) = secure_storage_paths(&app, &key)?;
    if !file_path.exists() {
        return Ok(None);
    }
    let raw = std::fs::read(&file_path).map_err(|e| format!("读取安全存储失败: {e}"))?;
    match unprotect_bytes(&secure_dir, &raw) {
        Ok(bytes) => Ok(Some(String::from_utf8(bytes).map_err(|e| format!("安全存储内容非法: {e}"))?)),
        Err(_) => {
            // 兼容旧版明文文件：取到值后立刻升级为加密存储
            let legacy = String::from_utf8(raw).map_err(|_| "安全存储内容无法解析".to_string())?;
            let _ = sys_store_secure_string(app, key, legacy.clone()).await;
            Ok(Some(legacy))
        }
    }
}

fn sanitize_filename(key: &str) -> String {
    key.chars()
        .map(|c| if c.is_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
        .collect()
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ProxyTestResult {
    pub success: bool,
    pub latency_ms: Option<u64>,
    pub message: String,
}

#[cfg(windows)]
pub fn read_windows_system_proxy() -> Option<String> {
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
    let server = server.trim();
    if server.is_empty() {
        return None;
    }
    if server.contains('=') {
        for part in server.split(';') {
            let mut it = part.splitn(2, '=');
            let (proto, val) = match (it.next(), it.next()) {
                (Some(p), Some(v)) => (p.trim().to_ascii_lowercase(), v.trim()),
                _ => continue,
            };
            if proto == "https" || proto == "http" {
                return Some(normalize_proxy_url(val));
            }
        }
        None
    } else {
        Some(normalize_proxy_url(server))
    }
}

pub fn normalize_proxy_url(val: &str) -> String {
    let trimmed = val.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if trimmed.starts_with("http://")
        || trimmed.starts_with("https://")
        || trimmed.starts_with("socks5://")
        || trimmed.starts_with("socks5h://")
    {
        trimmed.to_string()
    } else {
        format!("http://{trimmed}")
    }
}

pub fn get_effective_proxy(app: &tauri::AppHandle) -> Option<String> {
    if let Some(db_state) = app.try_state::<crate::db::DbState>() {
        if let Ok(conn) = db_state.conn.lock() {
            let get_val = |k: &str| -> Option<String> {
                conn.query_row(
                    "SELECT value FROM app_settings WHERE key = ?1",
                    rusqlite::params![k],
                    |row| row.get(0),
                )
                .ok()
            };

            let enabled = get_val("proxyEnabled")
                .map(|v| v == "true" || v == "\"true\"")
                .unwrap_or(false);
            if !enabled {
                return None;
            }

            let mode = get_val("proxyType").unwrap_or_else(|| "custom".to_string());
            let mode = mode.trim_matches('"');

            if mode == "system" {
                #[cfg(windows)]
                {
                    return read_windows_system_proxy();
                }
                #[cfg(not(windows))]
                {
                    return std::env::var("HTTPS_PROXY")
                        .or_else(|_| std::env::var("HTTP_PROXY"))
                        .ok();
                }
            } else {
                let url = get_val("proxyUrl").unwrap_or_default();
                let url = url.trim().trim_matches('"');
                if !url.is_empty() {
                    return Some(normalize_proxy_url(url));
                }
            }
        }
    }
    None
}

#[tauri::command]
pub async fn sys_test_proxy(proxy_url: Option<String>) -> Result<ProxyTestResult, String> {
    let mut builder = reqwest::Client::builder().timeout(Duration::from_secs(8));

    if let Some(url) = proxy_url.filter(|u| !u.trim().is_empty()) {
        let norm = normalize_proxy_url(&url);
        let proxy = reqwest::Proxy::all(&norm).map_err(|e| format!("无效的代理地址: {e}"))?;
        builder = builder.proxy(proxy);
    }

    let client = builder.build().map_err(|e| format!("创建测试客户端失败: {e}"))?;
    let start = std::time::Instant::now();

    let res = client
        .get("https://www.cloudflare.com/cdn-cgi/trace")
        .send()
        .await
        .map_err(|e| format!("代理连接超时或失败: {e}"))?;

    let elapsed = start.elapsed().as_millis() as u64;
    if res.status().is_success() {
        Ok(ProxyTestResult {
            success: true,
            latency_ms: Some(elapsed),
            message: format!("连接成功 ({}ms)", elapsed),
        })
    } else {
        Ok(ProxyTestResult {
            success: false,
            latency_ms: None,
            message: format!("HTTP 状态码: {}", res.status()),
        })
    }
}

#[tauri::command]
pub async fn sys_apply_proxy(
    app: tauri::AppHandle,
    enabled: bool,
    proxy_type: String,
    proxy_url: Option<String>,
) -> Result<(), String> {
    let effective = if enabled {
        if proxy_type == "system" {
            #[cfg(windows)]
            {
                read_windows_system_proxy()
            }
            #[cfg(not(windows))]
            {
                std::env::var("HTTPS_PROXY")
                    .or_else(|_| std::env::var("HTTP_PROXY"))
                    .ok()
            }
        } else {
            proxy_url
                .map(|u| normalize_proxy_url(&u))
                .filter(|u| !u.is_empty())
        }
    } else {
        None
    };

    if let Some(ref url) = effective {
        std::env::set_var("HTTP_PROXY", url);
        std::env::set_var("HTTPS_PROXY", url);
        std::env::set_var("ALL_PROXY", url);
        std::env::set_var("http_proxy", url);
        std::env::set_var("https_proxy", url);
        std::env::set_var("all_proxy", url);
    } else {
        std::env::remove_var("HTTP_PROXY");
        std::env::remove_var("HTTPS_PROXY");
        std::env::remove_var("ALL_PROXY");
        std::env::remove_var("http_proxy");
        std::env::remove_var("https_proxy");
        std::env::remove_var("all_proxy");
    }

    // 同步更新 VRChat API 客户端代理
    if let Some(vrc_state) = app.try_state::<crate::vrc_api::VrcState>() {
        let auth_cookie = if let Some(db_state) = app.try_state::<crate::db::DbState>() {
            db_state
                .conn
                .lock()
                .ok()
                .and_then(|c| {
                    c.query_row("SELECT cookie FROM auth WHERE id = 1", [], |r| r.get(0))
                        .ok()
                })
        } else {
            None
        };
        let _ = crate::vrc_api::vrc_set_proxy(vrc_state, effective.clone(), auth_cookie).await;
    }

    let _ = app.emit("app://proxy-updated", effective);
    Ok(())
}

#[cfg(test)]
mod write_guard_tests {
    use super::{ensure_not_protected_dir, protected_write_roots, SCRIPT_LIKE_EXT};

    #[test]
    fn protected_roots_are_discovered() {
        // 至少应当能识别出 Windows 系统目录（非 Windows 平台允许为空）
        #[cfg(target_os = "windows")]
        assert!(!protected_write_roots().is_empty());
        #[cfg(not(target_os = "windows"))]
        let _ = protected_write_roots();
    }

    #[test]
    fn system_directories_are_rejected() {
        #[cfg(target_os = "windows")]
        {
            if let Ok(windir) = std::env::var("WINDIR") {
                if let Ok(canonical) = std::fs::canonicalize(&windir) {
                    assert!(ensure_not_protected_dir(&canonical).is_err());
                    assert!(ensure_not_protected_dir(&canonical.join("System32")).is_err());
                }
            }
        }
        // 用户目录必须放行，否则「导出日志到桌面」这类功能会被误伤
        if let Some(home) = dirs::home_dir() {
            if let Ok(canonical) = std::fs::canonicalize(&home) {
                assert!(ensure_not_protected_dir(&canonical).is_ok());
            }
        }
    }

    #[test]
    fn script_extensions_cover_persistence_vectors() {
        for ext in ["bat", "cmd", "ps1", "vbs", "lnk", "exe", "dll", "reg"] {
            assert!(
                SCRIPT_LIKE_EXT.contains(&ext),
                "{ext} 应当被列为可执行/脚本类扩展名"
            );
        }
        // 普通导出格式不应被限制
        for ext in ["txt", "json", "csv", "xlsx", "log", "db"] {
            assert!(!SCRIPT_LIKE_EXT.contains(&ext), "{ext} 不应被限制");
        }
    }
}

