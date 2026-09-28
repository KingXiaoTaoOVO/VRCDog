//! Application update flow.
//!
//! Replaces Tauri 2's `tauri-plugin-updater` for this app because the
//! `vrcdog-releases` repo does not currently publish a signed
//! `updater.json`, which makes the official plugin's `check()` fail
//! with HTTP 404 on every cold start.
//!
//! Auto-update pipeline:
//!
//!   1. Query GitHub's REST API directly (`/repos/{owner}/{repo}/releases`),
//!      honoring user-configured proxy if available.
//!   2. Multi-channel accelerated streaming:
//!      - When no proxy is configured (typical domestic environment), automatically
//!        prioritizes high-speed CDN mirrors (`ghfast.top`, `gh-proxy.com`, `ghproxy.net`)
//!        with direct GitHub fallback.
//!      - When a proxy is configured, prioritizes direct download through the user proxy
//!        with CDN mirrors as high-availability fallbacks.
//!      - Streams to `%TEMP%\vrcdog-setup-<stamp>.exe` using a 512 KB `BufWriter`.
//!      - Strict SHA-256 cryptographic verification against the GitHub release digest.
//!      - File handles are explicitly flushed and closed (`drop`) immediately upon
//!        completion to prevent Windows file-locking / `ERROR_SHARING_VIOLATION`.
//!   3. Zero-window silent installation:
//!      - Generates an invisible VBScript bootstrapper executed via `wscript.exe`
//!        (`IMAGE_SUBSYSTEM_WINDOWS_GUI`) with `CREATE_NO_WINDOW | DETACHED_PROCESS`.
//!        This completely prevents ANY CMD console / Windows Terminal black boxes.
//!      - Polls process exit via WMI every 100 ms instead of heavy external `tasklist`
//!        and `timeout` loops.
//!      - Runs the NSIS installer silently (`/S /D=<install_dir>`) hidden (`SW_HIDE`).
//!      - Sweeps `.exe.old` and `.exe.bak` leftovers.
//!      - Launches the new `VRCDog.exe` in the foreground (`SW_SHOWNORMAL`).
//!      - Self-deletes temporary installer and bootstrap script.
//!      - Backed by an invisible PowerShell fallback if `wscript.exe` is disabled.

use std::path::{Path, PathBuf};
use std::time::Duration;

use futures_util::StreamExt;
use serde::Serialize;
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter};
use tokio::io::AsyncWriteExt;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

const GITHUB_RELEASES_API: &str =
    "https://api.github.com/repos/KingXiaoTaoOVO/vrcdog-releases/releases";

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;
#[cfg(windows)]
const DETACHED_PROCESS: u32 = 0x00000008;

