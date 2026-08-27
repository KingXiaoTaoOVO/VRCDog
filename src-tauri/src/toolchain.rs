use crate::{AppError, AppResult, EnvironmentStatus, ProgressPayload};
use futures_util::StreamExt;
use reqwest::header::{ACCEPT_RANGES, CONTENT_LENGTH, RANGE};
use std::fs;
use std::io::SeekFrom;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncSeekExt, AsyncWriteExt};
use tokio::sync::{Mutex, Semaphore};
use winreg::enums::*;
use winreg::{RegKey, HKEY};

#[cfg(target_os = "windows")]
fn is_network_drive(drive_letter: char) -> bool {
    use windows::Win32::Storage::FileSystem::GetDriveTypeW;
    use windows::core::PCWSTR;
    let path = format!(r"\\.\{}:", drive_letter);
    let wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe { GetDriveTypeW(PCWSTR(wide.as_ptr())) == 4 }
}

#[cfg(not(target_os = "windows"))]
fn is_network_drive(_drive_letter: char) -> bool {
    false
}

fn is_registry_key_exists(hkey: HKEY, path: &str) -> bool {
    let hk = RegKey::predef(hkey);
    hk.open_subkey(path).is_ok()
}

fn get_registry_string(hkey: HKEY, path: &str, value_name: &str) -> Option<String> {
    let hk = RegKey::predef(hkey);
    if let Ok(subkey) = hk.open_subkey(path) {
        if let Ok(val) = subkey.get_value::<String, _>(value_name) {
            return Some(val);
        }
    }
    None
}

/// Helper: Scans across all available Windows drives (C: through Z:) for given relative paths.
fn find_in_drives(relative_paths: &[&str]) -> Option<String> {
    for drive in b'C'..=b'Z' {
        let drive_str = format!("{}:", drive as char);
        // Skip network drives to avoid timeouts on disconnected mapped drives
        if is_network_drive(drive as char) {
            continue;
        }
        // Only check valid drives to avoid slow network/unmounted drive timeouts
        let drive_path = format!("{}\\", drive_str);
        if !Path::new(&drive_path).exists() {
            continue;
        }

        for rel_path in relative_paths {
            let full_path = format!("{}\\{}", drive_str, rel_path);
            if Path::new(&full_path).exists() {
                return Some(full_path);
            }
        }
    }
    None
}

