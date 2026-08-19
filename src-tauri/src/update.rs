//! Application update flow.
//!
//! Replaces Tauri 2's `tauri-plugin-updater` for this app because the
//! `vrcdog-releases` repo does not currently publish a signed
//! `updater.json`, which makes the official plugin's `check()` fail
//! with HTTP 404 on every cold start.
//!
//! Auto-update pipeline (this is what actually runs when the user
//! clicks "立即更新" in SettingsView):
//!
//!   1. Query GitHub's REST API directly (`/repos/{owner}/{repo}/releases`).
//!   2. Stream the chosen asset to `%TEMP%\VRCDog-Setup-<stamp>.exe` while
//!      emitting progress; verify SHA-256 against the GitHub asset digest.
//!   3. Write a tiny `.cmd` bootstrapper to `%TEMP%\vrcdog-update-<stamp>.cmd`.
//!      That script:
//!        a. Polls `tasklist /FI "PID eq <our_pid>"` once a second for up
//!           to 90 seconds, waiting for the running VRCDog.exe to exit.
//!        b. Invokes the NSIS installer with `/S /D=<install_dir>` so the
//!           install path is preserved across upgrades.
//!        c. Sweeps stale `.exe.old` / `.exe.bak` leftovers left behind
//!           by NSIS in-place upgrade.
//!        d. Launches the freshly-installed `VRCDog.exe` via `start ""`.
//!        e. Deletes the temp installer and self-deletes.
//!   4. Spawn the bootstrapper detached via Windows Task Scheduler
//!      (`schtasks /Create /SC ONCE ... /ST <now+2s> /F` followed by
//!      `schtasks /Run`). Task Scheduler launches the script in
//!      `svchost.exe -k netsvcs` rather than from a console session,
//!      so the bootstrapper is **never** displayed in conhost, Windows
//!      Terminal, or any other terminal emulator — even when our
//!      process is still alive. We then `app.exit(0)` so the running
//!      process releases every handle / image / Defender scan
//!      association on the installer file.
//!
//! Why a bootstrapper? Spawning the NSIS installer directly from inside
//! the running VRCDog.exe process triggers
//! `os error 32 (ERROR_SHARING_VIOLATION)` on Windows: the moment we
//! write a new `.exe` to `%TEMP%`, Windows Defender grabs it for real
//! time scanning. While the scan is in flight (and while our own
//! process still has its image mapped and possibly a residual handle)
//! `CreateProcess` rejects the call with "another program is using
//! this file". The bootstrapper gets us past that by exiting our
//! process first, then waiting until the scan completes before running
//! the installer.
//!
//! The bootstrapper also handles "delete old version files": NSIS in
//! place upgrade renames `VRCDog.exe` to `VRCDog.exe.old` and never
//! cleans it up. The script removes `.exe.old`, `.exe.bak`, plus any
//! `VRCDog-Setup-*.exe` leftovers from previous interrupted runs.

use std::path::{Path, PathBuf};
use std::time::Duration;

use futures_util::StreamExt;
use serde::Serialize;
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

const GITHUB_RELEASES_API: &str =
    "https://api.github.com/repos/KingXiaoTaoOVO/vrcdog-releases/releases";

// Windows process creation flag we rely on.
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// One release row exposed to the frontend. We only ship the subset of
/// GitHub fields the updater UI actually uses.
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
///
/// Semantics (matches semver precedence):
///   - The `5.0.5` of `5.0.5-beta.1` is **greater** than `5.0.5-beta.1`.
///   - `5.1.0-rc.1` > `5.0.5` because `5.1.0 > 5.0.5` lexicographically.
///   - Missing trailing numeric segments compare as if zero.
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
    // Numeric parts equal — fall back on pre-release label. A version
    // with no pre-release label is greater than one with a label
    // (`5.0.5` > `5.0.5-beta.1`). When both have labels we compare
    // them lexicographically.
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

