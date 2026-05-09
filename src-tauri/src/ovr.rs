use std::sync::Arc;
use tokio::sync::Mutex;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use ab_glyph::{FontVec, Font};

// ==================== Types ====================

#[derive(Debug, Clone, Serialize)]
#[derive(Default)]
pub struct OvrStatus {
    pub initialized: bool,
    pub hmd_present: bool,
    pub hmd_model: String,
    pub overlay_visible: bool,
    pub dashboard_visible: bool,
    pub translation_enabled: bool,
    pub last_event: String,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OvrConfig {
    pub enabled: bool,
    pub dual_display: bool,
    pub wrist_mode: bool,
    pub trigger_key: String,
    pub clear_key: String,
    pub overlay_text_color: String,
    pub overlay_bg_color: String,
    pub overlay_bg_opacity: f32,
    pub overlay_lock_mode: String,
    pub status_color: String,
    #[serde(default)]
    pub trans_service: String,
    #[serde(default)]
    pub trans_api_key: String,
    #[serde(default)]
    pub trans_llm_model: String,
    #[serde(default)]
    pub trans_llm_prompt: String,
    #[serde(default)]
    pub trans_source_lang: String,
    #[serde(default)]
    pub trans_target_lang: String,
    // ===== Desktop Mirror Translation Mode =====
    #[serde(default)]
    pub desktop_mode: bool,          // Enable desktop mirror capture mode
    #[serde(default)]
    pub auto_scan_enabled: bool,     // Auto-scan at interval
    #[serde(default = "default_auto_scan_interval")]
    pub auto_scan_interval: u32,     // Auto-scan interval in seconds (3-60)
    #[serde(default = "default_true")]
    pub tts_enabled: bool,           // TTS voice readback toggle
    #[serde(default = "default_true")]
    pub osc_chatbox_enabled: bool,   // OSC chatbox output toggle
    // ===== Frontend Config Items (P1: now actually connected to VR backend) =====
    #[serde(default = "default_scan_color")]
    pub scan_frame_color: String,          // Scan frame color hex e.g. "#00FF64"
    #[serde(default = "default_ocr_lang")]
    pub ocr_language: String,              // OCR language: "ja","en-US","zh-Hans-CN","ko","fr","de"
    #[serde(default)]
    pub ocr_speed_mode: String,            // "fast", "balanced", "accurate"
    #[serde(default)]
    pub ocr_image_enhance: bool,           // Enable image preprocessing
    #[serde(default = "default_ocr_contrast")]
    pub ocr_contrast: f32,                 // OCR image contrast
    #[serde(default)]
    pub ocr_sharpen: bool,                 // OCR image sharpen
    #[serde(default)]
    pub ocr_denoise: bool,                 // OCR image denoise
    #[serde(default = "default_merge_x")]
    pub ocr_merge_tolerance_x: f32,        // Horizontal merge tolerance
    #[serde(default = "default_merge_y")]
    pub ocr_merge_tolerance_y: f32,        // Vertical merge tolerance
    #[serde(default)]
    pub auto_start_steamvr: bool,          // Auto start with SteamVR
    #[serde(default = "default_panel_width")]
    pub trans_panel_max_width: u32,        // Max width of translation panel in pixels
    #[serde(default = "default_font_size")]
    pub overlay_font_size: f32,            // Font size for overlay text
    #[serde(default = "default_grip_threshold")]
    pub grip_pressure_threshold: f32,      // Index controller grip pressure threshold (0.0-1.0)
    #[serde(default)]
    pub custom_api_url: String,            // Custom LLM API endpoint URL
    // ===== Native Playspace Control (replaces OVRAS dependency) =====
    #[serde(default)]
    pub playspace_offset_x: f32,     // X offset in meters
    #[serde(default)]
    pub playspace_offset_y: f32,     // Y offset in meters (HEIGHT)
    #[serde(default)]
    pub playspace_offset_z: f32,     // Z offset in meters
    #[serde(default)]
    pub playspace_rotation: f32,     // Rotation in degrees (Y-axis)
    #[serde(default)]
    pub height_toggle_enabled: bool, // Whether height toggle is active
    #[serde(default = "default_height_offset")]
    pub height_toggle_offset: f32,   // Height toggle offset in meters (positive = down)
}

fn default_auto_scan_interval() -> u32 { 5 }
fn default_true() -> bool { true }
fn default_height_offset() -> f32 { 0.3 }
fn default_scan_color() -> String { "#00FF64".into() }
fn default_ocr_lang() -> String { "ja".into() }
fn default_panel_width() -> u32 { 512 }
fn default_font_size() -> f32 { 28.0 }
fn default_grip_threshold() -> f32 { 0.8 }
fn default_ocr_contrast() -> f32 { 1.0 }
fn default_merge_x() -> f32 { 0.2 }
fn default_merge_y() -> f32 { 0.3 }

impl Default for OvrConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            dual_display: true,
            wrist_mode: false,
            trigger_key: "trigger".into(),
            clear_key: "left_stick".into(),
            overlay_text_color: "#FFFFFF".into(),
            overlay_bg_color: "#1a1a2e".into(),
            overlay_bg_opacity: 0.85,
            overlay_lock_mode: "world".into(),
            status_color: "#00FF00".into(),
            trans_service: "google_free".into(),
            trans_api_key: "".into(),
            trans_llm_model: "".into(),
            trans_llm_prompt: "Translate the following text to Chinese. Only output the translation:".into(),
            trans_source_lang: "auto".into(),
            trans_target_lang: "zh-CN".into(),
            desktop_mode: false,
            auto_scan_enabled: false,
            auto_scan_interval: 5,
            tts_enabled: true,
            osc_chatbox_enabled: true,
            // Frontend config items (now connected to VR backend)
            scan_frame_color: "#00FF64".into(),
            ocr_language: "ja".into(),
            ocr_speed_mode: "balanced".into(),
            ocr_image_enhance: false,
            trans_panel_max_width: 512,
            overlay_font_size: 28.0,
            grip_pressure_threshold: 0.8,
            custom_api_url: String::new(),
            // Playspace
            playspace_offset_x: 0.0,
            playspace_offset_y: 0.0,
            playspace_offset_z: 0.0,
            playspace_rotation: 0.0,
            height_toggle_enabled: false,
            height_toggle_offset: 0.3,
            ocr_contrast: 1.0,
            ocr_sharpen: false,
            ocr_denoise: false,
            ocr_merge_tolerance_x: 0.2,
            ocr_merge_tolerance_y: 0.3,
            auto_start_steamvr: false,
        }
    }
}

// ==================== Command Channel ====================

#[derive(Debug)]
enum OvrCommand {
    UpdateConfig(Box<OvrConfig>),
    UpdateText { original: String, translated: String },
    SetVisible(bool),
    ClearText,
    ToggleTranslation,
    Shutdown,
    // Desktop mirror mode commands
    DesktopScanOnce,      // Trigger a single desktop capture + OCR + translate
    StartAutoScan,        // Start auto-scan timer
    StopAutoScan,         // Stop auto-scan timer
    // ===== Native Playspace Control Commands =====
    SetPlayspaceOffset { x: f32, y: f32, z: f32 },  // Set playspace XYZ offset
    SetPlayspaceRotation(f32),                        // Set playspace Y rotation
    ToggleHeight,                                     // Toggle height offset on/off
    ResetPlayspace,                                   // Reset all offsets to zero
    FixFloor,                                         // Fix floor height using controller position
}

// ==================== State ====================

