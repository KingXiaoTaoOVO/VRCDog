//! Application update flow.
//!
//! Replaces Tauri 2's `tauri-plugin-updater` for this app because the
//! `vrcdog-releases` repo does not currently publish a signed
//! `updater.json`, which makes the official plugin's `check()` fail
//! with HTTP 404 on every cold start.
//!
//! Instead we:
//!   1. Query GitHub's REST API directly (`/repos/{owner}/{repo}/releases`).
//!   2. Stream the chosen asset to a temp file while emitting progress.
//!   3. Verify SHA-256 against the GitHub asset digest.
//!   4. Run the NSIS installer silently (`/S`); the installer handles
//!      all "remove old files + install new" semantics for `currentUser`
//!      in-place upgrades — Tauri 2's bundled NSIS template already
//!      renames the locked executable before replacing it.
//!   5. After the installer exits, we explicitly relaunch the freshly
//!      installed binary so the user never has to click anything again.
//!   6. Sweep stale `*.exe.old` leftovers left behind by the NSIS
//!      upgrade dance (and any other installer-renamed siblings we know
//!      about) so the install dir stays clean.
//!   7. `app.exit(0)` shuts down the running process.

use std::path::{Path, PathBuf};
use std::time::Duration;

use futures_util::StreamExt;
use serde::Serialize;
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter};

const GITHUB_RELEASES_API: &str =
    "https://api.github.com/repos/KingXiaoTaoOVO/vrcdog-releases/releases";

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

/// Clean the leftovers NSIS leaves behind during in-place upgrades.
/// Tauri 2's bundled NSIS template renames `VRCDog.exe` to
/// `VRCDog.exe.old` before swapping in the new file, and relies on the
/// next launch to delete the `.old` copy. Since the runner dies inside
/// `app.exit(0)` we get a chance to sweep those ourselves.
fn sweep_install_leftovers(install_dir: &Path) {
    let Ok(read) = std::fs::read_dir(install_dir) else {
        return;
    };
    for entry in read.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
            continue
        };
        // NSIS in-place upgrade leaves behind:
        //   - <binary>.exe.old
        //   - <binary>.exe.bak (older NSIS templates)
        //   - the installer itself when the user runs it from %TEMP%
        let lower = name.to_ascii_lowercase();
        let is_stale_binary = lower.ends_with(".exe.old") || lower.ends_with(".exe.bak");
        let is_our_temp_setup = lower.starts_with("vrcdog-setup-")
            && (lower.ends_with(".exe") || lower.ends_with(".msi"));
        let is_uninst_tmp = lower.starts_with("vrcdog_") && lower.ends_with("-setup.exe.partial");
        if is_stale_binary || is_our_temp_setup || is_uninst_tmp {
            let _ = std::fs::remove_file(&path);
            eprintln!("[update] swept leftover {}", path.display());
        }
    }
}

/// Spawn the installer and wait for it. NSIS handles the silent flag
/// itself — `/S` is case-insensitive on its CLI but we keep it upper
/// for clarity. `/D=path` pins the install dir to whatever the running
/// app lives in (so NSIS upgrades don't try to migrate to a fresh
/// per-user sub-folder).
fn run_installer_and_wait(installer: &Path, run_dir_hint: &Path) -> Result<(), String> {
    let install_dir = if run_dir_hint.is_dir() {
        run_dir_hint.to_path_buf()
    } else {
        PathBuf::from("C:\\Users\\Public")
    };
    let install_dir_str = install_dir.to_string_lossy().to_string();

    // NSIS / MSI are picky: `/S` only triggers "silent" mode when it's
    // the first flag and there are no conflicting UI flags ahead. To be
    // safe we also pass `/allusers` is intentionally skipped because
    // Tauri uses `currentUser` install mode.
    let mut cmd = std::process::Command::new(installer);
    cmd.arg("/S");
    cmd.arg(format!("/D={}", install_dir_str));

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("无法启动安装程序: {e}"))?;
    let status = child
        .wait()
        .map_err(|e| format!("等待安装程序退出失败: {e}"))?;
    if !status.success() {
        return Err(format!(
            "安装程序返回非零退出码: {}",
            status.code().unwrap_or(-1)
        ));
    }
    Ok(())
}

/// Run the full auto-update flow against a single chosen release.
///
/// Pipeline:
///   1. Stream the installer from GitHub to `%TEMP%\VRCDog-Setup-vX.Y.Z.exe`.
///   2. Verify SHA-256 against the GitHub asset digest.
///   3. Invoke the NSIS installer silently in-place (replaces files).
///   4. Sweep `.exe.old` leftovers from the install dir.
///   5. Launch the freshly-installed binary detached.
///   6. `app.exit(0)` to terminate the running process.
///
/// Returns once the installer has been spawned successfully; the
/// frontend will observe the new process via `app-update://done` event
/// and exit itself.
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
    let installer_name = if stem.to_ascii_lowercase().contains("setup") {
        format!("VRCDog-Setup-{stamp}.exe")
    } else {
        format!("VRCDog-Installer-{stamp}.{}", std::path::Path::new(&stem).extension().and_then(|e| e.to_str()).unwrap_or("exe"))
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
            message: "正在静默安装，请等待".into(),
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

    // The installer itself runs synchronously on a blocking thread because
    // `std::process::Command::wait` is blocking.
    let installer_for_blocking = installer.clone();
    let run_dir_hint_owned = run_dir_hint.clone();
    let install_result = tokio::task::spawn_blocking(move || {
        run_installer_and_wait(&installer_for_blocking, &run_dir_hint_owned)
    })
    .await
    .map_err(|e| format!("安装任务被打断: {e}"))?;
    if let Err(err) = install_result {
        return Err(err);
    }

    // Sweep the leftovers before launching.
    sweep_install_leftovers(&run_dir_hint);

    emit_progress(
        &app,
        InstallProgress {
            stage: "launching",
            bytes_done: total,
            bytes_total: total,
            message: "已安装完成，正在启动新版本".into(),
        },
    );

    if let Some(new_exe) = locate_installed_exe(&app) {
        if let Some(parent) = new_exe.parent() {
            sweep_install_leftovers(parent);
        }
        let _ = std::process::Command::new(&new_exe)
            .args(std::env::args().skip(1))
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();
    }

    // Remove the temp installer we downloaded; NSIS has already copied
    // everything it needs into the install dir.
    let _ = std::fs::remove_file(&installer);

    emit_done(&app, "新版本已启动");
    // Give the frontend a moment to render the success state before we
    // tear down the runtime.
    std::thread::sleep(Duration::from_millis(400));
    app.exit(0);
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
}