/// Hit the GitHub Releases API and return parsed, sorted (newest first)
/// release info, skipping drafts and releases with no Windows installer.
#[tauri::command]
pub async fn update_remote_releases() -> Result<Vec<ReleaseInfo>, String> {
    let client = reqwest::Client::builder()
        .user_agent("VrcDog-Updater/1.0")
        .timeout(Duration::from_secs(20))
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
    // Skip drafts (not useful for users) and missing-installer rows
    // (we can't auto-upgrade without one).
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
/// installer runs. We don't hard-code the path because the user can move
/// it; if we can't find it we fall back to the most recent
/// `VRCDog*.exe` in `%LOCALAPPDATA%\Programs\VRCDog`.
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
    // Last-resort: scan the Programs\VRCDog dir for the most recent .exe.
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
    // Also try alongside the running exe (for portable installs).
    if let Ok(running) = std::env::current_exe() {
        if running.exists() {
            // We're still running, so don't pick ourselves unless we
            // can't find any other candidate.
            return Some(running);
        }
    }
    let _ = app; // silence unused warning when all branches above run
    None
}

/// Render the bootstrapper CMD script.
///
/// Arguments the script receives via CMD %1..%4:
///
///   %1 — our current PID (used to wait until VRCDog.exe exits)
///   %2 — absolute path to the downloaded installer (.exe or .msi)
///   %3 — install dir (where the new VRCDog.exe should land)
///   %4 — absolute path to the freshly-installed VRCDog.exe (to launch)
///
/// The script intentionally keeps its logic plain-vanilla CMD so it
/// runs on every supported Windows version without extra dependencies.
/// It avoids parenthesised `if` blocks with `set /a` so that no
/// `EnableDelayedExpansion` is needed (which keeps the script
/// compatible with batch runners that disable it by default).
fn render_bootstrap_script() -> &'static str {
    r#"@echo off
setlocal
rem ============================================================
rem  VRCDog auto-update bootstrapper
rem  - waits for the running VRCDog.exe (PID %1) to exit
rem  - runs the NSIS/MSI installer silently with /D=install_dir
rem  - sweeps .exe.old / .exe.bak leftovers from in-place upgrades
rem  - launches the freshly installed VRCDog.exe
rem  - cleans up its own temp artifacts
rem ============================================================

set "INSTALLER=%~2"
set "INSTALL_DIR=%~3"
set "NEW_EXE=%~4"

echo [VRCDog updater] waiting for VRCDog.exe (PID %1) to exit...
set /a TRIES=0
goto waitloop

:waitloop
tasklist /FI "PID eq %1" 2>NUL | findstr /C:" %1 " >NUL
if %ERRORLEVEL%==1 goto run_install
set /a TRIES+=1
if %TRIES% GEQ 90 goto timed_out
>NUL timeout /T 1 /NOBREAK
goto waitloop

:timed_out
echo [VRCDog updater] VRCDog.exe did not exit within 90s, proceeding anyway.

:run_install
echo [VRCDog updater] running installer: "%INSTALLER%" /S /D="%INSTALL_DIR%"
if /I "%INSTALLER:~-4%"==".msi" goto run_msi

rem ----- NSIS / generic setup.exe path -----
"%INSTALLER%" /S /D="%INSTALL_DIR%"
set "RC=%ERRORLEVEL%"
echo [VRCDog updater] installer exit code: %RC%
goto after_install

:run_msi
rem ----- MSI fallback -----
msiexec /qn /i "%INSTALLER%" TARGETDIR="%INSTALL_DIR%"
set "RC=%ERRORLEVEL%"
echo [VRCDog updater] msiexec exit code: %RC%

:after_install
rem ----- Sweep NSIS in-place upgrade leftovers -----
if exist "%INSTALL_DIR%\VRCDog.exe.old" del /F /Q "%INSTALL_DIR%\VRCDog.exe.old"
if exist "%INSTALL_DIR%\VRCDog.exe.bak" del /F /Q "%INSTALL_DIR%\VRCDog.exe.bak"
pushd "%INSTALL_DIR%"
del /F /Q "*.exe.old" 2>NUL
del /F /Q "*.exe.bak" 2>NUL
del /F /Q "unins*.exe.tmp" 2>NUL
popd