/// One release row exposed to the frontend.
#[derive(Debug, Clone, Serialize)]
pub struct ReleaseInfo {
    pub tag: String,
    pub name: String,
    pub prerelease: bool,
    pub draft: bool,
    pub published_at: String,
    pub body: String,
    pub html_url: String,
    /// Stable, Windows-friendly installer (.exe / .msi) download URL.
    pub installer_url: Option<String>,
    /// SHA-256 of the installer, in plain hex. Sourced from the asset
    /// object's `digest` field (`sha256:abcdef...`).
    pub installer_sha256: Option<String>,
    /// Asset size in bytes; used both to show progress and to verify the
    /// download wasn't truncated.
    pub installer_size: Option<u64>,
    /// Parsed semver-friendly version (strips a leading `v`).
    pub version: String,
    /// GitHub-assigned upload timestamp for sorting newest-first.
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct InstallProgress {
    pub stage: &'static str,
    pub bytes_done: u64,
    pub bytes_total: u64,
    pub message: String,
}

/// Parses a semver-flavoured version string into its numeric parts and
/// an optional pre-release tag. `5.0.5-beta.1` parses as
/// `Some(([5,0,5], "beta.1"))`; `5.0.5` parses as `Some(([5,0,5], ""))`.
fn parse_version(value: &str) -> Option<(Vec<u64>, &str)> {
    let stripped = value.trim_start_matches('v');
    let (core, pre) = match stripped.split_once('-') {
        Some((head, tail)) => (head, tail),
        None => (stripped, ""),
    };
    let nums: Vec<u64> = core
        .split('.')
        .filter(|segment| !segment.is_empty())
        .map(|segment| segment.parse::<u64>().ok())
        .collect::<Option<Vec<_>>>()?;
    Some((nums, pre))
}

/// Compare two "vX.Y.Z..." version strings, ignoring a leading `v`.
fn cmp_versions(a: &str, b: &str) -> std::cmp::Ordering {
    let Some((an, ap)) = parse_version(a) else {
        return a.cmp(b);
    };
    let Some((bn, bp)) = parse_version(b) else {
        return a.cmp(b);
    };
    let max = an.len().max(bn.len());
    for i in 0..max {
        let lhs = *an.get(i).unwrap_or(&0);
        let rhs = *bn.get(i).unwrap_or(&0);
        match lhs.cmp(&rhs) {
            std::cmp::Ordering::Equal => continue,
            ord => return ord,
        }
    }
    match (ap.is_empty(), bp.is_empty()) {
        (true, true) => std::cmp::Ordering::Equal,
        (true, false) => std::cmp::Ordering::Greater,
        (false, true) => std::cmp::Ordering::Less,
        (false, false) => ap.cmp(bp),
    }
}

pub fn is_newer(remote: &str, current: &str) -> bool {
    cmp_versions(remote, current) == std::cmp::Ordering::Greater
}

fn parse_release(json: &serde_json::Value) -> Option<ReleaseInfo> {
    let tag = json.get("tag_name")?.as_str()?.to_string();
    let assets = json.get("assets")?.as_array()?;
    let asset = assets.iter().find_map(|a| {
        let name = a.get("name")?.as_str()?;
        // Accept the .exe NSIS setup or .msi Windows installer.
        if name.ends_with(".exe") && name.to_ascii_lowercase().contains("setup") {
            return Some(a);
        }
        if name.ends_with(".msi") {
            return Some(a);
        }
        if name.ends_with(".exe") {
            return Some(a);
        }
        None
    });

    let (installer_url, installer_sha256, installer_size) = match asset {
        Some(a) => {
            let url = a
                .get("browser_download_url")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let digest = a.get("digest").and_then(|v| v.as_str()).unwrap_or("");
            let sha = digest.strip_prefix("sha256:").map(|s| s.to_string());
            let size = a.get("size").and_then(|v| v.as_u64());
            (url, sha, size)
        }
        None => (None, None, None),
    };

    let version = tag.trim_start_matches('v').to_string();
    let prerelease = json
        .get("prerelease")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let draft = json.get("draft").and_then(|v| v.as_bool()).unwrap_or(false);
    let published_at = json
        .get("published_at")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let created_at = json
        .get("created_at")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let body = json
        .get("body")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let name = json
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or(&tag)
        .to_string();
    let html_url = json
        .get("html_url")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    Some(ReleaseInfo {
        tag,
        name,
        prerelease,
        draft,
        published_at,
        body,
        html_url,
        installer_url,
        installer_sha256,
        installer_size,
        version,
        created_at,
    })
}

fn build_update_client(proxy_url: Option<&str>) -> Result<reqwest::Client, String> {
    let mut builder = reqwest::Client::builder()
        .user_agent("VrcDog-Updater/1.0")
        .timeout(Duration::from_secs(60 * 30));

    if let Some(proxy_str) = proxy_url.filter(|s| !s.trim().is_empty()) {
        if let Ok(proxy) = reqwest::Proxy::all(proxy_str) {
            builder = builder.proxy(proxy);
        }
    }

    builder
        .build()
        .map_err(|e| format!("HTTP client init failed: {e}"))
}

/// Hit the GitHub Releases API and return parsed, sorted (newest first)
/// release info, skipping drafts and releases with no Windows installer.
#[tauri::command]
pub async fn update_remote_releases(
    vrc_state: tauri::State<'_, crate::vrc_api::VrcState>,
) -> Result<Vec<ReleaseInfo>, String> {
    let proxy_url = vrc_state.proxy_url.read().await.clone();
    let mut builder = reqwest::Client::builder()
        .user_agent("VrcDog-Updater/1.0")
        .timeout(Duration::from_secs(20));

    if let Some(proxy_str) = proxy_url.as_ref().filter(|s| !s.trim().is_empty()) {
        if let Ok(proxy) = reqwest::Proxy::all(proxy_str) {
            builder = builder.proxy(proxy);
        }
    }

    let client = builder
        .build()
        .map_err(|e| format!("HTTP client init failed: {e}"))?;

    let resp = client
        .get(GITHUB_RELEASES_API)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| format!("Failed to reach GitHub Releases API: {e}"))?;
    let status = resp.status();
    if !status.is_success() {
        return Err(format!(
            "GitHub Releases API returned {} ({}). Check your network or the vrcdog-releases repo status.",
            status.as_u16(),
            status.canonical_reason().unwrap_or("unknown"),
        ));
    }
    let json: Vec<serde_json::Value> = resp
        .json()
        .await
        .map_err(|e| format!("Malformed JSON from GitHub: {e}"))?;