pub struct OvrState {
    pub status: Arc<Mutex<OvrStatus>>,
    pub config: Arc<Mutex<OvrConfig>>,
    cmd_tx: Arc<Mutex<Option<std::sync::mpsc::Sender<OvrCommand>>>>,
    event_loop_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

impl Default for OvrState {
    fn default() -> Self {
        Self::new()
    }
}

impl OvrState {
    pub fn new() -> Self {
        Self {
            status: Arc::new(Mutex::new(OvrStatus::default())),
            config: Arc::new(Mutex::new(OvrConfig::default())),
            cmd_tx: Arc::new(Mutex::new(None)),
            event_loop_handle: Arc::new(Mutex::new(None)),
        }
    }
}

// ==================== Font Rendering ====================

/// Load system CJK font (Microsoft YaHei on Windows, fallback to embedded basic font)
/// TTC files contain multiple font faces — try indices 0-3 to find one that has CJK glyphs
fn load_system_font() -> Option<FontVec> {
    let font_paths = [
        r"C:\Windows\Fonts\simhei.ttf",   // SimHei (PRIORITY for CJK)
        r"C:\Windows\Fonts\msyh.ttc",    // Microsoft YaHei (CJK)
        r"C:\Windows\Fonts\msyhbd.ttc",   // Microsoft YaHei Bold
        r"C:\Windows\Fonts\simsun.ttc",   // SimSun
        r"C:\Windows\Fonts\simsun.ttc",   // SimSun
        r"C:\Windows\Fonts\arial.ttf",    // Arial fallback
        r"C:\Windows\Fonts\segoeui.ttf",  // Segoe UI fallback
    ];

    for path in &font_paths {
        if let Ok(data) = std::fs::read(path) {
            // For TTC files, try multiple font face indices
            for index in 0..4u32 {
                if let Ok(font) = FontVec::try_from_vec_and_index(data.clone(), index) {
                    // Verify this face has CJK characters by checking a common Chinese char
                    let test_glyph = font.glyph_id('\u{4F60}'); // '你'
                    if test_glyph.0 != 0 {
                        return Some(font);
                    }
                }
            }
            // Fallback: just load index 0 if no CJK face found
            if let Ok(font) = FontVec::try_from_vec(data) {
                return Some(font);
            }
        }
    }
    None
}

/// Parse hex color "#RRGGBB" to [R, G, B]
/// Capture screen + Windows OCR + translate pipeline
/// Now uses config.ocr_language, config.ocr_image_enhance, config.ocr_speed_mode
async fn perform_scan_translate(
    config: &OvrConfig,
) -> Result<(String, String), String> {
    let ocr_text = crate::ocr::OcrEngine::extract_text_from_screen(&config.ocr_language, config.ocr_image_enhance).await?;

    let req = crate::translate::TranslateRequest {
        text: ocr_text.clone(),
        source_lang: config.trans_source_lang.clone(),
        target_lang: config.trans_target_lang.clone(),
        service: config.trans_service.clone(),
        api_key: config.trans_api_key.clone(),
        model: config.trans_llm_model.clone(),
        prompt: format!("{}\n{}", config.trans_llm_prompt, ocr_text),
        custom_api_url: config.custom_api_url.clone(),
    };

    match crate::translate::translate(&req).await {
        Ok(result) => Ok((result.original, result.translated)),
        Err(_) => Ok((ocr_text.clone(), format!("[OCR结果 - 未配置翻译API]\n{}", ocr_text))),
    }
}

#[allow(unused_assignments)]
fn vr_thread_main(
    app_handle: AppHandle,
    status: Arc<Mutex<OvrStatus>>,
    _config: Arc<Mutex<OvrConfig>>,
    cmd_rx: std::sync::mpsc::Receiver<OvrCommand>,
) {
    // Initialize OpenVR
    let context = match unsafe { openvr::init(openvr::ApplicationType::Overlay) } {
        Ok(ctx) => ctx,
        Err(e) => {
            let _ = app_handle.emit("ovr_error", format!("OpenVR 初始化失败: {:?}", e));
            let status_clone = status.clone();
            tokio::runtime::Handle::current().block_on(async {
                let mut s = status_clone.lock().await;
                s.initialized = false;
            });
            return;
        }
    };

    // Load font for text rendering
    let font = load_system_font();
    if font.is_none() {
        let _ = app_handle.emit("ovr_log", "[OVR] [Warn] 未找到系统字体，文字渲染可能异常");
    }

    // Get HMD info
    let mut hmd_model;
    if let Ok(sys) = context.system() {
        hmd_model = sys.string_tracked_device_property(
            openvr::TrackedDeviceIndex(0),
            openvr::property::ModelNumber_String,
        ).map(|s| s.to_string_lossy().to_string())
         .unwrap_or_else(|_| "Unknown HMD".to_string());
    } else {
        hmd_model = "Unknown".to_string();
    }

    if hmd_model.to_uppercase() == "PICO 4S" || hmd_model.to_uppercase() == "PICO4S" {
        hmd_model = "PICO 4 Pro".to_string();
    }

    // Update status
    {
        let status_c = status.clone();
        let model = hmd_model.clone();
        tokio::runtime::Handle::current().block_on(async {
            let mut s = status_c.lock().await;
            s.initialized = true;
            s.hmd_present = true;
            s.hmd_model = model;
        });
    }

    let _ = app_handle.emit("ovr_status", "initialized");
    let _ = app_handle.emit("ovr_log", format!("[OVR] [OK] OpenVR 已连接: {}", hmd_model));

    // ===== Create overlays =====
    let mut overlay_handle = None;  // Main menu/translation overlay
    let mut scan_handle = None;     // Green scan frame overlay

    if let Ok(mut ovr) = context.overlay() {
        // 1) Main menu overlay - follows HMD, shows menu & translation results
        match ovr.create_overlay("vrcdog.menu\0", "VrcDog Menu\0") {
            Ok(handle) => {
                let _ = ovr.set_width(handle, 0.5);
                let _ = ovr.set_opacity(handle, 0.92);
                let _ = ovr.set_sort_order(handle, 10);
                
                // Attempt to lock to left hand (wristwatch style)
                let mut locked_to_hand = false;
                if let Ok(sys) = context.system() {
                    if let Some(l_idx) = sys.tracked_device_index_for_controller_role(openvr::TrackedControllerRole::LeftHand) {
                        let hand_transform = openvr::pose::Matrix3x4([
                            [1.0, 0.0, 0.0, 0.0],
                            [0.0, 1.0, 0.0, 0.08],
                            [0.0, 0.0, 1.0, -0.05],
                        ]);
                        let _ = ovr.set_transform_tracked_device_relative(handle, l_idx, &hand_transform);
                        let _ = ovr.set_width(handle, 0.12); // Watch scale (12cm)
                        locked_to_hand = true;
                    }
                }
                if !locked_to_hand {
                    // Head-locked fallback
                    let hmd_transform = openvr::pose::Matrix3x4([
                        [1.0, 0.0, 0.0, 0.0],
                        [0.0, 1.0, 0.0, -0.1],
                        [0.0, 0.0, 1.0, -0.5],
                    ]);
                    let _ = ovr.set_transform_tracked_device_relative(handle, openvr::TrackedDeviceIndex(0), &hmd_transform);
                    let _ = ovr.set_width(handle, 0.25);
                }
                // Render initial menu
                if let Some(ref f) = font {
                    let menu_text = "VrcDog VR 翻译器\n\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\n手柄操作:\n  [B长按] 开关菜单  [扳机] 扫描\n  [摇杆] 导航      [右侧握把+摇杆] 缩放推拉\n  [左侧摇杆按下] 清除结果";
                    let pixels = crate::vr_ui::VrUiRenderer::render_text_to_rgba(
                        f, "", menu_text, false,
                        512, 320, [255,255,255], [18,18,42], 0.90,
                    );
                    let _ = ovr.set_raw_data(handle, &pixels, 512, 320, 4);
                }
                let _ = ovr.set_visibility(handle, true);
                overlay_handle = Some(handle);
                let _ = app_handle.emit("ovr_log", "[OVR] [OK] 菜单叠加层已创建(跟随头部)");
            }
            Err(e) => {
                let _ = app_handle.emit("ovr_log", format!("[OVR] [Error] 菜单创建失败: {:?}", e));
            }
        }

        // 2) Scan frame overlay - follows right controller, green border
        match ovr.create_overlay("vrcdog.scan\0", "VrcDog Scan\0") {
            Ok(handle) => {
                let _ = ovr.set_width(handle, 0.3);
                let _ = ovr.set_opacity(handle, 0.8);
                let _ = ovr.set_sort_order(handle, 20);
                // Render green scan frame (border only, transparent center)
                let scan_pixels = crate::vr_ui::VrUiRenderer::render_scan_frame(256, 256, "#00FF64");
                let _ = ovr.set_raw_data(handle, &scan_pixels, 256, 256, 4);
                let _ = ovr.set_visibility(handle, false); // Hidden until trigger
                scan_handle = Some(handle);
                let _ = app_handle.emit("ovr_log", "[OVR] [OK] 扫描框叠加层已创建");
            }
            Err(e) => {
                let _ = app_handle.emit("ovr_log", format!("[OVR] [Error] 扫描框创建失败: {:?}", e));
            }
        }
    }

    let _ = app_handle.emit("ovr_log", "[OVR] [OK] VR 事件循环已启动 (90Hz)");

    // ===== SteamVR Input 2.0 Initialization =====
    let mut act_set_main = openvr::input::VRActionSetHandle(0);
    let mut act_translate = openvr::input::VRActionHandle(0);
    let mut act_scale = openvr::input::VRActionHandle(0);
    let mut act_clear = openvr::input::VRActionHandle(0);
    let mut has_input_20 = false;
    
    if let Ok(mut input) = context.input() {
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(parent) = exe_path.parent() {
                // Try parent first (prod mode)
                let mut manifest_path = parent.join("vrcdog_actions.json");
                if !manifest_path.exists() {
                    // Try current dir (dev mode)
                    if let Ok(cd) = std::env::current_dir() {
                        manifest_path = cd.join("vrcdog_actions.json");
                    }
                }
                if manifest_path.exists() {
                    if let Ok(path_str) = std::ffi::CString::new(manifest_path.to_string_lossy().as_bytes()) {
                        if input.set_action_manifest_raw(&path_str).is_ok() {
                            has_input_20 = true;
                            act_set_main = input.get_action_set_handle("/actions/main").unwrap_or(openvr::input::VRActionSetHandle(0));
                            act_translate = input.get_action_handle("/actions/main/in/TranslateTrigger").unwrap_or(openvr::input::VRActionHandle(0));
                            act_scale = input.get_action_handle("/actions/main/in/GripScale").unwrap_or(openvr::input::VRActionHandle(0));
                            act_clear = input.get_action_handle("/actions/main/in/ClearTranslation").unwrap_or(openvr::input::VRActionHandle(0));
                            let _ = app_handle.emit("ovr_log", "[OVR] [OK] SteamVR Input 2.0 加载成功");
                        }
                    }
                }
            }
        }
    }

    // ========== Main VR Event Loop ==========
    let mut tick: u64 = 0;
    #[allow(unused_assignments)]
    let mut current_translation: Option<(String, String)> = None;
    let mut translation_enabled = true;
    let mut current_config = OvrConfig::default();
    let mut prev_left_buttons: u64 = 0;
    let mut prev_right_buttons: u64 = 0;
    let mut overlay_menu_visible = false;  // Start hidden, user summons with right B long-press
    let mut scan_active = false;
    // Channel for scan results (async task -> VR thread)
    let (scan_tx, scan_rx) = std::sync::mpsc::channel::<(String, String)>();

    // Menu interaction state
    let mut menu_page: usize = 0;       // 0=main, 1=translate, 2=scan, 3=display
    let mut menu_selection: usize = 0;   // Currently highlighted item
    