#[tauri::command]
pub async fn check_system_status() -> AppResult<EnvironmentStatus> {
    // 1. Detect Unity Hub
    let mut hub_installed =
        is_registry_key_exists(HKEY_LOCAL_MACHINE, r#"SOFTWARE\Unity Technologies\Hub"#)
            || is_registry_key_exists(
                HKEY_LOCAL_MACHINE,
                r#"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\Unity Hub"#,
            );

    if !hub_installed {
        hub_installed = find_in_drives(&[r"Program Files\Unity Hub\Unity Hub.exe"]).is_some();
    }

    // 2. Detect Unity 2022.3.22f1
    let mut unity_installed = false;
    if let Some(loc) = get_registry_string(
        HKEY_CURRENT_USER,
        r#"SOFTWARE\Unity Technologies\Installer\Unity 2022.3.22f1"#,
        "Location x64",
    ) {
        if Path::new(&loc).join("Editor").join("Unity.exe").exists() {
            unity_installed = true;
        }
    }
    if !unity_installed {
        unity_installed =
            find_in_drives(&[r"Program Files\Unity\Hub\Editor\2022.3.22f1\Editor\Unity.exe"])
                .is_some();
    }

    // 3. Detect VCC / ALCOM
    let local_app_data = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| "C:\\".to_string());

    let mut vcc_installed =
        is_registry_key_exists(HKEY_CURRENT_USER, r#"Software\VRChat\CreatorCompanion"#);
    if !vcc_installed {
        let vcc_paths = [
            r"Program Files\CreatorCompanion\CreatorCompanion.exe",
            r"Programs\VRChat Creator Companion\CreatorCompanion.exe",
            r"CreatorCompanion\CreatorCompanion.exe",
        ];
        if find_in_drives(&vcc_paths).is_some()
            || Path::new(&format!(
                r"{}\Programs\VRChat Creator Companion\CreatorCompanion.exe",
                local_app_data
            ))
            .exists()
        {
            vcc_installed = true;
        }
    }

    let alcom_paths = [
        r"Program Files\ALCOM\ALCOM.exe",
        r"Programs\ALCOM\ALCOM.exe",
        r"ALCOM\ALCOM.exe",
        r"Programs\vrc-get-gui\vrc-get-gui.exe",
    ];
    let alcom_installed = find_in_drives(&alcom_paths).is_some()
        || Path::new(&format!(r"{}\Programs\ALCOM\ALCOM.exe", local_app_data)).exists()
        || Path::new(&format!(
            r"{}\Programs\vrc-get-gui\vrc-get-gui.exe",
            local_app_data
        ))
        .exists();

    // 4. Detect FFmpeg
    let ffmpeg_paths = [
        r"ffmpeg\bin\ffmpeg.exe",
        r"Program Files\ffmpeg\bin\ffmpeg.exe",
        r"ProgramData\chocolatey\bin\ffmpeg.exe",
        r"scoop\apps\ffmpeg\current\bin\ffmpeg.exe",
    ];
    let ffmpeg_installed = find_in_drives(&ffmpeg_paths).is_some()
        || Path::new(&format!(r"{}\VrcDog\ffmpeg\ffmpeg.exe", local_app_data)).exists()
        || Path::new(&format!(r"{}\VrcDog\ffmpeg\ffmpeg.exe", local_app_data)).exists()
        // Check PATH
        || std::process::Command::new("ffmpeg").arg("-version").output().is_ok();

    Ok(EnvironmentStatus {
        hub_installed,
        unity_installed,
        tool_installed: vcc_installed || alcom_installed,
        vcc_installed,
        alcom_installed,
        ffmpeg_installed,
    })
}

#[tauri::command]
pub async fn uninstall_software(target: String) -> AppResult<()> {
    let local_app_data = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| "C:\\".to_string());
    let mut uninstall_paths = vec![];

    match target.as_str() {
        "hub" => {
            uninstall_paths
                .push(r#"C:\Program Files\Unity Hub\Uninstall Unity Hub.exe"#.to_string());
        }
        "unity" => {
            if let Some(p) = find_in_drives(&[
                r"Program Files\Unity\Hub\Editor\2022.3.22f1\Editor\Uninstall.exe",
            ]) {
                uninstall_paths.push(p);
            }
        }
        "tool" | "vcc" | "alcom" => {
            let base_unins = [
                r"Program Files\CreatorCompanion\uninstall.exe",
                r"Program Files\CreatorCompanion\unins000.exe",
                r"Programs\VRChat Creator Companion\uninstall.exe",
                r"Programs\VRChat Creator Companion\unins000.exe",
                r"CreatorCompanion\uninstall.exe",
                r"CreatorCompanion\unins000.exe",
                r"Program Files\ALCOM\Uninstall ALCOM.exe",
                r"Program Files\ALCOM\unins000.exe",
                r"Programs\ALCOM\Uninstall ALCOM.exe",
                r"Programs\ALCOM\unins000.exe",
                r"ALCOM\Uninstall ALCOM.exe",
                r"ALCOM\unins000.exe",
                r"Programs\vrc-get-gui\Uninstall VRChat Package Manager.exe",
                r"Programs\vrc-get-gui\unins000.exe",
            ];

            // Add appdata paths explicitly
            uninstall_paths.push(format!(
                r"{}\Programs\VRChat Creator Companion\uninstall.exe",
                local_app_data
            ));
            uninstall_paths.push(format!(
                r"{}\Programs\VRChat Creator Companion\unins000.exe",
                local_app_data
            ));
            uninstall_paths.push(format!(
                r"{}\Programs\ALCOM\Uninstall ALCOM.exe",
                local_app_data
            ));
            uninstall_paths.push(format!(r"{}\Programs\ALCOM\unins000.exe", local_app_data));
            uninstall_paths.push(format!(
                r"{}\Programs\vrc-get-gui\Uninstall VRChat Package Manager.exe",
                local_app_data
            ));
            uninstall_paths.push(format!(
                r"{}\Programs\vrc-get-gui\unins000.exe",
                local_app_data
            ));

            // Find across drives
            for drive in b'C'..=b'Z' {
                let drive_str = format!("{}:", drive as char);
                if !Path::new(&format!("{}\\", drive_str)).exists() {
                    continue;
                }
                for p in &base_unins {
                    let full = format!("{}\\{}", drive_str, p);
                    if Path::new(&full).exists() {
                        uninstall_paths.push(full);
                    }
                }
            }
        }
        _ => return Err("未知的卸载目标".into()),
    };

    for path in uninstall_paths {
        if Path::new(&path).exists() {
            let ps_cmd = format!(
                "Start-Process -FilePath '{}' -ArgumentList '/S' -Verb RunAs -Wait",
                path
            );
            let _ = Command::new("powershell")
                .args(["-Command", &ps_cmd])
                .status();
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn launch_software(target: String) -> AppResult<()> {
    let local_app_data = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| "C:\\".to_string());

    let launch_path = match target.as_str() {
        "hub" => {
            let mut p = find_in_drives(&[r"Program Files\Unity Hub\Unity Hub.exe"]);
            if p.is_none() {
                if let Some(loc) = get_registry_string(
                    HKEY_LOCAL_MACHINE,
                    r#"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\Unity Hub"#,
                    "InstallLocation",
                ) {
                    let check = Path::new(&loc).join("Unity Hub.exe");
                    if check.exists() {
                        p = Some(check.to_string_lossy().to_string());
                    }
                }
            }
            p.unwrap_or_else(|| r#"C:\Program Files\Unity Hub\Unity Hub.exe"#.to_string())
        }
        "unity" => {
            let mut p = None;
            if let Some(loc) = get_registry_string(
                HKEY_CURRENT_USER,
                r#"SOFTWARE\Unity Technologies\Installer\Unity 2022.3.22f1"#,
                "Location x64",
            ) {
                let check = Path::new(&loc).join("Editor").join("Unity.exe");
                if check.exists() {
                    p = Some(check.to_string_lossy().to_string());
                }
            }
            if p.is_none() {
                p = find_in_drives(&[
                    r"Program Files\Unity\Hub\Editor\2022.3.22f1\Editor\Unity.exe",
                ]);
            }
            p.unwrap_or_else(|| {
                r#"C:\Program Files\Unity\Hub\Editor\2022.3.22f1\Editor\Unity.exe"#.to_string()
            })
        }
        "tool" | "vcc" | "alcom" => {
            let mut found = None;
            if target == "alcom" || target == "tool" {
                found = find_in_drives(&[
                    r"Program Files\ALCOM\ALCOM.exe",
                    r"Programs\ALCOM\ALCOM.exe",
                    r"ALCOM\ALCOM.exe",
                    r"Programs\vrc-get-gui\vrc-get-gui.exe",
                ])
                .or_else(|| {
                    let a1 = format!(r"{}\Programs\ALCOM\ALCOM.exe", local_app_data);
                    let a2 = format!(r"{}\Programs\vrc-get-gui\vrc-get-gui.exe", local_app_data);
                    if Path::new(&a1).exists() {
                        Some(a1)
                    } else if Path::new(&a2).exists() {
                        Some(a2)
                    } else {
                        None
                    }
                });
            }
            if found.is_none() && (target == "vcc" || target == "tool") {
                found = find_in_drives(&[
                    r"Program Files\CreatorCompanion\CreatorCompanion.exe",
                    r"Programs\VRChat Creator Companion\CreatorCompanion.exe",
                    r"CreatorCompanion\CreatorCompanion.exe",
                ])
                .or_else(|| {
                    let v = format!(
                        r"{}\Programs\VRChat Creator Companion\CreatorCompanion.exe",
                        local_app_data
                    );
                    if Path::new(&v).exists() {
                        Some(v)
                    } else {
                        None
                    }
                });
            }

            found.ok_or_else(|| AppError::from("未找到已安装的开发工具 (VCC / ALCOM)"))?
        }
        _ => return Err("未知的启动目标".into()),
    };

    if Path::new(&launch_path).exists() {
        Command::new(&launch_path)
            .spawn()
            .map_err(|e| format!("启动失败: {}", e))?;
        Ok(())
    } else {
        Err(format!("文件不存在: {}", launch_path).into())
    }
}

fn create_optimized_client() -> AppResult<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent("VrcDog/5.0-HyperEngine")
        .tcp_nodelay(true)
        .pool_max_idle_per_host(32)
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| format!("构建下载引擎失败: {}", e).into())
}

async fn download_file(
    app_handle: AppHandle,
    target: &str,
    url: &str,
    dest_path: &PathBuf,
) -> AppResult<()> {
    let client = create_optimized_client()?;

    let head_res = client
        .head(url)
        .send()
        .await
        .map_err(|e| format!("服务器连接失败: {}", e))?;
    if !head_res.status().is_success() {
        return Err(format!("下载请求被拒绝，状态码: {}", head_res.status()).into());
    }

    let total_size = head_res
        .headers()
        .get(CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0);

    let supports_ranges = head_res
        .headers()
        .get(ACCEPT_RANGES)
        .map(|v| v == "bytes")
        .unwrap_or(false);

    if total_size < 10 * 1024 * 1024 || !supports_ranges {
        return download_single_thread(app_handle, client, target, url, dest_path, total_size)
            .await;
    }

    let chunk_size = 20 * 1024 * 1024;
    let num_chunks = total_size.div_ceil(chunk_size);
    let max_concurrent = 8;
    let semaphore = Arc::new(Semaphore::new(max_concurrent));

    let file = fs::File::create(dest_path).map_err(|e| e.to_string())?;
    file.set_len(total_size)
        .map_err(|e| format!("无法预分配磁盘空间: {}", e))?;
    let shared_file = Arc::new(Mutex::new(tokio::fs::File::from_std(file)));
    let downloaded_bytes = Arc::new(AtomicU64::new(0));
    let mut tasks = vec![];

    let progress_target = target.to_string();
    let progress_downloaded = downloaded_bytes.clone();
    let progress_app_handle = app_handle.clone();

    let reporter_task = tokio::spawn(async move {
        let mut last_bytes = 0;
        let mut last_time = tokio::time::Instant::now();
        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            let current = progress_downloaded.load(Ordering::Relaxed);
            let now = tokio::time::Instant::now();
            let elapsed = now.duration_since(last_time).as_secs_f64();
            let speed = ((current - last_bytes) as f64 / elapsed) / 1024.0 / 1024.0;
            last_bytes = current;
            last_time = now;
            let progress = (current as f64 / total_size as f64) * 100.0;
            let _ = progress_app_handle.emit(
                "install_progress",
                ProgressPayload {
                    target: progress_target.clone(),
                    progress,
                    status: format!("极速多线程下载中 ({:.1} MB/s) ... {:.1}%", speed, progress),
                },
            );
            if current >= total_size {
                break;
            }
        }
    });

    for i in 0..num_chunks {
        let start = i * chunk_size;
        let end = std::cmp::min(start + chunk_size - 1, total_size - 1);
        let client_c = client.clone();
        let url_c = url.to_string();
        let sem_c = semaphore.clone();
        let file_c = shared_file.clone();
        let downloaded_c = downloaded_bytes.clone();

        let task = tokio::spawn(async move {
            let _permit = sem_c.acquire().await.unwrap();
            let mut retries = 3;
            loop {
                let req = client_c
                    .get(&url_c)
                    .header(RANGE, format!("bytes={}-{}", start, end));
                if let Ok(res) = req.send().await {
                    if res.status().is_success() {
                        let mut stream = res.bytes_stream();
                        let mut current_offset = start;
                        let mut chunk_ok = true;
                        while let Some(chunk_res) = stream.next().await {
                            match chunk_res {
                                Ok(data) => {
                                    let mut f = file_c.lock().await;
                                    f.seek(SeekFrom::Start(current_offset)).await.unwrap();
                                    f.write_all(&data).await.unwrap();
                                    drop(f);
                                    current_offset += data.len() as u64;
                                    downloaded_c.fetch_add(data.len() as u64, Ordering::Relaxed);
                                }
                                Err(_) => {
                                    chunk_ok = false;
                                    break;
                                }
                            }
                        }
                        if chunk_ok && current_offset > end {
                            break;
                        }
                    }
                }
                retries -= 1;
                if retries == 0 {
                    return Err(format!("无法下载区块: {}-{}", start, end));
                }
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
            Ok::<(), String>(())
        });
        tasks.push(task);
    }

    for task in tasks {
        task.await.map_err(|e| format!("线程崩溃: {}", e))??;
    }
    let _ = reporter_task.await;

    {
        let mut f = shared_file.lock().await;
        f.flush().await.map_err(|e| format!("刷盘失败: {}", e))?;
    }

    let actual_downloaded = downloaded_bytes.load(Ordering::Relaxed);
    if actual_downloaded < total_size {
        return Err(format!(
            "下载不完整：期望 {} 字节，实际只下载了 {} 字节",
            total_size, actual_downloaded
        )
        .into());
    }

    let file_meta = fs::metadata(dest_path).map_err(|e| format!("无法读取文件信息: {}", e))?;
    if file_meta.len() != total_size {
        let _ = fs::remove_file(dest_path);
        return Err(format!("文件大小不匹配：已删除损坏文件").into());
    }

    Ok(())
}

async fn download_single_thread(
    app_handle: AppHandle,
    client: reqwest::Client,
    target: &str,
    url: &str,
    dest_path: &PathBuf,
    total_size: u64,
) -> AppResult<()> {
    let res = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("下载失败: {}", e))?;
    let mut file = tokio::fs::File::create(dest_path)
        .await
        .map_err(|e| e.to_string())?;
    let mut downloaded: u64 = 0;
    let mut stream = res.bytes_stream();
    let mut last_progress = 0.0;

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.map_err(|e| format!("下载中断: {}", e))?;
        file.write_all(&chunk)
            .await
            .map_err(|e| format!("写入失败: {}", e))?;
        downloaded += chunk.len() as u64;

        if total_size > 0 {
            let progress = (downloaded as f64 / total_size as f64) * 100.0;
            if progress - last_progress >= 1.0 || downloaded == total_size {
                last_progress = progress;
                let _ = app_handle.emit(
                    "install_progress",
                    ProgressPayload {
                        target: target.to_string(),
                        progress,
                        status: format!("下载中 ({:.1}%)", progress),
                    },
                );
            }
        }
    }

    file.flush().await.map_err(|e| format!("刷盘失败: {}", e))?;
    if total_size > 0 && downloaded != total_size {
        let _ = std::fs::remove_file(dest_path);
        return Err(format!(
            "下载不完整：期望 {} 字节，实际 {} 字节",
            total_size, downloaded
        )
        .into());
    }

    Ok(())
}

