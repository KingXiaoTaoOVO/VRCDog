use crate::AppResult;
use base64::Engine;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use tauri::Manager;

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

#[tauri::command]
pub async fn sys_save_text_file(path: String, content: String) -> Result<(), String> {
    std::fs::write(&path, content).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn sys_save_binary_file(path: String, content: Vec<u8>) -> Result<(), String> {
    std::fs::write(&path, content).map_err(|e| e.to_string())
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
        _ => return Err("未知的目录目标".to_string()),
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
    let dir = exe_path.parent().unwrap();
    let manifest_path = dir.join("vrcdog_manifest.vrmanifest");

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

    std::fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&manifest_content).unwrap(),
    )
    .map_err(|e| e.to_string())?;

    // Call vrpathreg to add the manifest
    let vrpathreg =
        "C:\\Program Files (x86)\\Steam\\steamapps\\common\\SteamVR\\bin\\win64\\vrpathreg.exe";
    if std::path::Path::new(vrpathreg).exists() {
        std::process::Command::new(vrpathreg)
            .args(["addapp", manifest_path.to_str().unwrap()])
            .output()
            .map_err(|e| e.to_string())?;
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
pub async fn sys_backup_database(app: tauri::AppHandle, dest_path: String) -> Result<(), String> {
    if let Ok(app_dir) = app.path().app_data_dir() {
        let database_path = app_dir.join("vrcdog.db");
        let legacy_path = app_dir.join("livehime.db");
        let source_path = if database_path.exists() {
            database_path
        } else {
            legacy_path
        };
        if source_path.exists() {
            std::fs::copy(&source_path, &dest_path).map_err(|e| format!("Backup failed: {}", e))?;
            return Ok(());
        }
        return Err("Database file not found".to_string());
    }
    Err("Cannot resolve app data directory".to_string())
}

#[tauri::command]
pub async fn sys_restore_database(app: tauri::AppHandle, src_path: String) -> Result<(), String> {
    if let Ok(app_dir) = app.path().app_data_dir() {
        let database_path = app_dir.join("vrcdog.db");
        std::fs::copy(&src_path, &database_path).map_err(|e| format!("Restore failed: {}", e))?;
        return Ok(());
    }
    Err("Cannot resolve app data directory".to_string())
}