    let mut releases: Vec<ReleaseInfo> = json.iter().filter_map(parse_release).collect();
    releases.retain(|r| {
        !r.draft
            && (r.installer_url.is_some() || !r.tag.trim_start_matches('v').is_empty())
    });
    releases.sort_by(|a, b| cmp_versions(&b.version, &a.version));
    Ok(releases)
}

fn emit_progress(app: &AppHandle, payload: InstallProgress) {
    let _ = app.emit("app-update://progress", payload);
}

fn emit_done(app: &AppHandle, message: &str) {
    let _ = app.emit("app-update://done", message.to_string());
}

/// Best-effort guess of where the upgraded binary lives after the
/// installer runs.
fn locate_installed_exe(app: &AppHandle) -> Option<PathBuf> {
    let local = dirs::data_local_dir()?;
    let base = local.join("Programs").join("VRCDog");
    let candidates = [
        base.join("VRCDog.exe"),
        base.join("VRCDog").join("VRCDog.exe"),
        local.join("Programs").join("vrcdog").join("VRCDog.exe"),
    ];
    for c in &candidates {
        if c.exists() {
            return Some(c.clone());
        }
    }
    if let Ok(read) = std::fs::read_dir(&base) {
        let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
        for entry in read.flatten() {
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
                continue;
            };
            if !name.to_ascii_lowercase().ends_with(".exe") || name.contains(".old") {
                continue;
            }
            if let Ok(meta) = entry.metadata() {
                if let Ok(modified) = meta.modified() {
                    match &best {
                        Some((t, _)) if *t >= modified => {}
                        _ => best = Some((modified, path.clone())),
                    }
                }
            }
        }
        if let Some((_, p)) = best {
            return Some(p);
        }
    }
    if let Ok(running) = std::env::current_exe() {
        if running.exists() {
            return Some(running);
        }
    }
    let _ = app;
    None
}

/// Generates a zero-window, pure GUI VBScript bootstrapper.
///
/// Executed by `wscript.exe` (`IMAGE_SUBSYSTEM_WINDOWS_GUI`), meaning Windows
/// will NEVER attach a console window (conhost/Windows Terminal).
///
/// Arguments received:
///   Arguments(0) - PID of the running VRCDog.exe
///   Arguments(1) - Path to the downloaded installer (.exe or .msi)
///   Arguments(2) - Install directory
///   Arguments(3) - Path to the freshly-installed VRCDog.exe to launch
fn render_silent_bootstrap_script() -> &'static str {
    r#"Option Explicit
On Error Resume Next

Dim targetPid, installer, installDir, newExe
Dim WshShell, fso, objWMIService, colProcesses, tries, ext, cmd, rc, i, fallbackExe

If WScript.Arguments.Count < 4 Then
    WScript.Quit 1
End If

targetPid = CLng(WScript.Arguments(0))
installer = WScript.Arguments(1)
installDir = WScript.Arguments(2)
newExe = WScript.Arguments(3)

Set WshShell = CreateObject("WScript.Shell")
Set fso = CreateObject("Scripting.FileSystemObject")