fn register_unity_to_hub(app_handle: &AppHandle, unity_install_dir: &str, target: &str) {
    let appdata = std::env::var("APPDATA").unwrap_or_default();
    if appdata.is_empty() {
        return;
    }

    let hub_dir = Path::new(&appdata).join("UnityHub");
    let _ = fs::create_dir_all(&hub_dir);

    let editors_path = hub_dir.join("editors.json");
    let mut editors: serde_json::Value = if editors_path.exists() {
        match fs::read_to_string(&editors_path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or(serde_json::json!({})),
            Err(_) => serde_json::json!({}),
        }
    } else {
        serde_json::json!({})
    };

    if let Some(obj) = editors.as_object_mut() {
        let entry = serde_json::json!({
            "version": "2022.3.22f1",
            "location": [unity_install_dir],
            "manual": true
        });
        obj.insert("2022.3.22f1".to_string(), entry);
    }

    if let Ok(json_str) = serde_json::to_string_pretty(&editors) {
        let _ = fs::write(&editors_path, json_str);
    }

    let secondary_path = hub_dir.join("secondaryInstallPath.json");
    let parent_dir = Path::new(unity_install_dir)
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    if !parent_dir.is_empty() {
        let mut paths: Vec<String> = if secondary_path.exists() {
            match fs::read_to_string(&secondary_path) {
                Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
                Err(_) => Vec::new(),
            }
        } else {
            Vec::new()
        };

        if !paths.iter().any(|p| p == &parent_dir) {
            paths.push(parent_dir);
        }

        if let Ok(json_str) = serde_json::to_string_pretty(&paths) {
            let _ = fs::write(&secondary_path, json_str);
        }
    }

    let hub_exe = Path::new(r#"C:\Program Files\Unity Hub\Unity Hub.exe"#);
    if hub_exe.exists() {
        let _ = Command::new(hub_exe)
            .args([
                "--",
                "--headless",
                "install-path",
                "--set",
                unity_install_dir,
            ])
            .status();
    }

    let _ = app_handle.emit(
        "install_progress",
        ProgressPayload {
            target: target.to_string(),
            progress: 100.0,
            status: "已将 Unity 2022.3.22f1 注册到 Unity Hub".into(),
        },
    );
}

#[tauri::command]
pub async fn install_software(
    app_handle: AppHandle,
    target: String,
    path: String,
    tool: Option<String>,
    auto_delete: bool,
) -> AppResult<()> {
    let url = match target.as_str() {
        "hub" => "https://public-cdn.cloud.unity3d.com/hub/prod/UnityHubSetup.exe",
        "unity" => "https://download.unity3d.com/download_unity/887be4894c44/Windows64EditorInstaller/UnitySetup64-2022.3.22f1.exe",
        "tool" => {
            if let Some(ref t) = tool {
                if t == "vcc" {
                    "https://vrcpm.vrchat.cloud/vcc/Builds/2.4.5/VRChat_CreatorCompanion_Setup_2.4.5.exe"
                } else {
                    "https://github.com/vrc-get/vrc-get/releases/download/gui-v1.1.5/ALCOM-1.1.5-x86_64-setup.exe"
                }
            } else {
                return Err("Missing tool selection".into());
            }
        },
        "ffmpeg" => "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-gpl.zip",
        _ => return Err("Unknown target".into())
    };

    let temp_dir = std::env::temp_dir();
    let file_name = url.split('/').next_back().unwrap_or("setup.exe");
    let dest_path = temp_dir.join(file_name);

    download_file(app_handle.clone(), &target, url, &dest_path).await?;

    if target == "ffmpeg" {
        let _ = app_handle.emit(
            "install_progress",
            ProgressPayload {
                target: target.clone(),
                progress: 100.0,
                status: "正在解压 FFmpeg...".into(),
            },
        );

        let local_app_data = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| "C:\\".to_string());
        let vrcdog_dir = Path::new(&local_app_data).join("VrcDog");
        let ffmpeg_dir = vrcdog_dir.join("ffmpeg");
        let _ = fs::create_dir_all(&ffmpeg_dir);

        let ps_cmd = format!(
            "Expand-Archive -Path '{}' -DestinationPath '{}' -Force; Move-Item -Path '{}\\ffmpeg-master-latest-win64-gpl\\bin\\ffmpeg.exe' -Destination '{}\\ffmpeg.exe' -Force; Remove-Item -Recurse -Force '{}\\ffmpeg-master-latest-win64-gpl'",
            dest_path.display(),
            ffmpeg_dir.display(),
            ffmpeg_dir.display(),
            ffmpeg_dir.display(),
            ffmpeg_dir.display()
        );
        let status = Command::new("powershell")
            .args(["-Command", &ps_cmd])
            .status();

        if auto_delete {
            let _ = fs::remove_file(&dest_path);
        }

        if status.map(|s| s.success()).unwrap_or(false) {
            let _ = app_handle.emit(
                "install_progress",
                ProgressPayload {
                    target: target.clone(),
                    progress: 100.0,
                    status: "FFmpeg 安装成功！".into(),
                },
            );
            return Ok(());
        } else {
            return Err("FFmpeg 解压失败".into());
        }
    }

    let _ = app_handle.emit(
        "install_progress",
        ProgressPayload {
            target: target.clone(),
            progress: 100.0,
            status: "下载完成，正在后台静默安装 (请耐心等待)...".into(),
        },
    );

    let mut cmd = Command::new(&dest_path);
    if target == "hub" {
        cmd.arg("/S");
    } else if target == "unity" {
        cmd.arg("/S");
        let unity_install_dir = if !path.trim().is_empty() && path != "C:\\Program Files\\" {
            let custom = format!("{}Unity\\Hub\\Editor\\2022.3.22f1", path);
            cmd.arg(format!("/D={}", custom));
            custom
        } else {
            "C:\\Program Files\\Unity\\Hub\\Editor\\2022.3.22f1".to_string()
        };

        let _ = app_handle.emit(
            "install_progress",
            ProgressPayload {
                target: target.clone(),
                progress: 100.0,
                status: "正在安装 Unity 编辑器 (可能需要数分钟，请耐心等待)...".into(),
            },
        );

        match cmd.status() {
            Ok(exit_status) => {
                if !exit_status.success() {
                    let code = exit_status.code().unwrap_or(-1);
                    if code != 0 {
                        let unity_exe = Path::new(&unity_install_dir)
                            .join("Editor")
                            .join("Unity.exe");
                        if !unity_exe.exists() {
                            return Err(format!(
                                "Unity 安装程序退出码: {}，且未检测到安装文件",
                                code
                            )
                            .into());
                        }
                    }
                }
            }
            Err(e) => return Err(format!("无法运行安装程序: {}", e).into()),
        }

        let unity_exe = Path::new(&unity_install_dir)
            .join("Editor")
            .join("Unity.exe");
        if unity_exe.exists() {
            register_unity_to_hub(&app_handle, &unity_install_dir, &target);
        }

        if auto_delete {
            let _ = fs::remove_file(&dest_path);
        }
        let _ = app_handle.emit(
            "install_progress",
            ProgressPayload {
                target: target.clone(),
                progress: 100.0,
                status: "Unity 2022.3.22f1 安装成功！已注册到 Unity Hub".into(),
            },
        );
        return Ok(());
    } else if target == "tool" {
        cmd.args(["/VERYSILENT", "/SUPPRESSMSGBOXES", "/NORESTART"]);
    }

    match cmd.status() {
        Ok(status) if status.success() => {
            if auto_delete {
                let _ = fs::remove_file(&dest_path);
            }
            let _ = app_handle.emit(
                "install_progress",
                ProgressPayload {
                    target: target.clone(),
                    progress: 100.0,
                    status: "安装成功！".into(),
                },
            );
            Ok(())
        }
        Ok(status) => Err(format!("安装程序异常退出，代码: {}", status).into()),
        Err(e) => Err(format!("无法运行安装程序: {}", e).into()),
    }
}
