use std::path::PathBuf;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use sysinfo::System;
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::AsyncWriteExt;
use std::sync::Mutex;
use super::midi_backend::MidiOutputBackend;

const FALLBACK_VERSION: &str = "1.0.16.27";
const FALLBACK_DOWNLOAD_URL: &str = "https://www.tobias-erichsen.de/wp-content/uploads/2020/01/loopMIDISetup_1_0_16_27.zip";
const OFFICIAL_PAGE_URL: &str = "https://www.tobias-erichsen.de/software/loopmidi.html";

lazy_static::lazy_static! {
    static ref CACHED_OFFICIAL_LOOPMIDI_INFO: Mutex<Option<(String, String)>> = Mutex::new(None);
    static ref SYS_PROCESS_TRACKER: Mutex<System> = Mutex::new(System::new());
}

pub fn get_cached_or_default_official_info() -> (String, String) {
    if let Ok(guard) = CACHED_OFFICIAL_LOOPMIDI_INFO.lock() {
        if let Some(ref info) = *guard {
            return info.clone();
        }
    }
    (FALLBACK_VERSION.to_string(), FALLBACK_DOWNLOAD_URL.to_string())
}

pub fn set_cached_official_info(ver: String, url: String) {
    if let Ok(mut guard) = CACHED_OFFICIAL_LOOPMIDI_INFO.lock() {
        *guard = Some((ver, url));
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopMidiStatus {
    pub installed: bool,
    pub running: bool,
    pub has_virtual_port: bool,
    pub port_names: Vec<String>,
    pub installed_path: Option<String>,
    pub installed_version: Option<String>,
    pub latest_version: String,
    pub download_url: String,
    pub is_latest: bool,
    pub last_checked_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopMidiInstallProgress {
    pub phase: String, // "checking", "downloading", "extracting", "launching", "completed", "error"
    pub progress: f64, // 0.0 ~ 1.0
    pub bytes_downloaded: u64,
    pub total_bytes: u64,
    pub message: String,
}

fn emit_progress(
    app: &AppHandle,
    phase: &str,
    progress: f64,
    downloaded: u64,
    total: u64,
    message: &str,
) {
    let payload = LoopMidiInstallProgress {
        phase: phase.to_string(),
        progress,
        bytes_downloaded: downloaded,
        total_bytes: total,
        message: message.to_string(),
    };
    let _ = app.emit("vrpiano_loopmidi_progress", payload);
}

/// Detects if loopMIDI is installed on the system and returns its path and detected version.
pub fn detect_installed_loopmidi() -> Option<(PathBuf, Option<String>)> {
    #[cfg(target_os = "windows")]
    {
        // 1. Check running processes first for exact executable path
        let mut sys = System::new();
        sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
        for p in sys.processes().values() {
            let name = p.name().to_string_lossy().to_lowercase();
            if name.contains("loopmidi") {
                if let Some(exe) = p.exe() {
                    if exe.exists() {
                        return Some((exe.to_path_buf(), None));
                    }
                }
            }
        }

        use winreg::enums::*;
        use winreg::RegKey;

        let reg_bases = [
            (HKEY_LOCAL_MACHINE, r#"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall"#),
            (HKEY_LOCAL_MACHINE, r#"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall"#),
            (HKEY_CURRENT_USER, r#"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall"#),
        ];

        for (hkey, base_path) in reg_bases {
            let root = RegKey::predef(hkey);
            if let Ok(uninstall_key) = root.open_subkey(base_path) {
                for subkey_name in uninstall_key.enum_keys().flatten() {
                    if let Ok(subkey) = uninstall_key.open_subkey(&subkey_name) {
                        let display_name: String = subkey.get_value("DisplayName").unwrap_or_default();
                        if display_name.to_lowercase().contains("loopmidi") {
                            let version: Option<String> = subkey.get_value("DisplayVersion").ok();
                            let install_loc: String = subkey.get_value("InstallLocation").unwrap_or_default();
                            if !install_loc.is_empty() {
                                let exe_path = PathBuf::from(&install_loc).join("loopMIDI.exe");
                                if exe_path.exists() {
                                    return Some((exe_path, version));
                                }
                            }
                            let display_icon: String = subkey.get_value("DisplayIcon").unwrap_or_default();
                            let icon_path = display_icon.split(',').next().unwrap_or("").trim().trim_matches('"');
                            if !icon_path.is_empty() && icon_path.ends_with(".exe") {
                                let p = PathBuf::from(icon_path);
                                if p.exists() {
                                    return Some((p, version));
                                }
                            }
                        }
                    }
                }
            }
        }

        // Check common Program Files locations across available drives
        let rel_paths = [
            r"Program Files (x86)\Tobias Erichsen\loopMIDI\loopMIDI.exe",
            r"Program Files\Tobias Erichsen\loopMIDI\loopMIDI.exe",
        ];

        for drive in b'C'..=b'Z' {
            let drive_char = drive as char;
            for rel in &rel_paths {
                let candidate = PathBuf::from(format!(r"{}:\{}", drive_char, rel));
                if candidate.exists() {
                    return Some((candidate, None));
                }
            }
        }
    }

    None
}

/// Checks if the loopMIDI process is actively running on Windows.
pub fn is_loopmidi_process_running() -> bool {
    #[cfg(target_os = "windows")]
    {
        use windows::core::w;
        use windows::Win32::UI::WindowsAndMessaging::FindWindowW;
        unsafe {
            if let Ok(hwnd) = FindWindowW(None, w!("loopMIDI")) {
                if !hwnd.is_invalid() {
                    return true;
                }
            }
        }
    }

    if let Ok(mut sys) = SYS_PROCESS_TRACKER.lock() {
        sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
        return sys.processes().values().any(|p| {
            let name = p.name().to_string_lossy().to_lowercase();
            name.contains("loopmidi")
        });
    }
    false
}

/// Lists all MIDI output ports that belong to loopMIDI or virtual MIDI cables.
pub fn get_active_loopmidi_ports() -> Vec<String> {
    let devices = MidiOutputBackend::list_usb_devices();
    devices
        .into_iter()
        .filter(|d| {
            let name_lower = d.name.to_lowercase();
            name_lower.contains("loopmidi") || name_lower.contains("virtual") || name_lower.contains("vrc")
        })
        .map(|d| d.name)
        .collect()
}

/// Checks the official Tobias Erichsen website for the latest loopMIDI version and download URL.
pub async fn check_official_latest_loopmidi() -> (String, String) {
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .build()
    {
        Ok(c) => c,
        Err(_) => return get_cached_or_default_official_info(),
    };

    let resp = match client.get(OFFICIAL_PAGE_URL).send().await {
        Ok(r) if r.status().is_success() => r,
        _ => return get_cached_or_default_official_info(),
    };

    let body = match resp.text().await {
        Ok(t) => t,
        Err(_) => return get_cached_or_default_official_info(),
    };

    // Regex match: https://www.tobias-erichsen.de/wp-content/uploads/2020/01/loopMIDISetup_1_0_16_27.zip
    let re = regex::Regex::new(r#"https?://[^\s"'<>]+/wp-content/uploads/[^\s"'<>]+/loopMIDISetup_([0-9_]+)\.zip"#).unwrap();
    if let Some(caps) = re.captures(&body) {
        let full_url = caps.get(0).map(|m| m.as_str().to_string()).unwrap_or_else(|| FALLBACK_DOWNLOAD_URL.to_string());
        let raw_ver = caps.get(1).map(|m| m.as_str()).unwrap_or("1_0_16_27");
        let ver = raw_ver.replace('_', ".");
        set_cached_official_info(ver.clone(), full_url.clone());
        return (ver, full_url);
    }

    get_cached_or_default_official_info()
}

/// Compares two version strings (e.g. "1.0.16.27" vs "1.0.16.27").
/// Returns true if installed >= latest.
fn is_version_up_to_date(installed: &str, latest: &str) -> bool {
    let parse_nums = |s: &str| -> Vec<u32> {
        s.split(|c: char| c == '.' || c == '_')
            .filter_map(|p| p.parse::<u32>().ok())
            .collect()
    };
    let inst_parts = parse_nums(installed);
    let late_parts = parse_nums(latest);

    if inst_parts.is_empty() || late_parts.is_empty() {
        return false;
    }

    for (a, b) in inst_parts.iter().zip(late_parts.iter()) {
        if a > b {
            return true;
        }
        if a < b {
            return false;
        }
    }

    inst_parts.len() >= late_parts.len()
}

/// Gets the complete current loopMIDI status, using cached or fallback official info (instant, 0 network blocking).
pub async fn get_loopmidi_status() -> LoopMidiStatus {
    let (latest_version, download_url) = get_cached_or_default_official_info();
    let installed_info = detect_installed_loopmidi();
    let port_names = get_active_loopmidi_ports();
    let has_virtual_port = !port_names.is_empty();
    let mut running = is_loopmidi_process_running();
    if has_virtual_port {
        running = true;
    }
    let installed = installed_info.is_some() || running || has_virtual_port;
    let installed_path = installed_info.as_ref().map(|(p, _)| p.to_string_lossy().to_string());
    let installed_version = installed_info.as_ref().and_then(|(_, v)| v.clone());

    let is_latest = if let Some(ref inst_ver) = installed_version {
        is_version_up_to_date(inst_ver, &latest_version)
    } else {
        installed
    };

    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    LoopMidiStatus {
        installed,
        running,
        has_virtual_port,
        port_names,
        installed_path,
        installed_version,
        latest_version,
        download_url,
        is_latest,
        last_checked_at: Some(now),
    }
}

/// Automatically downloads the official latest loopMIDI installer, unzips it,
/// and launches the installation wizard with administrative privileges.
pub async fn download_and_install_loopmidi(app: AppHandle) -> Result<(), String> {
    emit_progress(&app, "checking", 0.05, 0, 0, "正在连接官方服务器检测最新版 loopMIDI...");

    let (latest_version, download_url) = check_official_latest_loopmidi().await;

    let cache_dir = app
        .path()
        .app_cache_dir()
        .unwrap_or_else(|_| std::env::temp_dir().join("vrcdog"))
        .join("loopmidi");
    std::fs::create_dir_all(&cache_dir).map_err(|e| format!("创建缓存目录失败: {e}"))?;

    let zip_filename = format!("loopMIDISetup_{}.zip", latest_version.replace('.', "_"));
    let zip_path = cache_dir.join(&zip_filename);
    let extract_dir = cache_dir.join("installer");
    std::fs::create_dir_all(&extract_dir).map_err(|e| format!("创建解压目录失败: {e}"))?;

    emit_progress(
        &app,
        "downloading",
        0.10,
        0,
        0,
        &format!("正在下载官方最新版 loopMIDI v{latest_version}..."),
    );

    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .build()
        .map_err(|e| format!("HTTP Client 初始化失败: {e}"))?;

    let resp = client
        .get(&download_url)
        .send()
        .await
        .map_err(|e| format!("下载 loopMIDI 官方安装包失败: {e}"))?;

    if !resp.status().is_success() {
        let err_msg = format!("官方下载服务器返回错误状态: {}", resp.status());
        emit_progress(&app, "error", 0.0, 0, 0, &err_msg);
        return Err(err_msg);
    }

    let total_bytes = resp.content_length().unwrap_or(7_884_702);
    let mut downloaded: u64 = 0;
    let mut stream = resp.bytes_stream();
    let mut file = tokio::fs::File::create(&zip_path)
        .await
        .map_err(|e| format!("创建临时安装包文件失败: {e}"))?;

    while let Some(chunk_res) = stream.next().await {
        let chunk = chunk_res.map_err(|e| format!("下载过程发生中断: {e}"))?;
        file.write_all(&chunk)
            .await
            .map_err(|e| format!("写入文件失败: {e}"))?;
        downloaded += chunk.len() as u64;
        let pct = (downloaded as f64 / total_bytes as f64).clamp(0.0, 1.0);
        let prog = (0.05 + pct * 0.85).clamp(0.0, 1.0);
        emit_progress(
            &app,
            "downloading",
            prog,
            downloaded,
            total_bytes,
            "正在从官方源高速下载安装包...",
        );
    }
    file.flush().await.ok();
    drop(file);

    emit_progress(
        &app,
        "extracting",
        0.85,
        downloaded,
        total_bytes,
        "下载完成，正在自动解压官方安装程序...",
    );

    let ps_cmd = format!(
        "Expand-Archive -LiteralPath '{}' -DestinationPath '{}' -Force",
        zip_path.to_string_lossy(),
        extract_dir.to_string_lossy()
    );

    let extract_status = std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command", &ps_cmd])
        .status()
        .map_err(|e| format!("解压安装包失败: {e}"))?;

    if !extract_status.success() {
        let err_msg = "解压安装包失败，请检查系统 PowerShell 环境".to_string();
        emit_progress(&app, "error", 0.0, 0, 0, &err_msg);
        return Err(err_msg);
    }

    let installer_exe = extract_dir.join("loopMIDISetup.exe");
    if !installer_exe.exists() {
        let err_msg = "安装包内未找到 loopMIDISetup.exe".to_string();
        emit_progress(&app, "error", 0.0, 0, 0, &err_msg);
        return Err(err_msg);
    }

    emit_progress(
        &app,
        "launching",
        0.95,
        downloaded,
        total_bytes,
        "正在启动官方安装向导，请在系统提示时允许运行...",
    );

    let run_cmd = format!(
        "Start-Process -FilePath '{}' -Verb RunAs",
        installer_exe.to_string_lossy()
    );

    std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command", &run_cmd])
        .spawn()
        .map_err(|e| format!("启动安装向导失败: {e}"))?;

    emit_progress(
        &app,
        "completed",
        1.0,
        total_bytes,
        total_bytes,
        "已启动官方安装向导，完成安装后系统将自动识别并连接！",
    );

    Ok(())
}

/// Automatically launches loopMIDI.exe if installed and brings its window to the foreground.
pub fn launch_loopmidi() -> Result<(), String> {
    let (exe_path, _) = detect_installed_loopmidi()
        .ok_or_else(|| "未在系统中找到 loopMIDI 安装路径，请先执行安装".to_string())?;

    let parent_dir = exe_path.parent();
    let mut cmd = std::process::Command::new(&exe_path);
    if let Some(dir) = parent_dir {
        cmd.current_dir(dir);
    }

    let spawn_res = cmd.spawn();
    if spawn_res.is_err() {
        let ps_cmd = format!("Start-Process -FilePath '{}'", exe_path.to_string_lossy());
        std::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", &ps_cmd])
            .spawn()
            .map_err(|e| format!("启动 loopMIDI 进程失败: {e}"))?;
    }

    #[cfg(target_os = "windows")]
    {
        use windows::Win32::Foundation::{BOOL, HWND, LPARAM};
        use windows::Win32::UI::WindowsAndMessaging::{
            EnumWindows, GetWindowTextW, SetForegroundWindow, ShowWindow, SW_RESTORE, SW_SHOW,
        };

        unsafe extern "system" fn enum_window_proc(hwnd: HWND, _lparam: LPARAM) -> BOOL {
            let mut title = [0u16; 256];
            let len = GetWindowTextW(hwnd, &mut title);
            if len > 0 {
                let title_str = String::from_utf16_lossy(&title[..len as usize]);
                if title_str.to_lowercase().contains("loopmidi") {
                    let _ = ShowWindow(hwnd, SW_RESTORE);
                    let _ = ShowWindow(hwnd, SW_SHOW);
                    let _ = SetForegroundWindow(hwnd);
                    return BOOL(0);
                }
            }
            BOOL(1)
        }

        unsafe {
            let _ = EnumWindows(Some(enum_window_proc), LPARAM(0));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_up_to_date_comparison() {
        assert!(is_version_up_to_date("1.0.16.27", "1.0.16.27"));
        assert!(is_version_up_to_date("1.0.17.0", "1.0.16.27"));
        assert!(is_version_up_to_date("2.0.0.0", "1.0.16.27"));
        assert!(!is_version_up_to_date("1.0.15.0", "1.0.16.27"));
        assert!(!is_version_up_to_date("0.9.0.0", "1.0.16.27"));
    }

    #[test]
    fn test_regex_extracts_official_download_link() {
        let sample_html = r#"
            <h1>loopMIDI</h1>
            <p><a href="https://www.tobias-erichsen.de/wp-content/uploads/2020/01/loopMIDISetup_1_0_16_27.zip">download loopMIDI</a></p>
        "#;
        let re = regex::Regex::new(r#"https?://[^\s"'<>]+/wp-content/uploads/[^\s"'<>]+/loopMIDISetup_([0-9_]+)\.zip"#).unwrap();
        let caps = re.captures(sample_html).expect("Should match download URL");
        let full_url = caps.get(0).unwrap().as_str();
        let raw_ver = caps.get(1).unwrap().as_str();
        assert_eq!(full_url, "https://www.tobias-erichsen.de/wp-content/uploads/2020/01/loopMIDISetup_1_0_16_27.zip");
        assert_eq!(raw_ver.replace('_', "."), "1.0.16.27");
    }

    #[test]
    fn test_status_defaults_gracefully() {
        let status = LoopMidiStatus {
            installed: false,
            running: false,
            has_virtual_port: false,
            port_names: vec![],
            installed_path: None,
            installed_version: None,
            latest_version: "1.0.16.27".to_string(),
            download_url: FALLBACK_DOWNLOAD_URL.to_string(),
            is_latest: false,
            last_checked_at: None,
        };
        assert!(!status.installed);
        assert!(!status.is_latest);
    }
}