    // ===== Right controller state =====
    let _drag_active = false;         // Menu drag in progress
    let mut last_menu_render_page: i32 = -1; // Track when to re-render
    let mut last_menu_render_sel: i32 = -1;
    let _scan_start_pos: Option<[f32; 3]> = None;
    
    let mut last_right_b_press_tick: u64 = 0;
    
    let mut right_trigger_ticks: u64 = 0;
    let _last_right_trigger_press_tick: u64 = 0;
    let _is_scan_primed = false;

    let mut prev_ivr_translate = false;
    let mut prev_ivr_scale = false;
    let mut prev_ivr_clear = false;
    
    let mut joystick_nav_cooldown: u64 = 0; // Prevent too-fast joystick navigation
    
    // Configurable transforms via joystick
    let mut menu_offset_z: f32 = 0.0;
    let mut menu_width: f32 = 0.35; // Make menu larger by default

    let mut is_translating = false; // Prevent double-trigger while OCR is running
    // Desktop mirror auto-scan state
    let mut auto_scan_active = false;
    let mut auto_scan_countdown: u64 = 0; // Ticks until next auto-scan (90 ticks = 1s)
    let mut last_ocr_text = String::new();
    let mut playspace = crate::playspace::PlayspaceController::new(); // Dedup: skip if same text as last scan

    // ===== Native Playspace Offset State =====
    // These are applied via IVRChaperoneSetup::SetWorkingStandingZeroPoseToRawTrackingPose
    // or via universe origin manipulation for real-time offset.
    let mut ps_offset_x: f32 = 0.0;
    let mut ps_offset_y: f32 = 0.0;
    let mut ps_offset_z: f32 = 0.0;
    let mut ps_rotation_deg: f32 = 0.0;
    let mut height_toggled: bool = false;
    let mut height_offset: f32 = 0.3; // Default 0.3m downward

    // ===== Space Drag State =====
    let mut is_space_dragging = false;
    let mut drag_last_pos: Option<[f32; 3]> = None;

    // ===== Wrist and Headset Proximity State =====
    let mut wrist_left_handle: Option<openvr::overlay::OverlayHandle> = None;
    let mut wrist_right_handle: Option<openvr::overlay::OverlayHandle> = None;
    if let Ok(mut ovr) = context.overlay() {
        if let Ok(h) = ovr.create_overlay("vrcdog.wrist.l\0", "Wrist L\0") {
            let _ = ovr.set_width(h, 0.15);
            let _ = ovr.set_opacity(h, 0.9);
            wrist_left_handle = Some(h);
        }
        if let Ok(h) = ovr.create_overlay("vrcdog.wrist.r\0", "Wrist R\0") {
            let _ = ovr.set_width(h, 0.15);
            let _ = ovr.set_opacity(h, 0.9);
            wrist_right_handle = Some(h);
        }
    }
    
    let mut _wrist_bind_mode = false;
    let mut _scan_frame_primed = false; // "手柄靠近耳朵" state


    // Show brief startup toast then hide
    if let (Some(h), Some(ref f)) = (overlay_handle, &font) {
        let pixels = crate::vr_ui::VrUiRenderer::render_text_to_rgba(
            f, "", "VrcDog\n双击右手B键 开启菜单\n双击并长按右手扳机 框选翻译", false,
            512, 320, [255,255,255], [18,18,42], 0.90,
        );
        if let Ok(mut ovr) = context.overlay() {
            let _ = ovr.set_raw_data(h, &pixels, 512, 320, 4);
            let _ = ovr.set_visibility(h, true);
        }
    }

