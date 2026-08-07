use serde::Serialize;
use tauri::{Emitter, Manager};

pub mod audio_capture;
pub mod bilibili;
pub mod bilibili_live;
pub mod danmaku;
pub mod db;
pub mod gallery;
pub mod gamelog;
pub mod hardware;
pub mod local_server;
pub mod ocr;
pub mod osc;
pub mod ovr;
pub mod playspace;
pub mod remote_assist;
pub mod remote_assist_hub;
pub mod server_survey;
pub mod sys;
pub mod toolchain;
pub mod translate;
pub mod vr_ui;
pub mod vrc_api;
pub mod pipeline_ws;
pub mod vrct;
pub mod vrpiano;
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

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn hide_tray_menu_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("tray-menu") {
        let _ = window.hide();
    }
}

fn show_tray_menu_window(
    app: &tauri::AppHandle,
    cursor_x: f64,
    cursor_y: f64,
) -> Result<(), String> {
    const MENU_WIDTH: f64 = 288.0;
    const MENU_HEIGHT: f64 = 272.0;
    const OFFSET: f64 = 12.0;

    let position_window = |window: &tauri::WebviewWindow| -> Result<(), String> {
        let scale_factor = window.scale_factor().map_err(|e| e.to_string())?;
        let physical_width = MENU_WIDTH * scale_factor;
        let physical_height = MENU_HEIGHT * scale_factor;
        let x = (cursor_x - physical_width + OFFSET * scale_factor).max(0.0) as i32;
        let y = (cursor_y - physical_height - OFFSET * scale_factor).max(0.0) as i32;
        window
            .set_position(tauri::PhysicalPosition::new(x, y))
            .map_err(|e| e.to_string())
    };

    if let Some(window) = app.get_webview_window("tray-menu") {
        window
            .set_size(tauri::LogicalSize::new(MENU_WIDTH, MENU_HEIGHT))
            .map_err(|e| e.to_string())?;
        position_window(&window)?;
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
        return Ok(());
    }

    let window = tauri::WebviewWindowBuilder::new(
        app,
        "tray-menu",
        tauri::WebviewUrl::App("index.html?mode=tray-menu".into()),
    )
    .title("VrcDog Menu")
    .decorations(false)
    .transparent(true)
    .shadow(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .resizable(false)
    .visible(false)
    .inner_size(MENU_WIDTH, MENU_HEIGHT)
    .build()
    .map_err(|e| e.to_string())?;

    position_window(&window)?;
    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
fn tray_show_main_window(app: tauri::AppHandle) -> Result<(), String> {
    hide_tray_menu_window(&app);
    show_main_window(&app);
    Ok(())
}

#[tauri::command]
fn tray_open_settings(app: tauri::AppHandle) -> Result<(), String> {
    hide_tray_menu_window(&app);
    show_main_window(&app);
    let _ = app.emit("tray_open_settings", ());
    Ok(())
}

#[tauri::command]
fn tray_close_menu(app: tauri::AppHandle) -> Result<(), String> {
    hide_tray_menu_window(&app);
    Ok(())
}

#[tauri::command]
fn tray_quit_app(app: tauri::AppHandle) -> Result<(), String> {
    app.exit(0);
    Ok(())
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
            let mut tray = tauri::tray::TrayIconBuilder::new()
                .tooltip("VrcDog")
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::Click {
                        button,
                        button_state: tauri::tray::MouseButtonState::Up,
                        position,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        match button {
                            tauri::tray::MouseButton::Left => {
                                hide_tray_menu_window(app);
                                show_main_window(app);
                            }
                            tauri::tray::MouseButton::Right => {
                                let _ = show_tray_menu_window(app, position.x, position.y);
                            }
                            _ => {}
                        }
                    }
                });

            if let Some(icon) = app.default_window_icon() {
                tray = tray.icon(icon.clone());
            }

            let _ = tray.build(app);

            Ok(())
        })
        .manage(vrc_api::VrcState::new())
        .manage(ovr::OvrState::new())
        .manage(danmaku::DanmakuState::new())
        .manage(vrpiano::VrpianoState::default())
        .manage(vrct::VrctState::new())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .invoke_handler(tauri::generate_handler![
            scan_local_project_dependencies,
            tray_show_main_window,
            tray_open_settings,
            tray_close_menu,
            tray_quit_app,
            toolchain::check_system_status,
            toolchain::install_software,
            toolchain::uninstall_software,
            toolchain::launch_software,
            vrc_api::vrc_execute,
            vrc_api::vrc_get_image_bytes,
            vrc_api::vrc_set_proxy,
            vrc_api::vrc_apply_auth_cookie,
            vrc_api::vrc_load_cookies_on_startup,
            vrc_api::vrc_clear_cookies,
            pipeline_ws::start_pipeline_ws,
            pipeline_ws::stop_pipeline_ws,
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
            sys::sys_gpt_sovits_synthesize,
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
            osc::osc_send_message,
            osc::osc_send_chatbox,
            osc::osc_start_monitor,
            osc::osc_stop_monitor,
            osc::osc_get_system_snapshot,
            osc::osc_start_automation,
            osc::osc_stop_automation,
            osc::osc_get_status,
            sys::sys_save_text_file,
            sys::sys_save_binary_file,
            sys::sys_set_autostart,
            sys::sys_register_url_scheme,
            sys::sys_get_launch_args,
            sys::sys_get_client_server_config,
            sys::sys_save_client_server_config,
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
            danmaku::danmaku_set_vr_input_text,
            danmaku::danmaku_submit_vr_input,
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
            bilibili_live::bili_live_get_room_info,
            bilibili_live::bili_live_get_own_room,
            bilibili_live::bili_live_get_areas,
            bilibili_live::bili_live_update_title,
            bilibili_live::bili_live_update_area,
            bilibili_live::bili_live_update_announcement,
            bilibili_live::bili_live_start,
            bilibili_live::bili_live_stop,
            bilibili_live::bili_live_send_danmaku,
            bilibili_live::bili_live_get_contribution_rank,
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
            remote_assist::remote_assist_start_view,
            remote_assist::remote_assist_stop_view,
            remote_assist::remote_assist_send_input,
            remote_assist::remote_assist_refresh_password,
            remote_assist::remote_assist_get_sessions,
            remote_assist::remote_assist_send_chat,
            remote_assist::remote_assist_get_chat,
            remote_assist::remote_assist_get_state,
            remote_assist::remote_assist_toggle_accept,
            vrpiano::vrpiano_init,
            vrpiano::vrpiano_list_songs,
            vrpiano::vrpiano_import_song,
            vrpiano::vrpiano_rename_song,
            vrpiano::vrpiano_delete_song,
            vrpiano::vrpiano_preview_song,
            vrpiano::vrpiano_read_song_data,
            vrpiano::vrpiano_download_url,
            vrpiano::vrpiano_search_midishow,
            vrpiano::vrpiano_download_midishow,
            vrpiano::vrpiano_midishow_preview_data,
            vrpiano::vrpiano_midishow_accounts,
            vrpiano::vrpiano_midishow_login,
            vrpiano::vrpiano_midishow_login_status,
            vrpiano::vrpiano_midishow_remove_account,
            vrpiano::vrpiano_open_songs_dir,
            vrpiano::vrpiano_get_status,
            vrpiano::vrpiano_start,
            vrpiano::vrpiano_stop,
            vrpiano::vrpiano_toggle_pause,
            vrpiano::vrpiano_set_speed,
            vrpiano::vrpiano_set_hotkeys,
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
    let base_url = url.trim().trim_end_matches('/');
    let parsed =
        reqwest::Url::parse(base_url).map_err(|error| format!("Invalid server URL: {error}"))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(format!(
            "Unsupported server URL scheme: {}",
            parsed.scheme()
        ));
    }
    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(3))
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|error| format!("Unable to create server probe: {error}"))?;
    let res = client
        .get(format!("{base_url}/ping"))
        .send()
        .await
        .map_err(|error| format!("Unable to connect to VRCDog server: {error}"))?;
    let status = res.status();
    if !status.is_success() {
        return Err(format!("Server responded with status: {status}"));
    }
    let payload = res
        .json::<serde_json::Value>()
        .await
        .map_err(|error| format!("Server returned an invalid ping response: {error}"))?;
    if payload.get("status").and_then(|value| value.as_str()) != Some("ok") {
        return Err("The address is not a compatible VRCDog server".to_string());
    }
    Ok("ok".to_string())
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

        // OVRAS native Qt settings. Keep the legacy compatibility keys below too:
        // different OVRAS versions have used both Qt category names and older
        // hand-written section names.
        update(
            "steamVRSettings",
            "perappBindEnabled",
            "steamvr",
            "perAppBinds",
        );

        update(
            "applicationSettings",
            "enableDebug",
            "general",
            "advDebugMode",
        );
        update(
            "applicationSettings",
            "disableVersionCheck",
            "general",
            "settingsVersionCheck",
        );
        update(
            "applicationSettings",
            "enableExclusiveInput",
            "general",
            "settingsExclusiveInput",
        );
        update(
            "applicationSettings",
            "vsyncDisabled",
            "general",
            "settingsDisableVsync",
        );
        update(
            "applicationSettings",
            "desktopModeToggle",
            "general",
            "desktopMode",
        );

        update("audioSettings", "pttEnabled", "audio", "pTT");
        update("audioSettings", "pttShowNotification", "audio", "pTTNotif");
        update(
            "audioSettings",
            "micProximitySensorCanMute",
            "audio",
            "proxSensor",
        );
        update("audioSettings", "micReversePtt", "audio", "pTM");

        update(
            "utilitiesSettings",
            "alarmEnabled",
            "utilities",
            "alarmEnabled",
        );
        update(
            "utilitiesSettings",
            "trackerOverlayEnabled",
            "utilities",
            "trackerBattery",
        );
        update("utilitiesSettings", "vrcDebug", "utilities", "keyboard");
        update(
            "keyboardShortcuts",
            "keyboardOne",
            "utilities",
            "keyShortcut1",
        );
        update(
            "keyboardShortcuts",
            "keyboardTwo",
            "utilities",
            "keyShortcut2",
        );
        update(
            "keyboardShortcuts",
            "keyboardThree",
            "utilities",
            "keyShortcut3",
        );

        update(
            "videoSettings",
            "brightnessEnabled",
            "video",
            "brightnessOn",
        );
        update(
            "videoSettings",
            "brightnessOpacityValue",
            "video",
            "brightnessValue",
        );
        update(
            "videoSettings",
            "isOverlayMethodActive",
            "video",
            "advSSFilter",
        );
        update(
            "videoSettings",
            "colorOverlayEnabled",
            "video",
            "overlayColor",
        );
        update(
            "videoSettings",
            "colorOverlayOpacity",
            "video",
            "overlayOpacity",
        );
        update("videoSettings", "colorRedNew", "video", "colorR");
        update("videoSettings", "colorGreenNew", "video", "colorG");
        update("videoSettings", "colorBlueNew", "video", "colorB");

        update(
            "chaperoneSettings",
            "fadeDistanceRemembered",
            "chaperone",
            "fadeDistance",
        );
        update("chaperoneSettings", "dimHeight", "chaperone", "height");
        update(
            "chaperoneSettings",
            "chaperoneSwitchToBeginnerEnabled",
            "chaperone",
            "beginnerMode",
        );
        update(
            "chaperoneSettings",
            "chaperoneHapticFeedbackEnabled",
            "chaperone",
            "hapticFeedback",
        );
        update(
            "chaperoneSettings",
            "chaperoneAlarmSoundEnabled",
            "chaperone",
            "audioWarning",
        );
        update(
            "chaperoneSettings",
            "chaperoneAlarmSoundLooping",
            "chaperone",
            "loopAudio",
        );
        update(
            "chaperoneSettings",
            "chaperoneAlarmSoundAdjustVolume",
            "chaperone",
            "audioVolume",
        );
        update(
            "chaperoneSettings",
            "chaperoneShowDashboardEnabled",
            "chaperone",
            "openDashboard",
        );
        update(
            "chaperoneSettings",
            "disableChaperone",
            "chaperone",
            "disable",
        );
        update(
            "chaperoneSettings",
            "centerMarkerNew",
            "chaperone",
            "centerMarker",
        );

        update(
            "playspaceSettings",
            "adjustChaperone",
            "general",
            "spaceAdjustChap",
        );
        update(
            "playspaceSettings",
            "adjustChaperone2",
            "general",
            "spaceAdjustChap",
        );
        update(
            "playspaceSettings",
            "adjustChaperone3",
            "general",
            "spaceAdjustChap",
        );
        update(
            "playspaceSettings",
            "moveShortcutLeft",
            "playspace",
            "dragLeft",
        );
        update(
            "playspaceSettings",
            "moveShortcutRight",
            "playspace",
            "dragRight",
        );
        update(
            "playspaceSettings",
            "momentumSave",
            "general",
            "motionSaveMomentum",
        );
        update(
            "playspaceSettings",
            "heightToggleOffset",
            "playspace",
            "heightOffset",
        );
        update(
            "playspaceSettings",
            "gravityStrength",
            "general",
            "motionGravityStrength",
        );
        update(
            "playspaceSettings",
            "flingStrength",
            "general",
            "motionFlingStrength",
        );
        update(
            "playspaceSettings",
            "dragMult",
            "general",
            "motionDragMultiplier",
        );
        update(
            "playspaceSettings",
            "dragComfortFactor",
            "general",
            "motionDragComfort",
        );
        update(
            "playspaceSettings",
            "turnComfortFactor",
            "general",
            "rotationTurnComfort",
        );
        update(
            "playspaceSettings",
            "snapTurnAngle",
            "general",
            "rotationSnapTurnAngle",
        );
        update(
            "playspaceSettings",
            "smoothTurnRate",
            "general",
            "rotationSmoothTurnRate",
        );
        update(
            "playspaceSettings",
            "frictionPercent",
            "general",
            "motionGravityFriction",
        );
        update(
            "playspaceSettings",
            "universeCenteredRotation",
            "general",
            "settingsUniverseRotation",
        );

        update(
            "rotationSettings",
            "autoturnEnabled",
            "general",
            "rotationAutoTurn",
        );
        update(
            "rotationSettings",
            "activationDistance",
            "general",
            "rotationActivationDist",
        );
        update(
            "rotationSettings",
            "deactivateDistance",
            "general",
            "rotationDeactivationDist",
        );
        update(
            "rotationSettings",
            "autoturnUseCornerAngle",
            "general",
            "rotationUseCornerAngle",
        );
        update(
            "rotationSettings",
            "autoturnLinearTurnSpeed",
            "general",
            "rotationTurnSpeed",
        );
        update(
            "rotationSettings",
            "autoturnVestibularMotionEnabled",
            "general",
            "rotationRedirectedWalk",
        );
        update(
            "rotationSettings",
            "autoturnVestibularMotionRadius",
            "general",
            "rotationRedirectRadius",
        );
        update(
            "rotationSettings",
            "autoturnViewRatchettingPercent",
            "general",
            "rotationViewRatchet",
        );

        // Legacy compatibility sections
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
        update("motion", "GravitySim", "general", "motionGravityOn");
        update("motion", "SpaceDragLeft", "playspace", "dragLeft");
        update("motion", "SpaceDragRight", "playspace", "dragRight");
        update("motion", "HeightToggle", "playspace", "heightToggle");
        update("motion", "HeightOffset", "playspace", "heightOffset");

        // Video settings
        update("video", "MotionSmoothing", "video", "motionSmooth");
        update("video", "SuperSampleOverride", "video", "superSampling");

        // Utilities
        update("utilities", "MediaKeysEnabled", "utilities", "mediaKeys");

        if let Some(crash_recovery_enabled) = obj
            .get("general")
            .and_then(|v| v.as_object())
            .and_then(|general| general.get("settingsCrashRecovery"))
            .and_then(|v| v.as_bool())
        {
            conf.with_section(Some("applicationSettings")).set(
                "crashRecoveryDisabled2",
                (!crash_recovery_enabled).to_string(),
            );
        }

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
        macro_rules! get_str {
            ($section:expr, $key:expr, $map_key:expr) => {
                if let Some(val) = conf.get_from(Some($section), $key) {
                    map.insert($map_key.to_string(), serde_json::json!(val));
                }
            };
        }

        // OVRAS native Qt settings
        get_bool!("steamVRSettings", "perappBindEnabled", "steamvrPerAppBinds");

        get_bool!("applicationSettings", "enableDebug", "advDebugMode");
        get_bool!(
            "applicationSettings",
            "disableVersionCheck",
            "settingsVersionCheck"
        );
        if let Some(val) = conf.get_from(Some("applicationSettings"), "crashRecoveryDisabled2") {
            map.insert(
                "settingsCrashRecovery".to_string(),
                serde_json::json!(val.to_lowercase() != "true"),
            );
        }
        get_bool!(
            "applicationSettings",
            "enableExclusiveInput",
            "settingsExclusiveInput"
        );
        get_bool!(
            "applicationSettings",
            "vsyncDisabled",
            "settingsDisableVsync"
        );
        get_bool!("applicationSettings", "desktopModeToggle", "desktopMode");

        get_bool!("audioSettings", "pttEnabled", "audioPTT");
        get_bool!("audioSettings", "pttShowNotification", "audioPTTNotif");
        get_bool!(
            "audioSettings",
            "micProximitySensorCanMute",
            "audioProxSensor"
        );
        get_bool!("audioSettings", "micReversePtt", "audioPTM");

        get_bool!("utilitiesSettings", "alarmEnabled", "utilAlarmEnabled");
        get_bool!(
            "utilitiesSettings",
            "trackerOverlayEnabled",
            "utilTrackerBattery"
        );
        get_bool!("utilitiesSettings", "vrcDebug", "utilKeyboard");
        get_str!("keyboardShortcuts", "keyboardOne", "utilKeyShortcut1");
        get_str!("keyboardShortcuts", "keyboardTwo", "utilKeyShortcut2");
        get_str!("keyboardShortcuts", "keyboardThree", "utilKeyShortcut3");

        get_bool!("videoSettings", "brightnessEnabled", "videoBrightnessOn");
        get_num!(
            "videoSettings",
            "brightnessOpacityValue",
            "videoBrightnessValue"
        );
        get_bool!("videoSettings", "isOverlayMethodActive", "videoAdvSSFilter");
        get_bool!("videoSettings", "colorOverlayEnabled", "videoOverlayColor");
        get_num!(
            "videoSettings",
            "colorOverlayOpacity",
            "videoOverlayOpacity"
        );
        get_num!("videoSettings", "colorRedNew", "videoColorR");
        get_num!("videoSettings", "colorGreenNew", "videoColorG");
        get_num!("videoSettings", "colorBlueNew", "videoColorB");

        get_num!(
            "chaperoneSettings",
            "fadeDistanceRemembered",
            "chapFadeDistance"
        );
        get_num!("chaperoneSettings", "dimHeight", "chapHeight");
        get_bool!(
            "chaperoneSettings",
            "chaperoneSwitchToBeginnerEnabled",
            "chapBeginnerMode"
        );
        get_bool!(
            "chaperoneSettings",
            "chaperoneHapticFeedbackEnabled",
            "chapHapticFeedback"
        );
        get_bool!(
            "chaperoneSettings",
            "chaperoneAlarmSoundEnabled",
            "chapAudioWarning"
        );
        get_bool!(
            "chaperoneSettings",
            "chaperoneAlarmSoundLooping",
            "chapLoopAudio"
        );
        get_bool!(
            "chaperoneSettings",
            "chaperoneAlarmSoundAdjustVolume",
            "chapAudioVolume"
        );
        get_bool!(
            "chaperoneSettings",
            "chaperoneShowDashboardEnabled",
            "chapOpenDashboard"
        );
        get_bool!("chaperoneSettings", "disableChaperone", "chapDisable");
        get_bool!("chaperoneSettings", "centerMarkerNew", "chapCenterMarker");

        get_bool!("playspaceSettings", "adjustChaperone", "spaceAdjustChap");
        get_bool!("playspaceSettings", "moveShortcutLeft", "motionDragLeft");
        get_bool!("playspaceSettings", "moveShortcutRight", "motionDragRight");
        get_bool!("playspaceSettings", "momentumSave", "motionSaveMomentum");
        get_num!(
            "playspaceSettings",
            "heightToggleOffset",
            "spaceHeightOffset"
        );
        get_num!(
            "playspaceSettings",
            "gravityStrength",
            "motionGravityStrength"
        );
        get_num!("playspaceSettings", "flingStrength", "motionFlingStrength");
        get_num!("playspaceSettings", "dragMult", "motionDragMultiplier");
        get_num!(
            "playspaceSettings",
            "dragComfortFactor",
            "motionDragComfort"
        );
        get_num!(
            "playspaceSettings",
            "turnComfortFactor",
            "rotationTurnComfort"
        );
        get_num!(
            "playspaceSettings",
            "snapTurnAngle",
            "rotationSnapTurnAngle"
        );
        get_num!(
            "playspaceSettings",
            "smoothTurnRate",
            "rotationSmoothTurnRate"
        );
        get_num!(
            "playspaceSettings",
            "frictionPercent",
            "motionGravityFriction"
        );
        get_bool!(
            "playspaceSettings",
            "universeCenteredRotation",
            "settingsUniverseRotation"
        );

        get_bool!("rotationSettings", "autoturnEnabled", "rotationAutoTurn");
        get_num!(
            "rotationSettings",
            "activationDistance",
            "rotationActivationDist"
        );
        get_num!(
            "rotationSettings",
            "deactivateDistance",
            "rotationDeactivationDist"
        );
        get_bool!(
            "rotationSettings",
            "autoturnUseCornerAngle",
            "rotationUseCornerAngle"
        );
        get_num!(
            "rotationSettings",
            "autoturnLinearTurnSpeed",
            "rotationTurnSpeed"
        );
        get_bool!(
            "rotationSettings",
            "autoturnVestibularMotionEnabled",
            "rotationRedirectedWalk"
        );
        get_num!(
            "rotationSettings",
            "autoturnVestibularMotionRadius",
            "rotationRedirectRadius"
        );
        get_num!(
            "rotationSettings",
            "autoturnViewRatchettingPercent",
            "rotationViewRatchet"
        );

        // Legacy compatibility sections
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