' 1. Fast wait for old process to exit (using WMI with 100ms intervals, 0 console windows)
If targetPid > 4 Then
    Set objWMIService = GetObject("winmgmts:\\.\root\cimv2")
    tries = 0
    Do While tries < 300
        Set colProcesses = objWMIService.ExecQuery("Select ProcessId from Win32_Process Where ProcessId = " & targetPid)
        If Err.Number <> 0 Or colProcesses.Count = 0 Then
            Exit Do
        End If
        WScript.Sleep 100
        tries = tries + 1
    Loop
    Err.Clear
End If

' 2. Run installer silently (0 = SW_HIDE, True = wait for exit)
rc = 0
If fso.FileExists(installer) Then
    ext = LCase(fso.GetExtensionName(installer))
    If ext = "msi" Then
        cmd = "msiexec.exe /qn /i """ & installer & """ TARGETDIR=""" & installDir & """"
    Else
        ' NSIS setup: /D= must be the last parameter and must NOT be quoted
        cmd = """" & installer & """ /S /D=" & installDir
    End If
    rc = WshShell.Run(cmd, 0, True)
End If

' 3. Clean up NSIS in-place upgrade leftovers (.exe.old, .exe.bak)
If fso.FolderExists(installDir) Then
    If fso.FileExists(installDir & "\VRCDog.exe.old") Then fso.DeleteFile installDir & "\VRCDog.exe.old", True
    If fso.FileExists(installDir & "\VRCDog.exe.bak") Then fso.DeleteFile installDir & "\VRCDog.exe.bak", True
End If

' 4. Launch new executable if installation succeeded (1 = SW_SHOWNORMAL, False = don't wait)
If rc = 0 Then
    If fso.FileExists(newExe) Then
        WshShell.Run """" & newExe & """", 1, False
    Else
        fallbackExe = installDir & "\VRCDog.exe"
        If fso.FileExists(fallbackExe) Then
            WshShell.Run """" & fallbackExe & """", 1, False
        End If
    End If
End If

' 5. Clean up installer file and self-delete
For i = 1 To 10
    If fso.FileExists(installer) Then
        fso.DeleteFile installer, True
        If Not fso.FileExists(installer) Then Exit For
        WScript.Sleep 200
    Else
        Exit For
    End If
Next

fso.DeleteFile WScript.ScriptFullName, True
"#
}

/// Write the bootstrapper `.vbs` file to `%TEMP%`. Returns the path.
fn write_bootstrap_script(pid: u32) -> Result<PathBuf, String> {
    let tmp_dir = std::env::temp_dir();
    std::fs::create_dir_all(&tmp_dir)
        .map_err(|e| format!("无法创建临时目录: {e}"))?;
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
    let script = tmp_dir.join(format!("vrcdog-update-{}-{}.vbs", pid, stamp));
    std::fs::write(&script, render_silent_bootstrap_script().as_bytes())
        .map_err(|e| format!("无法写入引导脚本: {e}"))?;
    Ok(script)
}

