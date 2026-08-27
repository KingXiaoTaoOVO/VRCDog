use ab_glyph::{Font, FontVec};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;

// ==================== Types ====================

#[derive(Debug, Clone, Serialize, Default)]
pub struct OvrStatus {
    pub initialized: bool,
    pub hmd_present: bool,
    pub hmd_model: String,
    pub overlay_visible: bool,
    #[serde(default)]
    pub menu_visible: bool,
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
    pub desktop_mode: bool, // Enable desktop mirror capture mode
    #[serde(default)]
    pub auto_scan_enabled: bool, // Auto-scan at interval
    #[serde(default = "default_auto_scan_interval")]
    pub auto_scan_interval: u32, // Auto-scan interval in seconds (3-60)
    #[serde(default = "default_true")]
    pub tts_enabled: bool, // TTS voice readback toggle
    #[serde(default = "default_true")]
    pub osc_chatbox_enabled: bool, // OSC chatbox output toggle
    // ===== Frontend Config Items (P1: now actually connected to VR backend) =====
    #[serde(default = "default_scan_color")]
    pub scan_frame_color: String, // Scan frame color hex e.g. "#00FF64"
    #[serde(default = "default_ocr_lang")]
    pub ocr_language: String, // OCR language: "ja","en-US","zh-Hans-CN","ko","fr","de"
    #[serde(default)]
    pub ocr_speed_mode: String, // "fast", "balanced", "accurate"
    #[serde(default)]
    pub ocr_image_enhance: bool, // Enable image preprocessing
    #[serde(default = "default_ocr_contrast")]
    pub ocr_contrast: f32, // OCR image contrast
    #[serde(default)]
    pub ocr_sharpen: bool, // OCR image sharpen
    #[serde(default)]
    pub ocr_denoise: bool, // OCR image denoise
    #[serde(default = "default_merge_x")]
    pub ocr_merge_tolerance_x: f32, // Horizontal merge tolerance
    #[serde(default = "default_merge_y")]
    pub ocr_merge_tolerance_y: f32, // Vertical merge tolerance
    #[serde(default)]
    pub auto_start_steamvr: bool, // Auto start with SteamVR
    #[serde(default = "default_panel_width")]
    pub trans_panel_max_width: u32, // Max width of translation panel in pixels
    #[serde(default = "default_font_size")]
    pub overlay_font_size: f32, // Font size for overlay text
    #[serde(default = "default_true")]
    pub overlay_glass: bool, // Transparent glass panel style
    #[serde(default = "default_border_opacity")]
    pub overlay_border_opacity: f32, // Glass border opacity
    #[serde(default = "default_corner_radius")]
    pub overlay_corner_radius: u32, // Rounded panel corner radius
    #[serde(default = "default_shadow_strength")]
    pub overlay_shadow_strength: f32, // Subtle panel shadow strength
    #[serde(default = "default_grip_threshold")]
    pub grip_pressure_threshold: f32, // Index controller grip pressure threshold (0.0-1.0)
    #[serde(default)]
    pub custom_api_url: String, // Custom LLM API endpoint URL
    // ===== Native Playspace Control (replaces OVRAS dependency) =====
    #[serde(default)]
    pub playspace_offset_x: f32, // X offset in meters
    #[serde(default)]
    pub playspace_offset_y: f32, // Y offset in meters (HEIGHT)
    #[serde(default)]
    pub playspace_offset_z: f32, // Z offset in meters
    #[serde(default)]
    pub playspace_rotation: f32, // Rotation in degrees (Y-axis)
    #[serde(default)]
    pub height_toggle_enabled: bool, // Whether height toggle is active
    #[serde(default = "default_height_offset")]
    pub height_toggle_offset: f32, // Height toggle offset in meters (positive = down)
    // ===== VRCDog Space Translation =====
    #[serde(default)]
    pub gravity_enabled: bool, // Gravity/slingshot feature
    #[serde(default = "default_gravity_strength")]
    pub gravity_strength: f32, // Gravity strength (0.0-1.0)
    #[serde(default = "default_fling_strength")]
    pub fling_strength: f32, // Fling momentum strength
    #[serde(default = "default_snap_turn_angle")]
    pub snap_turn_angle: i32, // Snap turn angle in degrees (15, 30, 45, 60)
    #[serde(default)]
    pub snap_turn_enabled: bool, // Enable snap turning
    #[serde(default = "default_smooth_turn_rate")]
    pub smooth_turn_rate: i32, // Smooth turn rate in degrees/sec
    #[serde(default)]
    pub smooth_turn_enabled: bool, // Enable smooth turning
    #[serde(default)]
    pub lock_x_enabled: bool, // Lock X axis (prevent left/right movement)
    #[serde(default)]
    pub lock_y_enabled: bool, // Lock Y axis (prevent up/down movement)
    #[serde(default)]
    pub lock_z_enabled: bool, // Lock Z axis (prevent forward/backward movement)
    #[serde(default = "default_drag_multiplier")]
    pub drag_multiplier: f32, // Space drag speed multiplier
    #[serde(default)]
    pub comfort_turn_enabled: bool, // Comfort mode for turning (fade to black)
    // ===== Dynamic Screenshot/Capture System =====
    #[serde(default)]
    pub capture_mode: String, // "static" | "dynamic" | "follow"
    #[serde(default = "default_capture_quality")]
    pub capture_quality: String, // "fast" | "balanced" | "high"
    #[serde(default)]
    pub capture_auto_save: bool, // Auto-save captures to disk
    #[serde(default = "default_capture_format")]
    pub capture_format: String, // "png" | "jpg" | "webp"
    // ===== VR Overlay Layout (runtime-adjustable inside VR) =====
    #[serde(default = "default_menu_width_m")]
    pub menu_width_m: f32,
    #[serde(default)]
    pub menu_offset_x: f32,
    #[serde(default = "default_menu_offset_y")]
    pub menu_offset_y: f32,
    #[serde(default = "default_menu_offset_z")]
    pub menu_offset_z: f32,
    #[serde(default = "default_result_width_m")]
    pub result_width_m: f32,
    #[serde(default)]
    pub result_offset_x: f32,
    #[serde(default = "default_result_offset_y")]
    pub result_offset_y: f32,
    #[serde(default = "default_result_offset_z")]
    pub result_offset_z: f32,
    #[serde(default = "default_scan_frame_width_m")]
    pub scan_frame_width_m: f32,
    #[serde(default = "default_scan_frame_distance_m")]
    pub scan_frame_distance_m: f32,
    // ===== Native VR Menu Theme (driven by the app's live theme) =====
    #[serde(default = "default_vr_menu_accent")]
    pub vr_menu_accent: String,
    #[serde(default = "default_vr_menu_bg")]
    pub vr_menu_bg: String,
    #[serde(default = "default_vr_menu_text")]
    pub vr_menu_text: String,
    #[serde(default = "default_vr_menu_text_muted")]
    pub vr_menu_text_muted: String,
}

fn default_auto_scan_interval() -> u32 {
    5
}
fn default_true() -> bool {
    true
}
fn default_height_offset() -> f32 {
    0.3
}
fn default_scan_color() -> String {
    "#00FF64".into()
}
fn default_ocr_lang() -> String {
    "ja".into()
}
fn default_panel_width() -> u32 {
    512
}
fn default_font_size() -> f32 {
    28.0
}
fn default_border_opacity() -> f32 {
    0.42
}
fn default_corner_radius() -> u32 {
    18
}
fn default_shadow_strength() -> f32 {
    0.35
}
fn default_grip_threshold() -> f32 {
    0.8
}
fn default_ocr_contrast() -> f32 {
    1.0
}
fn default_menu_width_m() -> f32 {
    0.55
}
fn default_menu_offset_y() -> f32 {
    -0.06
}
fn default_menu_offset_z() -> f32 {
    -0.75
}
fn default_result_width_m() -> f32 {
    0.72
}
fn default_result_offset_y() -> f32 {
    -0.42
}
fn default_result_offset_z() -> f32 {
    -1.10
}
fn default_scan_frame_width_m() -> f32 {
    0.34
}
fn default_scan_frame_distance_m() -> f32 {
    0.72
}

fn default_gravity_strength() -> f32 {
    0.5
}
fn default_fling_strength() -> f32 {
    0.7
}
fn default_snap_turn_angle() -> i32 {
    30
}
fn default_smooth_turn_rate() -> i32 {
    90
}
fn default_drag_multiplier() -> f32 {
    1.0
}
fn default_capture_quality() -> String {
    "balanced".into()
}
fn default_capture_format() -> String {
    "png".into()
}

fn default_vr_menu_accent() -> String {
    "#d97706".into()
}
fn default_vr_menu_bg() -> String {
    "#faf7ed".into()
}
fn default_vr_menu_text() -> String {
    "#5d4037".into()
}
fn default_vr_menu_text_muted() -> String {
    "#76584d".into()
}
fn default_merge_x() -> f32 {
    0.2
}
fn default_merge_y() -> f32 {
    0.3
}

impl Default for OvrConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            dual_display: true,
            wrist_mode: false,
            trigger_key: "trigger".into(),
            clear_key: "left_stick".into(),
            overlay_text_color: "#FFFFFF".into(),
            overlay_bg_color: "#101826".into(),
            overlay_bg_opacity: 0.46,
            overlay_lock_mode: "world".into(),
            status_color: "#00FF00".into(),
            trans_service: "google_free".into(),
            trans_api_key: "".into(),
            trans_llm_model: "".into(),
            trans_llm_prompt:
                "Translate the following text to Chinese. Only output the translation:".into(),
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
            overlay_glass: true,
            overlay_border_opacity: 0.42,
            overlay_corner_radius: 18,
            overlay_shadow_strength: 0.35,
            grip_pressure_threshold: 0.8,
            custom_api_url: String::new(),
            // Playspace
            playspace_offset_x: 0.0,
            playspace_offset_y: 0.0,
            playspace_offset_z: 0.0,
            playspace_rotation: 0.0,
            height_toggle_enabled: false,
            height_toggle_offset: 0.3,
            menu_width_m: 0.55,
            menu_offset_x: 0.0,
            menu_offset_y: -0.06,
            menu_offset_z: -0.75,
            result_width_m: 0.72,
            result_offset_x: 0.0,
            result_offset_y: -0.42,
            result_offset_z: -1.10,
            scan_frame_width_m: 0.34,
            scan_frame_distance_m: 0.72,
            vr_menu_accent: default_vr_menu_accent(),
            vr_menu_bg: default_vr_menu_bg(),
            vr_menu_text: default_vr_menu_text(),
            vr_menu_text_muted: default_vr_menu_text_muted(),
            ocr_contrast: 1.0,
            ocr_sharpen: false,
            ocr_denoise: false,
            ocr_merge_tolerance_x: 0.2,
            ocr_merge_tolerance_y: 0.3,
            auto_start_steamvr: false,
            // VRCDog space translation
            gravity_enabled: false,
            gravity_strength: 0.5,
            fling_strength: 0.7,
            snap_turn_angle: 30,
            snap_turn_enabled: true,
            smooth_turn_rate: 90,
            smooth_turn_enabled: false,
            lock_x_enabled: false,
            lock_y_enabled: false,
            lock_z_enabled: false,
            drag_multiplier: 1.0,
            comfort_turn_enabled: false,
            // Dynamic screenshot/capture
            capture_mode: "dynamic".into(),
            capture_quality: "balanced".into(),
            capture_auto_save: false,
            capture_format: "png".into(),
        }
    }
}

// ==================== Command Channel ====================

