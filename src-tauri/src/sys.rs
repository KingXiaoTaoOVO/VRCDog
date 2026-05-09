use crate::AppResult;
use std::fs;
use std::path::PathBuf;

#[tauri::command]
pub async fn sys_clear_vrchat_cache() -> AppResult<u64> {
    let mut cache_dir = dirs::data_local_dir()
        .ok_or_else(|| "Could not find LocalAppData".to_string())?;
    
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
pub async fn sys_check_steamvr() -> crate::AppResult<bool> {
    use sysinfo::System;
    let sys = System::new_all();
    // Detect all major VR runtime processes:
    // SteamVR, Meta/Oculus Link, Pico Streaming, HTC Vive, Virtual Desktop, ALVR
    let vr_processes: &[&str] = &[
        // SteamVR
        "vrserver.exe", "vrmonitor.exe", "vrdashboard.exe", "vrcompositor.exe",
        // Meta / Oculus (Quest Link / Air Link)
        "ovrserver_x64.exe", "oculusclient.exe", "ovrserviceprocess.exe",
        "oculus_runtime.exe", "oculusdash.exe",
        // Pico (Streaming Assistant / Business Streaming)
        "pico_streaming_assistant.exe", "pico connect.exe", "picolink.exe",
        // HTC Vive (Vive Console / VIVEPORT)
        "htcvrs.exe", "viveport.exe", "viveconsole.exe",
        // Virtual Desktop (cross-device)
        "virtual desktop.server.exe", "virtualdesktop.server.exe",
        // ALVR (open source VR streaming)
        "alvr_dashboard.exe", "alvr_server.exe",
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
        
        let (key, _) = hkcu.create_subkey(path)
            .map_err(|e| e.to_string())?;
            
        let app_name = "VrcDog";
        
        if enable {
            let exe_path = std::env::current_exe()
                .map_err(|e| e.to_string())?;
                
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
pub async fn sys_open_dir(target: String) -> Result<(), String> {
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
        },
        "cache" => {
            if let Some(mut local_low) = dirs::data_local_dir() {
                local_low.pop();
                local_low.push("LocalLow");
                local_low.push("VRChat");
                local_low.push("vrchat");
                local_low.push("HTTPCache");
                path = local_low;
            }
        },
        "screenshots" => {
            if let Some(mut pic_dir) = dirs::picture_dir() {
                pic_dir.push("VRChat");
                path = pic_dir;
            }
        },
        "crash_reports" => {
            if let Some(mut local) = dirs::data_local_dir() {
                local.push("Temp");
                local.push("VRChat");
                path = local;
            }
        },
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
    
    std::fs::write(&manifest_path, serde_json::to_string_pretty(&manifest_content).unwrap()).map_err(|e| e.to_string())?;
    
    // Call vrpathreg to add the manifest
    let vrpathreg = "C:\\Program Files (x86)\\Steam\\steamapps\\common\\SteamVR\\bin\\win64\\vrpathreg.exe";
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

#[tauri::command]
pub async fn sys_open_url(url: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&url)
            .spawn()
            .map_err(|e| format!("Failed to open URL: {}", e))?;
    }
    #[cfg(not(target_os = "windows"))]
    {
        // Simple fallback
        std::process::Command::new("xdg-open")
            .arg(&url)
            .spawn()
            .unwrap_or_else(|_| std::process::Command::new("open").arg(&url).spawn().unwrap());
    }
    Ok(())
}