/// Spawns the silent bootstrapper detached and windowless.
///
/// Uses `wscript.exe` with `CREATE_NO_WINDOW | DETACHED_PROCESS` so no console
/// window is ever created. Includes a fallback to PowerShell with `-WindowStyle Hidden`
/// if `wscript.exe` is blocked or unavailable.
fn spawn_silent_bootstrapper(
    script: &Path,
    pid: u32,
    installer: &str,
    install_dir: &str,
    new_exe: &str,
) -> Result<(), String> {
    use std::process::{Command, Stdio};

    // Primary: wscript.exe (native Windows GUI script host, 0 console windows)
    let mut cmd = Command::new("wscript.exe");
    cmd.arg("//nologo")
        .arg(script)
        .arg(pid.to_string())
        .arg(installer)
        .arg(install_dir)
        .arg(new_exe)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    #[cfg(windows)]
    {
        cmd.creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS);
    }

    match cmd.spawn() {
        Ok(_) => {
            eprintln!("[update] spawned silent bootstrapper via wscript.exe");
            Ok(())
        }
        Err(e) => {
            eprintln!("[update] wscript.exe spawn failed: {e}, falling back to powershell -WindowStyle Hidden");
            // Secondary fallback: powershell.exe with -WindowStyle Hidden and CREATE_NO_WINDOW | DETACHED_PROCESS
            let mut ps = Command::new("powershell.exe");
            let ps_script = format!(
                "$targetPid = {}; $installer = '{}'; $installDir = '{}'; $newExe = '{}'; \
                 $tries = 0; \
                 while ($tries -lt 300) {{ \
                     if (-not (Get-Process -Id $targetPid -ErrorAction SilentlyContinue)) {{ break }}; \
                     Start-Sleep -Milliseconds 100; \
                     $tries++; \
                 }}; \
                 if (Test-Path $installer) {{ \
                     if ($installer.ToLower().EndsWith('.msi')) {{ \
                         Start-Process -FilePath 'msiexec.exe' -ArgumentList \"/qn /i `\"$installer`\" TARGETDIR=`\"$installDir`\"\" -Wait; \
                     }} else {{ \
                         Start-Process -FilePath $installer -ArgumentList \"/S /D=$installDir\" -Wait; \
                     }} \
                 }}; \
                 Remove-Item \"$installDir\\VRCDog.exe.old\" -Force -ErrorAction SilentlyContinue; \
                 Remove-Item \"$installDir\\VRCDog.exe.bak\" -Force -ErrorAction SilentlyContinue; \
                 if (Test-Path $newExe) {{ Start-Process -FilePath $newExe }} \
                 elseif (Test-Path \"$installDir\\VRCDog.exe\") {{ Start-Process -FilePath \"$installDir\\VRCDog.exe\" }}; \
                 Remove-Item $installer -Force -ErrorAction SilentlyContinue; \
                 Remove-Item '{}' -Force -ErrorAction SilentlyContinue;",
                pid,
                installer.replace('\'', "''"),
                install_dir.replace('\'', "''"),
                new_exe.replace('\'', "''"),
                script.to_string_lossy().replace('\'', "''")
            );

            ps.arg("-WindowStyle")
                .arg("Hidden")
                .arg("-NoProfile")
                .arg("-NonInteractive")
                .arg("-ExecutionPolicy")
                .arg("Bypass")
                .arg("-Command")
                .arg(ps_script)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null());

            #[cfg(windows)]
            {
                ps.creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS);
            }

            ps.spawn().map(|_| ()).map_err(|e2| {
                format!("无法启动更新引导程序 (wscript 失败: {e}, powershell 失败: {e2})")
            })
        }
    }
}

/// Best-effort sweep of stale updater artifacts from previous runs.
#[tauri::command]
pub fn update_cleanup_stale_artifacts() -> Result<u32, String> {
    let tmp_dir = std::env::temp_dir();
    let cutoff = std::time::SystemTime::now() - Duration::from_secs(60 * 60 * 24);
    let mut removed = 0u32;
    let Ok(read) = std::fs::read_dir(&tmp_dir) else {
        return Ok(0);
    };
    for entry in read.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        let lower = name.to_ascii_lowercase();
        let is_stale_setup = (lower.starts_with("vrcdog-setup-")
            || lower.starts_with("vrcdog-installer-"))
            && (lower.ends_with(".exe") || lower.ends_with(".msi"));
        let is_stale_bootstrap = lower.starts_with("vrcdog-update-")
            && (lower.ends_with(".vbs") || lower.ends_with(".cmd") || lower.ends_with(".ps1"));
        if !(is_stale_setup || is_stale_bootstrap) {
            continue;
        }
        let Ok(meta) = entry.metadata() else { continue };
        let Ok(modified) = meta.modified() else { continue };
        if modified > cutoff {
            continue;
        }
        if std::fs::remove_file(&path).is_ok() {
            removed += 1;
            eprintln!("[update] swept stale artifact {}", path.display());
        }
    }
    Ok(removed)
}

/// Builds candidate URLs for download with mirrors and direct links.
pub fn build_candidate_download_urls(download_url: &str, proxy_url: Option<&str>) -> Vec<String> {
    let mut candidates = Vec::new();
    let has_proxy = proxy_url.map(|s| !s.trim().is_empty()).unwrap_or(false);

    if has_proxy {
        candidates.push(download_url.to_string());
        candidates.push(format!("https://ghfast.top/{download_url}"));
        candidates.push(format!("https://gh-proxy.com/{download_url}"));
        candidates.push(format!("https://ghproxy.net/{download_url}"));
    } else {
        candidates.push(format!("https://ghfast.top/{download_url}"));
        candidates.push(format!("https://gh-proxy.com/{download_url}"));
        candidates.push(format!("https://ghproxy.net/{download_url}"));
        candidates.push(download_url.to_string());
    }

    candidates
}