#[derive(Debug)]
enum OvrCommand {
    UpdateConfig(Box<OvrConfig>),
    SetMenuTheme {
        accent: String,
        bg: String,
        text: String,
        muted: String,
    },
    UpdateText {
        original: String,
        translated: String,
    },
    SetVisible(bool),
    ClearText,
    ToggleTranslation,
    ToggleMenu,
    OpenBindingUi,
    Shutdown,
    // Desktop mirror mode commands
    DesktopScanOnce, // Trigger a single desktop capture + OCR + translate
    StartAutoScan,   // Start auto-scan timer
    StopAutoScan,    // Stop auto-scan timer
    // ===== Native Playspace Control Commands =====
    SetPlayspaceOffset {
        x: f32,
        y: f32,
        z: f32,
    }, // Set playspace XYZ offset
    SetPlayspaceRotation(f32), // Set playspace Y rotation
    ToggleHeight,              // Toggle height offset on/off
    ResetPlayspace,            // Reset all offsets to zero
    FixFloor,                  // Fix floor height using controller position
    // ===== VRCDog Space Translation =====
    SetGravityEnabled(bool),
    SetGravityStrength(f32),
    SetFlingStrength(f32),
    SetSnapTurnAngle(i32),
    SetSmoothTurnRate(i32),
    SetSmoothTurnEnabled(bool),
    SetLockXEnabled(bool),
    SetLockYEnabled(bool),
    SetLockZEnabled(bool),
    SetDragMultiplier(f32),
    SetComfortTurnEnabled(bool),
    // ===== Dynamic Screenshot/Capture =====
    SetCaptureMode(String),
    SetCaptureQuality(String),
    SetCaptureAutoSave(bool),
    SetCaptureFormat(String),
    SetSurveyGate { status: String, pending: u32 }, // Backend survey gate status (drives VR menu visibility)
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
        r"C:\Windows\Fonts\simhei.ttf",  // SimHei (PRIORITY for CJK)
        r"C:\Windows\Fonts\msyh.ttc",    // Microsoft YaHei (CJK)
        r"C:\Windows\Fonts\msyhbd.ttc",  // Microsoft YaHei Bold
        r"C:\Windows\Fonts\simsun.ttc",  // SimSun
        r"C:\Windows\Fonts\simsun.ttc",  // SimSun
        r"C:\Windows\Fonts\arial.ttf",   // Arial fallback
        r"C:\Windows\Fonts\segoeui.ttf", // Segoe UI fallback
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
async fn perform_scan_translate(config: &OvrConfig) -> Result<(String, String), String> {
    let ocr_text = crate::ocr::OcrEngine::extract_text_from_screen(
        &config.ocr_language,
        config.ocr_image_enhance,
    )
    .await?;

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
        Err(_) => Ok((
            ocr_text.clone(),
            format!("[OCR结果 - 未配置翻译API]\n{}", ocr_text),
        )),
    }
}

fn normalize_overlay_layout(config: &mut OvrConfig) {
    config.menu_width_m = config.menu_width_m.clamp(0.25, 1.40);
    config.menu_offset_x = config.menu_offset_x.clamp(-1.50, 1.50);
    config.menu_offset_y = config.menu_offset_y.clamp(-0.90, 0.80);
    config.menu_offset_z = config.menu_offset_z.clamp(-2.50, -0.25);
    config.result_width_m = config.result_width_m.clamp(0.25, 1.60);
    config.result_offset_x = config.result_offset_x.clamp(-1.50, 1.50);
    config.result_offset_y = config.result_offset_y.clamp(-1.20, 0.60);
    config.result_offset_z = config.result_offset_z.clamp(-2.80, -0.30);
    config.scan_frame_width_m = config.scan_frame_width_m.clamp(0.12, 1.60);
    config.scan_frame_distance_m = config.scan_frame_distance_m.clamp(0.30, 2.00);
    config.gravity_strength = config.gravity_strength.clamp(0.0, 1.0);
    config.fling_strength = config.fling_strength.clamp(0.0, 1.0);
    config.snap_turn_angle = config.snap_turn_angle.clamp(15, 90);
    config.smooth_turn_rate = config.smooth_turn_rate.clamp(30, 180);
    config.drag_multiplier = config.drag_multiplier.clamp(0.1, 3.0);
}

fn hmd_relative_transform(x: f32, y: f32, z: f32) -> openvr::pose::Matrix3x4 {
    openvr::pose::Matrix3x4([[1.0, 0.0, 0.0, x], [0.0, 1.0, 0.0, y], [0.0, 0.0, 1.0, z]])
}

fn wrist_relative_transform(x: f32, y: f32, z: f32) -> openvr::pose::Matrix3x4 {
    openvr::pose::Matrix3x4([
        [1.0, 0.0, 0.0, x],
        [0.0, 0.707, 0.707, y],
        [0.0, -0.707, 0.707, z],
    ])
}

fn world_overlay_transform(x: f32, y: f32, z: f32) -> openvr::pose::Matrix3x4 {
    openvr::pose::Matrix3x4([
        [1.0, 0.0, 0.0, x],
        [0.0, 1.0, 0.0, 1.45 + y],
        [0.0, 0.0, 1.0, z],
    ])
}

fn trigger_haptic_pulse(
    system: &openvr::System,
    device_index: openvr::TrackedDeviceIndex,
    duration_secs: f32,
    _amplitude: f32,
) {
    let microseconds = (duration_secs * 1_000_000.0).clamp(0.0, 4000.0) as u16;
    system.trigger_haptic_pulse(device_index, 0, microseconds);
}

#[derive(Clone, Copy)]
enum TextOverlayKind {
    Menu,
    Result,
}

fn apply_text_overlay_layout(
    ovr: &mut openvr::Overlay,
    handle: openvr::overlay::OverlayHandle,
    config: &OvrConfig,
    left_idx: Option<openvr::TrackedDeviceIndex>,
    kind: TextOverlayKind,
) {
    let (width, x, y, z) = match kind {
        TextOverlayKind::Menu => (
            config.menu_width_m,
            config.menu_offset_x,
            config.menu_offset_y,
            config.menu_offset_z,
        ),
        TextOverlayKind::Result => (
            config.result_width_m,
            config.result_offset_x,
            config.result_offset_y,
            config.result_offset_z,
        ),
    };

    let _ = ovr.set_width(handle, width.clamp(0.12, 1.80));

    match config.overlay_lock_mode.as_str() {
        "head" => {
            let transform = hmd_relative_transform(x, y, z);
            let _ = ovr.set_transform_tracked_device_relative(
                handle,
                openvr::TrackedDeviceIndex(0),
                &transform,
            );
        }
        "wrist" => {
            if let Some(l_idx) = left_idx {
                let transform = wrist_relative_transform(x, y, z);
                let _ = ovr.set_transform_tracked_device_relative(handle, l_idx, &transform);
            } else {
                let transform = hmd_relative_transform(x, y, z);
                let _ = ovr.set_transform_tracked_device_relative(
                    handle,
                    openvr::TrackedDeviceIndex(0),
                    &transform,
                );
            }
        }
        _ => {
            let transform = world_overlay_transform(x, y, z);
            let _ = ovr.set_transform_absolute(
                handle,
                openvr::TrackingUniverseOrigin::Standing,
                &transform,
            );
        }
    }
}

fn apply_scan_frame_layout(
    ovr: &mut openvr::Overlay,
    handle: openvr::overlay::OverlayHandle,
    config: &OvrConfig,
    right_idx: Option<openvr::TrackedDeviceIndex>,
) {
    let _ = ovr.set_width(handle, config.scan_frame_width_m.clamp(0.12, 1.60));
    if let Some(r_idx) = right_idx {
        let transform = hmd_relative_transform(0.0, 0.0, -config.scan_frame_distance_m);
        let _ = ovr.set_transform_tracked_device_relative(handle, r_idx, &transform);
    }
}

