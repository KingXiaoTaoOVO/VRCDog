use serde::Serialize;
use tauri::Emitter;

pub mod audio_capture;
pub mod bilibili;
pub mod danmaku;
pub mod db;
pub mod gallery;
pub mod gamelog;
pub mod hardware;
pub mod local_server;
pub mod ocr;
pub mod ovr;
pub mod playspace;
pub mod remote_assist;
pub mod sys;
pub mod toolchain;
pub mod translate;
pub mod vr_ui;
pub mod vrc_api;
pub mod vrct;
pub mod xiaohongshu;

#[derive(Debug, serde::Serialize)]
pub struct AppError {
    pub message: String,
    // Future expansion: pub code: Option<String>,
}

impl From<String> for AppError {
    fn from(err: String) -> Self {
        AppError { message: err }
    }
}

impl From<&str> for AppError {
    fn from(err: &str) -> Self {
        AppError {
            message: err.to_string(),
        }
    }
}

pub type AppResult<T> = Result<T, AppError>;

#[derive(Clone, serde::Serialize)]
pub struct EnvironmentStatus {
    hub_installed: bool,
    unity_installed: bool,
    tool_installed: bool,
    vcc_installed: bool,
    alcom_installed: bool,
    ffmpeg_installed: bool,
}

#[derive(Clone, Serialize)]
pub struct ProgressPayload {
    target: String,
    progress: f64,
    status: String,
}

#[derive(serde::Serialize)]
pub struct LocalDependency {
    name: String,
    version: String,
}