async fn try_download_from_url(
    client: &reqwest::Client,
    url: &str,
    installer: &Path,
    expected_size: Option<u64>,
    app: &AppHandle,
    last_emit: &mut u64,
) -> Result<(u64, String), String> {
    let resp = client
        .get(url)
        .header("Accept", "application/octet-stream")
        .send()
        .await
        .map_err(|e| format!("请求失败: {e}"))?;

    let status = resp.status();
    if !status.is_success() {
        return Err(format!(
            "服务器返回 {}: {}",
            status.as_u16(),
            status.canonical_reason().unwrap_or("error")
        ));
    }

    let total = resp.content_length().unwrap_or_else(|| expected_size.unwrap_or(0));
    let mut stream = resp.bytes_stream();
    let file = tokio::fs::File::create(installer)
        .await
        .map_err(|e| format!("无法创建临时文件 {}: {e}", installer.display()))?;

    let mut writer = tokio::io::BufWriter::with_capacity(512 * 1024, file);
    let mut hasher = Sha256::new();
    let mut bytes_done: u64 = 0;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("下载中断: {e}"))?;
        writer
            .write_all(&chunk)
            .await
            .map_err(|e| format!("写入临时文件失败: {e}"))?;
        hasher.update(&chunk);
        bytes_done = bytes_done.saturating_add(chunk.len() as u64);

        if total > 0 && bytes_done - *last_emit > (total / 50).max(512 * 1024) {
            *last_emit = bytes_done;
            emit_progress(
                app,
                InstallProgress {
                    stage: "downloading",
                    bytes_done,
                    bytes_total: total,
                    message: format!(
                        "已下载 {:.1} MB / {:.1} MB",
                        bytes_done as f64 / 1_048_576.0,
                        total as f64 / 1_048_576.0
                    ),
                },
            );
        }
    }

    writer.flush().await.map_err(|e| format!("刷盘失败: {e}"))?;
    writer.shutdown().await.ok();
    // Drop writer and underlying file handle to avoid ERROR_SHARING_VIOLATION
    drop(writer);

    let computed = hasher.finalize();
    let computed_hex = format!("{:x}", computed);

    Ok((bytes_done, computed_hex))
}