#[allow(unused_assignments)]
fn vr_thread_main(
    app_handle: AppHandle,
    status: Arc<Mutex<OvrStatus>>,
    config: Arc<Mutex<OvrConfig>>,
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
        hmd_model = sys
            .string_tracked_device_property(
                openvr::TrackedDeviceIndex(0),
                openvr::property::ModelNumber_String,
            )
            .map(|s| s.to_string_lossy().to_string())
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
    let _ = app_handle.emit(
        "ovr_log",
        format!("[OVR] [OK] OpenVR 已连接: {}", hmd_model),
    );

    // ===== Create overlays =====
    let mut overlay_handle = None; // Main menu overlay
    let mut result_handle = None; // Bottom translation result overlay
    let mut scan_handle = None; // Green scan frame overlay

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
                    if let Some(l_idx) = sys.tracked_device_index_for_controller_role(
                        openvr::TrackedControllerRole::LeftHand,
                    ) {
                        let hand_transform = openvr::pose::Matrix3x4([
                            [1.0, 0.0, 0.0, 0.0],
                            [0.0, 1.0, 0.0, 0.08],
                            [0.0, 0.0, 1.0, -0.05],
                        ]);
                        let _ = ovr.set_transform_tracked_device_relative(
                            handle,
                            l_idx,
                            &hand_transform,
                        );
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
                    let _ = ovr.set_transform_tracked_device_relative(
                        handle,
                        openvr::TrackedDeviceIndex(0),
                        &hmd_transform,
                    );
                    let _ = ovr.set_width(handle, 0.25);
                }
                // Render initial menu
                if let Some(ref f) = font {
                    let menu_text = "VrcDog VR 翻译器\n\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\n手柄操作:\n  [双手柄握把] 开关菜单  [扳机] 扫描\n  [摇杆] 导航      [右侧握把+摇杆] 缩放推拉\n  [左侧摇杆按下] 清除结果";
                    let pixels = crate::vr_ui::VrUiRenderer::render_text_to_rgba(
                        f,
                        "",
                        menu_text,
                        false,
                        512,
                        320,
                        [255, 255, 255],
                        [18, 18, 42],
                        0.90,
                    );
                    let _ = ovr.set_raw_data(handle, &pixels, 512, 320, 4);
                }
                // Keep the menu hidden until the controller shortcut or desktop
                // command explicitly opens it.
                let _ = ovr.set_visibility(handle, false);
                overlay_handle = Some(handle);
                let _ = app_handle.emit("ovr_log", "[OVR] [OK] 菜单叠加层已创建(跟随头部)");
            }
            Err(e) => {
                let _ = app_handle.emit("ovr_log", format!("[OVR] [Error] 菜单创建失败: {:?}", e));
            }
        }

        // 2) Translation result overlay - shown near the bottom of the VR view
        match ovr.create_overlay("vrcdog.result\0", "VrcDog Translation\0") {
            Ok(handle) => {
                let _ = ovr.set_width(handle, default_result_width_m());
                let _ = ovr.set_opacity(handle, 0.92);
                let _ = ovr.set_sort_order(handle, 15);
                let _ = ovr.set_visibility(handle, false);
                result_handle = Some(handle);
                let _ = app_handle.emit("ovr_log", "[OVR] [OK] Translation result overlay created");
            }
            Err(e) => {
                let _ = app_handle.emit(
                    "ovr_log",
                    format!("[OVR] [Error] Translation result overlay failed: {:?}", e),
                );
            }
        }

        // 3) Scan frame overlay - follows right controller, green border
        match ovr.create_overlay("vrcdog.scan\0", "VrcDog Scan\0") {
            Ok(handle) => {
                let _ = ovr.set_width(handle, 0.3);
                let _ = ovr.set_opacity(handle, 0.8);
                let _ = ovr.set_sort_order(handle, 20);
                // Render green scan frame (border only, transparent center)
                let scan_pixels =
                    crate::vr_ui::VrUiRenderer::render_scan_frame(256, 256, "#00FF64");
                let _ = ovr.set_raw_data(handle, &scan_pixels, 256, 256, 4);
                let _ = ovr.set_visibility(handle, false); // Hidden until trigger
                scan_handle = Some(handle);
                let _ = app_handle.emit("ovr_log", "[OVR] [OK] 扫描框叠加层已创建");
            }
            Err(e) => {
                let _ =
                    app_handle.emit("ovr_log", format!("[OVR] [Error] 扫描框创建失败: {:?}", e));
            }
        }
    }

    let _ = app_handle.emit("ovr_log", "[OVR] [OK] VR 事件循环已启动 (90Hz)");

    // ===== SteamVR Input 2.0 Initialization =====
    let mut act_set_main = openvr::input::VRActionSetHandle(0);
    let mut act_translate = openvr::input::VRActionHandle(0);
    let mut act_scale = openvr::input::VRActionHandle(0);
    let mut act_clear = openvr::input::VRActionHandle(0);
    let mut act_menu_navigate = openvr::input::VRActionHandle(0);
    let mut has_input_20 = false;

    if let Ok(mut input) = context.input() {
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(parent) = exe_path.parent() {
                // Try parent first (prod mode)
                let mut manifest_path = parent.join("vrcdog_actions.json");
                if !manifest_path.exists() {
                    // Try current dir (dev mode)
                    if let Ok(cd) = std::env::current_dir() {
                        let root_candidate = cd.join("vrcdog_actions.json");
                        let tauri_candidate = cd.join("src-tauri").join("vrcdog_actions.json");
                        manifest_path = if root_candidate.exists() {
                            root_candidate
                        } else {
                            tauri_candidate
                        };
                    }
                }
                if manifest_path.exists() {
                    if let Ok(path_str) =
                        std::ffi::CString::new(manifest_path.to_string_lossy().as_bytes())
                    {
                        if input.set_action_manifest_raw(&path_str).is_ok() {
                            has_input_20 = true;
                            act_set_main = input
                                .get_action_set_handle("/actions/main")
                                .unwrap_or(openvr::input::VRActionSetHandle(0));
                            act_translate = input
                                .get_action_handle("/actions/main/in/TranslateTrigger")
                                .unwrap_or(openvr::input::VRActionHandle(0));
                            act_scale = input
                                .get_action_handle("/actions/main/in/GripScale")
                                .unwrap_or(openvr::input::VRActionHandle(0));
                            act_clear = input
                                .get_action_handle("/actions/main/in/ClearTranslation")
                                .unwrap_or(openvr::input::VRActionHandle(0));
                            act_menu_navigate = input
                                .get_action_handle("/actions/main/in/MenuNavigate")
                                .unwrap_or(openvr::input::VRActionHandle(0));
                            let _ =
                                app_handle.emit("ovr_log", "[OVR] [OK] SteamVR Input 2.0 加载成功");
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
    let mut overlay_menu_visible = false; // Start hidden; shown by holding both grips (anti-VRChat chord)
    let mut result_overlay_visible = false;
    let mut scan_active = false;
    let mut survey_status: String = String::from("unknown"); // backend survey gate status (survey_required|survey_available|ok)
    let mut survey_pending: u32 = 0; // pending survey count from backend gate
    // Channel for scan results (async task -> VR thread)
    let (scan_tx, scan_rx) = std::sync::mpsc::channel::<(String, String)>();

    // Menu interaction state
    let mut menu_page: usize = 0; // 0=main, 1=translate, 2=scan, 3=display
    let mut menu_selection: usize = 0; // Currently highlighted item

    // ===== Right controller state =====
    let _drag_active = false; // Menu drag in progress
    let mut last_menu_render_page: i32 = -1; // Track when to re-render
    let mut last_menu_render_sel: i32 = -1;
    let _scan_start_pos: Option<[f32; 3]> = None;

    let mut menu_combo_ticks: u64 = 0;

    let _last_right_trigger_press_tick: u64 = 0;
    let _is_scan_primed = false;

    let mut prev_ivr_translate = false;
    let mut prev_ivr_scale = false;
    let mut prev_ivr_clear = false;

    let mut joystick_nav_cooldown: u64 = 0; // Prevent too-fast joystick navigation

    // Configurable transforms via joystick / trackpad.
    let mut scan_drag_origin: Option<[f32; 3]> = None;
    let mut scan_drag_origin_width: f32 = default_scan_frame_width_m();

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

    // ===== VRCDog Space Translation State =====
    let mut last_turn_time: u64 = 0;
    let mut smooth_turn_active = false;
    let mut smooth_turn_direction: f32 = 0.0;

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
            f,
            "",
            "VrcDog\n双手柄握把同时按住 开启菜单\n双击并长按右手扳机 框选翻译",
            false,
            512,
            320,
            [255, 255, 255],
            [18, 18, 42],
            0.90,
        );
        if let Ok(mut ovr) = context.overlay() {
            let _ = ovr.set_raw_data(h, &pixels, 512, 320, 4);
            let _ = ovr.set_visibility(h, false);
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
                        if let Some(h) = result_handle {
                            let _ = ovr.set_visibility(h, false);
                        }
                        if let Some(h) = scan_handle {
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
                    normalize_overlay_layout(&mut current_config);
                    // Force the VR menu to re-render with the (possibly) new theme colors.
                    last_menu_render_page = -1;
                    last_menu_render_sel = -1;
                    // Apply config changes to overlay
                    if let Ok(mut ovr) = context.overlay() {
                        let left_idx = context.system().ok().and_then(|sys| {
                            sys.tracked_device_index_for_controller_role(
                                openvr::TrackedControllerRole::LeftHand,
                            )
                        });
                        let right_idx = context.system().ok().and_then(|sys| {
                            sys.tracked_device_index_for_controller_role(
                                openvr::TrackedControllerRole::RightHand,
                            )
                        });
                        if let Some(h) = overlay_handle {
                            let _ = ovr.set_opacity(h, current_config.overlay_bg_opacity);
                            apply_text_overlay_layout(
                                &mut ovr,
                                h,
                                &current_config,
                                left_idx,
                                TextOverlayKind::Menu,
                            );
                        }
                        if let Some(h) = result_handle {
                            let _ = ovr.set_opacity(h, current_config.overlay_bg_opacity);
                            apply_text_overlay_layout(
                                &mut ovr,
                                h,
                                &current_config,
                                left_idx,
                                TextOverlayKind::Result,
                            );
                        }
                        // Update scan frame color if changed
                        if let Some(sh) = scan_handle {
                            let scan_pixels = crate::vr_ui::VrUiRenderer::render_scan_frame(
                                256,
                                256,
                                &current_config.scan_frame_color,
                            );
                            let _ = ovr.set_raw_data(sh, &scan_pixels, 256, 256, 4);
                            apply_scan_frame_layout(&mut ovr, sh, &current_config, right_idx);
                        }
                    }
                    // Apply height toggle offset from config
                    height_offset = current_config.height_toggle_offset;
                    let _ = app_handle.emit(
                        "ovr_log",
                        format!(
                            "[OVR] ⚙ 配置已更新 (OCR={}, 扫描框={}, 字号={:.0})",
                            current_config.ocr_language,
                            current_config.scan_frame_color,
                            current_config.overlay_font_size,
                        ),
                    );
                }
                OvrCommand::ToggleMenu => {
                    overlay_menu_visible = !overlay_menu_visible;
                    if let Ok(mut ovr) = context.overlay() {
                        if let Some(h) = overlay_handle {
                            let _ = ovr.set_visibility(h, overlay_menu_visible);
                            if overlay_menu_visible {
                                let left_idx = context.system().ok().and_then(|sys| {
                                    sys.tracked_device_index_for_controller_role(
                                        openvr::TrackedControllerRole::LeftHand,
                                    )
                                });
                                apply_text_overlay_layout(
                                    &mut ovr,
                                    h,
                                    &current_config,
                                    left_idx,
                                    TextOverlayKind::Menu,
                                );
                            }
                        }
                    }
                    if overlay_menu_visible {
                        menu_page = 0;
                        menu_selection = 0;
                        last_menu_render_page = -1;
                    }
                    let status_c = status.clone();
                    let visible = overlay_menu_visible;
                    tokio::runtime::Handle::current().block_on(async {
                        status_c.lock().await.menu_visible = visible;
                    });
                    let _ = app_handle.emit("ovr_menu_visibility", visible);
                }
                OvrCommand::OpenBindingUi => {
                    match context.input() {
                        Ok(mut input) => {
                            let action_set = if act_set_main.0 == 0 {
                                None
                            } else {
                                Some(act_set_main)
                            };
                            let result = input.open_binding_ui(
                                None,
                                action_set,
                                openvr::input::VRInputValueHandle(0),
                                true,
                            );
                            let _ = app_handle.emit(
                                "ovr_log",
                                format!("[OVR] SteamVR 按键绑定编辑器: {:?}", result),
                            );
                        }
                        Err(error) => {
                            let _ = app_handle.emit(
                                "ovr_log",
                                format!("[OVR] 无法打开 SteamVR 按键绑定编辑器: {:?}", error),
                            );
                        }
                    }
                }
                OvrCommand::SetMenuTheme { accent, bg, text, muted } => {
                    current_config.vr_menu_accent = accent;
                    current_config.vr_menu_bg = bg;
                    current_config.vr_menu_text = text;
                    current_config.vr_menu_text_muted = muted;
                    // Force the menu to repaint with the new theme.
                    last_menu_render_page = -1;
                    last_menu_render_sel = -1;
                }
                OvrCommand::UpdateText {
                    original,
                    translated,
                } => {
                    current_translation = Some((original, translated));
                    // Render text to overlay using configured panel width
                    if let (Some(h), Some(ref f)) = (result_handle, &font) {
                        let text_color = crate::vr_ui::VrUiRenderer::parse_hex_rgb(
                            &current_config.overlay_text_color,
                        );
                        let bg_color = crate::vr_ui::VrUiRenderer::parse_hex_rgb(
                            &current_config.overlay_bg_color,
                        );
                        let (orig, trans) = current_translation.as_ref().unwrap();
                        let panel_w = current_config.trans_panel_max_width.max(256).min(1024);
                        let panel_h = (panel_w / 2).max(128);
                        let pixels = crate::vr_ui::VrUiRenderer::render_text_to_rgba_styled(
                            f,
                            orig,
                            trans,
                            current_config.dual_display,
                            panel_w,
                            panel_h,
                            text_color,
                            bg_color,
                            current_config.overlay_bg_opacity,
                            current_config.overlay_font_size,
                            current_config.overlay_border_opacity,
                            current_config.overlay_corner_radius,
                            current_config.overlay_shadow_strength,
                        );
                        if let Ok(mut ovr) = context.overlay() {
                            let _ =
                                ovr.set_raw_data(h, &pixels, panel_w as usize, panel_h as usize, 4);
                            let left_idx = context.system().ok().and_then(|sys| {
                                sys.tracked_device_index_for_controller_role(
                                    openvr::TrackedControllerRole::LeftHand,
                                )
                            });
                            apply_text_overlay_layout(
                                &mut ovr,
                                h,
                                &current_config,
                                left_idx,
                                TextOverlayKind::Result,
                            );
                            let _ = ovr.set_visibility(h, true);
                            result_overlay_visible = true;
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
                        if let Some(h) = result_handle {
                            let _ = ovr.set_visibility(h, vis);
                        }
                    }
                    result_overlay_visible = vis;
                    let status_c = status.clone();
                    tokio::runtime::Handle::current().block_on(async {
                        let mut s = status_c.lock().await;
                        s.overlay_visible = vis;
                    });
                }
                OvrCommand::ClearText => {
                    current_translation = None;
                    if let Ok(mut ovr) = context.overlay() {
                        if let Some(h) = result_handle {
                            let _ = ovr.set_visibility(h, false);
                        }
                    }
                    result_overlay_visible = false;
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
                    let _ = app_handle.emit(
                        "ovr_log",
                        format!(
                            "[OVR] 翻译模式: {}",
                            if translation_enabled {
                                "开启"
                            } else {
                                "关闭"
                            }
                        ),
                    );
                }
                OvrCommand::DesktopScanOnce => {
                    if !is_translating {
                        is_translating = true;
                        let _ = app_handle.emit("ovr_log", "[OVR] 📸 桌面截图翻译中...");
                        // Show progress overlay
                        if let (Some(h), Some(ref f)) = (result_handle, &font) {
                            let pixels = crate::vr_ui::VrUiRenderer::render_text_to_rgba(
                                f,
                                "",
                                "桌面截图翻译中...\n请稍候",
                                false,
                                512,
                                320,
                                [255, 200, 50],
                                [18, 18, 42],
                                0.92,
                            );
                            if let Ok(mut ovr) = context.overlay() {
                                let _ = ovr.set_raw_data(h, &pixels, 512, 320, 4);
                                let left_idx = context.system().ok().and_then(|sys| {
                                    sys.tracked_device_index_for_controller_role(
                                        openvr::TrackedControllerRole::LeftHand,
                                    )
                                });
                                apply_text_overlay_layout(
                                    &mut ovr,
                                    h,
                                    &current_config,
                                    left_idx,
                                    TextOverlayKind::Result,
                                );
                                let _ = ovr.set_visibility(h, true);
                                result_overlay_visible = true;
                            }
                        }
                        let cfg_clone = current_config.clone();
                        let tx = scan_tx.clone();
                        let app_h = app_handle.clone();
                        let status_c = status.clone();
                        tokio::runtime::Handle::current().spawn(async move {
                            match perform_scan_translate(&cfg_clone).await {
                                Ok((original, translated)) => {
                                    let _ = app_h.emit(
                                        "ovr_log",
                                        format!(
                                            "[OVR] ✅ 桌面翻译完成: {}",
                                            &translated[..translated.len().min(50)]
                                        ),
                                    );
                                    tokio::runtime::Handle::current().block_on(async {
                                        let mut s = status_c.lock().await;
                                        s.last_event = "desktop_scan_result".to_string();
                                        s.overlay_visible = true;
                                    });
                                    let _ = tx.send((original, translated));
                                }
                                Err(e) => {
                                    let _ = app_h
                                        .emit("ovr_log", format!("[OVR] ❌ 桌面扫描失败: {}", e));
                                    let _ = tx.send(("".to_string(), format!("❌ {}", e)));
                                }
                            }
                        });
                    }
                }
                OvrCommand::StartAutoScan => {
                    auto_scan_active = true;
                    auto_scan_countdown = (current_config.auto_scan_interval.max(3) as u64) * 90;
                    let _ = app_handle.emit(
                        "ovr_log",
                        format!(
                            "[OVR] 🔄 自动扫描已开启 (每 {}s)",
                            current_config.auto_scan_interval.max(3)
                        ),
                    );
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
                        let y_total =
                            ps_offset_y + if height_toggled { height_offset } else { 0.0 };
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
                        let y_total =
                            ps_offset_y + if height_toggled { height_offset } else { 0.0 };
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
                        let y_total =
                            ps_offset_y + if height_toggled { height_offset } else { 0.0 };
                        ps.apply_offset(ps_offset_x, y_total, ps_offset_z, ps_rotation_deg);
                    }
                    let _ = app_handle.emit("ovr_playspace_changed", serde_json::json!({
                        "offset_x": ps_offset_x, "offset_y": ps_offset_y, "offset_z": ps_offset_z,
                        "rotation": ps_rotation_deg, "height_toggled": height_toggled,
                    }));
                }
                OvrCommand::SetSurveyGate { status, pending } => {
                    survey_status = status;
                    survey_pending = pending;
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
                    let _ = app_handle.emit(
                        "ovr_playspace_changed",
                        serde_json::json!({
                            "offset_x": 0.0, "offset_y": 0.0, "offset_z": 0.0,
                            "rotation": 0.0, "height_toggled": false,
                        }),
                    );
                }
                OvrCommand::FixFloor => {
                    if let Ok(sys) = context.system() {
                        if let Some(r_idx) = sys.tracked_device_index_for_controller_role(
                            openvr::TrackedControllerRole::RightHand,
                        ) {
                            let poses = sys.device_to_absolute_tracking_pose(
                                openvr::TrackingUniverseOrigin::Standing,
                                0.0,
                            );
                            let pose = poses[r_idx.0 as usize];
                            if pose.pose_is_valid() {
                                let mat = pose.device_to_absolute_tracking();
                                let controller_y = mat[1][3];
                                if let Ok(ref mut ps) = playspace {
                                    ps.set_base_floor_to(controller_y);
                                    ps_offset_x = 0.0;
                                    ps_offset_y = 0.0;
                                    ps_offset_z = 0.0;
                                    ps_rotation_deg = 0.0;
                                    height_toggled = false;
                                    let _ = app_handle
                                        .emit("ovr_log", "[OVR] ✅ 地板已修复！(右手柄位置置底)");
                                    let _ = app_handle.emit(
                                        "ovr_playspace_changed",
                                        serde_json::json!({
                                            "offset_x": 0.0, "offset_y": 0.0, "offset_z": 0.0,
                                            "rotation": 0.0, "height_toggled": false,
                                        }),
                                    );
                                }
                            } else {
                                let _ = app_handle
                                    .emit("ovr_log", "[OVR] ⚠ 右手柄未跟踪，无法修复地板");
                            }
                        } else {
                            let _ = app_handle.emit("ovr_log", "[OVR] ⚠ 未检测到右手柄");
                        }
                    }
                }
                // ===== VRCDog Space Translation =====
                OvrCommand::SetGravityEnabled(enabled) => {
                    current_config.gravity_enabled = enabled;
                    let _ = app_handle.emit("ovr_log", format!("[OVR] 重力: {}", if enabled { "开" } else { "关" }));
                }
                OvrCommand::SetGravityStrength(strength) => {
                    current_config.gravity_strength = strength.clamp(0.0, 1.0);
                    let _ = app_handle.emit("ovr_log", format!("[OVR] 重力强度: {:.0}%", current_config.gravity_strength * 100.0));
                }
                OvrCommand::SetFlingStrength(strength) => {
                    current_config.fling_strength = strength.clamp(0.0, 1.0);
                }
                OvrCommand::SetSnapTurnAngle(angle) => {
                    current_config.snap_turn_angle = angle.clamp(15, 90);
                    let _ = app_handle.emit("ovr_log", format!("[OVR] 瞬转角度: {}°", current_config.snap_turn_angle));
                }
                OvrCommand::SetSmoothTurnRate(rate) => {
                    current_config.smooth_turn_rate = rate.clamp(30, 180);
                }
                OvrCommand::SetSmoothTurnEnabled(enabled) => {
                    current_config.smooth_turn_enabled = enabled;
                    if enabled { current_config.snap_turn_enabled = false; }
                }
                OvrCommand::SetLockXEnabled(enabled) => { current_config.lock_x_enabled = enabled; }
                OvrCommand::SetLockYEnabled(enabled) => { current_config.lock_y_enabled = enabled; }
                OvrCommand::SetLockZEnabled(enabled) => { current_config.lock_z_enabled = enabled; }
                OvrCommand::SetDragMultiplier(mult) => {
                    current_config.drag_multiplier = mult.clamp(0.1, 3.0);
                }
                OvrCommand::SetComfortTurnEnabled(enabled) => {
                    current_config.comfort_turn_enabled = enabled;
                }
                // ===== Dynamic Screenshot/Capture =====
                OvrCommand::SetCaptureMode(mode) => {
                    current_config.capture_mode = mode;
                }
                OvrCommand::SetCaptureQuality(quality) => {
                    current_config.capture_quality = quality;
                }
                OvrCommand::SetCaptureAutoSave(enabled) => {
                    current_config.capture_auto_save = enabled;
                }
                OvrCommand::SetCaptureFormat(format) => {
                    current_config.capture_format = format;
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
            if let (Some(h), Some(ref f)) = (result_handle, &font) {
                let text_color =
                    crate::vr_ui::VrUiRenderer::parse_hex_rgb(&current_config.overlay_text_color);
                let bg_color =
                    crate::vr_ui::VrUiRenderer::parse_hex_rgb(&current_config.overlay_bg_color);
                let panel_w = current_config.trans_panel_max_width.max(320).min(1024);
                let panel_h = ((panel_w as f32) * 0.625).round() as u32;
                let pixels = crate::vr_ui::VrUiRenderer::render_text_to_rgba_styled(
                    f,
                    &original,
                    &translated,
                    current_config.dual_display,
                    panel_w,
                    panel_h,
                    text_color,
                    bg_color,
                    current_config.overlay_bg_opacity,
                    current_config.overlay_font_size,
                    current_config.overlay_border_opacity,
                    current_config.overlay_corner_radius,
                    current_config.overlay_shadow_strength,
                );
                if let Ok(mut ovr) = context.overlay() {
                    let _ = ovr.set_raw_data(h, &pixels, panel_w as usize, panel_h as usize, 4);
                    let left_idx = context.system().ok().and_then(|sys| {
                        sys.tracked_device_index_for_controller_role(
                            openvr::TrackedControllerRole::LeftHand,
                        )
                    });
                    apply_text_overlay_layout(
                        &mut ovr,
                        h,
                        &current_config,
                        left_idx,
                        TextOverlayKind::Result,
                    );
                    let _ = ovr.set_visibility(h, true);
                    result_overlay_visible = true;

                    // --- NEW: Render to Wrist Overlay if Wrist Mode enabled ---
                    if current_config.wrist_mode {
                        if let Some(wh_l) = wrist_left_handle {
                            let wrist_pixels =
                                crate::vr_ui::VrUiRenderer::render_text_to_rgba_styled(
                                    f,
                                    "",
                                    &translated,
                                    false,
                                    256,
                                    128,
                                    text_color,
                                    bg_color,
                                    current_config.overlay_bg_opacity,
                                    (current_config.overlay_font_size * 0.75).clamp(18.0, 34.0),
                                    current_config.overlay_border_opacity,
                                    current_config.overlay_corner_radius.min(16),
                                    current_config.overlay_shadow_strength,
                                );
                            let _ = ovr.set_raw_data(wh_l, &wrist_pixels, 256, 128, 4);
                            let _ = ovr.set_visibility(wh_l, true);

                            // Anchor Left Wrist
                            if let Ok(sys) = context.system() {
                                if let Some(l_idx) = sys.tracked_device_index_for_controller_role(
                                    openvr::TrackedControllerRole::LeftHand,
                                ) {
                                    let transform = openvr::pose::Matrix3x4([
                                        [1.0, 0.0, 0.0, 0.0],
                                        [0.0, 0.0, 1.0, -0.05], // Tilt up on wrist
                                        [0.0, -1.0, 0.0, 0.15],
                                    ]);
                                    let _ = ovr.set_transform_tracked_device_relative(
                                        wh_l, l_idx, &transform,
                                    );
                                }
                            }
                        }
                    } else {
                        if let Some(wh_l) = wrist_left_handle {
                            let _ = ovr.set_visibility(wh_l, false);
                        }
                        if let Some(wh_r) = wrist_right_handle {
                            let _ = ovr.set_visibility(wh_r, false);
                        }
                    }
                }
            }

            // Emit to frontend for desktop UI display
            let _ = app_handle.emit(
                "ovr_desktop_translation",
                serde_json::json!({
                    "original": &original,
                    "translated": &translated,
                }),
            );

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
            let left_idx = sys
                .tracked_device_index_for_controller_role(openvr::TrackedControllerRole::LeftHand);
            let right_idx = sys
                .tracked_device_index_for_controller_role(openvr::TrackedControllerRole::RightHand);

            let grip_mask = 1u64 << openvr::button_id::GRIP;
            let trigger_mask = legacy_button_mask(&current_config.trigger_key);
            let menu_confirm_mask = 1u64 << openvr::button_id::STEAM_VR_TRIGGER;
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
            let mut ivr_menu_x = 0.0f32;
            let mut ivr_menu_y = 0.0f32;

            if has_input_20 {
                if let Ok(mut input) = context.input() {
                    let mut active_sets = vec![openvr::input::VRActiveActionSet(
                        openvr_sys::VRActiveActionSet_t {
                            ulActionSet: act_set_main.0,
                            ulRestrictedToDevice: 0,
                            ulSecondaryActionSet: 0,
                            unPadding: 0,
                            nPriority: 0,
                        },
                    )];
                    if input.update_actions(&mut active_sets).is_ok() {
                        if let Ok(data) = input.get_digital_action_data(
                            act_translate,
                            openvr::input::VRInputValueHandle(0),
                        ) {
                            ivr_translate_pressed = data.0.bState;
                        }
                        if let Ok(data) = input.get_digital_action_data(
                            act_scale,
                            openvr::input::VRInputValueHandle(0),
                        ) {
                            ivr_scale_pressed = data.0.bState;
                        }
                        if let Ok(data) = input.get_digital_action_data(
                            act_clear,
                            openvr::input::VRInputValueHandle(0),
                        ) {
                            ivr_clear_pressed = data.0.bState;
                        }
                        if let Ok(data) = input.get_analog_action_data(
                            act_menu_navigate,
                            openvr::input::VRInputValueHandle(0),
                        ) {
                            if data.0.bActive {
                                ivr_menu_x = data.0.x;
                                ivr_menu_y = data.0.y;
                            }
                        }
                    }
                }
            }

            // --- VR menu toggle ---
            // Activated ONLY by holding BOTH grips together. This chord never collides
            // with VRChat's own bindings (which use B/X/Y, triggers and single grips for
            // grab/scale), keeping the overlay menu separated from VRChat's Quick Menu
            // (B button). ~0.75s hold at 60Hz: responsive yet anti-accidental.
            let left_grip_held_for_menu = left_pressed & grip_mask != 0;
            let right_grip_held_for_menu = (right_pressed & grip_mask != 0) || ivr_scale_pressed;

            let mut should_toggle = false;
            if left_grip_held_for_menu && right_grip_held_for_menu {
                menu_combo_ticks += 1;
                if menu_combo_ticks == 45 {
                    should_toggle = true;
                }
            } else {
                menu_combo_ticks = 0;
            }

            if should_toggle {
                overlay_menu_visible = !overlay_menu_visible;
                if overlay_menu_visible {
                    scan_active = false;
                    scan_drag_origin = None;
                }
                if let Ok(mut ovr) = context.overlay() {
                    if let Some(h) = overlay_handle {
                        let _ = ovr.set_visibility(h, overlay_menu_visible);
                        if overlay_menu_visible {
                            apply_text_overlay_layout(
                                &mut ovr,
                                h,
                                &current_config,
                                left_idx,
                                TextOverlayKind::Menu,
                            );
                        }
                    }
                    if let Some(sh) = scan_handle {
                        if overlay_menu_visible {
                            let _ = ovr.set_visibility(sh, false);
                        }
                    }
                }
                if overlay_menu_visible {
                    menu_page = 0;
                    menu_selection = 0;
                    last_menu_render_page = -1; // Force re-render
                }
                let status_c = status.clone();
                let visible = overlay_menu_visible;
                tokio::runtime::Handle::current().block_on(async {
                    status_c.lock().await.menu_visible = visible;
                });
                let _ = app_handle.emit("ovr_menu_visibility", visible);
                let _ = app_handle.emit(
                    "ovr_log",
                    format!(
                        "[OVR] 菜单: {}",
                        if overlay_menu_visible {
                            "已打开"
                        } else {
                            "已关闭"
                        }
                    ),
                );
                if let Ok(sys) = context.system() {
                    if let Some(idx) = left_idx {
                        trigger_haptic_pulse(&sys, idx, 0.05, 0.3);
                    }
                    if let Some(idx) = right_idx {
                        trigger_haptic_pulse(&sys, idx, 0.05, 0.3);
                    }
                }
            }

            // Decrement joystick navigation cooldown
            joystick_nav_cooldown = joystick_nav_cooldown.saturating_sub(1);

            // --- Menu interaction (only when menu visible) ---
            if overlay_menu_visible {
                 let max_items = match menu_page {
                      0 => 8usize, // 0:主菜单有8项
                      1 | 2 | 3 | 4 | 6 | 7 | 14 => 4,
                      5 => 3,
                      8 | 10 | 11 => 4,
                      9 => 6,
                      12 | 13 | 15 => 5,
                      _ => 4,
                };

                // Joystick axes for navigation
                let legacy_joy_x = right_state.map(|s| s.axis[0].x).unwrap_or(0.0);
                let legacy_joy_y = right_state.map(|s| s.axis[0].y).unwrap_or(0.0);
                let joy_x = if ivr_menu_x.abs() > legacy_joy_x.abs() { ivr_menu_x } else { legacy_joy_x };
                let joy_y = if ivr_menu_y.abs() > legacy_joy_y.abs() { ivr_menu_y } else { legacy_joy_y };
                let left_joy_x = left_state.map(|s| s.axis[0].x).unwrap_or(0.0);
                let left_joy_y = left_state.map(|s| s.axis[0].y).unwrap_or(0.0);
                let right_grip_held = (right_pressed & grip_mask != 0) || ivr_scale_pressed;

                // Use joystick X to switch pages (with cooldown to prevent rapid switching)
                if joystick_nav_cooldown == 0 && !right_grip_held {
                    let mut page_changed = false;
                    if joy_x > 0.7 && menu_page < 15 {
                        menu_page += 1;
                        menu_selection = 0;
                        page_changed = true;
                        joystick_nav_cooldown = 18; // ~200ms cooldown at 90Hz
                    } else if joy_x < -0.7 && menu_page > 0 {
                        menu_page -= 1;
                        menu_selection = 0;
                        page_changed = true;
                        joystick_nav_cooldown = 18;
                    }

                    // Use joystick Y for up/down selection (natural, consistent)
                    let mut sel_changed = false;
                    if joy_y > 0.6 {
                        menu_selection = if menu_selection == 0 {
                            max_items - 1
                        } else {
                            menu_selection - 1
                        };
                        sel_changed = true;
                        joystick_nav_cooldown = 18;
                    } else if joy_y < -0.6 {
                        menu_selection = (menu_selection + 1) % max_items;
                        sel_changed = true;
                        joystick_nav_cooldown = 18;
                    }

                    if page_changed || sel_changed {
                        if let Ok(sys) = context.system() {
                            if let Some(idx) = right_idx {
                                trigger_haptic_pulse(&sys, idx, 0.02, 0.15);
                            }
                        }
                    }
                }

                // Grip + sticks/trackpads edit the menu overlay in-place:
                // right Y = up/down, right X = size, left X/Y = lateral/depth.
                if right_grip_held {
                    if joy_y.abs() > 0.1
                        || joy_x.abs() > 0.1
                        || left_joy_x.abs() > 0.1
                        || left_joy_y.abs() > 0.1
                    {
                        current_config.menu_width_m += joy_x * 0.006;
                        current_config.menu_offset_y += joy_y * 0.004;
                        current_config.menu_offset_x += left_joy_x * 0.004;
                        current_config.menu_offset_z += left_joy_y * 0.006;
                        normalize_overlay_layout(&mut current_config);

                        if let Ok(mut ovr) = context.overlay() {
                            if let Some(h) = overlay_handle {
                                apply_text_overlay_layout(
                                    &mut ovr,
                                    h,
                                    &current_config,
                                    left_idx,
                                    TextOverlayKind::Menu,
                                );
                            }
                        }
                        if tick % 12 == 0 {
                            let _ = app_handle.emit(
                                "ovr_layout_config_changed",
                                serde_json::json!({
                                    "menu_width_m": current_config.menu_width_m,
                                    "menu_offset_x": current_config.menu_offset_x,
                                    "menu_offset_y": current_config.menu_offset_y,
                                    "menu_offset_z": current_config.menu_offset_z,
                                }),
                            );
                        }
                    }
                }

                // Trigger = confirm/activate selected item
                if any_new & menu_confirm_mask != 0
                    || (ivr_translate_pressed && !prev_ivr_translate)
                {
                    let back_idx = max_items - 1;
                    if menu_selection == back_idx && menu_page > 0 {
                        menu_page = if menu_page == 12 || menu_page == 13 { 9 } else if menu_page == 14 || menu_page == 15 { 0 } else { 0 };
                        menu_selection = 0;
                    } else {
                        match menu_page {
                            0 => match menu_selection {
                                0 => {
                                    menu_page = 1;
                                    menu_selection = 0;
                                }
                                1 => {
                                    menu_page = 14;
                                    menu_selection = 0;
                                }
                                2 => {
                                    menu_page = 15;
                                    menu_selection = 0;
                                }
                                3 => {
                                    menu_page = 8;
                                    menu_selection = 0;
                                }
                                4 => {
                                    menu_page = 9;
                                    menu_selection = 0;
                                }
                                 5 => {
                                     menu_page = 10;
                                     menu_selection = 0;
                                 }
                                 6 => {
                                     menu_page = 11;
                                     menu_selection = 0;
                                 }
                                 7 => {
                                     menu_page = 7;
                                     menu_selection = 0;
                                 }
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
                                1 => {
                                    current_config.dual_display = !current_config.dual_display;
                                }
                                2 => {
                                    current_config.wrist_mode = !current_config.wrist_mode;
                                }
                                _ => {}
                            },
                            2 => match menu_selection {
                                0 => {
                                    current_config.desktop_mode = !current_config.desktop_mode;
                                }
                                1 => {
                                    current_config.auto_scan_enabled =
                                        !current_config.auto_scan_enabled;
                                    auto_scan_active = current_config.auto_scan_enabled;
                                    auto_scan_countdown = if auto_scan_active {
                                        (current_config.auto_scan_interval.max(3) as u64) * 90
                                    } else {
                                        0
                                    };
                                    let _ = app_handle.emit("ovr_auto_scan_status", auto_scan_active);
                                }
                                2 => {
                                    current_config.auto_scan_interval = match current_config.auto_scan_interval {
                                        3 => 5,
                                        5 => 10,
                                        10 => 15,
                                        15 => 30,
                                        _ => 3,
                                    };
                                }
                                _ => {}
                            },
                            3 => match menu_selection {
                                0 => {
                                    current_config.ocr_language = match current_config.ocr_language.as_str() {
                                        "zh-Hans-CN" => "en-US",
                                        "en-US" => "ja",
                                        "ja" => "ko",
                                        _ => "zh-Hans-CN",
                                    }.to_string();
                                }
                                1 => current_config.ocr_image_enhance = !current_config.ocr_image_enhance,
                                2 => {
                                    current_config.ocr_speed_mode = match current_config.ocr_speed_mode.as_str() {
                                        "fast" => "balanced",
                                        "balanced" | "standard" => "accurate",
                                        _ => "fast",
                                    }.to_string();
                                }
                                _ => {}
                            },
                            4 => match menu_selection {
                                0 => {
                                    current_config.trans_service = match current_config.trans_service.as_str() {
                                        "google_free" => "microsoft",
                                        "microsoft" => "deepl_free",
                                        "deepl_free" => "openai",
                                        _ => "google_free",
                                    }.to_string();
                                }
                                1 => {
                                    current_config.trans_target_lang = match current_config.trans_target_lang.as_str() {
                                        "zh-CN" => "en",
                                        "en" => "ja",
                                        "ja" => "ko",
                                        _ => "zh-CN",
                                    }.to_string();
                                }
                                _ => {}
                            },
                            5 => match menu_selection {
                                0 => {
                                    current_config.overlay_lock_mode = match current_config.overlay_lock_mode.as_str() {
                                        "world" => "head",
                                        "head" => "wrist",
                                        _ => "world",
                                    }.to_string();
                                }
                                1 => {
                                    current_config.overlay_bg_opacity = if current_config.overlay_bg_opacity >= 0.9 {
                                        0.4
                                    } else {
                                        (current_config.overlay_bg_opacity + 0.1).min(1.0)
                                    };
                                    if let Ok(mut ovr) = context.overlay() {
                                        if let Some(h) = overlay_handle { let _ = ovr.set_opacity(h, current_config.overlay_bg_opacity); }
                                        if let Some(h) = result_handle { let _ = ovr.set_opacity(h, current_config.overlay_bg_opacity); }
                                    }
                                }
                                _ => {}
                            },
                            6 => match menu_selection {
                                0 => {
                                    current_config.tts_enabled = !current_config.tts_enabled;
                                }
                                1 => {
                                    current_config.osc_chatbox_enabled =
                                        !current_config.osc_chatbox_enabled;
                                }
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
                            8 => match menu_selection {
                                0 => { let _ = app_handle.emit("ovr_menu_navigate", serde_json::json!({"tab":"friendslist"})); }
                                1 => { let _ = app_handle.emit("ovr_menu_navigate", serde_json::json!({"tab":"playerlist"})); }
                                2 => { let _ = app_handle.emit("ovr_menu_navigate", serde_json::json!({"tab":"notifications"})); }
                                _ => {}
                            },
                            9 => {
                                if survey_status == "survey_required" || survey_status == "survey_available" {
                                    match menu_selection {
                                        0 => { let _ = app_handle.emit("ovr_menu_navigate", serde_json::json!({"tab":"translator"})); }
                                        1 => {
                                            let _ = app_handle.emit("ovr_menu_navigate", serde_json::json!({"tab":"vrpiano"}));
                                            menu_page = 12;
                                            menu_selection = 0;
                                        }
                                        2 => { let _ = app_handle.emit("ovr_menu_navigate", serde_json::json!({"tab":"danmaku"})); }
                                        3 => {
                                            let _ = app_handle.emit("ovr_menu_navigate", serde_json::json!({"tab":"drawing"}));
                                            menu_page = 13;
                                            menu_selection = 0;
                                        }
                                        4 => { let _ = app_handle.emit("ovr_menu_navigate", serde_json::json!({"tab":"survey"})); }
                                        _ => {}
                                    }
                                } else {
                                    match menu_selection {
                                        0 => { let _ = app_handle.emit("ovr_menu_navigate", serde_json::json!({"tab":"translator"})); }
                                        1 => {
                                            let _ = app_handle.emit("ovr_menu_navigate", serde_json::json!({"tab":"vrpiano"}));
                                            menu_page = 12;
                                            menu_selection = 0;
                                        }
                                        2 => { let _ = app_handle.emit("ovr_menu_navigate", serde_json::json!({"tab":"danmaku"})); }
                                        3 => {
                                            let _ = app_handle.emit("ovr_menu_navigate", serde_json::json!({"tab":"drawing"}));
                                            menu_page = 13;
                                            menu_selection = 0;
                                        }
                                        _ => {}
                                    }
                                }
                            },
                            10 => match menu_selection {
                                0 => { let _ = app_handle.emit("ovr_menu_navigate", serde_json::json!({"tab":"feed"})); }
                                1 => { let _ = app_handle.emit("ovr_menu_navigate", serde_json::json!({"tab":"charts"})); }
                                2 => { let _ = app_handle.emit("ovr_menu_navigate", serde_json::json!({"tab":"notifications"})); }
                                _ => {}
                            },
                            11 => match menu_selection {
                                0 => {
                                    height_toggled = !height_toggled;
                                    current_config.height_toggle_enabled = height_toggled;
                                    if let Ok(ref mut ps) = playspace {
                                        let y_total = ps_offset_y + if height_toggled { height_offset } else { 0.0 };
                                        ps.apply_offset(ps_offset_x, y_total, ps_offset_z, ps_rotation_deg);
                                    }
                                    let _ = app_handle.emit("ovr_playspace_changed", serde_json::json!({"offset_x":ps_offset_x,"offset_y":ps_offset_y,"offset_z":ps_offset_z,"rotation":ps_rotation_deg,"height_toggled":height_toggled}));
                                }
                                1 => {
                                    ps_offset_x = 0.0; ps_offset_y = 0.0; ps_offset_z = 0.0;
                                    ps_rotation_deg = 0.0; height_toggled = false;
                                    current_config.playspace_offset_x = 0.0;
                                    current_config.playspace_offset_y = 0.0;
                                    current_config.playspace_offset_z = 0.0;
                                    current_config.playspace_rotation = 0.0;
                                    current_config.height_toggle_enabled = false;
                                    if let Ok(ref mut ps) = playspace { ps.apply_offset(0.0, 0.0, 0.0, 0.0); }
                                    let _ = app_handle.emit("ovr_playspace_changed", serde_json::json!({"offset_x":0.0,"offset_y":0.0,"offset_z":0.0,"rotation":0.0,"height_toggled":false}));
                                }
                                2 => {
                                    if let Ok(sys) = context.system() {
                                        if let Some(right_idx) = sys.tracked_device_index_for_controller_role(
                                            openvr::TrackedControllerRole::RightHand,
                                        ) {
                                            let poses = sys.device_to_absolute_tracking_pose(
                                                openvr::TrackingUniverseOrigin::Standing,
                                                0.0,
                                            );
                                            let pose = poses[right_idx.0 as usize];
                                            if pose.pose_is_valid() {
                                                let controller_y = pose.device_to_absolute_tracking()[1][3];
                                                if let Ok(ref mut ps) = playspace {
                                                    ps.set_base_floor_to(controller_y);
                                                    ps_offset_x = 0.0;
                                                    ps_offset_y = 0.0;
                                                    ps_offset_z = 0.0;
                                                    ps_rotation_deg = 0.0;
                                                    height_toggled = false;
                                                }
                                            }
                                        }
                                    }
                                }
                                _ => {}
                            },
                            12 => match menu_selection {
                                0 => { let _ = app_handle.emit("vrpiano_vr_action", "previous"); }
                                1 => { let _ = app_handle.emit("vrpiano_vr_action", "toggle"); }
                                2 => { let _ = app_handle.emit("vrpiano_vr_action", "restart"); }
                                3 => { let _ = app_handle.emit("vrpiano_vr_action", "next"); }
                                _ => {}
                            },
                             13 => match menu_selection {
                                 0 => { let _ = crate::vrdrawing::handle_vr_action("start"); }
                                 1 => { let _ = crate::vrdrawing::handle_vr_action("toggle_pause"); }
                                 2 => { let _ = crate::vrdrawing::handle_vr_action("stop"); }
                                 3 => { let _ = app_handle.emit("ovr_menu_navigate", serde_json::json!({"tab":"drawing"})); }
                                 _ => {}
                             },
                             14 => match menu_selection {
                                 0 => {
                                     current_config.snap_turn_enabled = !current_config.snap_turn_enabled;
                                     if current_config.snap_turn_enabled {
                                         current_config.smooth_turn_enabled = false;
                                     }
                                 }
                                 1 => {
                                     current_config.snap_turn_angle = match current_config.snap_turn_angle {
                                         15 => 30,
                                         30 => 45,
                                         45 => 60,
                                         _ => 15,
                                     };
                                 }
                                 2 => {
                                     current_config.gravity_enabled = !current_config.gravity_enabled;
                                 }
                                 3 => {
                                     current_config.lock_x_enabled = !current_config.lock_x_enabled;
                                     current_config.lock_y_enabled = !current_config.lock_y_enabled;
                                     current_config.lock_z_enabled = !current_config.lock_z_enabled;
                                 }
                                 _ => {}
                             },
                             15 => match menu_selection {
                                 0 => {
                                     current_config.capture_mode = match current_config.capture_mode.as_str() {
                                         "static" => "dynamic".to_string(),
                                         "dynamic" => "follow".to_string(),
                                         _ => "static".to_string(),
                                     };
                                 }
                                 1 => {
                                     current_config.capture_auto_save = !current_config.capture_auto_save;
                                 }
                                 2 => {
                                      current_config.capture_format = match current_config.capture_format.as_str() {
                                          "png" => "jpg".to_string(),
                                          "jpg" => "webp".to_string(),
                                          _ => "png".to_string(),
                                      };
                                 }
                                 3 => {
                                      let _ = app_handle.emit("ovr_log", "[OVR] 📸 正在截图...");
                                      if let Ok(mut ovr) = context.overlay() {
                                          if let Some(sh) = scan_handle {
                                              let _ = ovr.set_visibility(sh, true);
                                          }
                                      }
                                      let _cfg = current_config.clone();
                                      let app_h = app_handle.clone();
                                      std::thread::spawn(move || {
                                          let rt = tokio::runtime::Runtime::new().unwrap();
                                          rt.block_on(async move {
                                              match crate::ocr::OcrEngine::capture_primary_screen_to_file(
                                                  &std::env::temp_dir().join(format!("vrcdog_capture_{}.png", chrono::Utc::now().timestamp()))
                                              ).await {
                                                  Ok(_) => {
                                                      let _ = app_h.emit("ovr_log", "[OVR] ✅ 截图已保存");
                                                      let _ = app_h.emit("ovr_screenshot_ready", "");
                                                  }
                                                  Err(e) => {
                                                      let _ = app_h.emit("ovr_log", format!("[OVR] ❌ 截图失败: {}", e));
                                                  }
                                              }
                                          });
                                      });
                                  }
                                 _ => {}
                             },
                             _ => {}
                        }
                        // Haptic feedback for menu confirmation
                        if let Ok(sys) = context.system() {
                            if let Some(idx) = right_idx {
                                trigger_haptic_pulse(&sys, idx, 0.03, 0.2);
                            }
                            if let Some(idx) = left_idx {
                                trigger_haptic_pulse(&sys, idx, 0.03, 0.2);
                            }
                        }
                        // Persist VR-side changes and immediately apply transforms. This keeps
                        // controller edits and the desktop settings panel in sync.
                        normalize_overlay_layout(&mut current_config);
                        if let Ok(mut ovr) = context.overlay() {
                            let left_idx = context.system().ok().and_then(|sys| {
                                sys.tracked_device_index_for_controller_role(
                                    openvr::TrackedControllerRole::LeftHand,
                                )
                            });
                            if let Some(h) = overlay_handle {
                                apply_text_overlay_layout(
                                    &mut ovr,
                                    h,
                                    &current_config,
                                    left_idx,
                                    TextOverlayKind::Menu,
                                );
                            }
                            if let Some(h) = result_handle {
                                apply_text_overlay_layout(
                                    &mut ovr,
                                    h,
                                    &current_config,
                                    left_idx,
                                    TextOverlayKind::Result,
                                );
                            }
                        }
                        let shared_config = config.clone();
                        let config_snapshot = current_config.clone();
                        tokio::runtime::Handle::current().block_on(async {
                            *shared_config.lock().await = config_snapshot.clone();
                        });
                        let _ = app_handle.emit("ovr_config_changed", config_snapshot);
                        last_menu_render_page = -1;
                        last_menu_render_sel = -1;
                    }
                }

                // Re-render menu if state changed
                if last_menu_render_page != menu_page as i32
                    || last_menu_render_sel != menu_selection as i32
                    || (menu_page == 13 && tick % 30 == 0)
                {
                    if let (Some(h), Some(ref f)) = (overlay_handle, &font) {
                        let pixels = crate::vr_ui::VrUiRenderer::render_vr_menu(
                            f,
                            menu_page,
                            menu_selection,
                            scan_active,
                            translation_enabled,
                            &current_config,
                            &survey_status,
                            survey_pending,
                        );
                        if let Ok(mut ovr) = context.overlay() {
                            let _ = ovr.set_raw_data(h, &pixels, 1024, 640, 4);
                        }
                    }
                    last_menu_render_page = menu_page as i32;
                    last_menu_render_sel = menu_selection as i32;
                }
            } else {
                // Menu hidden: direct scan controls based on OVR Overlay Translator spec
                let trigger_down = (right_pressed & trigger_mask != 0) || ivr_translate_pressed;
                let trigger_just_pressed = (right_new & trigger_mask != 0)
                    || (ivr_translate_pressed && !prev_ivr_translate);
                let trigger_just_released = (prev_right_buttons & trigger_mask != 0
                    && !trigger_down)
                    || (!ivr_translate_pressed && prev_ivr_translate);

                let right_grip_down = (right_pressed & grip_mask != 0) || ivr_scale_pressed;
                let right_grip_just_pressed =
                    (right_new & grip_mask != 0) || (ivr_scale_pressed && !prev_ivr_scale);
                let left_joy_x = left_state.map(|s| s.axis[0].x).unwrap_or(0.0);
                let left_joy_y = left_state.map(|s| s.axis[0].y).unwrap_or(0.0);
                let right_joy_x = right_state.map(|s| s.axis[0].x).unwrap_or(0.0);
                let right_joy_y = right_state.map(|s| s.axis[0].y).unwrap_or(0.0);
                let clear_mask = legacy_button_mask(&current_config.clear_key);
                let configured_clear_pressed = match current_config.clear_key.as_str() {
                    "left_stick" => left_new & clear_mask != 0,
                    "right_stick" => right_new & clear_mask != 0,
                    _ => any_new & clear_mask != 0,
                };

                // Get absolute positions for Left Hand and Right Hand.
                let poses = sys.device_to_absolute_tracking_pose(
                    openvr::TrackingUniverseOrigin::Standing,
                    0.0,
                );
                let mut right_pos = [0.0, 0.0, 0.0];
                let mut left_pos = [0.0, 0.0, 0.0];

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

                let dist_right_to_left = ((right_pos[0] - left_pos[0]).powi(2)
                    + (right_pos[1] - left_pos[1]).powi(2)
                    + (right_pos[2] - left_pos[2]).powi(2))
                .sqrt();
                let is_near_wrist = dist_right_to_left < 0.3;

                if trigger_just_pressed && translation_enabled && !is_translating {
                    scan_active = true;
                    scan_drag_origin = Some(right_pos);
                    scan_drag_origin_width = current_config.scan_frame_width_m;
                    if let Ok(mut ovr) = context.overlay() {
                        if let Some(h) = result_handle {
                            let _ = ovr.set_visibility(h, false);
                            result_overlay_visible = false;
                        }
                        if let Some(sh) = scan_handle {
                            apply_scan_frame_layout(&mut ovr, sh, &current_config, right_idx);
                            let _ = ovr.set_visibility(sh, true);
                        }
                    }
                }

                if trigger_down && scan_active {
                    if let Some(origin) = scan_drag_origin {
                        let mut dx = right_pos[0] - origin[0];
                        let mut dy = right_pos[1] - origin[1];
                        let mut dz = right_pos[2] - origin[2];

                        // Apply axis locks
                        if current_config.lock_x_enabled { dx = 0.0; }
                        if current_config.lock_y_enabled { dy = 0.0; }
                        if current_config.lock_z_enabled { dz = 0.0; }

                        let drag_distance = (dx.powi(2) + dy.powi(2) + dz.powi(2)).sqrt();
                        current_config.scan_frame_width_m =
                            (scan_drag_origin_width + drag_distance * 1.55).clamp(0.12, 1.60);
                    }
                    if right_joy_y.abs() > 0.12 {
                        current_config.scan_frame_distance_m += right_joy_y * 0.01;
                    }
                    normalize_overlay_layout(&mut current_config);

                    if let Ok(mut ovr) = context.overlay() {
                        if let Some(sh) = scan_handle {
                            apply_scan_frame_layout(&mut ovr, sh, &current_config, right_idx);
                            let _ = ovr.set_visibility(sh, true);
                        }
                    }

                    if tick % 12 == 0 {
                        let _ = app_handle.emit(
                            "ovr_layout_config_changed",
                            serde_json::json!({
                                "scan_frame_width_m": current_config.scan_frame_width_m,
                                "scan_frame_distance_m": current_config.scan_frame_distance_m,
                            }),
                        );
                    }
                }

                // ===== VRCDog Space Translation =====
                // Snap turn (when menu is hidden and snap turn is enabled)
                if !overlay_menu_visible && current_config.snap_turn_enabled {
                    let snap_threshold = 0.6;
                    let now = tick;
                    if now - last_turn_time > 30 { // Cooldown
                        if right_joy_x > snap_threshold {
                            if let Ok(ref mut ps) = playspace {
                                ps.apply_offset(ps_offset_x, ps_offset_y, ps_offset_z, ps_rotation_deg + current_config.snap_turn_angle as f32);
                                ps_rotation_deg += current_config.snap_turn_angle as f32;
                                last_turn_time = now;
                                let _ = app_handle.emit("ovr_playspace_changed", serde_json::json!({"offset_x":ps_offset_x,"offset_y":ps_offset_y,"offset_z":ps_offset_z,"rotation":ps_rotation_deg}));
                            }
                        } else if right_joy_x < -snap_threshold {
                            if let Ok(ref mut ps) = playspace {
                                ps.apply_offset(ps_offset_x, ps_offset_y, ps_offset_z, ps_rotation_deg - current_config.snap_turn_angle as f32);
                                ps_rotation_deg -= current_config.snap_turn_angle as f32;
                                last_turn_time = now;
                                let _ = app_handle.emit("ovr_playspace_changed", serde_json::json!({"offset_x":ps_offset_x,"offset_y":ps_offset_y,"offset_z":ps_offset_z,"rotation":ps_rotation_deg}));
                            }
                        }
                    }
                }

                // Smooth turn
                if !overlay_menu_visible && current_config.smooth_turn_enabled {
                    if right_joy_x.abs() > 0.3 {
                        smooth_turn_active = true;
                        smooth_turn_direction = right_joy_x.signum();
                    } else {
                        smooth_turn_active = false;
                    }
                    if smooth_turn_active && tick % 2 == 0 {
                        let turn_amount = smooth_turn_direction * current_config.smooth_turn_rate as f32 * 0.005;
                        ps_rotation_deg += turn_amount;
                        if let Ok(ref mut ps) = playspace {
                            ps.apply_offset(ps_offset_x, ps_offset_y, ps_offset_z, ps_rotation_deg);
                        }
                        if tick % 10 == 0 {
                            let _ = app_handle.emit("ovr_playspace_changed", serde_json::json!({"offset_x":ps_offset_x,"offset_y":ps_offset_y,"offset_z":ps_offset_z,"rotation":ps_rotation_deg}));
                        }
                    }
                }

                // Gravity application (continuous when enabled)
                if current_config.gravity_enabled && !overlay_menu_visible {
                    // Apply gentle gravity pull downward when not grounded
                    // This is a simplified version - real implementation would check floor height
                    if tick % 60 == 0 && ps_offset_y > -0.5 {
                        let gravity_pull = current_config.gravity_strength * 0.01;
                        ps_offset_y -= gravity_pull;
                        if let Ok(ref mut ps) = playspace {
                            ps.apply_offset(ps_offset_x, ps_offset_y, ps_offset_z, ps_rotation_deg);
                        }
                    }
                }

                // 1. Release trigger -> OCR & Translate
                if trigger_just_released && translation_enabled && !is_translating {
                    is_translating = true;
                    let _ = app_handle.emit("ovr_log", "[OVR] 📸 触发截图识别...");

                    if let Ok(mut ovr) = context.overlay() {
                        if let Some(h) = result_handle {
                            let _ = ovr.set_visibility(h, false);
                            result_overlay_visible = false;
                        }
                        if let Some(sh) = scan_handle {
                            let _ = ovr.set_visibility(sh, false);
                        }
                    }
                    scan_active = false;
                    scan_drag_origin = None;

                    let cfg_clone = current_config.clone();
                    let app_h = app_handle.clone();
                    let status_c = status.clone();
                    let tx = scan_tx.clone();

                    tokio::runtime::Handle::current().spawn(async move {
                        let result = perform_scan_translate(&cfg_clone).await;
                        match result {
                            Ok((original, translated)) => {
                                let _ = app_h.emit(
                                    "ovr_log",
                                    format!(
                                        "[OVR] ✅ 翻译完成: {}",
                                        &translated[..translated.len().min(50)]
                                    ),
                                );
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

                // 2. Flick left joystick or trigger clear action -> Clear current translation
                if left_joy_x.abs() > 0.8
                    || left_joy_y.abs() > 0.8
                    || configured_clear_pressed
                    || (ivr_clear_pressed && !prev_ivr_clear)
                {
                    if let Ok(mut ovr) = context.overlay() {
                        if let Some(h) = result_handle {
                            let _ = ovr.set_visibility(h, false);
                            result_overlay_visible = false;
                        }
                    }
                }

                // 3. Hold right grip to edit the bottom result overlay in VR.
                if right_grip_down
                    && result_overlay_visible
                    && !scan_active
                    && (right_joy_x.abs() > 0.1
                        || right_joy_y.abs() > 0.1
                        || left_joy_x.abs() > 0.1
                        || left_joy_y.abs() > 0.1)
                {
                    current_config.result_width_m += right_joy_x * 0.006;
                    current_config.result_offset_y += right_joy_y * 0.004;
                    current_config.result_offset_x += left_joy_x * 0.004;
                    current_config.result_offset_z += left_joy_y * 0.006;
                    normalize_overlay_layout(&mut current_config);

                    if let Ok(mut ovr) = context.overlay() {
                        if let Some(h) = result_handle {
                            apply_text_overlay_layout(
                                &mut ovr,
                                h,
                                &current_config,
                                left_idx,
                                TextOverlayKind::Result,
                            );
                        }
                    }

                    if tick % 12 == 0 {
                        let _ = app_handle.emit(
                            "ovr_layout_config_changed",
                            serde_json::json!({
                                "result_width_m": current_config.result_width_m,
                                "result_offset_x": current_config.result_offset_x,
                                "result_offset_y": current_config.result_offset_y,
                                "result_offset_z": current_config.result_offset_z,
                            }),
                        );
                    }
                }

                // 4. Right controller near left wrist + press grip -> dock to wrist
                if right_grip_just_pressed && is_near_wrist {
                    current_config.overlay_lock_mode = "wrist".to_string();
                    if let Ok(mut ovr) = context.overlay() {
                        if let (Some(h), Some(l_idx)) = (overlay_handle, left_idx) {
                            let hand_transform = openvr::pose::Matrix3x4([
                                [1.0, 0.0, 0.0, 0.0],
                                [0.0, 1.0, 0.0, 0.08],
                                [0.0, 0.0, 1.0, -0.05],
                            ]);
                            let _ = ovr.set_transform_tracked_device_relative(
                                h,
                                l_idx,
                                &hand_transform,
                            );
                            let _ = ovr.set_width(h, 0.15); // small wrist size
                            let _ = ovr.set_visibility(h, true); // ensure it's visible
                        }
                    }
                    let _ = app_handle.emit("ovr_log", "[OVR] ⌚ 翻译结果已放置到手腕常驻显示");
                }
            }

            // ===== Space Drag (OVRAS Replacement) =====
            // Left Grip drags the world only while menu/combo shortcuts are idle.
            let left_grip_held = left_pressed & grip_mask != 0;
            if !overlay_menu_visible && !(left_grip_held_for_menu && right_grip_held_for_menu) {
                if left_grip_held {
                    if let Some(l_idx) = left_idx {
                        // Use RawAndUncalibrated so the coordinate space doesn't move while we drag it!
                        let poses = sys.device_to_absolute_tracking_pose(
                            openvr::TrackingUniverseOrigin::RawAndUncalibrated,
                            0.0,
                        );
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

                                    if let Ok(ref mut ps) = playspace {
                                        let y_total = ps_offset_y
                                            + if height_toggled { height_offset } else { 0.0 };
                                        ps.apply_offset(
                                            ps_offset_x,
                                            y_total,
                                            ps_offset_z,
                                            ps_rotation_deg,
                                        );
                                    }

                                    drag_last_pos = Some(curr_raw_pos);
                                }
                            }
                        }
                    }
                } else {
                    if is_space_dragging {
                        is_space_dragging = false;
                        // On release, commit to Live to make it persistent
                        let _ = app_handle.emit(
                            "ovr_log",
                            "[OVR] Space drag applied through native playspace controller",
                        );
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
            let st =
                tokio::runtime::Handle::current().block_on(async { status_c.lock().await.clone() });
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
                            let _ = app_h.emit(
                                "ovr_log",
                                format!(
                                    "[OVR] ✅ 自动翻译: {}",
                                    &translated[..translated.len().min(40)]
                                ),
                            );
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
                    openvr::TrackingUniverseOrigin::Standing,
                    0.0,
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
                        let _ = app_handle.emit(
                            "ovr_perf_stats",
                            serde_json::json!({
                                "pid": 0,
                                "num_frame_presents": stats.m_nNumFramePresents,
                                "num_dropped_frames": stats.m_nNumDroppedFrames,
                                "num_reprojected_frames": 0,
                                "reprojection_ratio": 0.0,
                                "tick": tick,
                            }),
                        );
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
        if tx.is_some() {
            drop(tx);
            tokio::time::sleep(tokio::time::Duration::from_millis(900)).await;
            return Ok(status.lock().await.clone());
        }
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

    let result = status.lock().await.clone();
    if !result.initialized {
        *state.cmd_tx.lock().await = None;
        let mut handle = state.event_loop_handle.lock().await;
        if handle.as_ref().is_some_and(|task| task.is_finished()) {
            if let Some(task) = handle.take() {
                let _ = task.await;
            }
        }
    }
    Ok(result)
}

#[tauri::command]
pub async fn ovr_shutdown(state: tauri::State<'_, OvrState>) -> crate::AppResult<()> {
    {
        let tx = state.cmd_tx.lock().await;
        if let Some(ref sender) = *tx {
            let _ = sender.send(OvrCommand::Shutdown);
        }
    }
    let mut h = state.event_loop_handle.lock().await;
    if let Some(handle) = h.take() {
        let _ = handle.await;
    }
    // Release the command channel so `ovr_init` can start a fresh overlay
    // thread later. Without this, its `tx.is_some()` guard blocks every
    // restart after a shutdown.
    *state.cmd_tx.lock().await = None;
    Ok(())
}

#[tauri::command]
pub async fn ovr_get_status(state: tauri::State<'_, OvrState>) -> crate::AppResult<OvrStatus> {
    Ok(state.status.lock().await.clone())
}

#[tauri::command]
pub async fn ovr_set_config(
    state: tauri::State<'_, OvrState>,
    config: OvrConfig,
) -> crate::AppResult<()> {
    let tx = state.cmd_tx.lock().await;
    if let Some(ref sender) = *tx {
        let _ = sender.send(OvrCommand::UpdateConfig(Box::new(config.clone())));
    }
    *state.config.lock().await = config;
    Ok(())
}

#[tauri::command]
pub async fn ovr_set_menu_theme(
    state: tauri::State<'_, OvrState>,
    accent: String,
    bg: String,
    text: String,
    muted: String,
) -> crate::AppResult<()> {
    let tx = state.cmd_tx.lock().await;
    if let Some(ref sender) = *tx {
        let _ = sender.send(OvrCommand::SetMenuTheme {
            accent: accent.clone(),
            bg: bg.clone(),
            text: text.clone(),
            muted: muted.clone(),
        });
    }
    let mut cfg = state.config.lock().await;
    cfg.vr_menu_accent = accent;
    cfg.vr_menu_bg = bg;
    cfg.vr_menu_text = text;
    cfg.vr_menu_text_muted = muted;
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
pub async fn ovr_set_survey_gate(
    _app_handle: AppHandle,
    state: tauri::State<'_, OvrState>,
    status: String,
    pending: u32,
) -> crate::AppResult<()> {
    let tx = state.cmd_tx.lock().await;
    if let Some(ref sender) = *tx {
        let _ = sender.send(OvrCommand::SetSurveyGate { status, pending });
    }
    Ok(())
}

fn legacy_button_mask(key: &str) -> u64 {
    let button = match key {
        "grip" => openvr::button_id::GRIP,
        "a_button" => openvr::button_id::A,
        // OpenVR's legacy controller API exposes the application/menu button;
        // SteamVR bindings map this to the controller's B/menu equivalent.
        "b_button" => openvr::button_id::APPLICATION_MENU,
        "left_stick" | "right_stick" => openvr::button_id::STEAM_VR_TOUCHPAD,
        _ => openvr::button_id::STEAM_VR_TRIGGER,
    };
    1u64 << button
}

/// Toggle the native SteamVR menu overlay without requiring a controller gesture.
#[tauri::command]
pub async fn ovr_toggle_menu(
    state: tauri::State<'_, OvrState>,
) -> crate::AppResult<()> {
    let tx = state.cmd_tx.lock().await;
    if let Some(ref sender) = *tx {
        let _ = sender.send(OvrCommand::ToggleMenu);
    }
    Ok(())
}

/// Open SteamVR's binding editor for the VrcDog action manifest.
#[tauri::command]
pub async fn ovr_open_binding_ui(
    state: tauri::State<'_, OvrState>,
) -> crate::AppResult<()> {
    let tx = state.cmd_tx.lock().await;
    if let Some(ref sender) = *tx {
        let _ = sender.send(OvrCommand::OpenBindingUi);
        Ok(())
    } else {
        Err("OpenVR 尚未初始化".into())
    }
}

#[tauri::command]
pub async fn ovr_capture_screenshot(
    app_handle: AppHandle,
    state: tauri::State<'_, OvrState>,
) -> crate::AppResult<String> {
    if !state.status.lock().await.initialized {
        return Err("OpenVR 尚未初始化".into());
    }
    let _ = app_handle.emit("ovr_log", "[OVR] 正在捕获 VR 截图...");

    // Use Windows screen capture as fallback (captures VR mirror window)
    let temp_dir = std::env::temp_dir();
    let path = temp_dir.join("vrcdog_vr_capture.png");
    let path_str = path.to_string_lossy().to_string();

    crate::ocr::OcrEngine::capture_primary_screen_to_file(&path).await?;

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
    update_overlay_text(&app_handle, &state, original, translated)
        .await
        .map(|_| ())
}

pub async fn update_overlay_text(
    app_handle: &AppHandle,
    state: &OvrState,
    original: String,
    translated: String,
) -> crate::AppResult<bool> {
    let initialized = state.status.lock().await.initialized;
    let tx = state.cmd_tx.lock().await;
    let updated = if initialized {
        if let Some(ref sender) = *tx {
            sender.send(OvrCommand::UpdateText {
                original: original.clone(),
                translated: translated.clone(),
            }).is_ok()
        } else {
            false
        }
    } else {
        false
    };
    let _ = app_handle.emit(
        "ovr_translation_updated",
        serde_json::json!({
            "original": original,
            "translated": translated,
        }),
    );
    Ok(updated)
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
                    let _ = app_h.emit(
                        "ovr_desktop_translation",
                        serde_json::json!({
                            "original": &original,
                            "translated": &translated,
                        }),
                    );
                    let _ = app_h.emit(
                        "ovr_log",
                        format!(
                            "[OVR] ✅ 桌面翻译完成: {}",
                            &translated[..translated.len().min(50)]
                        ),
                    );
                }
                Err(e) => {
                    let _ = app_h.emit("ovr_log", format!("[OVR] ❌ 桌面扫描失败: {}", e));
                    let _ = app_h.emit(
                        "ovr_desktop_translation",
                        serde_json::json!({
                            "original": "",
                            "translated": format!("❌ {}", e),
                        }),
                    );
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

// ===== VRCDog Space Translation Commands =====

#[tauri::command]
pub async fn ovr_set_gravity_enabled(
    _app_handle: AppHandle,
    state: tauri::State<'_, OvrState>,
    enabled: bool,
) -> crate::AppResult<()> {
    let tx = state.cmd_tx.lock().await;
    if let Some(ref sender) = *tx {
        let _ = sender.send(OvrCommand::SetGravityEnabled(enabled));
    }
    Ok(())
}

#[tauri::command]
pub async fn ovr_set_gravity_strength(
    _app_handle: AppHandle,
    state: tauri::State<'_, OvrState>,
    strength: f32,
) -> crate::AppResult<()> {
    let tx = state.cmd_tx.lock().await;
    if let Some(ref sender) = *tx {
        let _ = sender.send(OvrCommand::SetGravityStrength(strength));
    }
    Ok(())
}

#[tauri::command]
pub async fn ovr_set_snap_turn_angle(
    _app_handle: AppHandle,
    state: tauri::State<'_, OvrState>,
    angle: i32,
) -> crate::AppResult<()> {
    let tx = state.cmd_tx.lock().await;
    if let Some(ref sender) = *tx {
        let _ = sender.send(OvrCommand::SetSnapTurnAngle(angle));
    }
    Ok(())
}

#[tauri::command]
pub async fn ovr_set_lock_axis(
    _app_handle: AppHandle,
    state: tauri::State<'_, OvrState>,
    axis: String,
    enabled: bool,
) -> crate::AppResult<()> {
    let tx = state.cmd_tx.lock().await;
    if let Some(ref sender) = *tx {
        let cmd = match axis.as_str() {
            "x" => OvrCommand::SetLockXEnabled(enabled),
            "y" => OvrCommand::SetLockYEnabled(enabled),
            "z" => OvrCommand::SetLockZEnabled(enabled),
            _ => return Ok(()),
        };
        let _ = sender.send(cmd);
    }
    Ok(())
}

// ===== Dynamic Screenshot/Capture Commands =====

#[tauri::command]
pub async fn ovr_set_capture_mode(
    _app_handle: AppHandle,
    state: tauri::State<'_, OvrState>,
    mode: String,
) -> crate::AppResult<()> {
    let tx = state.cmd_tx.lock().await;
    if let Some(ref sender) = *tx {
        let _ = sender.send(OvrCommand::SetCaptureMode(mode));
    }
    Ok(())
}

#[tauri::command]
pub async fn ovr_set_capture_auto_save(
    _app_handle: AppHandle,
    state: tauri::State<'_, OvrState>,
    enabled: bool,
) -> crate::AppResult<()> {
    let tx = state.cmd_tx.lock().await;
    if let Some(ref sender) = *tx {
        let _ = sender.send(OvrCommand::SetCaptureAutoSave(enabled));
    }
    Ok(())
}

#[tauri::command]
pub async fn ovr_set_capture_format(
    _app_handle: AppHandle,
    state: tauri::State<'_, OvrState>,
    format: String,
) -> crate::AppResult<()> {
    let tx = state.cmd_tx.lock().await;
    if let Some(ref sender) = *tx {
        let _ = sender.send(OvrCommand::SetCaptureFormat(format));
    }
    Ok(())
}

#[tauri::command]
pub async fn ovr_set_fling_strength(
    _app_handle: AppHandle,
    state: tauri::State<'_, OvrState>,
    strength: f32,
) -> crate::AppResult<()> {
    let tx = state.cmd_tx.lock().await;
    if let Some(ref sender) = *tx {
        let _ = sender.send(OvrCommand::SetFlingStrength(strength));
    }
    Ok(())
}

#[tauri::command]
pub async fn ovr_set_smooth_turn_rate(
    _app_handle: AppHandle,
    state: tauri::State<'_, OvrState>,
    rate: i32,
) -> crate::AppResult<()> {
    let tx = state.cmd_tx.lock().await;
    if let Some(ref sender) = *tx {
        let _ = sender.send(OvrCommand::SetSmoothTurnRate(rate));
    }
    Ok(())
}

#[tauri::command]
pub async fn ovr_set_smooth_turn_enabled(
    _app_handle: AppHandle,
    state: tauri::State<'_, OvrState>,
    enabled: bool,
) -> crate::AppResult<()> {
    let tx = state.cmd_tx.lock().await;
    if let Some(ref sender) = *tx {
        let _ = sender.send(OvrCommand::SetSmoothTurnEnabled(enabled));
    }
    Ok(())
}

#[tauri::command]
pub async fn ovr_set_drag_multiplier(
    _app_handle: AppHandle,
    state: tauri::State<'_, OvrState>,
    mult: f32,
) -> crate::AppResult<()> {
    let tx = state.cmd_tx.lock().await;
    if let Some(ref sender) = *tx {
        let _ = sender.send(OvrCommand::SetDragMultiplier(mult));
    }
    Ok(())
}

#[tauri::command]
pub async fn ovr_set_comfort_turn_enabled(
    _app_handle: AppHandle,
    state: tauri::State<'_, OvrState>,
    enabled: bool,
) -> crate::AppResult<()> {
    let tx = state.cmd_tx.lock().await;
    if let Some(ref sender) = *tx {
        let _ = sender.send(OvrCommand::SetComfortTurnEnabled(enabled));
    }
    Ok(())
}

#[tauri::command]
pub async fn ovr_set_capture_quality(
    _app_handle: AppHandle,
    state: tauri::State<'_, OvrState>,
    quality: String,
) -> crate::AppResult<()> {
    let tx = state.cmd_tx.lock().await;
    if let Some(ref sender) = *tx {
        let _ = sender.send(OvrCommand::SetCaptureQuality(quality));
    }
    Ok(())
}