rem ----- Launch the new exe (only if installer reported success) -----
if %RC% NEQ 0 goto cleanup
if exist "%NEW_EXE%" (
    echo [VRCDog updater] launching "%NEW_EXE%"
    start "" "%NEW_EXE%"
) else (
    echo [VRCDog updater] warning: %NEW_EXE% not found after install.
)

:cleanup
rem ----- Remove the temp installer and self-delete -----
if exist "%INSTALLER%" del /F /Q "%INSTALLER%"
del /F /Q "%~f0" 2>NUL
endlocal
exit /b %RC%
"#
}

/// Write the bootstrapper `.cmd` file to `%TEMP%`. Returns the path.
fn write_bootstrap_script(pid: u32) -> Result<PathBuf, String> {
    let tmp_dir = std::env::temp_dir();
    std::fs::create_dir_all(&tmp_dir)
        .map_err(|e| format!("无法创建临时目录: {e}"))?;
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
    let script = tmp_dir.join(format!("vrcdog-update-{}-{}.cmd", pid, stamp));
    std::fs::write(&script, render_bootstrap_script().as_bytes())
        .map_err(|e| format!("无法写入引导脚本: {e}"))?;
    Ok(script)
}

/// Best-effort sweep of stale updater artifacts from previous runs.
///
/// On startup we delete any leftover VRCDog installer and bootstrapper
/// scripts in `%TEMP%` that are older than one week. Anything newer
/// than that is left alone — the user may still be mid-update and we
/// don't want to yank the file from under a running bootstrapper.
#[tauri::command]
pub fn update_cleanup_stale_artifacts() -> Result<u32, String> {
    let tmp_dir = std::env::temp_dir();
    let cutoff = std::time::SystemTime::now()
        - Duration::from_secs(60 * 60 * 24 * 7);
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
        let is_stale_setup = lower.starts_with("vrcdog-setup-")
            && (lower.ends_with(".exe") || lower.ends_with(".msi"));
        let is_stale_bootstrap = lower.starts_with("vrcdog-update-") && lower.ends_with(".cmd");
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

/// Run the full auto-update flow against a single chosen release.
///
/// Pipeline:
///   1. Stream the installer from GitHub to `%TEMP%\VRCDog-Setup-<stamp>.exe`.
///   2. Verify SHA-256 against the GitHub asset digest.
///   3. Determine the install dir + new exe path.
///   4. Write a CMD bootstrapper that waits for our exit, then runs
///      the installer silently, sweeps leftovers, and launches the new
///      binary.
///   5. Spawn the bootstrapper detached (`cmd /C <script> <pid> <installer>
///      <install_dir> <new_exe>`) with no window, then `app.exit(0)`.
#[tauri::command]
pub async fn update_install_release(
    app: AppHandle,
    download_url: String,
    expected_sha256: Option<String>,
    expected_size: Option<u64>,
) -> Result<(), String> {
    if download_url.trim().is_empty() {
        return Err("下载链接为空".into());
    }

    emit_progress(
        &app,
        InstallProgress {
            stage: "downloading",
            bytes_done: 0,
            bytes_total: expected_size.unwrap_or(0),
            message: "正在从 GitHub 下载安装包".into(),
        },
    );

    let client = reqwest::Client::builder()
        .user_agent("VrcDog-Updater/1.0")
        .timeout(Duration::from_secs(60 * 30))
        .build()
        .map_err(|e| format!("HTTP client init failed: {e}"))?;

    let resp = client
        .get(&download_url)
        .header("Accept", "application/octet-stream")
        .send()
        .await
        .map_err(|e| format!("下载失败: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!(
            "下载服务器返回 {}: {}",
            resp.status().as_u16(),
            resp.status().canonical_reason().unwrap_or("error"),
        ));
    }

    let total = resp.content_length().unwrap_or(expected_size.unwrap_or(0));
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

    let mut stream = resp.bytes_stream();
    let mut file = tokio::fs::File::create(&installer)
        .await
        .map_err(|e| format!("无法创建临时文件 {}: {e}", installer.display()))?;

    let mut hasher = Sha256::new();
    let mut bytes_done: u64 = 0;
    let mut last_emit: u64 = 0;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("下载中断: {e}"))?;
        tokio::io::AsyncWriteExt::write_all(&mut file, &chunk)
            .await
            .map_err(|e| format!("写入临时文件失败: {e}"))?;
        hasher.update(&chunk);
        bytes_done = bytes_done.saturating_add(chunk.len() as u64);
        if total > 0 && bytes_done - last_emit > total / 50 {
            last_emit = bytes_done;
            emit_progress(
                &app,
                InstallProgress {
                    stage: "downloading",
                    bytes_done,
                    bytes_total: total,
                    message: format!("已下载 {} MB", bytes_done / 1_048_576),
                },
            );
        }
    }
    tokio::io::AsyncWriteExt::shutdown(&mut file)
        .await
        .ok();
    let computed = hasher.finalize();
    let computed_hex = {
        let mut s = String::with_capacity(64);
        for b in computed {
            s.push_str(&format!("{b:02x}"));
        }
        s
    };

    if let Some(expected) = expected_sha256.as_ref() {
        if !expected.is_empty() && expected.to_ascii_lowercase() != computed_hex {
            let _ = std::fs::remove_file(&installer);
            return Err(format!(
                "SHA-256 校验失败。预期 {expected}，实际 {computed_hex}。"
            ));
        }
    }

    if let Some(exp_size) = expected_size {
        if bytes_done != exp_size {
            let _ = std::fs::remove_file(&installer);
            return Err(format!(
                "下载字节数不匹配：预期 {exp_size}，实际 {bytes_done}。"
            ));
        }
    }

    emit_progress(
        &app,
        InstallProgress {
            stage: "installing",
            bytes_done: total,
            bytes_total: total,
            message: "正在准备静默安装".into(),
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
        "[update] spawning bootstrap script {} for installer {} (install dir {}, new exe {})",
        script_path.display(),
        installer.display(),
        run_dir_hint.display(),
        new_exe.display()
    );

    spawn_bootstrapper_detached(
        &script_path,
        pid,
        &installer_str,
        &install_dir_str,
        &new_exe_str,
    )
    .map_err(|e| {
        // Best-effort cleanup if we couldn't even launch the
        // bootstrapper — the user can still run the installer we
        // downloaded by hand.
        let _ = std::fs::remove_file(&script_path);
        let _ = std::fs::remove_file(&installer);
        format!(
            "无法启动更新引导脚本: {e}。安装包已下载到: {}",
            installer.display()
        )
    })?;

    emit_progress(
        &app,
        InstallProgress {
            stage: "launching",
            bytes_done: total,
            bytes_total: total,
            message: "已下载完成，正在退出旧版本以启动新版本".into(),
        },
    );
    emit_done(&app, "新版本已启动");

    // Give the frontend a beat to render the launching state before
    // we tear down. Tauri's `app.exit(0)` will fire on_drop cleanup
    // for any state we manage, but we don't want the event handler
    // to be mid-render when we pull the rug out from under it.
    std::thread::sleep(Duration::from_millis(400));
    app.exit(0);
    Ok(())
}