/// Run the full auto-update flow against a single chosen release.
#[tauri::command]
pub async fn update_install_release(
    app: AppHandle,
    vrc_state: tauri::State<'_, crate::vrc_api::VrcState>,
    download_url: String,
    expected_sha256: Option<String>,
    expected_size: Option<u64>,
) -> Result<(), String> {
    if download_url.trim().is_empty() {
        return Err("下载链接为空".into());
    }

    // R6: 仅允许官方 GitHub Releases 域名，避免下载并执行任意 URL 导致安全隐患
    let parsed = reqwest::Url::parse(&download_url)
        .map_err(|_| "下载链接格式非法".to_string())?;
    match parsed.host_str() {
        Some(h)
            if h == "github.com"
                || h == "objects.githubusercontent.com"
                || h.ends_with(".github.com")
                || h.ends_with(".githubusercontent.com") => {}
        _ => return Err("非法的下载源：仅允许 GitHub Releases 官方域名".into()),
    }

    // 强制完整性校验：未提供 SHA-256 时拒绝安装
    if expected_sha256
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .is_none()
    {
        return Err("缺少 SHA-256 校验值，出于安全考虑拒绝安装".into());
    }

    emit_progress(
        &app,
        InstallProgress {
            stage: "downloading",
            bytes_done: 0,
            bytes_total: expected_size.unwrap_or(0),
            message: "正在连接高速更新网络...".into(),
        },
    );

    let proxy_url = vrc_state.proxy_url.read().await.clone();
    let candidates = build_candidate_download_urls(&download_url, proxy_url.as_deref());

    let tmp_dir = std::env::temp_dir();
    let _ = std::fs::create_dir_all(&tmp_dir);
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
    let stem = download_url
        .rsplit('/')
        .next()
        .unwrap_or("VRCDog-Setup.exe")
        .to_string();
    let ext = std::path::Path::new(&stem)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("exe");
    let installer_name = if stem.to_ascii_lowercase().contains("setup") {
        format!("vrcdog-setup-{stamp}.{ext}")
    } else {
        format!("vrcdog-installer-{stamp}.{ext}")
    };
    let installer = tmp_dir.join(&installer_name);

    let client_configured = build_update_client(proxy_url.as_deref())?;
    let client_direct = build_update_client(None)?;

    let mut download_success = false;
    let mut last_err = String::new();
    let mut last_emit: u64 = 0;

    for candidate_url in &candidates {
        eprintln!("[update] downloading candidate: {candidate_url}");

        // For mirrors, direct client avoids unnecessary proxy bottlenecks;
        // for official github.com, use configured proxy if available.
        let is_mirror = candidate_url.starts_with("https://gh");
        let active_client = if is_mirror && proxy_url.is_none() {
            &client_direct
        } else {
            &client_configured
        };

        match try_download_from_url(
            active_client,
            candidate_url,
            &installer,
            expected_size,
            &app,
            &mut last_emit,
        )
        .await
        {
            Ok((bytes_done, computed_hex)) => {
                // Verify SHA-256
                if let Some(expected) = expected_sha256.as_ref() {
                    if !expected.is_empty() && expected.to_ascii_lowercase() != computed_hex {
                        eprintln!("[update] SHA-256 mismatch for {candidate_url}: expected {expected}, got {computed_hex}");
                        let _ = std::fs::remove_file(&installer);
                        last_err = format!("SHA-256 校验不匹配 (来源: {candidate_url})");
                        continue;
                    }
                }
                // Verify size
                if let Some(exp_size) = expected_size {
                    if bytes_done != exp_size {
                        eprintln!("[update] size mismatch for {candidate_url}: expected {exp_size}, got {bytes_done}");
                        let _ = std::fs::remove_file(&installer);
                        last_err = format!("文件大小不匹配 (来源: {candidate_url})");
                        continue;
                    }
                }
                download_success = true;
                break;
            }
            Err(e) => {
                eprintln!("[update] candidate {candidate_url} failed: {e}");
                let _ = std::fs::remove_file(&installer);
                last_err = e;
            }
        }
    }

    if !download_success {
        return Err(format!(
            "所有线路下载均失败，无法完成更新。最后错误: {last_err}"
        ));
    }

    let total = expected_size.unwrap_or(0);
    emit_progress(
        &app,
        InstallProgress {
            stage: "installing",
            bytes_done: total,
            bytes_total: total,
            message: "下载完成，正在进行无感静默更新...".into(),
        },
    );

    let run_dir_hint = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| {
            dirs::data_local_dir()
                .map(|d| d.join("Programs").join("VRCDog"))
                .unwrap_or_else(|| PathBuf::from("."))
        });
    let new_exe = locate_installed_exe(&app).unwrap_or_else(|| run_dir_hint.join("VRCDog.exe"));
    let pid = std::process::id();
    let script_path = write_bootstrap_script(pid)?;
    let installer_str = installer.to_string_lossy().to_string();
    let install_dir_str = run_dir_hint.to_string_lossy().to_string();
    let new_exe_str = new_exe.to_string_lossy().to_string();

    eprintln!(
        "[update] spawning silent bootstrap script {} for installer {} (install dir {}, new exe {})",
        script_path.display(),
        installer.display(),
        run_dir_hint.display(),
        new_exe.display()
    );

    spawn_silent_bootstrapper(
        &script_path,
        pid,
        &installer_str,
        &install_dir_str,
        &new_exe_str,
    )
    .map_err(|e| {
        let _ = std::fs::remove_file(&script_path);
        let _ = std::fs::remove_file(&installer);
        format!(
            "无法启动更新引导程序: {e}。安装包已下载到: {}",
            installer.display()
        )
    })?;

    emit_progress(
        &app,
        InstallProgress {
            stage: "launching",
            bytes_done: total,
            bytes_total: total,
            message: "更新完成，正在拉起新版本并退出旧版本...".into(),
        },
    );
    emit_done(&app, "新版本已启动");

    std::thread::sleep(Duration::from_millis(400));
    app.exit(0);
    Ok(())
}