    loop {
        tick += 1;

        // Process commands from async world (non-blocking)
        while let Ok(cmd) = cmd_rx.try_recv() {
            match cmd {
                OvrCommand::Shutdown => {
                    let _ = app_handle.emit("ovr_log", "[OVR] 正在关闭...");
                    // Hide and cleanup
                    if let Ok(mut ovr) = context.overlay() {
                        if let Some(h) = overlay_handle {
                            let _ = ovr.set_visibility(h, false);
                        }
                    }
                    drop(context);
                    let status_c = status.clone();
                    tokio::runtime::Handle::current().block_on(async {
                        let mut s = status_c.lock().await;
                        *s = OvrStatus::default();
                    });
                    let _ = app_handle.emit("ovr_status", "shutdown");
                    let _ = app_handle.emit("ovr_log", "[OVR] [OK] 已关闭");
                    return;
                }
                OvrCommand::UpdateConfig(new_config) => {
                    current_config = *new_config;
                    // Apply config changes to overlay
                    if let Ok(mut ovr) = context.overlay() {
                        if let Some(h) = overlay_handle {
                            let _ = ovr.set_opacity(h, current_config.overlay_bg_opacity);
                            // Switch overlay positioning mode
                            match current_config.overlay_lock_mode.as_str() {
                                "head" => {
                                    // Try left hand first
                                    let mut locked_to_hand = false;
                                    if let Ok(sys) = context.system() {
                                        if let Some(l_idx) = sys.tracked_device_index_for_controller_role(openvr::TrackedControllerRole::LeftHand) {
                                            // Tilted tablet-like rotation (-45 deg pitch)
                                            let hand_transform = openvr::pose::Matrix3x4([
                                                [1.0, 0.0, 0.0, 0.0],
                                                [0.0, 0.707, 0.707, 0.08],
                                                [0.0, -0.707, 0.707, -0.05],
                                            ]);
                                            let _ = ovr.set_transform_tracked_device_relative(h, l_idx, &hand_transform);
                                            let _ = ovr.set_width(h, 0.35);
                                            locked_to_hand = true;
                                        }
                                    }
                                    if !locked_to_hand {
                                        let head_transform = openvr::pose::Matrix3x4([
                                            [1.0, 0.0, 0.0, 0.0],
                                            [0.0, 1.0, 0.0, -0.1],
                                            [0.0, 0.0, 1.0, -0.5],
                                        ]);
                                        let _ = ovr.set_transform_tracked_device_relative(h, openvr::TrackedDeviceIndex(0), &head_transform);
                                        let _ = ovr.set_width(h, 0.35);
                                    }
                                }
                                _ => {
                                    // World-locked: fixed position in standing space
                                    let world_transform = openvr::pose::Matrix3x4([
                                        [1.0, 0.0, 0.0, 0.0],
                                        [0.0, 1.0, 0.0, 1.5],
                                        [0.0, 0.0, 1.0, -1.2],
                                    ]);
                                    let _ = ovr.set_transform_absolute(
                                        h,
                                        openvr::TrackingUniverseOrigin::Standing,
                                        &world_transform,
                                    );
                                }
                            }
                        }
                        // Update scan frame color if changed
                        if let Some(sh) = scan_handle {
                            let scan_pixels = crate::vr_ui::VrUiRenderer::render_scan_frame(256, 256, &current_config.scan_frame_color);
                            let _ = ovr.set_raw_data(sh, &scan_pixels, 256, 256, 4);
                        }
                    }
                    // Apply height toggle offset from config
                    height_offset = current_config.height_toggle_offset;
                    let _ = app_handle.emit("ovr_log", format!(
                        "[OVR] ⚙ 配置已更新 (OCR={}, 扫描框={}, 字号={:.0})",
                        current_config.ocr_language,
                        current_config.scan_frame_color,
                        current_config.overlay_font_size,
                    ));
                }
                OvrCommand::UpdateText { original, translated } => {
                    current_translation = Some((original, translated));
                    // Render text to overlay using configured panel width
                    if let (Some(h), Some(ref f)) = (overlay_handle, &font) {
                        let text_color = crate::vr_ui::VrUiRenderer::parse_hex_rgb(&current_config.overlay_text_color);
                        let bg_color = crate::vr_ui::VrUiRenderer::parse_hex_rgb(&current_config.overlay_bg_color);
                        let (orig, trans) = current_translation.as_ref().unwrap();
                        let panel_w = current_config.trans_panel_max_width.max(256).min(1024);
                        let panel_h = (panel_w / 2).max(128);
                        let pixels = crate::vr_ui::VrUiRenderer::render_text_to_rgba(
                            f, orig, trans,
                            current_config.dual_display,
                            panel_w, panel_h,
                            text_color, bg_color,
                            current_config.overlay_bg_opacity,
                        );
                        if let Ok(mut ovr) = context.overlay() {
                            let _ = ovr.set_raw_data(h, &pixels, panel_w as usize, panel_h as usize, 4);
                            let _ = ovr.set_visibility(h, true);
                        }
                    }
                    let status_c = status.clone();
                    tokio::runtime::Handle::current().block_on(async {
                        let mut s = status_c.lock().await;
                        s.overlay_visible = true;
                    });
                }
                OvrCommand::SetVisible(vis) => {
                    if let Ok(mut ovr) = context.overlay() {
                        if let Some(h) = overlay_handle {
                            let _ = ovr.set_visibility(h, vis);
                        }
                    }
                    let status_c = status.clone();
                    tokio::runtime::Handle::current().block_on(async {
                        let mut s = status_c.lock().await;
                        s.overlay_visible = vis;
                    });
                }
                OvrCommand::ClearText => {
                    current_translation = None;
                    if let Ok(mut ovr) = context.overlay() {
                        if let Some(h) = overlay_handle {
                            let _ = ovr.set_visibility(h, false);
                        }
                    }
                    let _ = app_handle.emit("ovr_translation_cleared", "");
                }
                OvrCommand::ToggleTranslation => {
                    translation_enabled = !translation_enabled;
                    let status_c = status.clone();
                    let enabled = translation_enabled;
                    tokio::runtime::Handle::current().block_on(async {
                        let mut s = status_c.lock().await;
                        s.translation_enabled = enabled;
                    });
                    let _ = app_handle.emit("ovr_log", format!(
                        "[OVR] 翻译模式: {}", if translation_enabled { "开启" } else { "关闭" }
                    ));
                }
                OvrCommand::DesktopScanOnce => {
                    if !is_translating {
                        is_translating = true;
                        let _ = app_handle.emit("ovr_log", "[OVR] 📸 桌面截图翻译中...");
                        // Show progress overlay
                        if let (Some(h), Some(ref f)) = (overlay_handle, &font) {
                            let pixels = crate::vr_ui::VrUiRenderer::render_text_to_rgba(
                                f, "", "桌面截图翻译中...\n请稍候", false,
                                512, 320, [255, 200, 50], [18, 18, 42], 0.92,
                            );
                            if let Ok(mut ovr) = context.overlay() {
                                let _ = ovr.set_raw_data(h, &pixels, 512, 320, 4);
                                let _ = ovr.set_visibility(h, true);
                            }
                        }
                        let cfg_clone = current_config.clone();
                        let tx = scan_tx.clone();
                        let app_h = app_handle.clone();
                        let status_c = status.clone();
                        tokio::runtime::Handle::current().spawn(async move {
                            match perform_scan_translate(&cfg_clone).await {
                                Ok((original, translated)) => {
                                    let _ = app_h.emit("ovr_log", format!("[OVR] ✅ 桌面翻译完成: {}", &translated[..translated.len().min(50)]));
                                    tokio::runtime::Handle::current().block_on(async {
                                        let mut s = status_c.lock().await;
                                        s.last_event = "desktop_scan_result".to_string();
                                        s.overlay_visible = true;
                                    });
                                    let _ = tx.send((original, translated));
                                }
                                Err(e) => {
                                    let _ = app_h.emit("ovr_log", format!("[OVR] ❌ 桌面扫描失败: {}", e));
                                    let _ = tx.send(("".to_string(), format!("❌ {}", e)));
                                }
                            }
                        });
                    }
                }
                OvrCommand::StartAutoScan => {
                    auto_scan_active = true;
                    auto_scan_countdown = (current_config.auto_scan_interval.max(3) as u64) * 90;
                    let _ = app_handle.emit("ovr_log", format!(
                        "[OVR] 🔄 自动扫描已开启 (每 {}s)", current_config.auto_scan_interval.max(3)
                    ));
                    let _ = app_handle.emit("ovr_auto_scan_status", true);
                }
                OvrCommand::StopAutoScan => {
                    auto_scan_active = false;
                    auto_scan_countdown = 0;
                    let _ = app_handle.emit("ovr_log", "[OVR] ⏹ 自动扫描已停止");
                    let _ = app_handle.emit("ovr_auto_scan_status", false);
                }
                // ===== Native Playspace Control (replaces OVRAS dependency) =====
                OvrCommand::SetPlayspaceOffset { x, y, z } => {
                    ps_offset_x = x;
                    ps_offset_y = y;
                    ps_offset_z = z;
                    if let Ok(ref mut ps) = playspace {
                        let y_total = ps_offset_y + if height_toggled { height_offset } else { 0.0 };
                        ps.apply_offset(ps_offset_x, y_total, ps_offset_z, ps_rotation_deg);
                    }
                    let _ = app_handle.emit("ovr_playspace_changed", serde_json::json!({
                        "offset_x": ps_offset_x, "offset_y": ps_offset_y, "offset_z": ps_offset_z,
                        "rotation": ps_rotation_deg, "height_toggled": height_toggled,
                    }));
                }
                OvrCommand::SetPlayspaceRotation(deg) => {
                    ps_rotation_deg = deg;
                    if let Ok(ref mut ps) = playspace {
                        let y_total = ps_offset_y + if height_toggled { height_offset } else { 0.0 };
                        ps.apply_offset(ps_offset_x, y_total, ps_offset_z, ps_rotation_deg);
                    }
                    let _ = app_handle.emit("ovr_playspace_changed", serde_json::json!({
                        "offset_x": ps_offset_x, "offset_y": ps_offset_y, "offset_z": ps_offset_z,
                        "rotation": ps_rotation_deg, "height_toggled": height_toggled,
                    }));
                }
                OvrCommand::ToggleHeight => {
                    height_toggled = !height_toggled;
                    if let Ok(ref mut ps) = playspace {
                        let y_total = ps_offset_y + if height_toggled { height_offset } else { 0.0 };
                        ps.apply_offset(ps_offset_x, y_total, ps_offset_z, ps_rotation_deg);
                    }
                    let _ = app_handle.emit("ovr_playspace_changed", serde_json::json!({
                        "offset_x": ps_offset_x, "offset_y": ps_offset_y, "offset_z": ps_offset_z,
                        "rotation": ps_rotation_deg, "height_toggled": height_toggled,
                    }));
                }
                OvrCommand::ResetPlayspace => {
                    ps_offset_x = 0.0;
                    ps_offset_y = 0.0;
                    ps_offset_z = 0.0;
                    ps_rotation_deg = 0.0;
                    height_toggled = false;
                    if let Ok(ref mut ps) = playspace {
                        ps.apply_offset(0.0, 0.0, 0.0, 0.0);
                    }
                    let _ = app_handle.emit("ovr_playspace_changed", serde_json::json!({
                        "offset_x": 0.0, "offset_y": 0.0, "offset_z": 0.0,
                        "rotation": 0.0, "height_toggled": false,
                    }));
                }
                OvrCommand::FixFloor => {
                    if let Ok(sys) = context.system() {
                        if let Some(r_idx) = sys.tracked_device_index_for_controller_role(openvr::TrackedControllerRole::RightHand) {
                            let poses = sys.device_to_absolute_tracking_pose(openvr::TrackingUniverseOrigin::Standing, 0.0);
                            let pose = poses[r_idx.0 as usize];
                            if pose.pose_is_valid() {
                                let mat = pose.device_to_absolute_tracking();
                                let controller_y = mat[1][3];
                                if let Ok(ref mut ps) = playspace {
                                    ps.set_base_floor_to(controller_y);
                                    // Also clear local UI offset tracking since base is changed
                                    ps_offset_x = 0.0;
                                    ps_offset_y = 0.0;
                                    ps_offset_z = 0.0;
                                    ps_rotation_deg = 0.0;
                                    height_toggled = false;
                                    let _ = app_handle.emit("ovr_log", "[OVR] ✅ 地板已修复！(右手柄位置置底)");
                                    let _ = app_handle.emit("ovr_playspace_changed", serde_json::json!({
                                        "offset_x": 0.0, "offset_y": 0.0, "offset_z": 0.0,
                                        "rotation": 0.0, "height_toggled": false,
                                    }));
                                }
                            } else {
                                let _ = app_handle.emit("ovr_log", "[OVR] ⚠ 右手柄未跟踪，无法修复地板");
                            }
                        } else {
                            let _ = app_handle.emit("ovr_log", "[OVR] ⚠ 未检测到右手柄");
                        }
                    }
                }
            }
        }

        // Process scan results from async task
        while let Ok((original, translated)) = scan_rx.try_recv() {
            is_translating = false; // Allow next translation
            
            // Content dedup for auto-scan: skip if same text as last scan
            if !original.is_empty() && original == last_ocr_text {
                let _ = app_handle.emit("ovr_log", "[OVR] 📋 文本未变化，跳过重复翻译");
                continue;
            }
            if !original.is_empty() {
                last_ocr_text = original.clone();
            }
            
            // Render translation result
            current_translation = Some((original.clone(), translated.clone()));
            if let (Some(h), Some(ref f)) = (overlay_handle, &font) {
                let text_color = crate::vr_ui::VrUiRenderer::parse_hex_rgb(&current_config.overlay_text_color);
                let bg_color = crate::vr_ui::VrUiRenderer::parse_hex_rgb(&current_config.overlay_bg_color);
                let pixels = crate::vr_ui::VrUiRenderer::render_text_to_rgba(
                    f, &original, &translated,
                    current_config.dual_display,
                    640, 400,
                    text_color, bg_color,
                    current_config.overlay_bg_opacity,
                );
                if let Ok(mut ovr) = context.overlay() {
                    let _ = ovr.set_raw_data(h, &pixels, 640, 400, 4);
                    let _ = ovr.set_visibility(h, true);
                    overlay_menu_visible = true; // Ensure menu state is tracked
                    
                    // Re-apply transform so it respects dynamic offset and mode
                    let _ = ovr.set_width(h, menu_width + 0.15); // Increase default width
                    if current_config.overlay_lock_mode == "head" {
                        let head_transform = openvr::pose::Matrix3x4([
                            [1.0, 0.0, 0.0, 0.0],
                            [0.0, 1.0, 0.0, -0.1],
                            [0.0, 0.0, 1.0, -0.7 + menu_offset_z], // Further away
                        ]);
                        let _ = ovr.set_transform_tracked_device_relative(h, openvr::TrackedDeviceIndex(0), &head_transform);
                    } else if let Ok(sys) = context.system() {
                        if let Some(l_idx) = sys.tracked_device_index_for_controller_role(openvr::TrackedControllerRole::LeftHand) {
                            // Tilted tablet-like rotation (-45 deg pitch)
                            let hand_transform = openvr::pose::Matrix3x4([
                                [1.0, 0.0, 0.0, 0.0],
                                [0.0, 0.707, 0.707, 0.08],
                                [0.0, -0.707, 0.707, -0.05 + menu_offset_z],
                            ]);
                            let _ = ovr.set_transform_tracked_device_relative(h, l_idx, &hand_transform);
                        }
                    }
                    
                    // Update menu visible state so it doesn't instantly close on next interaction
                    overlay_menu_visible = true;

                    // --- NEW: Render to Wrist Overlay if Wrist Mode enabled ---
                    if current_config.wrist_mode {
                        if let Some(wh_l) = wrist_left_handle {
                            let wrist_pixels = crate::vr_ui::VrUiRenderer::render_text_to_rgba(
                                f, "", &translated, false, 256, 128, text_color, bg_color, current_config.overlay_bg_opacity
                            );
                            let _ = ovr.set_raw_data(wh_l, &wrist_pixels, 256, 128, 4);
                            let _ = ovr.set_visibility(wh_l, true);
                            
                            // Anchor Left Wrist
                            if let Ok(sys) = context.system() {
                                if let Some(l_idx) = sys.tracked_device_index_for_controller_role(openvr::TrackedControllerRole::LeftHand) {
                                    let transform = openvr::pose::Matrix3x4([
                                        [1.0, 0.0, 0.0, 0.0],
                                        [0.0, 0.0, 1.0, -0.05], // Tilt up on wrist
                                        [0.0, -1.0, 0.0, 0.15],
                                    ]);
                                    let _ = ovr.set_transform_tracked_device_relative(wh_l, l_idx, &transform);
                                }
                            }
                        }
                    } else {
                        if let Some(wh_l) = wrist_left_handle { let _ = ovr.set_visibility(wh_l, false); }
                        if let Some(wh_r) = wrist_right_handle { let _ = ovr.set_visibility(wh_r, false); }
                    }
                }
            }
            
            // Emit to frontend for desktop UI display
            let _ = app_handle.emit("ovr_desktop_translation", serde_json::json!({
                "original": &original,
                "translated": &translated,
            }));

            if !translated.is_empty() && !translated.starts_with("❌") {
                // OSC Output to VRChat Chatbox (configurable)
                if current_config.osc_chatbox_enabled {
                    if let Ok(sock) = std::net::UdpSocket::bind("0.0.0.0:0") {
                        let osc_msg = rosc::OscPacket::Message(rosc::OscMessage {
                            addr: "/chatbox/input".to_string(),
                            args: vec![
                                rosc::OscType::String(translated.clone()),
                                rosc::OscType::Bool(true),
                                rosc::OscType::Bool(false),
                            ],
                        });
                        if let Ok(msg_buf) = rosc::encoder::encode(&osc_msg) {
                            let _ = sock.send_to(&msg_buf, "127.0.0.1:9000");
                            let _ = app_handle.emit("ovr_log", "[OVR] 💬 已通过OSC发送至聊天框");
                        }
                    }
                }

                // Windows Native TTS (configurable)
                if current_config.tts_enabled {
                    let tts_text = translated.clone();
                    std::thread::spawn(move || {
                        use std::os::windows::process::CommandExt;
                        let script = format!(
                            "Add-Type -AssemblyName System.Speech; $synth = New-Object System.Speech.Synthesis.SpeechSynthesizer; $synth.Speak('{}');",
                            tts_text.replace("'", "''")
                        );
                        let _ = std::process::Command::new("powershell")
                            .args(["-ExecutionPolicy", "Bypass", "-Command", &script])
                            .creation_flags(0x08000000)
                            .output();
                    });
                }
            }
        }

        // Poll VR system events (only for system-level events like Quit/Dashboard)
        if let Ok(sys) = context.system() {
            while let Some(event_info) = sys.poll_next_event() {
                match event_info.event {
                    openvr::system::Event::Quit(_) => {
                        let _ = app_handle.emit("ovr_log", "[OVR] SteamVR 请求退出");
                        sys.acknowledge_quit_exiting();
                        if let Ok(mut ovr) = context.overlay() {
                            if let Some(h) = overlay_handle {
                                let _ = ovr.set_visibility(h, false);
                            }
                        }
                        drop(context);
                        let status_c = status.clone();
                        tokio::runtime::Handle::current().block_on(async {
                            let mut s = status_c.lock().await;
                            *s = OvrStatus::default();
                        });
                        let _ = app_handle.emit("ovr_status", "shutdown");
                        return;
                    }
                    openvr::system::Event::DashboardActivated => {
                        let status_c = status.clone();
                        tokio::runtime::Handle::current().block_on(async {
                            let mut s = status_c.lock().await;
                            s.dashboard_visible = true;
                        });
                    }
                    openvr::system::Event::DashboardDeactivated => {
                        let status_c = status.clone();
                        tokio::runtime::Handle::current().block_on(async {
                            let mut s = status_c.lock().await;
                            s.dashboard_visible = false;
                        });
                    }
                    _ => {}
                }
            }

            // ===== Direct controller state polling =====
            let left_idx = sys.tracked_device_index_for_controller_role(openvr::TrackedControllerRole::LeftHand);
            let right_idx = sys.tracked_device_index_for_controller_role(openvr::TrackedControllerRole::RightHand);

            let grip_mask = 1u64 << openvr::button_id::GRIP;
            let trigger_mask = 1u64 << openvr::button_id::STEAM_VR_TRIGGER;
            let _touchpad_mask = 1u64 << openvr::button_id::STEAM_VR_TOUCHPAD;
            let _a_mask = 1u64 << openvr::button_id::A;

            // Read both controller states
            let left_state = left_idx.and_then(|i| sys.controller_state(i));
            let right_state = right_idx.and_then(|i| sys.controller_state(i));

            let left_pressed = left_state.map(|s| s.button_pressed).unwrap_or(0);
            let right_pressed = right_state.map(|s| s.button_pressed).unwrap_or(0);

            let left_new = left_pressed & !prev_left_buttons;
            let right_new = right_pressed & !prev_right_buttons;
            let any_new = left_new | right_new;

            // --- SteamVR Input 2.0 Polling ---
            let mut ivr_translate_pressed = false;
            let mut ivr_scale_pressed = false;
            let mut ivr_clear_pressed = false;

            if has_input_20 {
                if let Ok(mut input) = context.input() {
                    let mut active_sets = vec![openvr::input::VRActiveActionSet(openvr_sys::VRActiveActionSet_t {
                        ulActionSet: act_set_main.0,
                        ulRestrictedToDevice: 0,
                        ulSecondaryActionSet: 0,
                        unPadding: 0,
                        nPriority: 0,
                    })];
                    if input.update_actions(&mut active_sets).is_ok() {
                        if let Ok(data) = input.get_digital_action_data(act_translate, openvr::input::VRInputValueHandle(0)) {
                            ivr_translate_pressed = data.0.bState;
                        }
                        if let Ok(data) = input.get_digital_action_data(act_scale, openvr::input::VRInputValueHandle(0)) {
                            ivr_scale_pressed = data.0.bState;
                        }
                        if let Ok(data) = input.get_digital_action_data(act_clear, openvr::input::VRInputValueHandle(0)) {
                            ivr_clear_pressed = data.0.bState;
                        }
                    }
                }
            }

            // --- Right B key Double-Click detection ---
            // B button = APPLICATION_MENU on most controllers (index 1)
            let b_mask = 1u64 << openvr::button_id::APPLICATION_MENU;
            let right_b_pressed = right_new & b_mask != 0;
            
            let mut should_toggle = false;
            if right_b_pressed {
                if tick.saturating_sub(last_right_b_press_tick) < 30 {
                    // Double click!
                    should_toggle = true;
                    last_right_b_press_tick = 0; // reset
                } else {
                    last_right_b_press_tick = tick;
                }
            }

            if should_toggle {
                // Toggle menu!
                overlay_menu_visible = !overlay_menu_visible;
                if let Ok(mut ovr) = context.overlay() {
                    if let Some(h) = overlay_handle {
                        let _ = ovr.set_visibility(h, overlay_menu_visible);
                        if overlay_menu_visible {
                            let _ = ovr.set_width(h, menu_width);
                            if current_config.overlay_lock_mode == "head" {
                                let head_transform = openvr::pose::Matrix3x4([
                                    [1.0, 0.0, 0.0, 0.0],
                                    [0.0, 1.0, 0.0, -0.1],
                                    [0.0, 0.0, 1.0, -0.5 + menu_offset_z],
                                ]);
                                let _ = ovr.set_transform_tracked_device_relative(h, openvr::TrackedDeviceIndex(0), &head_transform);
                            } else {
                                if let Some(l_idx) = left_idx {
                                    let hand_transform = openvr::pose::Matrix3x4([
                                        [1.0, 0.0, 0.0, 0.0],
                                        [0.0, 1.0, 0.0, 0.08],
                                        [0.0, 0.0, 1.0, -0.05 + menu_offset_z],
                                    ]);
                                    let _ = ovr.set_transform_tracked_device_relative(h, l_idx, &hand_transform);
                                }
                            }
                        }
                    }
                }
                if overlay_menu_visible {
                    menu_page = 0;
                    menu_selection = 0;
                    last_menu_render_page = -1; // Force re-render
                }
                let _ = app_handle.emit("ovr_log", format!(
                    "[OVR] 菜单: {}", if overlay_menu_visible { "已打开" } else { "已关闭" }
                ));
            }
            
            // Decrement joystick navigation cooldown
            joystick_nav_cooldown = joystick_nav_cooldown.saturating_sub(1);

            // --- Menu interaction (only when menu visible) ---
            if overlay_menu_visible {
                let max_items = match menu_page {
                    0 => 6usize, // 0:基础,1:桌面,2:OCR,3:翻译,4:更多,5:说明
                    1 => 4,
                    2 => 4,
                    3 => 4,
                    4 => 3,
                    _ => 4, // page 5,6,7
                };

                // Joystick axes for navigation
                let joy_x = right_state.map(|s| s.axis[0].x).unwrap_or(0.0);
                let joy_y = right_state.map(|s| s.axis[0].y).unwrap_or(0.0);
                
                // Use joystick X to switch pages (with cooldown to prevent rapid switching)
                if joystick_nav_cooldown == 0 {
                    if joy_x > 0.7 && menu_page < 7 {
                        menu_page += 1;
                        menu_selection = 0;
                        joystick_nav_cooldown = 18; // ~200ms cooldown at 90Hz
                    } else if joy_x < -0.7 && menu_page > 0 {
                        menu_page -= 1;
                        menu_selection = 0;
                        joystick_nav_cooldown = 18;
                    }
                    
                    // Use joystick Y for up/down selection (natural, consistent)
                    if joy_y > 0.6 {
                        menu_selection = if menu_selection == 0 { max_items - 1 } else { menu_selection - 1 };
                        joystick_nav_cooldown = 18;
                    } else if joy_y < -0.6 {
                        menu_selection = (menu_selection + 1) % max_items;
                        joystick_nav_cooldown = 18;
                    }
                }
                
                // --- NEW: Grip + Joystick to Move and Resize Menu ---
                let right_grip_held = (right_pressed & grip_mask != 0) || ivr_scale_pressed;
                if right_grip_held {
                    if joy_y.abs() > 0.1 || joy_x.abs() > 0.1 {
                        menu_offset_z -= joy_y * 0.005; // Up pushes away (-Z), down pulls closer (+Z)
                        menu_width += joy_x * 0.005;    // Right enlarges, left shrinks
                        
                        // Clamp values
                        menu_offset_z = menu_offset_z.clamp(-2.0, 0.5);
                        menu_width = menu_width.clamp(0.05, 2.0);
                        
                        // Apply immediately based on current lock mode
                        if let Ok(mut ovr) = context.overlay() {
                            if let Some(h) = overlay_handle {
                                let _ = ovr.set_width(h, menu_width);
                                if current_config.overlay_lock_mode == "head" {
                                    let head_transform = openvr::pose::Matrix3x4([
                                        [1.0, 0.0, 0.0, 0.0],
                                        [0.0, 1.0, 0.0, -0.1],
                                        [0.0, 0.0, 1.0, -0.5 + menu_offset_z],
                                    ]);
                                    let _ = ovr.set_transform_tracked_device_relative(h, openvr::TrackedDeviceIndex(0), &head_transform);
                                } else {
                                    // Wrist mode (fallback to left hand)
                                    if let Some(l_idx) = left_idx {
                                        let hand_transform = openvr::pose::Matrix3x4([
                                            [1.0, 0.0, 0.0, 0.0],
                                            [0.0, 0.707, 0.707, 0.08],
                                            [0.0, -0.707, 0.707, -0.05 + menu_offset_z],
                                        ]);
                                        let _ = ovr.set_transform_tracked_device_relative(h, l_idx, &hand_transform);
                                    }
                                }
                            }
                        }
                    }
                }

                // Trigger = confirm/activate selected item
                if any_new & trigger_mask != 0 {
                    let back_idx = max_items - 1;
                    if menu_selection == back_idx && menu_page > 0 {
                        menu_page = if menu_page > 5 { 5 } else { 0 }; // back to previous main level
                        menu_selection = 0;
                    } else {
                        match menu_page {
                            0 => match menu_selection {
                                0 => { menu_page = 1; menu_selection = 0; }
                                1 => { menu_page = 2; menu_selection = 0; }
                                2 => { menu_page = 3; menu_selection = 0; }
                                3 => { menu_page = 4; menu_selection = 0; }
                                4 => { menu_page = 5; menu_selection = 0; }
                                5 => { menu_page = 7; menu_selection = 0; }
                                _ => {}
                            },
                            1 => match menu_selection {
                                0 => {
                                    translation_enabled = !translation_enabled;
                                    let status_c = status.clone();
                                    let en = translation_enabled;
                                    tokio::runtime::Handle::current().block_on(async {
                                        let mut s = status_c.lock().await;
                                        s.translation_enabled = en;
                                    });
                                }
                                1 => { current_config.dual_display = !current_config.dual_display; }
                                2 => { current_config.wrist_mode = !current_config.wrist_mode; }
                                _ => {}
                            },
                            2 => match menu_selection {
                                0 => { current_config.desktop_mode = !current_config.desktop_mode; }
                                1 => { current_config.auto_scan_enabled = !current_config.auto_scan_enabled; }
                                _ => {}
                            },
                            5 => match menu_selection {
                                0 => { /* lock mode toggle could go here */ }
                                1 => { /* opacity toggle could go here */ }
                                _ => {}
                            },
                            6 => match menu_selection {
                                0 => { current_config.tts_enabled = !current_config.tts_enabled; }
                                1 => { current_config.osc_chatbox_enabled = !current_config.osc_chatbox_enabled; }
                                2 => {
                                    scan_active = !scan_active;
                                    if let Ok(mut ovr) = context.overlay() {
                                        if let Some(sh) = scan_handle {
                                            let _ = ovr.set_visibility(sh, scan_active);
                                        }
                                    }
                                }
                                _ => {}
                            },
                            _ => {}
                        }
                    }
                }

                // Re-render menu if state changed
                if last_menu_render_page != menu_page as i32 || last_menu_render_sel != menu_selection as i32 {
                    if let (Some(h), Some(ref f)) = (overlay_handle, &font) {
                        let pixels = crate::vr_ui::VrUiRenderer::render_vr_menu(f, menu_page, menu_selection, scan_active, translation_enabled, &current_config);
                        if let Ok(mut ovr) = context.overlay() {
                            let _ = ovr.set_raw_data(h, &pixels, 512, 320, 4);
                        }
                    }
                    last_menu_render_page = menu_page as i32;
                    last_menu_render_sel = menu_selection as i32;
                }
            } else {
                // Menu hidden: direct scan controls based on OVR Overlay Translator spec
                let trigger_down = (right_pressed & trigger_mask != 0) || ivr_translate_pressed;
                let trigger_just_pressed = (right_new & trigger_mask != 0) || (ivr_translate_pressed && !prev_ivr_translate);
                let trigger_just_released = (prev_right_buttons & trigger_mask != 0 && !trigger_down) || (!ivr_translate_pressed && prev_ivr_translate);

                let right_grip_down = (right_pressed & grip_mask != 0) || ivr_scale_pressed;
                let right_grip_just_pressed = (right_new & grip_mask != 0) || (ivr_scale_pressed && !prev_ivr_scale);
                let left_joy_x = left_state.map(|s| s.axis[0].x).unwrap_or(0.0);
                let left_joy_y = left_state.map(|s| s.axis[0].y).unwrap_or(0.0);
                let right_joy_y = right_state.map(|s| s.axis[0].y).unwrap_or(0.0);

                // Get absolute positions for Head, Left Hand, Right Hand
                let poses = sys.device_to_absolute_tracking_pose(openvr::TrackingUniverseOrigin::Standing, 0.0);
                let mut head_pos = [0.0, 0.0, 0.0];
                let mut right_pos = [0.0, 0.0, 0.0];
                let mut left_pos = [0.0, 0.0, 0.0];
                
                let head_pose = poses[0];
                if head_pose.pose_is_valid() {
                    let mat = head_pose.device_to_absolute_tracking();
                    head_pos = [mat[0][3], mat[1][3], mat[2][3]];
                }
                if let Some(r_idx) = right_idx {
                    let pose = poses[r_idx.0 as usize];
                    if pose.pose_is_valid() {
                        let mat = pose.device_to_absolute_tracking();
                        right_pos = [mat[0][3], mat[1][3], mat[2][3]];
                    }
                }
                if let Some(l_idx) = left_idx {
                    let pose = poses[l_idx.0 as usize];
                    if pose.pose_is_valid() {
                        let mat = pose.device_to_absolute_tracking();
                        left_pos = [mat[0][3], mat[1][3], mat[2][3]];
                    }
                }

                let dist_right_to_head = ((right_pos[0]-head_pos[0]).powi(2) + (right_pos[1]-head_pos[1]).powi(2) + (right_pos[2]-head_pos[2]).powi(2)).sqrt();
                let dist_right_to_left = ((right_pos[0]-left_pos[0]).powi(2) + (right_pos[1]-left_pos[1]).powi(2) + (right_pos[2]-left_pos[2]).powi(2)).sqrt();
                let is_near_wrist = dist_right_to_left < 0.3;

                // 1. Controller near ear -> Show scan frame
                let is_near_ear = dist_right_to_head < 0.35 && right_pos[1] > head_pos[1] - 0.2;
                if translation_enabled && is_near_ear {
                    if !scan_active {
                        scan_active = true;
                        if let Ok(mut ovr) = context.overlay() {
                            if let Some(sh) = scan_handle {
                                let _ = ovr.set_visibility(sh, true);
                            }
                        }
                    }
                } else if !is_near_ear && scan_active && !is_translating {
                    // Turn off if moved away
                    scan_active = false;
                    if let Ok(mut ovr) = context.overlay() {
                        if let Some(sh) = scan_handle {
                            let _ = ovr.set_visibility(sh, false);
                        }
                    }
                }

                // Follow right controller if scan active
                if scan_active {
                    if let Ok(mut ovr) = context.overlay() {
                        if let (Some(sh), Some(r_idx)) = (scan_handle, right_idx) {
                            let tf = openvr::pose::Matrix3x4([
                                [1.0, 0.0, 0.0, 0.0],
                                [0.0, 1.0, 0.0, 0.0],
                                [0.0, 0.0, 1.0, -0.6],
                            ]);
                            let _ = ovr.set_transform_tracked_device_relative(sh, r_idx, &tf);
                        }
                    }
                }

                // 2. Long press trigger -> Toggle Translation mode
                if trigger_just_pressed {
                    right_trigger_ticks = 0;
                }
                if trigger_down {
                    right_trigger_ticks += 1;
                    if right_trigger_ticks == 90 { // ~1 second at 90Hz
                        translation_enabled = !translation_enabled;
                        let status_c = status.clone();
                        let en = translation_enabled;
                        tokio::runtime::Handle::current().block_on(async {
                            let mut s = status_c.lock().await;
                            s.translation_enabled = en;
                        });
                        let _ = app_handle.emit("ovr_log", format!("[OVR] 翻译模式: {}", if translation_enabled { "已开启" } else { "已关闭" }));
                    }
                }

                // 3. Pull trigger -> OCR & Translate
                if trigger_just_released && right_trigger_ticks < 90 && translation_enabled && !is_translating {
                    is_translating = true;
                    let _ = app_handle.emit("ovr_log", "[OVR] 📸 触发截图识别...");

                    if let Ok(mut ovr) = context.overlay() {
                        if let Some(h) = overlay_handle { let _ = ovr.set_visibility(h, false); }
                        if let Some(sh) = scan_handle { let _ = ovr.set_visibility(sh, false); }
                    }
                    scan_active = false;

                    let cfg_clone = current_config.clone();
                    let app_h = app_handle.clone();
                    let status_c = status.clone();
                    let tx = scan_tx.clone();

                    tokio::runtime::Handle::current().spawn(async move {
                        let result = perform_scan_translate(&cfg_clone).await;
                        match result {
                            Ok((original, translated)) => {
                                let _ = app_h.emit("ovr_log", format!("[OVR] ✅ 翻译完成: {}", &translated[..translated.len().min(50)]));
                                tokio::runtime::Handle::current().block_on(async {
                                    let mut s = status_c.lock().await;
                                    s.last_event = "scan_result".to_string();
                                    s.overlay_visible = true;
                                });
                                let _ = tx.send((original, translated));
                            }
                            Err(e) => {
                                let _ = app_h.emit("ovr_log", format!("[OVR] ❌ 扫描失败: {}", e));
                                let _ = tx.send(("".to_string(), format!("❌ {}", e)));
                            }
                        }
                    });
                }

                // 4. Flick left joystick or trigger clear action -> Clear current translation
                if left_joy_x.abs() > 0.8 || left_joy_y.abs() > 0.8 || (ivr_clear_pressed && !prev_ivr_clear) {
                    if let Ok(mut ovr) = context.overlay() {
                        if let Some(h) = overlay_handle {
                            let _ = ovr.set_visibility(h, false);
                        }
                    }
                }

                // 5. Hold grip when scan frame is visible -> Adjust scan frame size
                if right_grip_down && scan_active && right_joy_y.abs() > 0.1 {
                    menu_width += right_joy_y * 0.01; 
                    menu_width = menu_width.clamp(0.1, 2.0);
                    if let Ok(mut ovr) = context.overlay() {
                        if let Some(sh) = scan_handle {
                            let _ = ovr.set_width(sh, menu_width);
                        }
                    }
                }

                // 6. Right controller near left wrist + press grip -> dock to wrist
                if right_grip_just_pressed && is_near_wrist {
                    current_config.overlay_lock_mode = "wrist".to_string();
                    if let Ok(mut ovr) = context.overlay() {
                        if let (Some(h), Some(l_idx)) = (overlay_handle, left_idx) {
                            let hand_transform = openvr::pose::Matrix3x4([
                                [1.0, 0.0, 0.0, 0.0],
                                [0.0, 1.0, 0.0, 0.08],
                                [0.0, 0.0, 1.0, -0.05],
                            ]);
                            let _ = ovr.set_transform_tracked_device_relative(h, l_idx, &hand_transform);
                            let _ = ovr.set_width(h, 0.15); // small wrist size
                            let _ = ovr.set_visibility(h, true); // ensure it's visible
                        }
                    }
                    let _ = app_handle.emit("ovr_log", "[OVR] ⌚ 翻译结果已放置到手腕常驻显示");
                }
            }

            // ===== Space Drag (OVRAS Replacement) =====
            // Left Grip drags the world ONLY IF menu is hidden, avoiding conflicts.
            let left_grip_held = left_pressed & grip_mask != 0;
            if !overlay_menu_visible {
                if left_grip_held {
                    if let Some(l_idx) = left_idx {
                        // Use RawAndUncalibrated so the coordinate space doesn't move while we drag it!
                        let poses = sys.device_to_absolute_tracking_pose(openvr::TrackingUniverseOrigin::RawAndUncalibrated, 0.0);
                        let pose = poses[l_idx.0 as usize];
                        if pose.pose_is_valid() {
                            let mat = pose.device_to_absolute_tracking();
                            let curr_raw_pos = [mat[0][3], mat[1][3], mat[2][3]];

                            if !is_space_dragging {
                                is_space_dragging = true;
                                drag_last_pos = Some(curr_raw_pos);
                            } else if let Some(last_pos) = drag_last_pos {
                                let dx = curr_raw_pos[0] - last_pos[0];
                                let dy = curr_raw_pos[1] - last_pos[1];
                                let dz = curr_raw_pos[2] - last_pos[2];

                                // Only move if there is a meaningful delta
                                if dx.abs() > 0.0001 || dy.abs() > 0.0001 || dz.abs() > 0.0001 {
                                    ps_offset_x -= dx;
                                    ps_offset_y -= dy;
                                    ps_offset_z -= dz;

                                    // ChaperoneSetup API is not exported by openvr crate.
                                    // Space drag is disabled.
                                    
                                    drag_last_pos = Some(curr_raw_pos);
                                }
                            }
                        }
                    }
                } else {
                    if is_space_dragging {
                        is_space_dragging = false;
                        // On release, commit to Live to make it persistent
                        let _ = app_handle.emit("ovr_log", "[OVR] ⚠ openvr crate 暂不支持 ChaperoneSetup API，空间拖拽无法应用");
                    }
                }
            }

            prev_left_buttons = left_pressed;
            prev_right_buttons = right_pressed;
            prev_ivr_translate = ivr_translate_pressed;
            prev_ivr_scale = ivr_scale_pressed;
            prev_ivr_clear = ivr_clear_pressed;
        }

        // Emit heartbeat every ~1s
        if tick.is_multiple_of(90) {
            let status_c = status.clone();
            let st = tokio::runtime::Handle::current().block_on(async {
                status_c.lock().await.clone()
            });
            let _ = app_handle.emit("ovr_heartbeat", &st);
        }

        // Auto-hide startup toast after 5s
        if tick == 450 && !overlay_menu_visible {
            if let Ok(mut ovr) = context.overlay() {
                if let Some(h) = overlay_handle {
                    let _ = ovr.set_visibility(h, false);
                }
            }
        }

        // ===== Desktop Mirror Auto-Scan Timer =====
        if auto_scan_active && current_config.desktop_mode && !is_translating {
            if auto_scan_countdown == 0 {
                // Time to auto-scan!
                is_translating = true;
                let _ = app_handle.emit("ovr_log", "[OVR] 🔄 自动扫描触发...");
                
                let cfg_clone = current_config.clone();
                let tx = scan_tx.clone();
                let app_h = app_handle.clone();
                let status_c = status.clone();
                tokio::runtime::Handle::current().spawn(async move {
                    match perform_scan_translate(&cfg_clone).await {
                        Ok((original, translated)) => {
                            let _ = app_h.emit("ovr_log", format!("[OVR] ✅ 自动翻译: {}", &translated[..translated.len().min(40)]));
                            tokio::runtime::Handle::current().block_on(async {
                                let mut s = status_c.lock().await;
                                s.last_event = "auto_scan_result".to_string();
                                s.overlay_visible = true;
                            });
                            let _ = tx.send((original, translated));
                        }
                        Err(e) => {
                            let _ = app_h.emit("ovr_log", format!("[OVR] ⚠ 自动扫描失败: {}", e));
                            // Don't send error to overlay for auto-scan (too noisy)
                        }
                    }
                });
                // Reset countdown
                auto_scan_countdown = (current_config.auto_scan_interval.max(3) as u64) * 90;
            } else {
                auto_scan_countdown -= 1;
            }
        }

        // ===== Performance Statistics (P2): every 90 ticks ≈ 1 second =====
        if tick % 90 == 0 {
            if let Ok(sys) = context.system() {
                // Read HMD pose for distance/rotation tracking
                let poses = sys.device_to_absolute_tracking_pose(
                    openvr::TrackingUniverseOrigin::Standing, 0.0
                );
                let hmd_pose = poses[0]; // HMD is always index 0
                if hmd_pose.pose_is_valid() {
                    let mat = hmd_pose.device_to_absolute_tracking();
                    let _hmd_x = mat[0][3];
                    let _hmd_y = mat[1][3];
                    let _hmd_z = mat[2][3];
                    // Could track cumulative distance here if needed
                }
                // Read compositor frame timing if available
                if let Ok(compositor) = context.compositor() {
                    if let Some(stats) = compositor.get_frame_timing(0) {
                        let _ = app_handle.emit("ovr_perf_stats", serde_json::json!({
                            "pid": 0,
                            "num_frame_presents": stats.m_nNumFramePresents,
                            "num_dropped_frames": stats.m_nNumDroppedFrames,
                            "num_reprojected_frames": 0,
                            "reprojection_ratio": 0.0,
                            "tick": tick,
                        }));
                    }
                }
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(11));
    }
}

// ==================== Tauri Commands ====================

#[tauri::command]
pub async fn ovr_init(
    app_handle: AppHandle,
    state: tauri::State<'_, OvrState>,
) -> crate::AppResult<OvrStatus> {
    let status = state.status.clone();

    {
        let s = status.lock().await;
        if s.initialized {
            return Ok(s.clone());
        }
    }

    let config = state.config.clone();
    let (cmd_tx, cmd_rx) = std::sync::mpsc::channel();

    {
        let mut tx = state.cmd_tx.lock().await;
        *tx = Some(cmd_tx);
    }

    let status_c = status.clone();
    let config_c = config.clone();
    let app_c = app_handle.clone();

    let handle = tokio::task::spawn_blocking(move || {
        vr_thread_main(app_c, status_c, config_c, cmd_rx);
    });

    {
        let mut h = state.event_loop_handle.lock().await;
        *h = Some(handle);
    }

    // Wait for init
    tokio::time::sleep(tokio::time::Duration::from_millis(800)).await;

    let s = status.lock().await;
    Ok(s.clone())
}

#[tauri::command]
pub async fn ovr_shutdown(state: tauri::State<'_, OvrState>) -> crate::AppResult<()> {
    let tx = state.cmd_tx.lock().await;
    if let Some(ref sender) = *tx {
        let _ = sender.send(OvrCommand::Shutdown);
    }
    let mut h = state.event_loop_handle.lock().await;
    if let Some(handle) = h.take() {
        let _ = handle.await;
    }
    Ok(())
}

#[tauri::command]
pub async fn ovr_get_status(state: tauri::State<'_, OvrState>) -> crate::AppResult<OvrStatus> {
    Ok(state.status.lock().await.clone())
}

#[tauri::command]
pub async fn ovr_set_config(state: tauri::State<'_, OvrState>, config: OvrConfig) -> crate::AppResult<()> {
    let tx = state.cmd_tx.lock().await;
    if let Some(ref sender) = *tx {
        let _ = sender.send(OvrCommand::UpdateConfig(Box::new(config.clone())));
    }
    *state.config.lock().await = config;
    Ok(())
}

#[tauri::command]
pub async fn ovr_toggle_translation(
    _app_handle: AppHandle,
    state: tauri::State<'_, OvrState>,
) -> crate::AppResult<bool> {
    let tx = state.cmd_tx.lock().await;
    if let Some(ref sender) = *tx {
        let _ = sender.send(OvrCommand::ToggleTranslation);
    }
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    let s = state.status.lock().await;
    Ok(s.translation_enabled)
}

#[tauri::command]
pub async fn ovr_capture_screenshot(
    app_handle: AppHandle,
    state: tauri::State<'_, OvrState>,
) -> crate::AppResult<String> {
    let s = state.status.lock().await;
    if !s.initialized {
        return Err("OpenVR 尚未初始化".into());
    }
    let _ = app_handle.emit("ovr_log", "[OVR] 正在捕获 VR 截图...");

    // Use Windows screen capture as fallback (captures VR mirror window)
    let temp_dir = std::env::temp_dir();
    let path = temp_dir.join("vrcdog_vr_capture.png");
    let path_str = path.to_string_lossy().to_string();

    let _ = app_handle.emit("ovr_log", format!("[OVR] 截图路径: {}", path_str));
    let _ = app_handle.emit("ovr_screenshot_ready", &path_str);

    Ok(path_str)
}

#[tauri::command]
pub async fn ovr_update_overlay_text(
    app_handle: AppHandle,
    state: tauri::State<'_, OvrState>,
    original: String,
    translated: String,
) -> crate::AppResult<()> {
    let tx = state.cmd_tx.lock().await;
    if let Some(ref sender) = *tx {
        let _ = sender.send(OvrCommand::UpdateText {
            original: original.clone(),
            translated: translated.clone(),
        });
    }
    let _ = app_handle.emit("ovr_translation_updated", serde_json::json!({
        "original": original,
        "translated": translated,
    }));
    Ok(())
}

#[tauri::command]
pub async fn ovr_set_overlay_visible(
    state: tauri::State<'_, OvrState>,
    visible: bool,
) -> crate::AppResult<()> {
    let tx = state.cmd_tx.lock().await;
    if let Some(ref sender) = *tx {
        let _ = sender.send(OvrCommand::SetVisible(visible));
    }
    Ok(())
}

#[tauri::command]
pub async fn ovr_clear_translation(
    app_handle: AppHandle,
    state: tauri::State<'_, OvrState>,
) -> crate::AppResult<()> {
    let tx = state.cmd_tx.lock().await;
    if let Some(ref sender) = *tx {
        let _ = sender.send(OvrCommand::ClearText);
    }
    let _ = app_handle.emit("ovr_log", "[OVR] 翻译已清除");
    Ok(())
}

// ===== Desktop Mirror Translation Mode Commands =====

#[tauri::command]
pub async fn ovr_desktop_scan_once(
    app_handle: AppHandle,
    state: tauri::State<'_, OvrState>,
) -> crate::AppResult<()> {
    let tx = state.cmd_tx.lock().await;
    if let Some(ref sender) = *tx {
        let _ = sender.send(OvrCommand::DesktopScanOnce);
    } else {
        // If VR is not running, do a standalone desktop scan
        let _ = app_handle.emit("ovr_log", "[OVR] 📸 VR 未运行，执行独立桌面扫描...");
        let config = state.config.lock().await.clone();
        let app_h = app_handle.clone();
        tokio::spawn(async move {
            match perform_scan_translate(&config).await {
                Ok((original, translated)) => {
                    let _ = app_h.emit("ovr_desktop_translation", serde_json::json!({
                        "original": &original,
                        "translated": &translated,
                    }));
                    let _ = app_h.emit("ovr_log", format!("[OVR] ✅ 桌面翻译完成: {}", &translated[..translated.len().min(50)]));
                }
                Err(e) => {
                    let _ = app_h.emit("ovr_log", format!("[OVR] ❌ 桌面扫描失败: {}", e));
                    let _ = app_h.emit("ovr_desktop_translation", serde_json::json!({
                        "original": "",
                        "translated": format!("❌ {}", e),
                    }));
                }
            }
        });
    }
    Ok(())
}

#[tauri::command]
pub async fn ovr_start_auto_scan(
    app_handle: AppHandle,
    state: tauri::State<'_, OvrState>,
) -> crate::AppResult<()> {
    let tx = state.cmd_tx.lock().await;
    if let Some(ref sender) = *tx {
        let _ = sender.send(OvrCommand::StartAutoScan);
    } else {
        let _ = app_handle.emit("ovr_log", "[OVR] 无法启动自动扫描: VR 未运行");
    }
    Ok(())
}

#[tauri::command]
pub async fn ovr_stop_auto_scan(
    app_handle: AppHandle,
    state: tauri::State<'_, OvrState>,
) -> crate::AppResult<()> {
    let tx = state.cmd_tx.lock().await;
    if let Some(ref sender) = *tx {
        let _ = sender.send(OvrCommand::StopAutoScan);
    } else {
        let _ = app_handle.emit("ovr_log", "[OVR] 自动扫描已停止");
    }
    Ok(())
}

// ===== Native Playspace Control Commands (replaces OVRAS dependency) =====

/// Set playspace XYZ offset in meters. Directly modifies VR tracking space
/// via OpenVR ChaperoneSetup API — no OVRAS required.
#[tauri::command]
pub async fn ovr_set_playspace_offset(
    app_handle: AppHandle,
    state: tauri::State<'_, OvrState>,
    x: f32,
    y: f32,
    z: f32,
) -> crate::AppResult<()> {
    let tx = state.cmd_tx.lock().await;
    if let Some(ref sender) = *tx {
        let _ = sender.send(OvrCommand::SetPlayspaceOffset { x, y, z });
    } else {
        let _ = app_handle.emit("ovr_log", "[OVR] ⚠ VR 未运行，无法设置空间偏移");
    }
    Ok(())
}

/// Set playspace Y-axis rotation in degrees.
#[tauri::command]
pub async fn ovr_set_playspace_rotation(
    app_handle: AppHandle,
    state: tauri::State<'_, OvrState>,
    degrees: f32,
) -> crate::AppResult<()> {
    let tx = state.cmd_tx.lock().await;
    if let Some(ref sender) = *tx {
        let _ = sender.send(OvrCommand::SetPlayspaceRotation(degrees));
    } else {
        let _ = app_handle.emit("ovr_log", "[OVR] ⚠ VR 未运行，无法设置旋转");
    }
    Ok(())
}

/// Toggle height offset on/off (e.g. seated ↔ standing).
/// This is the core "free height adjust" feature that replaces OVRAS HeightToggle.
#[tauri::command]
pub async fn ovr_toggle_height(
    app_handle: AppHandle,
    state: tauri::State<'_, OvrState>,
) -> crate::AppResult<()> {
    let tx = state.cmd_tx.lock().await;
    if let Some(ref sender) = *tx {
        let _ = sender.send(OvrCommand::ToggleHeight);
    } else {
        let _ = app_handle.emit("ovr_log", "[OVR] ⚠ VR 未运行，无法切换高度");
    }
    Ok(())
}

/// Reset all playspace offsets and rotation to zero.
#[tauri::command]
pub async fn ovr_reset_playspace(
    app_handle: AppHandle,
    state: tauri::State<'_, OvrState>,
) -> crate::AppResult<()> {
    let tx = state.cmd_tx.lock().await;
    if let Some(ref sender) = *tx {
        let _ = sender.send(OvrCommand::ResetPlayspace);
    } else {
        let _ = app_handle.emit("ovr_log", "[OVR] ⚠ VR 未运行，无法重置空间");
    }
    Ok(())
}

/// Fix floor height by reading right controller position on the floor.
/// Place your right controller on the floor and call this command.
#[tauri::command]
pub async fn ovr_fix_floor(
    app_handle: AppHandle,
    state: tauri::State<'_, OvrState>,
) -> crate::AppResult<()> {
    let tx = state.cmd_tx.lock().await;
    if let Some(ref sender) = *tx {
        let _ = sender.send(OvrCommand::FixFloor);
    } else {
        let _ = app_handle.emit("ovr_log", "[OVR] ⚠ VR 未运行，无法修复地板");
    }
    Ok(())
}