#[tauri::command]
async fn scan_local_project_dependencies() -> AppResult<Vec<LocalDependency>> {
    let mut deps = std::collections::HashMap::new();
    let local_app_data = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| "C:\\".to_string());

    // Scan VCC settings
    let settings_path = std::path::Path::new(&local_app_data)
        .join("VRChatCreatorCompanion")
        .join("settings.json");

    if let Ok(content) = std::fs::read_to_string(settings_path) {
        if let Ok(settings) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(projects) = settings.get("userProjects").and_then(|p| p.as_array()) {
                for proj in projects {
                    if let Some(path_str) = proj.as_str() {
                        let manifest_path = std::path::Path::new(path_str)
                            .join("Packages")
                            .join("vpm-manifest.json");

                        if let Ok(manifest_content) = std::fs::read_to_string(manifest_path) {
                            if let Ok(manifest) =
                                serde_json::from_str::<serde_json::Value>(&manifest_content)
                            {
                                if let Some(dependencies) =
                                    manifest.get("dependencies").and_then(|d| d.as_object())
                                {
                                    for (pkg_id, pkg_info) in dependencies {
                                        let version = if let Some(ver_str) = pkg_info.as_str() {
                                            ver_str.to_string()
                                        } else if let Some(ver_obj) = pkg_info.as_object() {
                                            ver_obj
                                                .get("version")
                                                .and_then(|v| v.as_str())
                                                .unwrap_or("unknown")
                                                .to_string()
                                        } else {
                                            "unknown".to_string()
                                        };

                                        let current_ver = deps
                                            .entry(pkg_id.clone())
                                            .or_insert_with(|| version.clone());
                                        if current_ver == "unknown" && version != "unknown" {
                                            *current_ver = version;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let mut result: Vec<LocalDependency> = deps
        .into_iter()
        .map(|(name, version)| LocalDependency { name, version })
        .collect();

    // Optional: Sort by name
    result.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(result)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            use tauri::Manager;
            let app_dir = app.path().app_data_dir().expect("无法获取应用数据目录");
            app.manage(db::DbState::new(app_dir));
            app.manage(gamelog::LogReaderState::new());
            app.manage(audio_capture::AudioCaptureState::new());

            // NOTE: OVRAS auto-install/launch has been REMOVED to prevent
            // interfering with player's own OpenVR Advanced Settings.
            // VrcDog now implements core playspace features natively via OpenVR API.
            // OVRAS INI sync is available as an opt-in manual action only.
            println!("[VrcDog] Independent mode: no OVRAS auto-install or auto-launch.");

            // System Tray Integration
            if let Ok(quit_i) =
                tauri::menu::MenuItem::with_id(app, "quit", "退出 VrcDog", true, None::<&str>)
            {
                if let Ok(show_i) =
                    tauri::menu::MenuItem::with_id(app, "show", "显示主面板", true, None::<&str>)
                {
                    if let Ok(menu) = tauri::menu::Menu::with_items(app, &[&show_i, &quit_i]) {
                        let mut tray = tauri::tray::TrayIconBuilder::new()
                            .menu(&menu)
                            .on_menu_event(|app, event| match event.id.as_ref() {
                                "quit" => {
                                    app.exit(0);
                                }
                                "show" => {
                                    if let Some(window) = app.get_webview_window("main") {
                                        let _ = window.show();
                                        let _ = window.set_focus();
                                    }
                                }
                                _ => {}
                            })
                            .on_tray_icon_event(|tray, event| {
                                if let tauri::tray::TrayIconEvent::Click {
                                    button: tauri::tray::MouseButton::Left,
                                    button_state: tauri::tray::MouseButtonState::Up,
                                    ..
                                } = event
                                {
                                    let app = tray.app_handle();
                                    if let Some(window) = app.get_webview_window("main") {
                                        let _ = window.show();
                                        let _ = window.set_focus();
                                    }
                                }
                            });

                        if let Some(icon) = app.default_window_icon() {
                            tray = tray.icon(icon.clone());
                        }

                        let _ = tray.build(app);
                    }
                }
            }

            Ok(())
        })
        .manage(vrc_api::VrcState::new())
        .manage(ovr::OvrState::new())
        .manage(danmaku::DanmakuState::new())
        .manage(vrct::VrctState::new())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            scan_local_project_dependencies,
            toolchain::check_system_status,
            toolchain::install_software,
            toolchain::uninstall_software,
            toolchain::launch_software,
            vrc_api::vrc_execute,
            vrc_api::vrc_get_image_bytes,
            vrc_api::vrc_set_proxy,
            vrc_api::vrc_apply_auth_cookie,
            vrc_api::vrc_clear_cookies,
            gamelog::vrc_get_latest_gamelogs,
            db::db_record_activity,
            db::db_get_heatmap,
            db::db_get_heatmap_details,
            db::db_save_note,
            db::db_get_note,
            db::db_get_all_notes,
            db::db_add_favorite_world,
            db::db_get_favorite_worlds,
            db::db_remove_favorite_world,
            db::db_add_favorite_avatar,
            db::db_get_favorite_avatars,
            db::db_remove_favorite_avatar,
            db::db_save_status_preset,
            db::db_get_status_presets,
            db::db_delete_status_preset,
            db::db_export_all,
            db::db_batch_record_friends,
            db::db_save_auth,
            db::db_get_auth,
            db::db_clear_auth,
            db::db_add_friend_log,
            db::db_get_friend_logs,
            db::db_clear_friend_logs,
            db::db_save_setting,
            db::db_get_setting,
            db::db_get_all_settings,
            db::db_save_friend,
            db::db_batch_save_friends,
            db::db_get_friends,
            db::db_remove_friend,
            db::db_save_game_logs,
            db::db_get_game_logs,
            db::db_clear_game_logs,
            db::db_save_notification,
            db::db_batch_save_notifications,
            db::db_get_notifications,
            db::db_delete_notification,
            gallery::gallery_get_images,
            gallery::gallery_delete_image,
            sys::sys_clear_vrchat_cache,
            sys::sys_check_steamvr,
            hardware::sys_is_vrchat_running,
            hardware::sys_launch_vrchat,
            hardware::sys_kill_vrchat,
            hardware::sys_send_osc_param,
            hardware::sys_send_osc_chatbox,
            hardware::sys_set_discord_rpc,
            hardware::sys_start_audio_capture,
            hardware::sys_stop_audio_capture,
            hardware::sys_start_osc_automation,
            hardware::sys_stop_osc_automation,
            hardware::sys_show_in_explorer,
            hardware::sys_start_auto_launch_apps,
            hardware::sys_kill_auto_launch_apps,
            sys::sys_save_text_file,
            sys::sys_set_autostart,
            sys::sys_register_url_scheme,
            sys::sys_get_launch_args,
            sys::sys_open_dir,
            sys::sys_open_url,
            sys::sys_register_steamvr_autostart,
            sys::sys_get_vrc_screenshot_dir,
            sys::sys_set_vrc_screenshot_dir,
            sys::sys_get_vrc_config,
            sys::sys_save_vrc_config,
            sys::sys_backup_database,
            sys::sys_restore_database,
            sys::sys_open_steamvr_bindings,
            audio_capture::vrct_get_audio_devices,
            audio_capture::vrct_start_stt_recording,
            audio_capture::vrct_stop_stt_recording,
            sys_start_server,
            sys_stop_server,
            sys_verify_server_password,
            sys_ping_server,
            sys_open_new_client,
            sys_is_server_running,
            ovr::ovr_init,
            ovr::ovr_shutdown,
            ovr::ovr_get_status,
            ovr::ovr_set_config,
            ovr::ovr_toggle_translation,
            ovr::ovr_capture_screenshot,
            ovr::ovr_update_overlay_text,
            ovr::ovr_set_overlay_visible,
            ovr::ovr_clear_translation,
            ovr::ovr_desktop_scan_once,
            ovr::ovr_start_auto_scan,
            ovr::ovr_stop_auto_scan,
            // Native Playspace Control (replaces OVRAS dependency)
            ovr::ovr_set_playspace_offset,
            ovr::ovr_set_playspace_rotation,
            ovr::ovr_toggle_height,
            ovr::ovr_reset_playspace,
            ovr::ovr_fix_floor,
            danmaku::danmaku_get_config,
            danmaku::danmaku_get_status,
            danmaku::danmaku_get_messages,
            danmaku::danmaku_set_config,
            danmaku::danmaku_start,
            danmaku::danmaku_stop,
            danmaku::danmaku_clear_messages,
            danmaku::danmaku_set_overlay_visible,
            danmaku::danmaku_send_test,
            translate::ovr_translate,
            vrct::vrct_process_message,
            vrct::vrct_get_history,
            vrct::vrct_clear_history,
            ovr_sync_ovras_ini,
            ovr_load_ovras_ini,
            bilibili::bili_check_login,
            bilibili::bili_new_qr,
            bilibili::bili_get_qr_status,
            bilibili::bili_get_video_info,
            bilibili::bili_get_play_info,
            bilibili::bili_get_mp4_play_info,
            bilibili::bili_download_video,
            bilibili::parser::bili_parse_url,
            xiaohongshu::xhs_parse_url,
            db::db_bili_add_task,
            db::db_bili_get_tasks,
            db::db_bili_update_task_status,
            db::db_bili_delete_task,
            db::db_get_api_cache,
            db::db_save_api_cache,
            remote_assist::remote_assist_init,
            remote_assist::remote_assist_get_servers,
            remote_assist::remote_assist_set_server,
            remote_assist::remote_assist_add_custom_server,
            remote_assist::remote_assist_start_service,
            remote_assist::remote_assist_stop_service,
            remote_assist::remote_assist_connect,
            remote_assist::remote_assist_disconnect,
            remote_assist::remote_assist_refresh_password,
            remote_assist::remote_assist_get_sessions,
            remote_assist::remote_assist_send_chat,
            remote_assist::remote_assist_get_chat,
            remote_assist::remote_assist_get_state,
            remote_assist::remote_assist_toggle_accept,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
async fn sys_verify_server_password(password: String) -> Result<(), String> {
    if !verify_server_password(&password) {
        return Err("服务端密码验证失败！".into());
    }
    Ok(())
}

const DEFAULT_SERVER_PASSWORD_BCRYPT: &str =
    "$2b$12$go9qphFk80mBGkPx9AiayObfu.gfsSvKCAL0sBMnTBYreWAGYDBiK";

fn server_password_hash() -> String {
    std::env::var("VRCDOG_SERVER_PASSWORD_BCRYPT")
        .ok()
        .map(|hash| hash.trim().to_string())
        .filter(|hash| !hash.is_empty())
        .unwrap_or_else(|| DEFAULT_SERVER_PASSWORD_BCRYPT.to_string())
}

fn verify_server_password(password: &str) -> bool {
    bcrypt::verify(password, &server_password_hash()).unwrap_or(false)
}

#[cfg(test)]
mod password_tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn with_server_password_hash_env<T>(hash: Option<&str>, run: impl FnOnce() -> T) -> T {
        let _guard = ENV_LOCK.lock().expect("server password env lock poisoned");
        let previous = std::env::var_os("VRCDOG_SERVER_PASSWORD_BCRYPT");

        match hash {
            Some(hash) => std::env::set_var("VRCDOG_SERVER_PASSWORD_BCRYPT", hash),
            None => std::env::remove_var("VRCDOG_SERVER_PASSWORD_BCRYPT"),
        }

        let result = run();

        match previous {
            Some(value) => std::env::set_var("VRCDOG_SERVER_PASSWORD_BCRYPT", value),
            None => std::env::remove_var("VRCDOG_SERVER_PASSWORD_BCRYPT"),
        }

        result
    }

    #[test]
    fn default_server_password_is_bcrypt_hash() {
        assert!(DEFAULT_SERVER_PASSWORD_BCRYPT.starts_with("$2b$"));
        assert_ne!(DEFAULT_SERVER_PASSWORD_BCRYPT, "root");
        assert!(bcrypt::verify("root", DEFAULT_SERVER_PASSWORD_BCRYPT).unwrap());
    }

    #[test]
    fn verifies_default_server_password_with_bcrypt() {
        with_server_password_hash_env(None, || {
            assert!(verify_server_password("root"));
            assert!(!verify_server_password("wrong-password"));
        });
    }

    #[test]
    fn supports_bcrypt_hash_override_from_environment() {
        let custom_hash = bcrypt::hash("custom-passphrase", bcrypt::DEFAULT_COST).unwrap();

        with_server_password_hash_env(Some(&custom_hash), || {
            assert!(verify_server_password("custom-passphrase"));
            assert!(!verify_server_password("root"));
        });
    }

    #[test]
    fn invalid_bcrypt_hash_fails_closed() {
        with_server_password_hash_env(Some("not-a-bcrypt-hash"), || {
            assert!(!verify_server_password("root"));
        });
    }
}

#[tauri::command]
async fn sys_start_server(
    app_handle: tauri::AppHandle,
    host: String,
    port: u16,
) -> Result<(), String> {
    local_server::start_server(app_handle, host, port).await
}

#[tauri::command]
fn sys_stop_server(app_handle: tauri::AppHandle) -> Result<(), String> {
    local_server::stop_server();
    let _ = app_handle.emit("server_log", "[INFO] 服务端已停止".to_string());
    Ok(())
}

#[tauri::command]
fn sys_is_server_running() -> Result<bool, String> {
    Ok(local_server::is_server_running())
}

#[tauri::command]
async fn sys_ping_server(url: String) -> Result<String, String> {
    let client = reqwest::Client::new();
    let res = client
        .get(format!("{}/ping", url))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if res.status().is_success() {
        Ok("ok".to_string())
    } else {
        Err(format!("Server responded with status: {}", res.status()))
    }
}

#[tauri::command]
#[allow(dead_code)]
fn sys_open_new_client() -> Result<(), String> {
    let exe_path = std::env::current_exe().map_err(|e| e.to_string())?;
    std::process::Command::new(exe_path)
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// OVRAS INI sync 鈥?OPT-IN ONLY, user must manually trigger this.
/// Will NOT auto-run on startup. Requires OVRAS to be already installed.
#[tauri::command]
async fn ovr_sync_ovras_ini(payload: String) -> Result<(), String> {
    // Safety check: Only sync if OVRAS is actually installed by the user
    let ovras_exe =
        std::path::Path::new(r"C:\Program Files\OpenVR-AdvancedSettings\AdvancedSettings.exe");
    if !ovras_exe.exists() {
        return Err(
            "OVRAS 未安装，无法同步。VrcDog 已内置原生 Playspace 功能，无需 OVRAS。".into(),
        );
    }

    let appdata = std::env::var("APPDATA").unwrap_or_else(|_| "C:\\".to_string());
    let ini_path = std::path::Path::new(&appdata)
        .join("AdvancedSettings-Team")
        .join("OpenVRAdvancedSettings.ini");

    // Parse the frontend JSON config
    let config: serde_json::Value = serde_json::from_str(&payload).map_err(|e| e.to_string())?;

    let mut conf = ini::Ini::load_from_file(&ini_path).unwrap_or_else(|_| ini::Ini::new());

    if let Some(obj) = config.as_object() {
        let mut update = |section: &str, key: &str, group: &str, subkey: &str| {
            if let Some(group_obj) = obj.get(group).and_then(|v| v.as_object()) {
                if let Some(v) = group_obj.get(subkey) {
                    let val_str = if v.is_string() {
                        v.as_str().unwrap().to_string()
                    } else {
                        v.to_string()
                    };
                    conf.with_section(Some(section)).set(key, val_str);
                }
            }
        };

        // Chaperone settings
        update(
            "chaperone",
            "CollisionBoundsColorAlpha",
            "chaperone",
            "visibility",
        );
        update(
            "chaperone",
            "ForceBoundsVisible",
            "chaperone",
            "forceBounds",
        );
        update("chaperone", "HapticFeedback", "chaperone", "hapticFeedback");

        // Playspace settings
        update("offsets", "RotationY", "playspace", "rotation");
        update("offsets", "X", "playspace", "offsetX");
        update("offsets", "Y", "playspace", "offsetY");
        update("offsets", "Z", "playspace", "offsetZ");
        update("motion", "GravitySim", "playspace", "gravity");
        update("motion", "SpaceDragLeft", "playspace", "dragLeft");
        update("motion", "SpaceDragRight", "playspace", "dragRight");
        update("motion", "HeightToggle", "playspace", "heightToggle");
        update("motion", "HeightOffset", "playspace", "heightOffset");

        // Video settings
        update("video", "MotionSmoothing", "video", "motionSmooth");
        update("video", "SuperSampleOverride", "video", "superSampling");

        // Utilities
        update("utilities", "MediaKeysEnabled", "utilities", "mediaKeys");

        // VrcDog Branding
        conf.with_section(Some("VrcDog"))
            .set("SyncedBy", "VrcDog_HyperEngine")
            .set("LastSyncTime", chrono::Local::now().to_rfc3339());
    }

    // Write back to disk (This will instantly trigger OVRAS to auto-reload)
    if let Some(parent) = ini_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create directory: {}", e))?;
    }
    conf.write_to_file(&ini_path)
        .map_err(|e| format!("Failed to write INI file: {}", e))?;

    println!("[VrcDog] Synced actual config keys to OVRAS native INI file using rust-ini.");
    Ok(())
}

#[tauri::command]
async fn ovr_load_ovras_ini() -> Result<String, String> {
    let appdata = std::env::var("APPDATA").unwrap_or_else(|_| "C:\\".to_string());
    let ini_path = std::path::Path::new(&appdata)
        .join("AdvancedSettings-Team")
        .join("OpenVRAdvancedSettings.ini");

    let mut map = serde_json::Map::new();

    if let Ok(conf) = ini::Ini::load_from_file(&ini_path) {
        macro_rules! get_bool {
            ($section:expr, $key:expr, $map_key:expr) => {
                if let Some(val) = conf.get_from(Some($section), $key) {
                    map.insert(
                        $map_key.to_string(),
                        serde_json::json!(val.to_lowercase() == "true"),
                    );
                }
            };
        }
        macro_rules! get_num {
            ($section:expr, $key:expr, $map_key:expr) => {
                if let Some(val) = conf.get_from(Some($section), $key) {
                    if let Ok(num) = val.parse::<f64>() {
                        map.insert($map_key.to_string(), serde_json::json!(num));
                    }
                }
            };
        }

        // Chaperone settings
        get_num!("chaperone", "CollisionBoundsColorAlpha", "chapVisibility");
        get_bool!("chaperone", "ForceBoundsVisible", "chapForceBounds");
        get_bool!("chaperone", "HapticFeedback", "chapHapticFeedback");

        // Playspace settings
        get_num!("offsets", "RotationY", "spaceRotation");
        get_num!("offsets", "X", "spaceOffsetX");
        get_num!("offsets", "Y", "spaceOffsetY");
        get_num!("offsets", "Z", "spaceOffsetZ");
        get_bool!("motion", "GravitySim", "motionGravity");
        get_bool!("motion", "SpaceDragLeft", "motionDragLeft");
        get_bool!("motion", "SpaceDragRight", "motionDragRight");
        get_bool!("motion", "HeightToggle", "motionHeightToggle");
        get_num!("motion", "HeightOffset", "motionHeightOffset");

        // Video settings
        get_bool!("video", "MotionSmoothing", "videoMotionSmooth");
        get_num!("video", "SuperSampleOverride", "videoSuperSampling");

        // Utilities
        get_bool!("utilities", "MediaKeysEnabled", "utilMediaKeys");
    }

    Ok(serde_json::Value::Object(map).to_string())
}