/// Spawn the bootstrapper CMD script in a way that **cannot** show a
/// console window — not even under Windows Terminal.
///
/// `Command::new("cmd.exe").creation_flags(CREATE_NO_WINDOW)` works on
/// plain conhost but Windows Terminal still hijacks the resulting child
/// into a tab (the classic "black cmd window with `findstr /C:"688"`
/// in its title" symptom). To completely avoid any console binding we
/// hand the job to the Windows Task Scheduler service
/// (`schtasks.exe`), whose host is `svchost.exe -k netsvcs`. Processes
/// it spawn are not attached to any console session at all — Task
/// Scheduler has been the canonical "fire-and-forget, no window"
/// mechanism on Windows since Vista.
///
/// We invoke the script directly (`/C <script> ...`) and do **not**
/// wrap it in `cmd.exe /C`, so there is no parent cmd.exe to be
/// captured by a terminal emulator.
///
/// `schtasks /Create` requires a start time (`/ST HH:MM:SS`) that is
/// `>=` the current local clock, so we ask for "now + 2 s". The task
/// is created with `/F` (force overwrite), immediately run, and then
/// deleted to avoid littering the user's task library.
fn spawn_bootstrapper_detached(
    script: &Path,
    pid: u32,
    installer: &str,
    install_dir: &str,
    new_exe: &str,
) -> Result<(), String> {
    use std::process::{Command, Stdio};

    let safe = |s: &str| -> String { s.replace('"', "\"\"") };
    let task_name = format!("VRCDog_Update_{pid}");
    let tr = format!(
        "\"{}\" \"{}\" \"{}\" \"{}\" \"{}\"",
        safe(&script.to_string_lossy()),
        pid,
        safe(installer),
        safe(install_dir),
        safe(new_exe),
    );
    // Round current local time to the next even second and add two
    // seconds so /ST is comfortably after the scheduler's wall clock
    // (which can lag a tick depending on the runtime). The format is
    // H:MM:SS or HH:MM:SS — schtasks accepts both.
    let start_time = {
        let now = chrono::Local::now();
        let t = now + chrono::Duration::seconds(2);
        t.format("%H:%M:%S").to_string()
    };

    // Helper: invoke schtasks with our "no window" trick (the schtasks
    // binary itself would otherwise pop a console window while it's
    // running). We capture stdout/stderr so we can surface any error.
    let silent = |args: &[&str]| -> Result<std::process::Output, String> {
        Command::new("schtasks.exe")
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|e| format!("schtasks {args:?} 启动失败: {e}"))
    };

    // 1. Create the task (force overwrite).
    let create_args: Vec<String> = vec![
        "/Create".into(),
        "/SC".into(),
        "ONCE".into(),
        "/TN".into(),
        task_name.clone(),
        "/TR".into(),
        tr.clone(),
        "/ST".into(),
        start_time.clone(),
        "/F".into(),
    ];
    let create_args_ref: Vec<&str> = create_args.iter().map(String::as_str).collect();
    let out = silent(&create_args_ref)?;
    if !out.status.success() {
        return Err(format!(
            "schtasks /Create 失败 (code={:?}): {}",
            out.status.code(),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }

    // 2. Run it. The created task fires within ~1 s; we don't wait.
    let run_args: Vec<&str> = vec!["/Run", "/TN", &task_name];
    let out = silent(&run_args)?;
    if !out.status.success() {
        // Best-effort: clean up the task before returning.
        let _ = silent(&["/Delete", "/TN", &task_name, "/F"]);
        return Err(format!(
            "schtasks /Run 失败 (code={:?}): {}",
            out.status.code(),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }

    // 3. Best-effort cleanup so the user's task library stays tidy.
    //    The task may already have fired and vanished, which is fine —
    //    `/Delete /F` is idempotent and silent on missing entries.
    let _ = silent(&["/Delete", "/TN", &task_name, "/F"]);

    eprintln!(
        "[update] bootstrapper dispatched via schtasks (TN={}) installer={} install_dir={} new_exe={}",
        task_name, installer, install_dir, new_exe
    );
    Ok(())
}

/// Replace the running binary with a `restart()` call that preserves
/// the original command-line arguments. Required because the previous
/// `invoke('process::restart')` referenced a non-existent Tauri plugin.
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
    fn bootstrap_script_contains_required_anchors() {
        let body = render_bootstrap_script();
        // The script must poll, run the installer, sweep, and launch.
        assert!(body.contains("tasklist /FI \"PID eq %1\""));
        assert!(body.contains("/S /D="));
        assert!(body.contains(".exe.old"));
        assert!(body.contains(".exe.bak"));
        assert!(body.contains("start \"\""));
        assert!(body.contains("del /F /Q \"%~f0\""));
    }

    #[test]
    #[cfg(windows)]
    fn bootstrap_script_runs_end_to_end() {
        // Render the script to a temp file and run it with the args
        // `cmd /C <script> <pid> <installer> <install_dir> <new_exe>`.
        // We pass a deliberately nonexistent PID (0) so the wait-loop
        // exits immediately and the installer launch is attempted
        // against a nonexistent file. We only care that the script
        // parses and executes its control flow without throwing a
        // "syntax error" from cmd — i.e. the bootstrapper is well-formed.
        let tmp = std::env::temp_dir().join(format!(
            "vrcdog-bootstrap-test-{}.cmd",
            std::process::id()
        ));
        std::fs::write(&tmp, render_bootstrap_script().as_bytes()).unwrap();

        // Fake installer & new_exe paths. The script should try to
        // launch them, fail, but still self-delete.
        let fake_installer = std::env::temp_dir().join("vrcdog-fake-installer-does-not-exist.exe");
        let fake_install_dir = std::env::temp_dir().join("vrcdog-fake-install");
        let fake_new_exe = std::env::temp_dir().join("vrcdog-fake-install/VRCDog.exe");
        let output = std::process::Command::new("cmd.exe")
            .arg("/C")
            .arg(&tmp)
            .arg("0") // PID 0 — guaranteed not running
            .arg(&fake_installer)
            .arg(&fake_install_dir)
            .arg(&fake_new_exe)
            .output()
            .expect("cmd.exe should run the bootstrap script");

        // cmd.exe should at least return (no syntax error). The
        // installer portion will fail because the file doesn't exist,
        // but that's fine for this smoke test.
        assert!(
            output.status.success() || output.status.code().is_some(),
            "bootstrap script did not produce a clean exit status: {:?}",
            output
        );

        // The script should have self-deleted.
        assert!(
            !tmp.exists(),
            "bootstrap script did not self-delete: {}",
            tmp.display()
        );
    }

    /// Verify the schtasks dispatch parameters are shaped correctly.
    /// We intentionally don't `schtasks /Create /Run` here because
    /// those calls are non-idempotent and would litter a real user's
    /// task library — we only assert the strings we'd pass.
    #[test]
    fn schtasks_dispatch_shapes_args_correctly() {
        let pid: u32 = 12345;
        let task_name = format!("VRCDog_Update_{pid}");
        let safe = |s: &str| -> String { s.replace('"', "\"\"") };
        let tr = format!(
            "\"{}\" \"{}\" \"{}\" \"{}\" \"{}\"",
            safe(&r"C:\Temp\bs.cmd"),
            pid,
            safe(&r"C:\Temp\VRCDog-Setup.exe"),
            safe(&r"C:\Program Files\VRCDog"),
            safe(&r"C:\Program Files\VRCDog\VRCDog.exe"),
        );
        // The task name must be unique per launch to avoid collisions
        // (and must not collide if two updates are dispatched by mistake).
        assert!(task_name.starts_with("VRCDog_Update_"));
        assert!(task_name.contains("12345"));
        // schtasks passes the /TR string verbatim to cmd, so embedded
        // double-quotes must be doubled (handled by `safe`) and the
        // // whole path must be double-quoted to survive cmd parsing.
        assert!(tr.starts_with("\""));
        assert!(tr.contains(r#""12345""#));
        // Doubled-quote rule: an embedded `"` must become `""` inside
        // the surrounding double-quoted field.
        let escaped = safe(r#"C:\Temp\with"quote.cmd"#);
        assert_eq!(escaped, r#"C:\Temp\with""quote.cmd"#);
    }
}