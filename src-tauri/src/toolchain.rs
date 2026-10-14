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
            // 安全：将路径作为独立参数传递，避免拼进 PowerShell 字符串后被 ' 注入并以 RunAs 提权执行
            let _ = Command::new("powershell")
                .args([
                    "-NoProfile",
                    "-Command",
                    "Start-Process",
                    "-FilePath",
                    &path,
                    "-ArgumentList",
                    "/S",
                    "-Verb",
                    "RunAs",
                    "-Wait",
                ])
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
        // 安全：保留 TLS 证书校验，防止 MITM 劫持下载并投放恶意二进制
        .build()
        .map_err(|e| format!("构建下载引擎失败: {}", e).into())
}

/// 把「各区块已完成字节之和」钳制在文件总大小内。
/// 区块重试、服务端多送字节等情况都可能让求和超过 total，直接拿来算进度会显示 100%+，
/// 并让完整性校验误判成「下载完成」。
fn clamp_total(sum: u64, total: u64) -> u64 {
    if total == 0 {
        0
    } else {
        sum.min(total)
    }
}

/// 单个区块的完成字节数永远不能超过该区块自身的长度。
fn clamp_up(done: u64, len: u64) -> u64 {
    done.min(len)
}

/// 把外部安装程序放到阻塞线程池里等待。
///
/// Unity / Unity Hub / ALCOM / PowerShell 解压一跑就是几十秒到几分钟，
/// 直接在 `async fn` 里调用 `Command::status()` 会把一个 tokio worker 占死，
/// 期间其它 IPC 命令、事件推送、心跳都会被拖慢甚至卡住。
async fn wait_for_child(mut cmd: std::process::Command) -> AppResult<std::process::ExitStatus> {
    tauri::async_runtime::spawn_blocking(move || cmd.status())
        .await
        .map_err(|e| AppError::from(format!("安装任务崩溃: {e}")))?
        .map_err(|e| AppError::from(format!("无法运行安装程序: {e}")))
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
    // 每个区块各自记录「本区块已完成字节数」，而不是往全局计数器 fetch_add：
    // 区块重试时旧的 `fetch_add(data.len())` 会把同一批字节重复累加，进度能冲到 100% 以上，
    // 并让下面的完整性校验误判成「下载完成」。
    let chunk_progress: Arc<Vec<AtomicU64>> =
        Arc::new((0..num_chunks as usize).map(|_| AtomicU64::new(0)).collect());
    // 任一区块彻底失败时置位，其余区块协作式退出，不再继续写盘占带宽。
    let cancelled = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let mut tasks = vec![];

    let progress_target = target.to_string();
    let progress_chunks = chunk_progress.clone();
    let progress_cancelled = cancelled.clone();
    let progress_app_handle = app_handle.clone();

    let reporter_task = tokio::spawn(async move {
        let mut last_bytes = 0u64;
        let mut last_time = tokio::time::Instant::now();
        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            let current =
                clamp_total(progress_chunks.iter().map(|c| c.load(Ordering::Relaxed)).sum(), total_size);
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
            if current >= total_size || progress_cancelled.load(Ordering::Relaxed) {
                break;
            }
        }
    });

    for i in 0..num_chunks {
        let start = i * chunk_size;
        let end = std::cmp::min(start + chunk_size - 1, total_size - 1);
        let chunk_len = end - start + 1;
        let client_c = client.clone();
        let url_c = url.to_string();
        let sem_c = semaphore.clone();
        let file_c = shared_file.clone();
        let progress_c = chunk_progress.clone();
        let cancelled_c = cancelled.clone();
        let index = i as usize;

        let task = tokio::spawn(async move {
            let _permit = sem_c
                .acquire()
                .await
                .map_err(|e| format!("下载信号量已关闭: {e}"))?;
            let mut retries = 3;
            loop {
                if cancelled_c.load(Ordering::Relaxed) {
                    return Err("下载已取消".to_string());
                }
                let req = client_c
                    .get(&url_c)
                    .header(RANGE, format!("bytes={}-{}", start, end));
                if let Ok(res) = req.send().await {
                    if res.status().is_success() {
                        let mut stream = res.bytes_stream();
                        let mut current_offset = start;
                        let mut chunk_ok = true;
                        while let Some(chunk_res) = stream.next().await {
                            if cancelled_c.load(Ordering::Relaxed) {
                                return Err("下载已取消".to_string());
                            }
                            match chunk_res {
                                Ok(data) => {
                                    let mut f = file_c.lock().await;
                                    // 磁盘满 / 句柄失效时这里是实打实的 IO 错误，不能 unwrap panic：
                                    // panic 只会让这个任务变成 JoinError，而旧的写法会在这之前
                                    // 直接 return，把其余区块和进度上报任务全部留在后台。
                                    if let Err(e) = f.seek(SeekFrom::Start(current_offset)).await {
                                        return Err(format!("定位写入偏移失败: {e}"));
                                    }
                                    if let Err(e) = f.write_all(&data).await {
                                        return Err(format!("写入文件失败: {e}"));
                                    }
                                    drop(f);
                                    current_offset += data.len() as u64;
                                    let done = clamp_up(current_offset - start, chunk_len);
                                    progress_c[index].store(done, Ordering::Relaxed);
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
                    cancelled_c.store(true, Ordering::Relaxed);
                    return Err(format!("无法下载区块: {}-{}", start, end));
                }
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
            Ok::<(), String>(())
        });
        tasks.push(task);
    }

    // 收敛顺序很关键：先等**所有**区块任务结束，再取消进度上报任务。
    // 原来的 `task.await??` 在第一个错误处就直接 return 了：既没有 await 剩余的区块任务
    // （JoinHandle 被 drop 只是 detach，任务仍在跑），也没有 abort 上报任务，
    // 于是下载失败后后台会残留最多 8 个继续占带宽/写盘的任务，
    // 外加一个每 500ms 无限 emit `install_progress` 的循环（current 永远等不到 total_size）。
    let mut first_error: Option<String> = None;
    for task in tasks {
        let failed = match task.await {
            Err(e) => {
                if first_error.is_none() {
                    first_error = Some(format!("线程崩溃: {e}"));
                }
                true
            }
            Ok(Err(e)) => {
                // 「下载已取消」是协作退出的信号，不是根因：真正的原因在失败的那条区块里。
                // 若把它当成 first_error，用户看到的就是无意义的「下载已取消」。
                if first_error.is_none() && e != "下载已取消" {
                    first_error = Some(e);
                }
                true
            }
            Ok(Ok(_)) => false,
        };
        if failed {
            cancelled.store(true, Ordering::Relaxed);
        }
    }
    cancelled.store(true, Ordering::Relaxed);
    reporter_task.abort();
    let _ = reporter_task.await;

    if let Some(e) = first_error {
        return Err(e.into());
    }

    {
        let mut f = shared_file.lock().await;
        f.flush().await.map_err(|e| format!("刷盘失败: {}", e))?;
    }

    let actual_downloaded = clamp_total(
        chunk_progress.iter().map(|c| c.load(Ordering::Relaxed)).sum(),
        total_size,
    );
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
        let mut extractor = Command::new("powershell");
        extractor.args(["-Command", &ps_cmd]);
        let extract_ok = match wait_for_child(extractor).await {
            Ok(status) => status.success(),
            Err(e) => {
                if auto_delete {
                    let _ = fs::remove_file(&dest_path);
                }
                return Err(e);
            }
        };

        if auto_delete {
            let _ = fs::remove_file(&dest_path);
        }

        if extract_ok {
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

        match wait_for_child(cmd).await {
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
            Err(e) => return Err(e),
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

    match wait_for_child(cmd).await {
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
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod download_progress_tests {
    use super::{clamp_total, clamp_up};

    #[test]
    fn total_progress_never_exceeds_file_size() {
        // 区块重试会重复累加，求和可能超过 total；进度必须被钳制。
        assert_eq!(clamp_total(1_500, 1_000), 1_000);
        assert_eq!(clamp_total(600, 1_000), 600);
        // total 为 0（服务端没给 Content-Length）时不产生除零式假进度。
        assert_eq!(clamp_total(123, 0), 0);
    }

    #[test]
    fn per_chunk_progress_is_capped_by_chunk_length() {
        assert_eq!(clamp_up(10, 4), 4);
        assert_eq!(clamp_up(3, 4), 3);
    }

    #[test]
    fn sum_of_chunks_reflects_real_downloaded_bytes() {
        use std::sync::atomic::{AtomicU64, Ordering};
        let chunks: Vec<AtomicU64> = (0..3).map(|_| AtomicU64::new(0)).collect();
        chunks[0].store(clamp_up(20, 20), Ordering::Relaxed);
        chunks[1].store(clamp_up(20, 20), Ordering::Relaxed);
        chunks[2].store(clamp_up(5, 10), Ordering::Relaxed);
        let sum: u64 = chunks.iter().map(|c| c.load(Ordering::Relaxed)).sum();
        assert_eq!(clamp_total(sum, 50), 45);
    }
}