/// Restart helper.
#[tauri::command]
pub fn update_restart(app: AppHandle) -> Result<(), String> {
    app.restart();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semver_compare_orders_basic() {
        assert_eq!(cmp_versions("5.0.5", "5.0.4"), std::cmp::Ordering::Greater);
        assert_eq!(cmp_versions("5.0.5", "5.0.5"), std::cmp::Ordering::Equal);
        assert_eq!(cmp_versions("5.0.5", "5.1.0"), std::cmp::Ordering::Less);
        assert_eq!(cmp_versions("v5.0.5", "5.0.4"), std::cmp::Ordering::Greater);
    }

    #[test]
    fn semver_compare_handles_prerelease() {
        assert_eq!(
            cmp_versions("5.0.5-beta.1", "5.0.5"),
            std::cmp::Ordering::Less
        );
        assert_eq!(
            cmp_versions("5.1.0-rc.1", "5.0.5"),
            std::cmp::Ordering::Greater
        );
    }

    #[test]
    fn is_newer_detects_promotion() {
        assert!(is_newer("5.0.6", "5.0.5"));
        assert!(!is_newer("5.0.5", "5.0.5"));
        assert!(!is_newer("5.0.4", "5.0.5"));
        assert!(is_newer("v6.0.0", "v5.0.5"));
    }

    #[test]
    fn silent_bootstrap_script_contains_required_anchors() {
        let body = render_silent_bootstrap_script();
        assert!(body.contains("WScript.Arguments"));
        assert!(body.contains("Win32_Process"));
        assert!(body.contains("/S /D="));
        assert!(body.contains(".exe.old"));
        assert!(body.contains(".exe.bak"));
        assert!(body.contains("WshShell.Run"));
        assert!(body.contains("fso.DeleteFile WScript.ScriptFullName, True"));
    }

    #[test]
    fn mirror_candidate_generation() {
        let url = "https://github.com/KingXiaoTaoOVO/vrcdog-releases/releases/download/v5.6.9/VRCDog_5.6.9_x64-setup.exe";
        let no_proxy = build_candidate_download_urls(url, None);
        assert_eq!(no_proxy.len(), 4);
        assert!(no_proxy[0].starts_with("https://ghfast.top/"));
        assert_eq!(no_proxy[3], url);

        let with_proxy = build_candidate_download_urls(url, Some("http://127.0.0.1:7890"));
        assert_eq!(with_proxy.len(), 4);
        assert_eq!(with_proxy[0], url);
        assert!(with_proxy[1].starts_with("https://ghfast.top/"));
    }

    #[test]
    #[cfg(windows)]
    fn silent_bootstrap_script_runs_cleanly() {
        let tmp = std::env::temp_dir().join(format!(
            "vrcdog-bootstrap-test-{}.vbs",
            std::process::id()
        ));
        std::fs::write(&tmp, render_silent_bootstrap_script().as_bytes()).unwrap();

        let fake_installer = std::env::temp_dir().join("vrcdog-fake-installer-does-not-exist.exe");
        let fake_install_dir = std::env::temp_dir().join("vrcdog-fake-install");
        let fake_new_exe = std::env::temp_dir().join("vrcdog-fake-install/VRCDog.exe");
        let output = std::process::Command::new("wscript.exe")
            .arg("//nologo")
            .arg(&tmp)
            .arg("0") // PID 0 <= 4 exits wait-loop immediately
            .arg(&fake_installer)
            .arg(&fake_install_dir)
            .arg(&fake_new_exe)
            .output()
            .expect("wscript.exe should run the silent bootstrap script");

        assert!(
            output.status.success(),
            "silent bootstrap script should return 0 exit code: {:?}",
            output
        );

        assert!(
            !tmp.exists(),
            "silent bootstrap script should have self-deleted: {}",
            tmp.display()
        );
    }
}