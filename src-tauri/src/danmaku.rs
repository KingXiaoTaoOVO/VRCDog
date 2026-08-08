use ab_glyph::{point, Font, FontVec, PxScale, ScaleFont};
use brotli::Decompressor as BrotliDecompressor;
use flate2::read::ZlibDecoder;
use futures_util::{SinkExt, StreamExt};
use openvr_sys as vr_sys;
use rosc::{OscMessage, OscPacket, OscType};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::ffi::{CStr, CString};
use std::io::Read;
use std::net::UdpSocket as StdUdpSocket;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, State};
use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::header::{COOKIE, ORIGIN, USER_AGENT};
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::tungstenite::Message;

lazy_static::lazy_static! {
    // Bilibili WBI signing: the mixin key is derived from the nav API's wbi_img
    // and cached for 10 minutes so we don't hit the nav endpoint on every reconnect.
    static ref WBI_MIXIN_CACHE: Mutex<Option<(String, Instant)>> = Mutex::new(None);
}

const WBI_MIXIN_KEY_ENC_TAB: [usize; 64] = [
    46, 47, 18, 2, 53, 8, 23, 32, 15, 50, 10, 31, 58, 3, 45, 35, 27, 43, 5, 49, 33, 9, 42, 19,
    29, 28, 14, 39, 12, 38, 41, 13, 37, 48, 7, 16, 24, 55, 40, 61, 26, 17, 0, 1, 60, 51, 30, 4,
    22, 25, 54, 21, 56, 59, 6, 63, 57, 62, 11, 36, 20, 34, 44, 52,
];

const DEFAULT_BILI_WS_HOST: &str = "broadcastlv.chat.bilibili.com";
const BILI_OP_HEARTBEAT: u32 = 2;
const BILI_OP_HEARTBEAT_REPLY: u32 = 3;
const BILI_OP_MESSAGE: u32 = 5;
const BILI_OP_AUTH: u32 = 7;
const BILI_OP_AUTH_REPLY: u32 = 8;
const DANMAKU_OVERLAY_WIDTH: u32 = 640;
const DANMAKU_OVERLAY_HEIGHT: u32 = 720;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DanmakuMessage {
    pub id: u64,
    pub source: String,
    pub message_type: String,
    pub user: String,
    pub text: String,
    pub price: Option<f64>,
    pub gift_count: Option<u32>,
    pub medal_name: Option<String>,
    pub medal_level: Option<u32>,
    pub guard_level: Option<u32>,
    pub timestamp_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DanmakuConfig {
    #[serde(default = "default_true")]
    pub enable_bilibili: bool,
    #[serde(default)]
    pub room_id: u64,
    #[serde(default)]
    pub bili_sessdata: String,
    #[serde(default = "default_true")]
    pub enable_osc_input: bool,
    #[serde(default = "default_osc_host")]
    pub osc_input_host: String,
    #[serde(default = "default_osc_input_port")]
    pub osc_input_port: u16,
    #[serde(default = "default_osc_input_address")]
    pub osc_input_address: String,
    #[serde(default)]
    pub enable_osc_output: bool,
    #[serde(default = "default_osc_host")]
    pub osc_output_host: String,
    #[serde(default = "default_osc_output_port")]
    pub osc_output_port: u16,
    #[serde(default = "default_osc_output_address")]
    pub osc_output_address: String,
    #[serde(default)]
    pub enable_vrc_chatbox: bool,
    #[serde(default = "default_osc_output_port")]
    pub vrc_chatbox_port: u16,
    #[serde(default = "default_chatbox_interval_ms")]
    pub chatbox_interval_ms: u64,
    #[serde(default = "default_true")]
    pub enable_vr_overlay: bool,
    #[serde(default = "default_true")]
    pub overlay_visible: bool,
    #[serde(default)]
    pub vr_menu_visible: bool,
    #[serde(default = "default_attach_mode")]
    pub attach_mode: String,
    #[serde(default = "default_toggle_hand")]
    pub toggle_hand: String,
    #[serde(default = "default_overlay_x")]
    pub x: f32,
    #[serde(default = "default_overlay_y")]
    pub y: f32,
    #[serde(default = "default_overlay_z")]
    pub z: f32,
    #[serde(default)]
    pub pitch: f32,
    #[serde(default = "default_overlay_yaw")]
    pub yaw: f32,
    #[serde(default)]
    pub roll: f32,
    #[serde(default = "default_overlay_width")]
    pub overlay_width_m: f32,
    #[serde(default = "default_overlay_alpha")]
    pub overlay_alpha: f32,
    #[serde(default = "default_bg_alpha")]
    pub bg_alpha: f32,
    #[serde(default = "default_font_size")]
    pub font_size: f32,
    #[serde(default = "default_text_color")]
    pub text_color: String,
    #[serde(default = "default_bg_color")]
    pub bg_color: String,
    #[serde(default = "default_max_messages")]
    pub max_messages: usize,
    #[serde(default = "default_true")]
    pub show_danmaku: bool,
    #[serde(default = "default_true")]
    pub show_gift: bool,
    #[serde(default = "default_true")]
    pub show_enter: bool,
    #[serde(default = "default_true")]
    pub show_follow: bool,
    #[serde(default = "default_true")]
    pub show_guard: bool,
    #[serde(default = "default_true")]
    pub show_sc: bool,
    #[serde(default)]
    pub vr_input_text: String,
}

impl Default for DanmakuConfig {
    fn default() -> Self {
        Self {
            enable_bilibili: true,
            room_id: 0,
            bili_sessdata: String::new(),
            enable_osc_input: true,
            osc_input_host: default_osc_host(),
            osc_input_port: default_osc_input_port(),
            osc_input_address: default_osc_input_address(),
            enable_osc_output: false,
            osc_output_host: default_osc_host(),
            osc_output_port: default_osc_output_port(),
            osc_output_address: default_osc_output_address(),
            enable_vrc_chatbox: false,
            vrc_chatbox_port: default_osc_output_port(),
            chatbox_interval_ms: default_chatbox_interval_ms(),
            enable_vr_overlay: true,
            overlay_visible: true,
            vr_menu_visible: false,
            attach_mode: default_attach_mode(),
            toggle_hand: default_toggle_hand(),
            x: default_overlay_x(),
            y: default_overlay_y(),
            z: default_overlay_z(),
            pitch: 0.0,
            yaw: default_overlay_yaw(),
            roll: 0.0,
            overlay_width_m: default_overlay_width(),
            overlay_alpha: default_overlay_alpha(),
            bg_alpha: default_bg_alpha(),
            font_size: default_font_size(),
            text_color: default_text_color(),
            bg_color: default_bg_color(),
            max_messages: default_max_messages(),
            show_danmaku: true,
            show_gift: true,
            show_enter: true,
            show_follow: true,
            show_guard: true,
            show_sc: true,
            vr_input_text: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct DanmakuStatus {
    pub running: bool,
    pub bili_connected: bool,
    pub osc_input_running: bool,
    pub vr_initialized: bool,
    pub overlay_visible: bool,
    pub vr_menu_visible: bool,
    pub room_id: u64,
    pub online: u64,
    pub message_count: usize,
    pub last_error: String,
    pub last_event: String,
    pub vr_input_text: String,
    pub vr_keyboard_open: bool,
}

pub struct DanmakuState {
    config: Arc<Mutex<DanmakuConfig>>,
    status: Arc<Mutex<DanmakuStatus>>,
    messages: Arc<Mutex<VecDeque<DanmakuMessage>>>,
    stop: Arc<AtomicBool>,
    next_id: Arc<AtomicU64>,
    aggregator_handle: Arc<Mutex<Option<JoinHandle<()>>>>,
    source_handles: Arc<Mutex<Vec<JoinHandle<()>>>>,
    vr_handle: Arc<Mutex<Option<JoinHandle<()>>>>,
}

#[derive(Clone)]
struct DanmakuRuntime {
    app: AppHandle,
    config: Arc<Mutex<DanmakuConfig>>,
    status: Arc<Mutex<DanmakuStatus>>,
    messages: Arc<Mutex<VecDeque<DanmakuMessage>>>,
    stop: Arc<AtomicBool>,
    next_id: Arc<AtomicU64>,
}

impl DanmakuState {
    pub fn new() -> Self {
        Self {
            config: Arc::new(Mutex::new(DanmakuConfig::default())),
            status: Arc::new(Mutex::new(DanmakuStatus::default())),
            messages: Arc::new(Mutex::new(VecDeque::new())),
            stop: Arc::new(AtomicBool::new(true)),
            next_id: Arc::new(AtomicU64::new(1)),
            aggregator_handle: Arc::new(Mutex::new(None)),
            source_handles: Arc::new(Mutex::new(Vec::new())),
            vr_handle: Arc::new(Mutex::new(None)),
        }
    }

    fn runtime(&self, app: AppHandle) -> DanmakuRuntime {
        DanmakuRuntime {
            app,
            config: self.config.clone(),
            status: self.status.clone(),
            messages: self.messages.clone(),
            stop: self.stop.clone(),
            next_id: self.next_id.clone(),
        }
    }
}

impl Default for DanmakuState {
    fn default() -> Self {
        Self::new()
    }
}

fn default_true() -> bool {
    true
}
fn default_osc_host() -> String {
    "127.0.0.1".to_string()
}
fn default_osc_input_port() -> u16 {
    9011
}
fn default_osc_output_port() -> u16 {
    9000
}
fn default_osc_input_address() -> String {
    "/vrcdog/danmaku".to_string()
}
fn default_osc_output_address() -> String {
    "/vrcdog/danmaku".to_string()
}
fn default_chatbox_interval_ms() -> u64 {
    1600
}
fn default_attach_mode() -> String {
    "hmd".to_string()
}
fn default_toggle_hand() -> String {
    "left".to_string()
}
fn default_overlay_x() -> f32 {
    -0.4
}
fn default_overlay_y() -> f32 {
    0.1
}
fn default_overlay_z() -> f32 {
    -0.8
}
fn default_overlay_yaw() -> f32 {
    15.0
}
fn default_overlay_width() -> f32 {
    0.4
}
fn default_overlay_alpha() -> f32 {
    0.92
}
fn default_bg_alpha() -> f32 {
    0.85
}
fn default_font_size() -> f32 {
    14.0
}
fn default_text_color() -> String {
    "#FFFFFF".to_string()
}
fn default_bg_color() -> String {
    "#10141F".to_string()
}
fn default_max_messages() -> usize {
    50
}

#[tauri::command]
pub fn danmaku_get_config(state: State<'_, DanmakuState>) -> crate::AppResult<DanmakuConfig> {
    state
        .config
        .lock()
        .map(|cfg| cfg.clone())
        .map_err(|_| "danmaku config lock poisoned".into())
}

#[tauri::command]
pub fn danmaku_get_status(state: State<'_, DanmakuState>) -> crate::AppResult<DanmakuStatus> {
    state
        .status
        .lock()
        .map(|status| status.clone())
        .map_err(|_| "danmaku status lock poisoned".into())
}

#[tauri::command]
pub fn danmaku_get_messages(
    state: State<'_, DanmakuState>,
) -> crate::AppResult<Vec<DanmakuMessage>> {
    state
        .messages
        .lock()
        .map(|messages| messages.iter().cloned().collect())
        .map_err(|_| "danmaku messages lock poisoned".into())
}

#[tauri::command]
pub async fn danmaku_set_config(
    app: AppHandle,
    state: State<'_, DanmakuState>,
    config: DanmakuConfig,
) -> crate::AppResult<DanmakuStatus> {
    {
        let mut cfg = state
            .config
            .lock()
            .map_err(|_| crate::AppError::from("danmaku config lock poisoned"))?;
        *cfg = config.clone();
    }
    {
        if let Ok(mut status) = state.status.lock() {
            status.overlay_visible = config.overlay_visible;
            status.vr_menu_visible = config.vr_menu_visible;
            status.room_id = config.room_id;
            status.vr_input_text = config.vr_input_text.clone();
        }
    }
    emit_status(&app, &state.status);
    danmaku_get_status(state)
}

#[tauri::command]
pub async fn danmaku_start(
    app: AppHandle,
    state: State<'_, DanmakuState>,
    config: DanmakuConfig,
) -> crate::AppResult<DanmakuStatus> {
    stop_state(&state).await;

    state.stop.store(false, Ordering::Release);
    {
        let mut cfg = state
            .config
            .lock()
            .map_err(|_| crate::AppError::from("danmaku config lock poisoned"))?;
        *cfg = config.clone();
    }
    {
        if let Ok(mut status) = state.status.lock() {
            *status = DanmakuStatus {
                running: true,
                overlay_visible: config.overlay_visible,
                vr_menu_visible: config.vr_menu_visible,
                room_id: config.room_id,
                vr_input_text: config.vr_input_text.clone(),
                last_event: "started".to_string(),
                ..DanmakuStatus::default()
            };
        }
    }
    emit_status(&app, &state.status);
    emit_log(&app, "Danmaku service starting");

    let runtime = state.runtime(app.clone());
    let (tx, rx) = mpsc::unbounded_channel::<DanmakuMessage>();

    let agg_runtime = runtime.clone();
    let aggregator = tokio::spawn(async move {
        aggregate_messages(agg_runtime, rx).await;
    });
    if let Ok(mut handle) = state.aggregator_handle.lock() {
        *handle = Some(aggregator);
    }

    let mut handles = Vec::new();
    if config.enable_bilibili && config.room_id > 0 {
        let bili_runtime = runtime.clone();
        let bili_tx = tx.clone();
        handles.push(tokio::spawn(async move {
            run_bilibili_source(bili_runtime, bili_tx).await;
        }));
    }

    if config.enable_osc_input {
        let osc_runtime = runtime.clone();
        let osc_tx = tx.clone();
        handles.push(tokio::spawn(async move {
            run_osc_input_source(osc_runtime, osc_tx).await;
        }));
    }

    if let Ok(mut guard) = state.source_handles.lock() {
        *guard = handles;
    }

    if config.enable_vr_overlay {
        let vr_runtime = runtime.clone();
        let handle = tokio::task::spawn_blocking(move || {
            run_vr_overlay_thread(vr_runtime);
        });
        if let Ok(mut vr) = state.vr_handle.lock() {
            *vr = Some(handle);
        }
    }

    danmaku_get_status(state)
}

#[tauri::command]
pub async fn danmaku_stop(
    app: AppHandle,
    state: State<'_, DanmakuState>,
) -> crate::AppResult<DanmakuStatus> {
    stop_state(&state).await;
    emit_log(&app, "Danmaku service stopped");
    emit_status(&app, &state.status);
    danmaku_get_status(state)
}

#[tauri::command]
pub fn danmaku_clear_messages(
    app: AppHandle,
    state: State<'_, DanmakuState>,
) -> crate::AppResult<()> {
    {
        let mut messages = state
            .messages
            .lock()
            .map_err(|_| crate::AppError::from("danmaku messages lock poisoned"))?;
        messages.clear();
    }
    {
        if let Ok(mut status) = state.status.lock() {
            status.message_count = 0;
            status.last_event = "messages_cleared".to_string();
        }
    }
    let _ = app.emit("danmaku_cleared", true);
    emit_status(&app, &state.status);
    Ok(())
}

#[tauri::command]
pub fn danmaku_set_overlay_visible(
    app: AppHandle,
    state: State<'_, DanmakuState>,
    visible: bool,
) -> crate::AppResult<DanmakuStatus> {
    {
        let mut cfg = state
            .config
            .lock()
            .map_err(|_| crate::AppError::from("danmaku config lock poisoned"))?;
        cfg.overlay_visible = visible;
        if !visible {
            cfg.vr_menu_visible = false;
        }
    }
    {
        if let Ok(mut status) = state.status.lock() {
            status.overlay_visible = visible;
            if !visible {
                status.vr_menu_visible = false;
            }
            status.last_event = if visible {
                "overlay_visible".to_string()
            } else {
                "overlay_hidden".to_string()
            };
        }
    }
    emit_status(&app, &state.status);
    danmaku_get_status(state)
}

#[tauri::command]
pub fn danmaku_set_vr_input_text(
    app: AppHandle,
    state: State<'_, DanmakuState>,
    text: String,
) -> crate::AppResult<DanmakuStatus> {
    let text = sanitize_text(&text, 80);
    {
        let mut cfg = state
            .config
            .lock()
            .map_err(|_| crate::AppError::from("danmaku config lock poisoned"))?;
        cfg.vr_input_text = text.clone();
    }
    {
        if let Ok(mut status) = state.status.lock() {
            status.vr_input_text = text;
            status.last_event = "vr_input_updated".to_string();
        }
    }
    emit_config(
        &app,
        &state
            .config
            .lock()
            .map(|cfg| cfg.clone())
            .unwrap_or_default(),
    );
    emit_status(&app, &state.status);
    danmaku_get_status(state)
}

#[tauri::command]
pub fn danmaku_submit_vr_input(
    app: AppHandle,
    state: State<'_, DanmakuState>,
    text: Option<String>,
) -> crate::AppResult<DanmakuMessage> {
    let runtime = state.runtime(app);
    let input = text
        .as_deref()
        .map(|value| sanitize_text(value, 80))
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            runtime
                .config
                .lock()
                .ok()
                .map(|cfg| sanitize_text(&cfg.vr_input_text, 80))
                .filter(|value| !value.trim().is_empty())
        })
        .ok_or_else(|| crate::AppError::from("VR input is empty"))?;

    Ok(submit_vr_input_message(&runtime, &input))
}

#[tauri::command]
pub fn danmaku_send_test(
    app: AppHandle,
    state: State<'_, DanmakuState>,
    message_type: String,
    text: Option<String>,
) -> crate::AppResult<DanmakuMessage> {
    let runtime = state.runtime(app.clone());
    let msg = make_test_message(&runtime, &message_type, text.as_deref());
    {
        let mut messages = state
            .messages
            .lock()
            .map_err(|_| crate::AppError::from("danmaku messages lock poisoned"))?;
        messages.push_back(msg.clone());
        trim_messages(&mut messages, default_max_messages());
    }
    {
        if let Ok(mut status) = state.status.lock() {
            status.message_count += 1;
            status.last_event = "test_message".to_string();
        }
    }
    let _ = app.emit("danmaku_message", &msg);
    emit_status(&app, &state.status);
    Ok(msg)
}

fn submit_vr_input_message(runtime: &DanmakuRuntime, text: &str) -> DanmakuMessage {
    let msg = make_message(runtime, "vr", "input", "VR输入", text);
    let config = runtime
        .config
        .lock()
        .map(|cfg| cfg.clone())
        .unwrap_or_default();

    {
        if let Ok(mut cfg) = runtime.config.lock() {
            cfg.vr_input_text = msg.text.clone();
        }
    }
    {
        if let Ok(mut messages) = runtime.messages.lock() {
            messages.push_back(msg.clone());
            trim_messages(&mut messages, config.max_messages.max(10));
        }
    }
    {
        if let Ok(mut status) = runtime.status.lock() {
            status.message_count += 1;
            status.vr_input_text = msg.text.clone();
            status.vr_keyboard_open = false;
            status.last_event = "vr_input_submitted".to_string();
        }
    }

    let _ = runtime.app.emit("danmaku_message", &msg);
    emit_config(&runtime.app, &config);
    emit_status(&runtime.app, &runtime.status);

    if config.enable_vrc_chatbox {
        let packet = OscPacket::Message(OscMessage {
            addr: "/chatbox/input".to_string(),
            args: vec![
                OscType::String(clip_for_chatbox(&msg.text, 140)),
                OscType::Bool(true),
                OscType::Bool(false),
            ],
        });
        let _ = send_osc_packet(&config.osc_output_host, config.vrc_chatbox_port, packet);
    }

    msg
}

async fn stop_state(state: &DanmakuState) {
    state.stop.store(true, Ordering::Release);

    let aggregator = state
        .aggregator_handle
        .lock()
        .ok()
        .and_then(|mut guard| guard.take());
    if let Some(handle) = aggregator {
        handle.abort();
        let _ = handle.await;
    }

    let handles = state
        .source_handles
        .lock()
        .map(|mut guard| guard.drain(..).collect::<Vec<_>>())
        .unwrap_or_default();
    for handle in handles {
        handle.abort();
        let _ = handle.await;
    }

    let vr_handle = state
        .vr_handle
        .lock()
        .ok()
        .and_then(|mut guard| guard.take());
    if let Some(handle) = vr_handle {
        let _ = handle.await;
    }

    if let Ok(mut status) = state.status.lock() {
        status.running = false;
        status.bili_connected = false;
        status.osc_input_running = false;
        status.vr_initialized = false;
        status.overlay_visible = false;
        status.vr_menu_visible = false;
        status.vr_keyboard_open = false;
        status.last_event = "stopped".to_string();
    }
}

async fn aggregate_messages(
    runtime: DanmakuRuntime,
    mut rx: mpsc::UnboundedReceiver<DanmakuMessage>,
) {
    let mut last_chatbox_sent = Instant::now()
        .checked_sub(Duration::from_millis(default_chatbox_interval_ms()))
        .unwrap_or_else(Instant::now);

    while let Some(message) = rx.recv().await {
        if runtime.stop.load(Ordering::Acquire) {
            break;
        }

        let config = runtime
            .config
            .lock()
            .map(|cfg| cfg.clone())
            .unwrap_or_default();

        {
            if let Ok(mut messages) = runtime.messages.lock() {
                if should_skip_duplicate(&messages, &message) {
                    continue;
                }
                if merge_recent_gift(&mut messages, &message) {
                    if let Ok(mut status) = runtime.status.lock() {
                        status.last_event = "message:gift_merged".to_string();
                    }
                    let _ = runtime.app.emit("danmaku_message", &message);
                    emit_status(&runtime.app, &runtime.status);
                    send_external_osc_outputs(&config, &message, &mut last_chatbox_sent);
                    continue;
                }
                messages.push_back(message.clone());
                trim_messages(&mut messages, config.max_messages.max(10));
            }
        }

        {
            if let Ok(mut status) = runtime.status.lock() {
                status.message_count += 1;
                status.last_event = format!("message:{}", message.message_type);
            }
        }

        let _ = runtime.app.emit("danmaku_message", &message);
        emit_status(&runtime.app, &runtime.status);
        send_external_osc_outputs(&config, &message, &mut last_chatbox_sent);
    }
}

fn trim_messages(messages: &mut VecDeque<DanmakuMessage>, max_messages: usize) {
    while messages.len() > max_messages {
        messages.pop_front();
    }
}

fn should_skip_duplicate(messages: &VecDeque<DanmakuMessage>, message: &DanmakuMessage) -> bool {
    if message.source != "bilibili" {
        return false;
    }
    if !matches!(
        message.message_type.as_str(),
        "sc" | "enter" | "follow" | "guard" | "vip_enter"
    ) {
        return false;
    }
    messages.iter().rev().take(12).any(|previous| {
        previous.source == message.source
            && previous.message_type == message.message_type
            && previous.user == message.user
            && previous.text == message.text
            && (message.timestamp_ms - previous.timestamp_ms).abs() <= 5_000
    })
}

fn merge_recent_gift(messages: &mut VecDeque<DanmakuMessage>, message: &DanmakuMessage) -> bool {
    if message.source != "bilibili" || message.message_type != "gift" {
        return false;
    }
    let Some((gift_name, count)) = parse_gift_text(&message.text) else {
        return false;
    };
    for previous in messages.iter_mut().rev().take(12) {
        if previous.source == "bilibili"
            && previous.message_type == "gift"
            && previous.user == message.user
            && (message.timestamp_ms - previous.timestamp_ms).abs() <= 5_000
        {
            if let Some((previous_gift, previous_count)) = parse_gift_text(&previous.text) {
                if previous_gift == gift_name {
                    let next_count = previous_count.saturating_add(count).max(count);
                    previous.text = format!("{gift_name} x{next_count}");
                    previous.gift_count = Some(next_count);
                    previous.timestamp_ms = message.timestamp_ms;
                    return true;
                }
            }
        }
    }
    false
}

fn parse_gift_text(text: &str) -> Option<(String, u32)> {
    let (name, count) = text.rsplit_once(" x")?;
    let count = count.parse::<u32>().ok()?;
    Some((name.to_string(), count))
}

async fn run_bilibili_source(runtime: DanmakuRuntime, tx: mpsc::UnboundedSender<DanmakuMessage>) {
    let mut reconnect_count = 0u32;
    let mut last_logged_error = String::new();
    let mut repeated_error_count = 0u32;

    while !runtime.stop.load(Ordering::Acquire) {
        match run_bilibili_once(runtime.clone(), tx.clone()).await {
            Ok(()) => {
                last_logged_error.clear();
                repeated_error_count = 0;
                // 连接正常关闭但未被要求停止，短暂等待并标记断开，避免立即重连造成空转
                reconnect_count += 1;
                set_status(&runtime, |status| {
                    status.bili_connected = false;
                    status.last_event = "bilibili_disconnected".to_string();
                });
                emit_status(&runtime.app, &runtime.status);
                if runtime.stop.load(Ordering::Acquire) {
                    break;
                }
                let delay = Duration::from_secs((3 * reconnect_count.min(10)) as u64);
                tokio::time::sleep(delay).await;
            }
            Err(err) => {
                reconnect_count += 1;
                set_status(&runtime, |status| {
                    status.bili_connected = false;
                    status.last_error = err.clone();
                    status.last_event = "bilibili_error".to_string();
                });
                if err == last_logged_error {
                    repeated_error_count = repeated_error_count.saturating_add(1);
                    if repeated_error_count == 10 {
                        emit_log(&runtime.app, "Bilibili 连接仍在重试，请稍候。");
                        repeated_error_count = 0;
                    }
                } else {
                    last_logged_error = err.clone();
                    repeated_error_count = 0;
                    emit_log(&runtime.app, "Bilibili 连接未完成，正在自动重试。");
                }
                let delay = Duration::from_secs((3 * reconnect_count.min(10)) as u64);
                tokio::time::sleep(delay).await;
            }
        }
    }

    set_status(&runtime, |status| {
        status.bili_connected = false;
    });
    emit_status(&runtime.app, &runtime.status);
}

/// Translate raw tokio-tungstenite errors into a calm, user-facing message.
/// Bilibili's danmaku servers frequently reset idle/congested connections; the
/// caller already reconnects, so we must not surface the scary raw string
/// ("Connection reset without closing handshake") to the UI.
fn friendly_bili_ws_error(err: &str) -> String {
    let lower = err.to_lowercase();
    if lower.contains("reset")
        || lower.contains("without closing handshake")
        || lower.contains("connection aborted")
        || lower.contains("broken pipe")
        || lower.contains("connection reset")
        || lower.contains("closed")
        || lower.contains("ended")
    {
        "Bilibili 弹幕连接已断开，正在自动重连…".to_string()
    } else {
        err.to_string()
    }
}

async fn run_bilibili_once(
    runtime: DanmakuRuntime,
    tx: mpsc::UnboundedSender<DanmakuMessage>,
) -> Result<(), String> {
    let cfg = runtime
        .config
        .lock()
        .map(|cfg| cfg.clone())
        .unwrap_or_default();

    if cfg.room_id == 0 {
        return Ok(());
    }

    let client = reqwest::Client::new();
    let room_info = resolve_bili_room_id(&client, cfg.room_id, &cfg.bili_sessdata).await?;
    let real_room_id = room_info.room_id;
    if room_info.live_status == 0 {
        set_status(&runtime, |status| {
            status.room_id = real_room_id;
            status.last_error.clear();
            status.last_event = "bilibili_room_waiting_live".to_string();
        });
        emit_status(&runtime.app, &runtime.status);
        emit_log(
            &runtime.app,
            &format!("直播间 {real_room_id} 当前未开播，已继续连接弹幕服务器并等待开播。"),
        );
    }
    let (token, host, port) =
        get_bili_danmaku_endpoint(&client, real_room_id, &cfg.bili_sessdata).await?;
    let url = format!("wss://{host}:{port}/sub");

    let mut request = url
        .as_str()
        .into_client_request()
        .map_err(|e| e.to_string())?;
    request.headers_mut().insert(
        USER_AGENT,
        HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) VrcDog/5.0"),
    );
    request.headers_mut().insert(
        ORIGIN,
        HeaderValue::from_static("https://live.bilibili.com"),
    );
    if !cfg.bili_sessdata.trim().is_empty() {
        let cookie = format!("SESSDATA={}", cfg.bili_sessdata.trim());
        if let Ok(value) = HeaderValue::from_str(&cookie) {
            request.headers_mut().insert(COOKIE, value);
        }
    }

    emit_log(
        &runtime.app,
        &format!("Connecting Bilibili live room {real_room_id} via {host}:{port}"),
    );
    let (stream, _) = tokio_tungstenite::connect_async(request)
        .await
        .map_err(|e| friendly_bili_ws_error(&e.to_string()))?;
    let (mut writer, mut reader) = stream.split();

    let auth_body = serde_json::json!({
        "uid": 0,
        "roomid": real_room_id,
        "protover": 3,
        "platform": "web",
        "type": 2,
        "buvid": make_bili_buvid(),
        "key": token,
    });
    let auth = encode_bili_packet(BILI_OP_AUTH, 1, auth_body.to_string().as_bytes());
    writer
        .send(Message::Binary(auth))
        .await
        .map_err(|e| e.to_string())?;
    writer
        .send(Message::Binary(encode_bili_packet(
            BILI_OP_HEARTBEAT,
            1,
            b"[object Object]",
        )))
        .await
        .map_err(|e| e.to_string())?;

    set_status(&runtime, |status| {
        status.room_id = real_room_id;
        status.bili_connected = false;
        status.last_error.clear();
        status.last_event = "bilibili_authenticating".to_string();
    });
    emit_status(&runtime.app, &runtime.status);

    let mut heartbeat = tokio::time::interval(Duration::from_secs(30));
    loop {
        if runtime.stop.load(Ordering::Acquire) {
            break;
        }

        tokio::select! {
            _ = heartbeat.tick() => {
                writer
                    .send(Message::Binary(encode_bili_packet(BILI_OP_HEARTBEAT, 1, b"[object Object]")))
                    .await
                    .map_err(|e| e.to_string())?;
            }
            next = reader.next() => {
                match next {
                    Some(Ok(Message::Binary(bytes))) => {
                        let frames = collect_bili_frames(&bytes);
                        for frame in frames {
                            handle_bili_frame(&runtime, &tx, frame).await?;
                        }
                    }
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
                            handle_bili_value(&runtime, &tx, value).await;
                        }
                    }
                    Some(Ok(Message::Close(_))) => return Err(friendly_bili_ws_error("bilibili websocket closed")),
                    Some(Ok(_)) => {}
                    Some(Err(err)) => return Err(friendly_bili_ws_error(&err.to_string())),
                    None => return Err(friendly_bili_ws_error("bilibili websocket ended")),
                }
            }
        }
    }

    Ok(())
}

struct BiliRoomInfo {
    room_id: u64,
    live_status: i64,
}

async fn resolve_bili_room_id(
    client: &reqwest::Client,
    room_id: u64,
    sessdata: &str,
) -> Result<BiliRoomInfo, String> {
    let url = format!("https://api.live.bilibili.com/room/v1/Room/room_init?id={room_id}");
    let body: serde_json::Value = client
        .get(url)
        .headers(crate::bilibili::make_headers(sessdata))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    if body["code"].as_i64() != Some(0) {
        return Err(body["message"]
            .as_str()
            .unwrap_or("room_init failed")
            .to_string());
    }

    Ok(BiliRoomInfo {
        room_id: body["data"]["room_id"].as_u64().unwrap_or(room_id),
        live_status: body["data"]["live_status"].as_i64().unwrap_or(1),
    })
}

async fn get_bili_danmaku_endpoint(
    client: &reqwest::Client,
    room_id: u64,
    sessdata: &str,
) -> Result<(String, String, u64), String> {
    // Bilibili requires WBI-signed requests for getDanmuInfo since 2023; without the
    // signature it returns code -352 ("request blocked") and the connection loops forever.
    let signed_url = match get_wbi_mixin_cached(client).await {
        Ok(mixin) => {
            let query = wbi_sign(
                &[
                    ("id", room_id.to_string()),
                    ("type", "0".to_string()),
                    ("web_location", "444.8".to_string()),
                ],
                &mixin,
            );
            Some(format!(
                "https://api.live.bilibili.com/xlive/web-room/v1/index/getDanmuInfo?{query}"
            ))
        }
        Err(_) => {
            // WBI key fetch failed (rare); fall through to the legacy endpoint.
            None
        }
    };

    if let Some(url) = signed_url {
        let result: Result<serde_json::Value, String> = async {
            let resp = client
                .get(&url)
                .headers(make_bili_live_headers(sessdata, room_id))
                .send()
                .await
                .map_err(|e| e.to_string())?;
            resp.json::<serde_json::Value>()
                .await
                .map_err(|e| e.to_string())
        }
        .await;
        if let Ok(body) = result {
            if body["code"].as_i64() == Some(0) {
                return extract_bili_endpoint(&body["data"], "host_list");
            }
        }
    }

    // Fallback: legacy getConf endpoint (deprecated but still works for many public rooms).
    get_bili_legacy_danmaku_endpoint(client, room_id, sessdata)
        .await
        .map_err(|fallback_error| {
            format!("getDanmuInfo (WBI) failed and getConf fallback also failed: {fallback_error}")
        })
}

async fn get_bili_legacy_danmaku_endpoint(
    client: &reqwest::Client,
    room_id: u64,
    sessdata: &str,
) -> Result<(String, String, u64), String> {
    let url = format!(
        "https://api.live.bilibili.com/room/v1/Danmu/getConf?room_id={room_id}&platform=pc&player=web"
    );
    let body: serde_json::Value = client
        .get(url)
        .headers(make_bili_live_headers(sessdata, room_id))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    if body["code"].as_i64() != Some(0) {
        return Err(body["message"]
            .as_str()
            .unwrap_or("getConf failed")
            .to_string());
    }

    extract_bili_endpoint(&body["data"], "host_server_list")
}

fn make_bili_live_headers(sessdata: &str, room_id: u64) -> reqwest::header::HeaderMap {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::USER_AGENT,
        reqwest::header::HeaderValue::from_static(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36",
        ),
    );
    headers.insert(
        reqwest::header::ORIGIN,
        reqwest::header::HeaderValue::from_static("https://live.bilibili.com"),
    );
    if let Ok(value) =
        reqwest::header::HeaderValue::from_str(&format!("https://live.bilibili.com/{room_id}"))
    {
        headers.insert(reqwest::header::REFERER, value);
    }
    if !sessdata.trim().is_empty() {
        let cookie = format!("SESSDATA={}", sessdata.trim());
        if let Ok(value) = reqwest::header::HeaderValue::from_str(&cookie) {
            headers.insert(reqwest::header::COOKIE, value);
        }
    }
    headers
}

fn extract_bili_endpoint(
    data: &serde_json::Value,
    host_list_key: &str,
) -> Result<(String, String, u64), String> {
    let token = data["token"].as_str().unwrap_or("").to_string();
    if token.is_empty() {
        return Err("Bilibili danmaku token is empty".to_string());
    }

    let mut host = DEFAULT_BILI_WS_HOST.to_string();
    let mut port = 443u64;

    if let Some(hosts) = data[host_list_key].as_array() {
        let selected = hosts
            .iter()
            .find(|item| item["host"].as_str() == Some(DEFAULT_BILI_WS_HOST))
            .or_else(|| hosts.first());
        if let Some(selected) = selected {
            host = selected["host"]
                .as_str()
                .unwrap_or(DEFAULT_BILI_WS_HOST)
                .to_string();
            port = selected["wss_port"]
                .as_u64()
                .or_else(|| selected["ws_port"].as_u64())
                .unwrap_or(443);
        }
    } else if let Some(value) = data["host"].as_str() {
        host = value.to_string();
        port = data["wss_port"]
            .as_u64()
            .or_else(|| data["port"].as_u64())
            .unwrap_or(443);
    }

    Ok((token, host, port))
}

fn make_bili_buvid() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_nanos())
        .unwrap_or_default();
    let seed = format!("vrcdog-{now}-{}", std::process::id());
    let digest = md5::compute(seed.as_bytes());
    format!("XY{:X}", digest)
}

/// Extract the bare key (filename without extension/query) from a wbi_img URL.
fn strip_wbi_key(url: &str) -> String {
    let name = url.rsplit('/').next().unwrap_or("");
    let name = name.split('?').next().unwrap_or(name);
    let name = name
        .strip_suffix(".png")
        .or_else(|| name.strip_suffix(".jpg"))
        .unwrap_or(name);
    name.to_string()
}

/// Build the 32-char WBI mixin key from the img/sub keys via the fixed permutation.
fn bili_mixin_key(img: &str, sub: &str) -> String {
    let s = format!("{img}{sub}");
    let mut key = String::with_capacity(32);
    for &idx in WBI_MIXIN_KEY_ENC_TAB.iter() {
        if let Some(c) = s.chars().nth(idx) {
            key.push(c);
        }
    }
    key
}

/// Fetch the WBI mixin key from the nav API.
async fn fetch_wbi_mixin(client: &reqwest::Client) -> Result<String, String> {
    let url = "https://api.bilibili.com/x/web-interface/nav";
    let body: serde_json::Value = client
        .get(url)
        .header(
            USER_AGENT,
            HeaderValue::from_static(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36",
            ),
        )
        .header(reqwest::header::REFERER, HeaderValue::from_static("https://www.bilibili.com"))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    if body["code"].as_i64() != Some(0) {
        return Err(format!(
            "获取 Bilibili WBI 密钥失败（{}）",
            body["message"].as_str().unwrap_or("nav")
        ));
    }
    let img = strip_wbi_key(body["data"]["wbi_img"]["img_url"].as_str().unwrap_or(""));
    let sub = strip_wbi_key(body["data"]["wbi_img"]["sub_url"].as_str().unwrap_or(""));
    Ok(bili_mixin_key(&img, &sub))
}

/// Return a cached WBI mixin key, refreshing it if missing or older than 10 minutes.
async fn get_wbi_mixin_cached(client: &reqwest::Client) -> Result<String, String> {
    {
        if let Ok(cache) = WBI_MIXIN_CACHE.lock() {
            if let Some((mixin, expires)) = &*cache {
                if *expires > Instant::now() {
                    return Ok(mixin.clone());
                }
            }
        }
    }
    let mixin = fetch_wbi_mixin(client).await?;
    if let Ok(mut cache) = WBI_MIXIN_CACHE.lock() {
        *cache = Some((mixin.clone(), Instant::now() + Duration::from_secs(600)));
    }
    Ok(mixin)
}

/// Sign query params with WBI: append `wts` (sorted), then `w_rid = md5(query + mixin)`.
/// Bilibili expects the params concatenated as `key=value` (no URL-encoding) before hashing.
fn wbi_sign(params: &[(&str, String)], mixin: &str) -> String {
    let wts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_secs())
        .unwrap_or(0);
    let mut pairs: Vec<(String, String)> = params
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect();
    pairs.push(("wts".to_string(), wts.to_string()));
    pairs.sort_by(|a, b| a.0.cmp(&b.0));
    let query: String = pairs
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("&");
    let w_rid = format!("{:x}", md5::compute(format!("{query}{mixin}")));
    format!("{query}&w_rid={w_rid}")
}

#[derive(Debug)]
struct BiliFrame {
    op: u32,
    version: u16,
    body: Vec<u8>,
}

fn encode_bili_packet(operation: u32, version: u16, body: &[u8]) -> Vec<u8> {
    let packet_len = 16 + body.len() as u32;
    let mut out = Vec::with_capacity(packet_len as usize);
    out.extend_from_slice(&packet_len.to_be_bytes());
    out.extend_from_slice(&(16u16).to_be_bytes());
    out.extend_from_slice(&version.to_be_bytes());
    out.extend_from_slice(&operation.to_be_bytes());
    out.extend_from_slice(&(1u32).to_be_bytes());
    out.extend_from_slice(body);
    out
}

fn collect_bili_frames(bytes: &[u8]) -> Vec<BiliFrame> {
    let mut frames = Vec::new();
    collect_bili_frames_inner(bytes, &mut frames);
    frames
}

fn collect_bili_frames_inner(bytes: &[u8], frames: &mut Vec<BiliFrame>) {
    let mut offset = 0usize;
    while offset + 16 <= bytes.len() {
        let packet_len = u32::from_be_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]) as usize;
        let header_len = u16::from_be_bytes([bytes[offset + 4], bytes[offset + 5]]) as usize;
        let version = u16::from_be_bytes([bytes[offset + 6], bytes[offset + 7]]);
        let op = u32::from_be_bytes([
            bytes[offset + 8],
            bytes[offset + 9],
            bytes[offset + 10],
            bytes[offset + 11],
        ]);

        if packet_len < header_len || offset + packet_len > bytes.len() {
            break;
        }

        let body = &bytes[offset + header_len..offset + packet_len];
        if op == BILI_OP_MESSAGE && version == 2 {
            let mut decoder = ZlibDecoder::new(body);
            let mut decompressed = Vec::new();
            if decoder.read_to_end(&mut decompressed).is_ok() {
                collect_bili_frames_inner(&decompressed, frames);
            }
        } else if op == BILI_OP_MESSAGE && version == 3 {
            let mut decoder = BrotliDecompressor::new(body, 4096);
            let mut decompressed = Vec::new();
            if decoder.read_to_end(&mut decompressed).is_ok() {
                collect_bili_frames_inner(&decompressed, frames);
            }
        } else {
            frames.push(BiliFrame {
                op,
                version,
                body: body.to_vec(),
            });
        }

        offset += packet_len;
    }
}

async fn handle_bili_frame(
    runtime: &DanmakuRuntime,
    tx: &mpsc::UnboundedSender<DanmakuMessage>,
    frame: BiliFrame,
) -> Result<(), String> {
    match frame.op {
        BILI_OP_HEARTBEAT_REPLY => {
            if frame.body.len() >= 4 {
                let online = u32::from_be_bytes([
                    frame.body[0],
                    frame.body[1],
                    frame.body[2],
                    frame.body[3],
                ]) as u64;
                set_status(runtime, |status| {
                    status.online = online;
                    status.last_event = "bilibili_heartbeat".to_string();
                });
                emit_status(&runtime.app, &runtime.status);
            }
        }
        BILI_OP_AUTH_REPLY => match parse_bili_auth_reply(&frame.body) {
            Ok(()) => {
                set_status(runtime, |status| {
                    status.bili_connected = true;
                    status.last_error.clear();
                    status.last_event = "bilibili_auth_ok".to_string();
                });
                emit_status(&runtime.app, &runtime.status);
            }
            Err(message) => {
                let status_message = message.clone();
                set_status(runtime, |status| {
                    status.bili_connected = false;
                    status.last_error = status_message;
                    status.last_event = "bilibili_auth_failed".to_string();
                });
                emit_status(&runtime.app, &runtime.status);
                return Err(message);
            }
        },
        BILI_OP_MESSAGE => {
            if frame.version == 0 || frame.version == 1 {
                if let Ok(value) = serde_json::from_slice::<serde_json::Value>(&frame.body) {
                    handle_bili_value(runtime, tx, value).await;
                }
            }
        }
        _ => {}
    }
    Ok(())
}

fn parse_bili_auth_reply(body: &[u8]) -> Result<(), String> {
    let value = serde_json::from_slice::<serde_json::Value>(body)
        .map_err(|_| "Bilibili 连接未完成，正在重试".to_string())?;
    match value.get("code").and_then(serde_json::Value::as_i64) {
        Some(0) => Ok(()),
        Some(_) | None => Err("Bilibili 连接未完成，正在重试".to_string()),
    }
}

async fn handle_bili_value(
    runtime: &DanmakuRuntime,
    tx: &mpsc::UnboundedSender<DanmakuMessage>,
    value: serde_json::Value,
) {
    let cmd = value["cmd"]
        .as_str()
        .unwrap_or("")
        .split(':')
        .next()
        .unwrap_or("");

    match cmd {
        "DANMU_MSG" => {
            if let Some(msg) = parse_bili_danmaku(runtime, &value) {
                let _ = tx.send(msg);
            }
        }
        "SEND_GIFT" => {
            if let Some(msg) = parse_bili_gift(runtime, &value) {
                let _ = tx.send(msg);
            }
        }
        "SUPER_CHAT_MESSAGE" | "SUPER_CHAT_MESSAGE_NEW" => {
            if let Some(msg) = parse_bili_super_chat(runtime, &value) {
                let _ = tx.send(msg);
            }
        }
        "INTERACT_WORD" | "INTERACT_WORD_V2" => {
            if let Some(msg) = parse_bili_interact(runtime, &value) {
                let _ = tx.send(msg);
            }
        }
        "ENTRY_EFFECT" => {
            let text = value["data"]["data"]["copy_writing"]
                .as_str()
                .unwrap_or("")
                .replace("<%", "")
                .replace("%>", "");
            if !text.trim().is_empty() {
                let _ = tx.send(make_message(runtime, "bilibili", "vip_enter", &text, ""));
            }
        }
        "WARNING" | "CUT_OFF" | "ROOM_LOCK" => {
            let text = value["data"]["data"]["msg"]
                .as_str()
                .or_else(|| value["data"]["msg"].as_str())
                .unwrap_or_else(|| match cmd {
                    "CUT_OFF" => "Live stream cut off",
                    "ROOM_LOCK" => "Room locked",
                    _ => "Live room warning",
                });
            let user = match cmd {
                "CUT_OFF" => "CutOff",
                "ROOM_LOCK" => "RoomLock",
                _ => "Warning",
            };
            let _ = tx.send(make_message(runtime, "bilibili", "warning", user, text));
        }
        "GUARD_BUY" => {
            let data = &value["data"]["data"];
            let user = data["username"].as_str().unwrap_or("VrcDog");
            let guard_level = data["guard_level"].as_u64().unwrap_or(0);
            let guard = match guard_level {
                1 => "Governor",
                2 => "Admiral",
                3 => "Captain",
                _ => data["gift_name"].as_str().unwrap_or("Captain"),
            };
            let mut msg = make_message(
                runtime,
                "bilibili",
                "guard",
                user,
                &format!("opened {guard}"),
            );
            msg.guard_level = Some(guard_level as u32);
            let _ = tx.send(msg);
        }
        "ONLINE_RANK_COUNT" => {
            if let Some(count) = value["data"]["data"]["count"].as_u64() {
                set_status(runtime, |status| {
                    status.online = count;
                });
                emit_status(&runtime.app, &runtime.status);
            }
        }
        "WATCHED_CHANGE" => {
            if let Some(count) = value["data"]["num"].as_u64() {
                set_status(runtime, |status| {
                    status.online = count;
                });
                emit_status(&runtime.app, &runtime.status);
            }
        }
        "ROOM_REAL_TIME_MESSAGE_UPDATE" => {
            if let Some(count) = value["data"]["watched_show"]["num"]
                .as_u64()
                .or_else(|| value["data"]["fans"].as_u64())
            {
                set_status(runtime, |status| {
                    status.online = count;
                });
                emit_status(&runtime.app, &runtime.status);
            }
        }
        "POPULARITY_RED_POCKET_START" => {
            let text = value["data"]["data"]["lot_name"]
                .as_str()
                .or_else(|| value["data"]["lot_name"].as_str())
                .unwrap_or("Red pocket started");
            let _ = tx.send(make_message(
                runtime,
                "bilibili",
                "warning",
                "RedPocket",
                text,
            ));
        }
        _ => {}
    }
}

fn parse_bili_danmaku(
    runtime: &DanmakuRuntime,
    value: &serde_json::Value,
) -> Option<DanmakuMessage> {
    let info = value["info"].as_array()?;
    let text = info.get(1)?.as_str()?.to_string();
    let user = info
        .get(2)
        .and_then(|v| v.get(1))
        .and_then(|v| v.as_str())
        .unwrap_or("VrcDog")
        .to_string();

    let mut msg = make_message(runtime, "bilibili", "danmaku", &user, &text);
    if let Some(medal) = info.get(3).and_then(|v| v.as_array()) {
        if !medal.is_empty() {
            msg.medal_level = medal.first().and_then(|v| v.as_u64()).map(|v| v as u32);
            msg.medal_name = medal.get(1).and_then(|v| v.as_str()).map(|s| s.to_string());
        }
    }
    msg.guard_level = info
        .get(7)
        .and_then(|v| v.as_u64())
        .map(|value| value as u32);
    Some(msg)
}

fn parse_bili_gift(runtime: &DanmakuRuntime, value: &serde_json::Value) -> Option<DanmakuMessage> {
    let data = &value["data"]["data"];
    let user = data["uname"].as_str().unwrap_or("VrcDog");
    let gift = data["giftName"].as_str().unwrap_or("礼物");
    let count = data["combo_num"]
        .as_u64()
        .or_else(|| data["batch_combo_num"].as_u64())
        .or_else(|| data["num"].as_u64())
        .unwrap_or(1) as u32;
    let mut msg = make_message(
        runtime,
        "bilibili",
        "gift",
        user,
        &format!("{gift} x{count}"),
    );
    msg.gift_count = Some(count);
    Some(msg)
}

fn parse_bili_super_chat(
    runtime: &DanmakuRuntime,
    value: &serde_json::Value,
) -> Option<DanmakuMessage> {
    let data = &value["data"]["data"];
    let user = data["user_info"]["uname"].as_str().unwrap_or("VrcDog");
    let text = data["message"].as_str().unwrap_or("");
    let mut msg = make_message(runtime, "bilibili", "sc", user, text);
    msg.price = data["price"].as_f64();
    Some(msg)
}

fn parse_bili_interact(
    runtime: &DanmakuRuntime,
    value: &serde_json::Value,
) -> Option<DanmakuMessage> {
    let data = &value["data"]["data"];
    let pb = &data["pb_decoded"];
    let user = pb["uname"]
        .as_str()
        .or_else(|| pb["user_info"]["base"]["name"].as_str())
        .or_else(|| data["uname"].as_str())
        .or_else(|| data["user_info"]["base"]["name"].as_str())?;
    let msg_type = data["msg_type"]
        .as_u64()
        .or_else(|| pb["msg_type"].as_u64())
        .unwrap_or(1);
    if msg_type == 2 {
        Some(make_message(
            runtime,
            "bilibili",
            "follow",
            user,
            "followed the room",
        ))
    } else {
        Some(make_message(
            runtime,
            "bilibili",
            "enter",
            user,
            "entered the room",
        ))
    }
}

async fn run_osc_input_source(runtime: DanmakuRuntime, tx: mpsc::UnboundedSender<DanmakuMessage>) {
    let cfg = runtime
        .config
        .lock()
        .map(|cfg| cfg.clone())
        .unwrap_or_default();
    let bind = format!("{}:{}", cfg.osc_input_host, cfg.osc_input_port);
    let socket = match UdpSocket::bind(&bind).await {
        Ok(socket) => socket,
        Err(err) => {
            set_status(&runtime, |status| {
                status.osc_input_running = false;
                status.last_error = format!("OSC bind failed: {err}");
            });
            emit_status(&runtime.app, &runtime.status);
            return;
        }
    };

    set_status(&runtime, |status| {
        status.osc_input_running = true;
        status.last_event = "osc_input_running".to_string();
    });
    emit_status(&runtime.app, &runtime.status);
    emit_log(&runtime.app, &format!("OSC input listening on {bind}"));

    let mut buf = vec![0u8; 4096];
    while !runtime.stop.load(Ordering::Acquire) {
        match socket.recv_from(&mut buf).await {
            Ok((size, _addr)) => {
                if let Ok((_remaining, packet)) = rosc::decoder::decode_udp(&buf[..size]) {
                    let cfg = runtime
                        .config
                        .lock()
                        .map(|cfg| cfg.clone())
                        .unwrap_or_default();
                    for msg in osc_packet_to_messages(&runtime, &cfg, packet) {
                        let _ = tx.send(msg);
                    }
                }
            }
            Err(err) => {
                set_status(&runtime, |status| {
                    status.last_error = format!("OSC receive failed: {err}");
                });
                emit_status(&runtime.app, &runtime.status);
                tokio::time::sleep(Duration::from_millis(250)).await;
            }
        }
    }

    set_status(&runtime, |status| {
        status.osc_input_running = false;
    });
    emit_status(&runtime.app, &runtime.status);
}

fn osc_packet_to_messages(
    runtime: &DanmakuRuntime,
    cfg: &DanmakuConfig,
    packet: OscPacket,
) -> Vec<DanmakuMessage> {
    match packet {
        OscPacket::Message(message) => osc_message_to_danmaku(runtime, cfg, message)
            .into_iter()
            .collect(),
        OscPacket::Bundle(bundle) => bundle
            .content
            .into_iter()
            .flat_map(|packet| osc_packet_to_messages(runtime, cfg, packet))
            .collect(),
    }
}

fn osc_message_to_danmaku(
    runtime: &DanmakuRuntime,
    cfg: &DanmakuConfig,
    message: OscMessage,
) -> Option<DanmakuMessage> {
    let accepts = cfg.osc_input_address == "*"
        || message.addr == cfg.osc_input_address
        || message.addr == "/danmaku/message"
        || message.addr == "/live/danmaku"
        || message.addr == "/chatbox/input";
    if !accepts {
        return None;
    }

    let strings = message
        .args
        .iter()
        .filter_map(|arg| match arg {
            OscType::String(value) => Some(value.clone()),
            OscType::Char(value) => Some(value.to_string()),
            _ => None,
        })
        .collect::<Vec<_>>();

    if let Some(first) = strings.first() {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(first) {
            let user = value["user"]
                .as_str()
                .or_else(|| value["name"].as_str())
                .unwrap_or("OSC");
            let text = value["text"]
                .as_str()
                .or_else(|| value["message"].as_str())
                .unwrap_or("");
            let kind = value["type"]
                .as_str()
                .or_else(|| value["message_type"].as_str())
                .or_else(|| value["kind"].as_str())
                .unwrap_or("osc");
            if !text.trim().is_empty() {
                let kind = normalize_message_type(kind);
                let mut msg = make_message(runtime, "osc", &kind, user, text);
                msg.price = value["price"].as_f64();
                msg.gift_count = value["gift_count"]
                    .as_u64()
                    .or_else(|| value["num"].as_u64())
                    .map(|count| count as u32);
                return Some(msg);
            }
        }
    }

    if strings.len() >= 3 {
        let first = strings[0].trim();
        let second = strings[1].trim();
        let third = strings[2].trim();
        let (kind, user, text) = if is_known_message_type(first) {
            (normalize_message_type(first), second, third)
        } else if is_known_message_type(third) {
            (normalize_message_type(third), first, second)
        } else {
            ("osc".to_string(), first, second)
        };
        if !text.is_empty() {
            return Some(make_message(runtime, "osc", &kind, user, text));
        }
    }

    if strings.len() >= 2 {
        let first = strings[0].trim();
        let second = strings[1].trim();
        if !second.is_empty() {
            let (kind, user, text) = if is_known_message_type(first) {
                (normalize_message_type(first), "OSC", second)
            } else {
                ("osc".to_string(), first, second)
            };
            return Some(make_message(runtime, "osc", &kind, user, text));
        }
    }

    if let Some(text) = strings
        .first()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        return Some(make_message(runtime, "osc", "osc", "OSC", text));
    }

    None
}

fn is_known_message_type(value: &str) -> bool {
    let kind = normalize_message_type(value);
    matches!(
        kind.as_str(),
        "danmaku" | "osc" | "gift" | "sc" | "enter" | "follow" | "guard" | "vip_enter" | "warning"
    )
}

fn normalize_message_type(value: &str) -> String {
    let normalized = value.trim().to_ascii_lowercase().replace(['-', ' '], "_");
    match normalized.as_str() {
        "danmu" | "danmaku_msg" | "message" | "msg" | "comment" => "danmaku",
        "superchat" | "super_chat" | "super_chat_message" => "sc",
        "join" | "entry" | "entered" | "welcome" => "enter",
        "fan" | "fans" | "subscribe" | "sub" => "follow",
        "captain" | "member" | "membership" => "guard",
        "vip" | "entry_effect" => "vip_enter",
        "warn" | "error" | "cut_off" | "room_lock" => "warning",
        "present" | "gift_message" => "gift",
        "" => "osc",
        _ => normalized.as_str(),
    }
    .to_string()
}

fn send_external_osc_outputs(
    cfg: &DanmakuConfig,
    msg: &DanmakuMessage,
    last_chatbox_sent: &mut Instant,
) {
    if cfg.enable_osc_output {
        let packet = OscPacket::Message(OscMessage {
            addr: cfg.osc_output_address.clone(),
            args: vec![
                OscType::String(msg.source.clone()),
                OscType::String(msg.message_type.clone()),
                OscType::String(msg.user.clone()),
                OscType::String(msg.text.clone()),
            ],
        });
        let _ = send_osc_packet(&cfg.osc_output_host, cfg.osc_output_port, packet);
    }

    if cfg.enable_vrc_chatbox
        && last_chatbox_sent.elapsed() >= Duration::from_millis(cfg.chatbox_interval_ms.max(250))
    {
        let text = clip_for_chatbox(&format_message_for_chatbox(msg), 140);
        let packet = OscPacket::Message(OscMessage {
            addr: "/chatbox/input".to_string(),
            args: vec![
                OscType::String(text),
                OscType::Bool(true),
                OscType::Bool(false),
            ],
        });
        let _ = send_osc_packet(&cfg.osc_output_host, cfg.vrc_chatbox_port, packet);
        *last_chatbox_sent = Instant::now();
    }
}

fn send_osc_packet(host: &str, port: u16, packet: OscPacket) -> Result<(), String> {
    let socket = StdUdpSocket::bind("0.0.0.0:0").map_err(|e| e.to_string())?;
    let buf = rosc::encoder::encode(&packet).map_err(|e| e.to_string())?;
    socket
        .send_to(&buf, format!("{host}:{port}"))
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn make_message(
    runtime: &DanmakuRuntime,
    source: &str,
    message_type: &str,
    user: &str,
    text: &str,
) -> DanmakuMessage {
    DanmakuMessage {
        id: runtime.next_id.fetch_add(1, Ordering::AcqRel),
        source: source.to_string(),
        message_type: message_type.to_string(),
        user: sanitize_text(user, 32),
        text: sanitize_text(text, 512),
        price: None,
        gift_count: None,
        medal_name: None,
        medal_level: None,
        guard_level: None,
        timestamp_ms: now_ms(),
    }
}

fn make_test_message(
    runtime: &DanmakuRuntime,
    message_type: &str,
    custom_text: Option<&str>,
) -> DanmakuMessage {
    match message_type {
        "sc" => {
            let mut msg = make_message(
                runtime,
                "test",
                "sc",
                "TestUser",
                custom_text.unwrap_or("This is a Super Chat preview message."),
            );
            msg.price = Some(30.0);
            msg
        }
        "gift" => {
            let mut msg = make_message(runtime, "test", "gift", "TestUser", "Small TV x3");
            msg.gift_count = Some(3);
            msg
        }
        "enter" => make_message(runtime, "test", "enter", "TestUser", "entered the room"),
        "follow" => make_message(runtime, "test", "follow", "TestUser", "followed the room"),
        "warning" => make_message(runtime, "test", "warning", "System", "Live warning preview"),
        _ => make_message(
            runtime,
            "test",
            "danmaku",
            "TestUser",
            custom_text.unwrap_or("VrcDog弹幕已接入 VR 视图。"),
        ),
    }
}

fn sanitize_text(input: &str, max_chars: usize) -> String {
    input
        .replace('\r', " ")
        .replace('\n', " ")
        .chars()
        .take(max_chars)
        .collect::<String>()
        .trim()
        .to_string()
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

fn format_message_for_chatbox(msg: &DanmakuMessage) -> String {
    match msg.message_type.as_str() {
        "sc" => format!(
            "[SC{}] {}: {}",
            msg.price
                .map(|price| format!(" 楼{price:.0}"))
                .unwrap_or_default(),
            msg.user,
            msg.text
        ),
        "gift" => format!("[Gift] {} {}", msg.user, msg.text),
        "enter" => format!("[Enter] {}", msg.user),
        "follow" => format!("[Follow] {}", msg.user),
        "guard" | "vip_enter" => format!("[Guard] {} {}", msg.user, msg.text),
        _ => format!("{}: {}", msg.user, msg.text),
    }
}

fn clip_for_chatbox(input: &str, max_chars: usize) -> String {
    input.chars().take(max_chars).collect()
}

fn set_status(runtime: &DanmakuRuntime, update: impl FnOnce(&mut DanmakuStatus)) {
    if let Ok(mut status) = runtime.status.lock() {
        update(&mut status);
    }
}

fn emit_status(app: &AppHandle, status: &Arc<Mutex<DanmakuStatus>>) {
    if let Ok(status) = status.lock() {
        let _ = app.emit("danmaku_status", status.clone());
    }
}

fn emit_config(app: &AppHandle, config: &DanmakuConfig) {
    let _ = app.emit("danmaku_config", config.clone());
}

fn emit_log(app: &AppHandle, message: &str) {
    let _ = app.emit("danmaku_log", message.to_string());
}

fn run_vr_overlay_thread(runtime: DanmakuRuntime) {
    let init_result =
        std::panic::catch_unwind(|| unsafe { openvr::init(openvr::ApplicationType::Overlay) });
    let context = match init_result {
        Ok(Ok(context)) => context,
        Ok(Err(err)) => {
            let err_str = format!("{err:?}");
            // 检测 OpenVR 是否已在当前进程初始化，OpenVR 同一进程只能 init 一次
            let msg = if err_str.to_lowercase().contains("init")
                && !err_str.to_lowercase().contains("not")
                || err_str.to_lowercase().contains("already")
            {
                "SteamVR overlay is already initialized by OVR Translator; release and retry"
                    .to_string()
            } else {
                format!("OpenVR init failed: {err_str}")
            };
            set_status(&runtime, |status| {
                status.vr_initialized = false;
                status.last_error = msg;
            });
            emit_log(
                &runtime.app,
                &format!("[VR Overlay] OpenVR init error: {err_str}"),
            );
            emit_status(&runtime.app, &runtime.status);
            return;
        }
        Err(_) => {
            set_status(&runtime, |status| {
                status.vr_initialized = false;
                status.last_error =
                    "SteamVR overlay is already initialized by OVR Translator; release and retry"
                        .to_string();
            });
            emit_log(
                &runtime.app,
                "[VR Overlay] OpenVR already initialized (panic path)",
            );
            emit_status(&runtime.app, &runtime.status);
            return;
        }
    };

    let font = match load_danmaku_font() {
        Some(font) => font,
        None => {
            set_status(&runtime, |status| {
                status.vr_initialized = false;
                status.last_error = "No usable system font found".to_string();
            });
            emit_status(&runtime.app, &runtime.status);
            return;
        }
    };

    let mut overlay = match context.overlay() {
        Ok(overlay) => overlay,
        Err(err) => {
            set_status(&runtime, |status| {
                status.vr_initialized = false;
                status.last_error = format!("OpenVR overlay interface failed: {err:?}");
            });
            emit_status(&runtime.app, &runtime.status);
            return;
        }
    };

    let handle = match overlay.create_overlay("vrcdog.danmaku\0", "VrcDog弹幕\0") {
        Ok(handle) => handle,
        Err(err) => {
            set_status(&runtime, |status| {
                status.vr_initialized = false;
                status.last_error = format!("Create overlay failed: {err:?}");
            });
            emit_status(&runtime.app, &runtime.status);
            return;
        }
    };
    let menu_handle = match overlay.create_overlay("vrcdog.danmaku.menu\0", "VrcDog弹幕菜单\0")
    {
        Ok(handle) => Some(handle),
        Err(err) => {
            emit_log(
                &runtime.app,
                &format!("[VR Overlay] Create menu overlay failed: {err:?}"),
            );
            None
        }
    };
    let raw_overlay = load_raw_openvr_overlay();

    set_status(&runtime, |status| {
        status.vr_initialized = true;
        status.last_error.clear();
        status.last_event = "vr_overlay_initialized".to_string();
    });
    emit_status(&runtime.app, &runtime.status);
    emit_log(&runtime.app, "SteamVR danmaku overlay initialized");

    let mut previous_pressed = 0u64;
    let mut last_config_emit = Instant::now()
        .checked_sub(Duration::from_secs(1))
        .unwrap_or_else(Instant::now);
    while !runtime.stop.load(Ordering::Acquire) {
        let cfg = runtime
            .config
            .lock()
            .map(|cfg| cfg.clone())
            .unwrap_or_default();
        let status_snapshot = runtime
            .status
            .lock()
            .map(|status| status.clone())
            .unwrap_or_default();
        let messages = runtime
            .messages
            .lock()
            .map(|messages| messages.iter().cloned().collect::<Vec<_>>())
            .unwrap_or_default();

        let _ = overlay.set_width(handle, cfg.overlay_width_m.clamp(0.15, 2.0));
        let _ = overlay.set_opacity(handle, cfg.overlay_alpha.clamp(0.05, 1.0));
        let _ = overlay.set_sort_order(handle, 50);
        apply_overlay_transform(&context, &mut overlay, handle, &cfg);

        if let Ok(sys) = context.system() {
            handle_vr_controller_controls(
                &sys,
                &runtime,
                &cfg,
                &mut previous_pressed,
                &mut last_config_emit,
                raw_overlay,
                menu_handle.or(Some(handle)),
            );
        }
        if let Some(raw_overlay) = raw_overlay {
            poll_vr_keyboard_events(&runtime, raw_overlay, menu_handle.or(Some(handle)));
        }

        let visible = runtime
            .config
            .lock()
            .map(|cfg| cfg.overlay_visible || cfg.toggle_hand == "always_on")
            .unwrap_or(true);

        let pixels = render_live_panel_overlay_clean(&font, &messages, &cfg, &status_snapshot);
        let _ = overlay.set_raw_data(
            handle,
            &pixels,
            DANMAKU_OVERLAY_WIDTH as usize,
            DANMAKU_OVERLAY_HEIGHT as usize,
            4,
        );
        let _ = overlay.set_visibility(handle, visible);
        if let Some(menu_handle) = menu_handle {
            let menu_visible = runtime
                .config
                .lock()
                .map(|cfg| {
                    cfg.vr_menu_visible && (cfg.overlay_visible || cfg.toggle_hand == "always_on")
                })
                .unwrap_or(false);
            let _ = overlay.set_width(menu_handle, 0.52);
            let _ = overlay.set_opacity(menu_handle, 0.96);
            let _ = overlay.set_sort_order(menu_handle, 60);
            apply_menu_transform(&mut overlay, menu_handle);
            let pixels = render_danmaku_menu_overlay(&font, &cfg, &status_snapshot);
            let _ = overlay.set_raw_data(menu_handle, &pixels, 640, 520, 4);
            let _ = overlay.set_visibility(menu_handle, menu_visible);
            set_status(&runtime, |status| {
                status.vr_menu_visible = menu_visible;
            });
        }
        set_status(&runtime, |status| {
            status.overlay_visible = visible;
        });

        std::thread::sleep(Duration::from_millis(80));
    }

    let _ = overlay.set_visibility(handle, false);
    if let Some(menu_handle) = menu_handle {
        let _ = overlay.set_visibility(menu_handle, false);
    }
    set_status(&runtime, |status| {
        status.vr_initialized = false;
        status.overlay_visible = false;
        status.vr_menu_visible = false;
    });
    emit_status(&runtime.app, &runtime.status);
}

fn handle_vr_controller_controls(
    sys: &openvr::System,
    runtime: &DanmakuRuntime,
    cfg_snapshot: &DanmakuConfig,
    previous_pressed: &mut u64,
    last_config_emit: &mut Instant,
    raw_overlay: Option<&'static vr_sys::VR_IVROverlay_FnTable>,
    keyboard_handle: Option<openvr::overlay::OverlayHandle>,
) {
    let Some(state) = selected_controller_state(sys, cfg_snapshot) else {
        *previous_pressed = 0;
        return;
    };
    let pressed = state.button_pressed;
    let grip_mask = 1u64 << openvr::button_id::GRIP;
    let menu_mask = 1u64 << openvr::button_id::APPLICATION_MENU;
    let trigger_mask = 1u64 << openvr::button_id::STEAM_VR_TRIGGER;
    let axis_mask = 1u64 << openvr::button_id::AXIS0;
    let grip_rising = pressed & grip_mask != 0 && *previous_pressed & grip_mask == 0;
    let menu_rising = pressed & menu_mask != 0 && *previous_pressed & menu_mask == 0;
    let trigger_rising = pressed & trigger_mask != 0 && *previous_pressed & trigger_mask == 0;
    let axis_active = pressed & axis_mask != 0 || state.button_touched & axis_mask != 0;
    let trigger_active = pressed & trigger_mask != 0;
    let axis = state.axis[0];
    let mut next_config = None;
    let mut last_event = None;
    let mut axis_adjusted = false;
    let mut open_keyboard = false;

    if let Ok(mut current) = runtime.config.lock() {
        if grip_rising {
            current.overlay_visible = !current.overlay_visible;
            if !current.overlay_visible {
                current.vr_menu_visible = false;
            }
            last_event = Some("overlay_toggled_by_controller");
        }
        if menu_rising {
            current.vr_menu_visible = !current.vr_menu_visible;
            if current.vr_menu_visible {
                current.overlay_visible = true;
            }
            last_event = Some("vr_menu_toggled_by_controller");
        }
        if current.vr_menu_visible
            && trigger_rising
            && !axis_active
            && raw_overlay.is_some()
            && keyboard_handle.is_some()
        {
            open_keyboard = true;
            last_event = Some("vr_keyboard_requested");
        }
        if current.vr_menu_visible && axis_active && (axis.x.abs() > 0.12 || axis.y.abs() > 0.12) {
            let hand_mode = current.attach_mode == "hand" || current.attach_mode == "left_hand";
            if trigger_active {
                let z_step = if hand_mode { 0.004 } else { 0.018 };
                current.z += axis.y * z_step;
                current.overlay_width_m += axis.x * 0.01;
            } else {
                let xy_step = if hand_mode { 0.003 } else { 0.018 };
                current.x += axis.x * xy_step;
                current.y += axis.y * xy_step;
            }
            clamp_vr_danmaku_config(&mut current);
            axis_adjusted = true;
            last_event = Some("vr_menu_position_adjusted");
        }
        if grip_rising || menu_rising || axis_adjusted || open_keyboard {
            next_config = Some(current.clone());
        }
    }

    if open_keyboard {
        if let (Some(raw_overlay), Some(handle)) = (raw_overlay, keyboard_handle) {
            let existing = runtime
                .config
                .lock()
                .map(|cfg| cfg.vr_input_text.clone())
                .unwrap_or_default();
            show_vr_keyboard(runtime, raw_overlay, handle, &existing);
        }
    }

    if let Some(config) = next_config {
        let should_emit =
            !axis_adjusted || last_config_emit.elapsed() >= Duration::from_millis(120);
        set_status(runtime, |status| {
            status.overlay_visible = config.overlay_visible || config.toggle_hand == "always_on";
            status.vr_menu_visible = config.vr_menu_visible && status.overlay_visible;
            if let Some(event) = last_event {
                status.last_event = event.to_string();
            }
        });
        emit_status(&runtime.app, &runtime.status);
        if should_emit {
            emit_config(&runtime.app, &config);
            *last_config_emit = Instant::now();
        }
    }
    *previous_pressed = pressed;
}

fn load_raw_openvr_overlay() -> Option<&'static vr_sys::VR_IVROverlay_FnTable> {
    let mut name = Vec::from(b"FnTable:".as_ref());
    name.extend(vr_sys::IVROverlay_Version);
    let mut error = vr_sys::EVRInitError_VRInitError_None;
    let ptr = unsafe { vr_sys::VR_GetGenericInterface(name.as_ptr() as *const i8, &mut error) };
    if error != vr_sys::EVRInitError_VRInitError_None || ptr == 0 {
        return None;
    }
    Some(unsafe { &*(ptr as *const vr_sys::VR_IVROverlay_FnTable) })
}

fn show_vr_keyboard(
    runtime: &DanmakuRuntime,
    overlay: &'static vr_sys::VR_IVROverlay_FnTable,
    handle: openvr::overlay::OverlayHandle,
    existing_text: &str,
) {
    let Some(show_keyboard) = overlay.ShowKeyboardForOverlay else {
        set_status(runtime, |status| {
            status.last_error = "SteamVR keyboard API is unavailable".to_string();
        });
        emit_status(&runtime.app, &runtime.status);
        return;
    };

    let description = CString::new("VR弹幕输入（可用系统输入法切换中文/EN）")
        .unwrap_or_else(|_| CString::new("VR input").unwrap());
    let existing =
        CString::new(existing_text.replace('\0', "")).unwrap_or_else(|_| CString::new("").unwrap());
    let flags = (vr_sys::EKeyboardFlags_KeyboardFlag_Modal
        | vr_sys::EKeyboardFlags_KeyboardFlag_ShowArrowKeys) as u32;
    let err = unsafe {
        show_keyboard(
            handle.0,
            vr_sys::EGamepadTextInputMode_k_EGamepadTextInputModeNormal,
            vr_sys::EGamepadTextInputLineMode_k_EGamepadTextInputLineModeSingleLine,
            flags,
            description.as_ptr() as *mut _,
            80,
            existing.as_ptr() as *mut _,
            0,
        )
    };

    if err == vr_sys::EVROverlayError_VROverlayError_None {
        set_status(runtime, |status| {
            status.vr_keyboard_open = true;
            status.last_error.clear();
            status.last_event = "vr_keyboard_opened".to_string();
        });
    } else {
        set_status(runtime, |status| {
            status.vr_keyboard_open = false;
            status.last_error = format!("SteamVR keyboard failed: {err}");
        });
    }
    emit_status(&runtime.app, &runtime.status);
}

fn poll_vr_keyboard_events(
    runtime: &DanmakuRuntime,
    overlay: &'static vr_sys::VR_IVROverlay_FnTable,
    handle: Option<openvr::overlay::OverlayHandle>,
) {
    let (Some(handle), Some(poll)) = (handle, overlay.PollNextOverlayEvent) else {
        return;
    };

    loop {
        let mut event: vr_sys::VREvent_t = unsafe { std::mem::zeroed() };
        let has_event = unsafe {
            poll(
                handle.0,
                &mut event,
                std::mem::size_of::<vr_sys::VREvent_t>() as u32,
            )
        };
        if !has_event {
            break;
        }

        match event.eventType {
            event if event == vr_sys::EVREventType_VREvent_KeyboardDone as u32 => {
                if let Some(text) = get_vr_keyboard_text(overlay) {
                    let clean = sanitize_text(&text, 80);
                    if !clean.is_empty() {
                        submit_vr_input_message(runtime, &clean);
                    } else {
                        set_status(runtime, |status| {
                            status.vr_keyboard_open = false;
                            status.last_event = "vr_keyboard_empty".to_string();
                        });
                        emit_status(&runtime.app, &runtime.status);
                    }
                }
            }
            event
                if event == vr_sys::EVREventType_VREvent_KeyboardClosed as u32
                    || event == vr_sys::EVREventType_VREvent_KeyboardClosed_Global as u32 =>
            {
                set_status(runtime, |status| {
                    status.vr_keyboard_open = false;
                    status.last_event = "vr_keyboard_closed".to_string();
                });
                emit_status(&runtime.app, &runtime.status);
            }
            _ => {}
        }
    }
}

fn get_vr_keyboard_text(overlay: &'static vr_sys::VR_IVROverlay_FnTable) -> Option<String> {
    let get_text = overlay.GetKeyboardText?;
    let mut buf = vec![0i8; 512];
    let len = unsafe { get_text(buf.as_mut_ptr(), buf.len() as u32) };
    if len == 0 {
        return Some(String::new());
    }
    let text = unsafe { CStr::from_ptr(buf.as_ptr()) }
        .to_string_lossy()
        .to_string();
    Some(text)
}

fn selected_controller_state(
    sys: &openvr::System,
    cfg: &DanmakuConfig,
) -> Option<openvr::ControllerState> {
    let role = match cfg.toggle_hand.as_str() {
        "right" => openvr::TrackedControllerRole::RightHand,
        _ => openvr::TrackedControllerRole::LeftHand,
    };
    sys.tracked_device_index_for_controller_role(role)
        .and_then(|index| sys.controller_state(index))
}

fn clamp_vr_danmaku_config(cfg: &mut DanmakuConfig) {
    let hand_mode = cfg.attach_mode == "hand" || cfg.attach_mode == "left_hand";
    if hand_mode {
        cfg.x = cfg.x.clamp(-0.1, 0.1);
        cfg.y = cfg.y.clamp(0.0, 0.15);
        cfg.z = cfg.z.clamp(-0.1, 0.1);
    } else {
        cfg.x = cfg.x.clamp(-1.0, 1.0);
        cfg.y = cfg.y.clamp(-0.8, 0.8);
        cfg.z = cfg.z.clamp(-1.5, -0.3);
    }
    cfg.pitch = cfg.pitch.clamp(-30.0, 30.0);
    cfg.yaw = cfg.yaw.clamp(-30.0, 30.0);
    cfg.roll = cfg.roll.clamp(-20.0, 20.0);
    cfg.overlay_width_m = cfg.overlay_width_m.clamp(0.15, 0.8);
    cfg.overlay_alpha = cfg.overlay_alpha.clamp(0.3, 1.0);
    cfg.bg_alpha = cfg.bg_alpha.clamp(0.0, 1.0);
    cfg.font_size = cfg.font_size.clamp(10.0, 20.0);
}

fn apply_overlay_transform(
    context: &openvr::Context,
    overlay: &mut openvr::Overlay,
    handle: openvr::overlay::OverlayHandle,
    cfg: &DanmakuConfig,
) {
    let transform = euler_transform(cfg.x, cfg.y, cfg.z, cfg.pitch, cfg.yaw, cfg.roll);
    if let Ok(sys) = context.system() {
        let role = match cfg.attach_mode.as_str() {
            "left_hand" | "hand" => Some(openvr::TrackedControllerRole::LeftHand),
            "right_hand" => Some(openvr::TrackedControllerRole::RightHand),
            _ => None,
        };
        if let Some(role) = role {
            if let Some(index) = sys.tracked_device_index_for_controller_role(role) {
                let _ = overlay.set_transform_tracked_device_relative(handle, index, &transform);
                return;
            }
        }
    }
    let _ = overlay.set_transform_tracked_device_relative(
        handle,
        openvr::TrackedDeviceIndex(0),
        &transform,
    );
}

fn apply_menu_transform(overlay: &mut openvr::Overlay, handle: openvr::overlay::OverlayHandle) {
    let transform = euler_transform(0.0, -0.28, -0.72, -12.0, 0.0, 0.0);
    let _ = overlay.set_transform_tracked_device_relative(
        handle,
        openvr::TrackedDeviceIndex(0),
        &transform,
    );
}

fn euler_transform(
    x: f32,
    y: f32,
    z: f32,
    pitch_deg: f32,
    yaw_deg: f32,
    roll_deg: f32,
) -> openvr::pose::Matrix3x4 {
    let pitch = pitch_deg.to_radians();
    let yaw = yaw_deg.to_radians();
    let roll = roll_deg.to_radians();
    let (cp, sp) = (pitch.cos(), pitch.sin());
    let (cy, sy) = (yaw.cos(), yaw.sin());
    let (cr, sr) = (roll.cos(), roll.sin());

    openvr::pose::Matrix3x4([
        [cy * cr, -cy * sr, sy, x],
        [sp * sy * cr + cp * sr, -sp * sy * sr + cp * cr, -sp * cy, y],
        [-cp * sy * cr + sp * sr, cp * sy * sr + sp * cr, cp * cy, z],
    ])
}

fn load_danmaku_font() -> Option<FontVec> {
    let paths = [
        r"C:\Windows\Fonts\msyh.ttc",
        r"C:\Windows\Fonts\msyhbd.ttc",
        r"C:\Windows\Fonts\simhei.ttf",
        r"C:\Windows\Fonts\simsun.ttc",
        r"C:\Windows\Fonts\segoeui.ttf",
        r"C:\Windows\Fonts\arial.ttf",
    ];

    for path in paths {
        if let Ok(data) = std::fs::read(path) {
            for index in 0..6u32 {
                if let Ok(font) = FontVec::try_from_vec_and_index(data.clone(), index) {
                    return Some(font);
                }
            }
            if let Ok(font) = FontVec::try_from_vec(data) {
                return Some(font);
            }
        }
    }
    None
}

fn render_live_panel_overlay_clean(
    font: &FontVec,
    messages: &[DanmakuMessage],
    cfg: &DanmakuConfig,
    status: &DanmakuStatus,
) -> Vec<u8> {
    let width = DANMAKU_OVERLAY_WIDTH;
    let height = DANMAKU_OVERLAY_HEIGHT;
    let mut pixels = vec![0u8; (width * height * 4) as usize];
    let panel_alpha = (cfg.bg_alpha.clamp(0.0, 1.0) * 255.0).round() as u8;
    let surface_alpha = (cfg.bg_alpha.clamp(0.0, 1.0) * 245.0).round() as u8;

    fill_rounded_rect(
        &mut pixels,
        width,
        height,
        0,
        0,
        width,
        height,
        14,
        [24, 25, 30, panel_alpha],
    );
    fill_rect(
        &mut pixels,
        width,
        height,
        14,
        0,
        width - 28,
        72,
        [27, 28, 33, panel_alpha],
    );
    fill_rect(
        &mut pixels,
        width,
        height,
        0,
        14,
        width,
        58,
        [27, 28, 33, panel_alpha],
    );
    fill_rect(
        &mut pixels,
        width,
        height,
        0,
        72,
        width,
        1,
        [48, 50, 57, 230],
    );
    fill_rect(
        &mut pixels,
        width,
        height,
        0,
        144,
        width,
        1,
        [48, 50, 57, 220],
    );
    fill_rect(
        &mut pixels,
        width,
        height,
        0,
        302,
        width,
        1,
        [40, 42, 49, 210],
    );

    let title = PxScale::from(22.0);
    let normal = PxScale::from(17.0);
    let small = PxScale::from(14.0);
    let hint = PxScale::from(15.0);
    let message_scale = PxScale::from(cfg.font_size.clamp(14.0, 22.0));
    let line_height = (cfg.font_size * 1.65).clamp(28.0, 42.0);
    let default_text = parse_hex_rgb(&cfg.text_color, [224, 226, 232]);

    draw_text_line(
        font,
        &mut pixels,
        width,
        height,
        "Bilibili 直播互动",
        22.0,
        40.0,
        title,
        [244, 245, 248],
    );
    draw_live_panel_icons(&mut pixels, width, height);
    draw_eye_icon(&mut pixels, width, height, 28, 106, [139, 143, 153, 230]);
    draw_text_line(
        font,
        &mut pixels,
        width,
        height,
        &status.online.to_string(),
        44.0,
        116.0,
        small,
        [168, 171, 181],
    );
    draw_text_line(
        font,
        &mut pixels,
        width,
        height,
        "点赞 0",
        112.0,
        116.0,
        small,
        [168, 171, 181],
    );

    let revenue = messages.iter().filter_map(|msg| msg.price).sum::<f64>();
    let revenue_text = if revenue > 0.0 {
        format!("¥{:.0}", revenue)
    } else {
        "¥0".to_string()
    };
    draw_text_line(
        font,
        &mut pixels,
        width,
        height,
        &revenue_text,
        178.0,
        116.0,
        small,
        [168, 171, 181],
    );
    draw_text_right(
        font,
        &mut pixels,
        width,
        height,
        "收益概览",
        width as f32 - 20.0,
        116.0,
        small,
        [223, 225, 232],
    );

    let mut pill_x = 16u32;
    pill_x = draw_live_pill(font, &mut pixels, width, height, "礼物", pill_x, 162);
    pill_x = draw_live_pill(font, &mut pixels, width, height, "舰长", pill_x + 10, 162);
    let _ = draw_live_pill(
        font,
        &mut pixels,
        width,
        height,
        "醒目留言",
        pill_x + 10,
        162,
    );
    draw_text_right(
        font,
        &mut pixels,
        width,
        height,
        "消息筛选",
        width as f32 - 20.0,
        184.0,
        normal,
        [244, 245, 248],
    );
    draw_text_centered(
        font,
        &mut pixels,
        width,
        height,
        "礼物、醒目留言与舰长消息会重点显示",
        260.0,
        hint,
        [135, 138, 148],
    );

    let filtered = messages
        .iter()
        .rev()
        .filter(|msg| should_show_message(cfg, msg))
        .take(10)
        .cloned()
        .collect::<Vec<_>>();

    if filtered.is_empty() {
        draw_live_spinner(&mut pixels, width, height, width as i32 / 2, 304);
        draw_text_centered(
            font,
            &mut pixels,
            width,
            height,
            "正在等待真实直播弹幕",
            432.0,
            normal,
            [135, 138, 148],
        );
        if !status.running {
            draw_text_centered(
                font,
                &mut pixels,
                width,
                height,
                "启动弹幕服务后同步桌面与 VR 面板",
                470.0,
                small,
                [103, 107, 118],
            );
        }
    } else {
        let mut y = 332.0f32;
        for msg in filtered.into_iter().rev() {
            if y > 610.0 {
                break;
            }

            let color = live_message_color(&msg, default_text);
            let user = truncate_chars(&msg.user, 16);
            let body = truncate_chars(&msg.text, 120);
            let label = live_message_label(&msg);
            let line = if label.is_empty() {
                format!("{user}: {body}")
            } else if body.is_empty() {
                format!("[{label}] {user}")
            } else {
                format!("[{label}] {user}: {body}")
            };

            if msg.message_type == "sc" {
                fill_rounded_rect(
                    &mut pixels,
                    width,
                    height,
                    20,
                    y as u32 - 22,
                    width - 40,
                    (line_height + 18.0) as u32,
                    8,
                    [53, 44, 34, 230],
                );
                fill_rect(
                    &mut pixels,
                    width,
                    height,
                    20,
                    y as u32 - 22,
                    4,
                    (line_height + 18.0) as u32,
                    [224, 150, 64, 245],
                );
            }

            y = draw_wrapped_text(
                font,
                &mut pixels,
                width,
                height,
                &line,
                30.0,
                y,
                width as f32 - 60.0,
                message_scale,
                color,
                line_height,
            );
            y += 8.0;
        }
    }

    fill_rounded_rect(
        &mut pixels,
        width,
        height,
        18,
        height - 72,
        width - 36,
        56,
        10,
        [49, 52, 58, surface_alpha],
    );
    let input_text = truncate_chars(&cfg.vr_input_text, 36);
    let placeholder = if status.vr_keyboard_open {
        "SteamVR 键盘已打开..."
    } else {
        "打开菜单，按扳机输入内容"
    };
    let display_text = if input_text.is_empty() {
        placeholder
    } else {
        &input_text
    };
    let input_color = if input_text.is_empty() {
        [143, 147, 157]
    } else {
        [234, 236, 244]
    };
    draw_text_line(
        font,
        &mut pixels,
        width,
        height,
        display_text,
        36.0,
        height as f32 - 38.0,
        small,
        input_color,
    );
    draw_text_line(
        font,
        &mut pixels,
        width,
        height,
        &format!("{}/80", cfg.vr_input_text.chars().count().min(80)),
        width as f32 - 122.0,
        height as f32 - 38.0,
        small,
        [107, 111, 121],
    );
    draw_text_line(
        font,
        &mut pixels,
        width,
        height,
        "发送",
        width as f32 - 70.0,
        height as f32 - 38.0,
        small,
        [211, 68, 126],
    );

    pixels
}

#[allow(dead_code)]
fn live_message_label(msg: &DanmakuMessage) -> &'static str {
    match msg.message_type.as_str() {
        "sc" => "SC",
        "gift" => "礼物",
        "enter" => "进入",
        "follow" => "关注",
        "guard" | "vip_enter" => "舰长",
        "warning" => "警告",
        "osc" => "OSC",
        "input" => "输入",
        _ => "",
    }
}

fn live_message_color(msg: &DanmakuMessage, default: [u8; 3]) -> [u8; 3] {
    match msg.message_type.as_str() {
        "sc" => [255, 224, 138],
        "gift" => [255, 130, 174],
        "enter" => [139, 220, 176],
        "follow" => [244, 114, 182],
        "guard" | "vip_enter" => [250, 204, 21],
        "warning" => [248, 113, 113],
        "osc" => [125, 211, 252],
        _ => default,
    }
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    let mut out = String::new();
    let mut chars = value.chars();
    for _ in 0..max_chars {
        if let Some(ch) = chars.next() {
            out.push(ch);
        } else {
            return out;
        }
    }
    if chars.next().is_some() {
        out.push_str("...");
    }
    out
}

fn draw_live_panel_icons(pixels: &mut [u8], width: u32, height: u32) {
    let color = [139, 143, 153, 230];
    draw_lock_icon(pixels, width, height, 356, 22, color);
    draw_mic_icon(pixels, width, height, 412, 18, color);
    draw_circle_outline(pixels, width, height, 476, 31, 13, color);
    draw_circle_outline(pixels, width, height, 532, 31, 12, color);
    draw_line(pixels, width, height, 576, 20, 602, 46, color);
    draw_line(pixels, width, height, 602, 20, 576, 46, color);
}

fn draw_live_pill(
    font: &FontVec,
    pixels: &mut [u8],
    width: u32,
    height: u32,
    label: &str,
    x: u32,
    y: u32,
) -> u32 {
    let scale = PxScale::from(15.0);
    let pill_w = (measure_text_width(font, label, scale) + 24.0).ceil() as u32;
    fill_rounded_rect(
        pixels,
        width,
        height,
        x,
        y,
        pill_w,
        30,
        15,
        [103, 35, 65, 235],
    );
    draw_text_line(
        font,
        pixels,
        width,
        height,
        label,
        x as f32 + 12.0,
        y as f32 + 21.0,
        scale,
        [255, 105, 166],
    );
    x + pill_w
}

fn draw_text_centered(
    font: &FontVec,
    pixels: &mut [u8],
    width: u32,
    height: u32,
    text: &str,
    y: f32,
    scale: PxScale,
    color: [u8; 3],
) {
    let x = ((width as f32 - measure_text_width(font, text, scale)) / 2.0).max(8.0);
    draw_text_line(font, pixels, width, height, text, x, y, scale, color);
}

#[allow(clippy::too_many_arguments)]
fn draw_text_right(
    font: &FontVec,
    pixels: &mut [u8],
    width: u32,
    height: u32,
    text: &str,
    right: f32,
    y: f32,
    scale: PxScale,
    color: [u8; 3],
) {
    let x = (right - measure_text_width(font, text, scale)).max(8.0);
    draw_text_line(font, pixels, width, height, text, x, y, scale, color);
}

fn measure_text_width(font: &FontVec, text: &str, scale: PxScale) -> f32 {
    let scaled = font.as_scaled(scale);
    text.chars()
        .map(|ch| scaled.h_advance(font.glyph_id(ch)))
        .sum()
}

fn draw_lock_icon(pixels: &mut [u8], width: u32, height: u32, x: i32, y: i32, color: [u8; 4]) {
    draw_rect_outline(pixels, width, height, x + 2, y + 12, 22, 18, color);
    draw_line(pixels, width, height, x + 7, y + 12, x + 7, y + 7, color);
    draw_line(pixels, width, height, x + 7, y + 7, x + 18, y + 7, color);
    draw_line(pixels, width, height, x + 18, y + 7, x + 18, y + 12, color);
    draw_line(pixels, width, height, x + 13, y + 18, x + 13, y + 24, color);
}

fn draw_mic_icon(pixels: &mut [u8], width: u32, height: u32, x: i32, y: i32, color: [u8; 4]) {
    draw_rect_outline(pixels, width, height, x + 9, y + 2, 12, 22, color);
    draw_line(pixels, width, height, x + 5, y + 15, x + 5, y + 22, color);
    draw_line(pixels, width, height, x + 5, y + 22, x + 25, y + 22, color);
    draw_line(pixels, width, height, x + 25, y + 15, x + 25, y + 22, color);
    draw_line(pixels, width, height, x + 15, y + 24, x + 15, y + 32, color);
    draw_line(pixels, width, height, x + 9, y + 32, x + 21, y + 32, color);
}

fn draw_eye_icon(pixels: &mut [u8], width: u32, height: u32, cx: i32, cy: i32, color: [u8; 4]) {
    draw_line(pixels, width, height, cx - 12, cy, cx - 5, cy - 6, color);
    draw_line(pixels, width, height, cx - 5, cy - 6, cx + 5, cy - 6, color);
    draw_line(pixels, width, height, cx + 5, cy - 6, cx + 12, cy, color);
    draw_line(pixels, width, height, cx - 12, cy, cx - 5, cy + 6, color);
    draw_line(pixels, width, height, cx - 5, cy + 6, cx + 5, cy + 6, color);
    draw_line(pixels, width, height, cx + 5, cy + 6, cx + 12, cy, color);
    draw_circle_outline(pixels, width, height, cx, cy, 3, color);
}

fn draw_live_spinner(pixels: &mut [u8], width: u32, height: u32, cx: i32, cy: i32) {
    let dots = [
        (0, -9, 230),
        (6, -6, 205),
        (9, 0, 180),
        (6, 6, 155),
        (0, 9, 130),
        (-6, 6, 105),
        (-9, 0, 90),
        (-6, -6, 75),
    ];
    for (dx, dy, alpha) in dots {
        fill_circle(
            pixels,
            width,
            height,
            cx + dx,
            cy + dy,
            2,
            [122, 125, 135, alpha],
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn fill_rounded_rect(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    radius: u32,
    color: [u8; 4],
) {
    let x2 = (x + w).min(width);
    let y2 = (y + h).min(height);
    let r = radius.min(w / 2).min(h / 2) as i32;
    let ru = r as u32;
    let r2 = r * r;

    for py in y..y2 {
        for px in x..x2 {
            let mut dx = 0i32;
            let mut dy = 0i32;
            if px < x + ru {
                dx = x as i32 + r - px as i32;
            } else if px >= x2.saturating_sub(ru) {
                dx = px as i32 - (x2 as i32 - r - 1);
            }
            if py < y + ru {
                dy = y as i32 + r - py as i32;
            } else if py >= y2.saturating_sub(ru) {
                dy = py as i32 - (y2 as i32 - r - 1);
            }

            if dx * dx + dy * dy <= r2 {
                let idx = ((py * width + px) * 4) as usize;
                pixels[idx..idx + 4].copy_from_slice(&color);
            }
        }
    }
}

fn fill_circle(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    cx: i32,
    cy: i32,
    radius: i32,
    color: [u8; 4],
) {
    let r2 = radius * radius;
    for y in (cy - radius)..=(cy + radius) {
        for x in (cx - radius)..=(cx + radius) {
            let d = (x - cx) * (x - cx) + (y - cy) * (y - cy);
            if d <= r2 {
                set_pixel(pixels, width, height, x, y, color);
            }
        }
    }
}

fn draw_rect_outline(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    color: [u8; 4],
) {
    draw_line(pixels, width, height, x, y, x + w, y, color);
    draw_line(pixels, width, height, x, y + h, x + w, y + h, color);
    draw_line(pixels, width, height, x, y, x, y + h, color);
    draw_line(pixels, width, height, x + w, y, x + w, y + h, color);
}

fn draw_circle_outline(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    cx: i32,
    cy: i32,
    radius: i32,
    color: [u8; 4],
) {
    let outer = radius * radius;
    let inner_radius = (radius - 2).max(0);
    let inner = inner_radius * inner_radius;
    for y in (cy - radius)..=(cy + radius) {
        for x in (cx - radius)..=(cx + radius) {
            let d = (x - cx) * (x - cx) + (y - cy) * (y - cy);
            if d <= outer && d >= inner {
                set_pixel(pixels, width, height, x, y, color);
            }
        }
    }
}

fn draw_line(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    mut x0: i32,
    mut y0: i32,
    x1: i32,
    y1: i32,
    color: [u8; 4],
) {
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        set_pixel(pixels, width, height, x0, y0, color);
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = err * 2;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
}

fn set_pixel(pixels: &mut [u8], width: u32, height: u32, x: i32, y: i32, color: [u8; 4]) {
    if x >= 0 && y >= 0 && (x as u32) < width && (y as u32) < height {
        let idx = (((y as u32) * width + x as u32) * 4) as usize;
        pixels[idx..idx + 4].copy_from_slice(&color);
    }
}

#[cfg(test)]
mod bilibili_protocol_tests {
    use super::{parse_bili_auth_reply, truncate_chars};

    #[test]
    fn accepts_only_successful_bilibili_auth_reply() {
        assert!(parse_bili_auth_reply(br#"{"code":0}"#).is_ok());
        assert!(parse_bili_auth_reply(br#"{"code":-101}"#).is_err());
        assert!(parse_bili_auth_reply(br#"{"message":"missing code"}"#).is_err());
        assert!(parse_bili_auth_reply(b"not-json").is_err());
    }

    #[test]
    fn truncates_overlay_text_without_splitting_unicode() {
        assert_eq!(truncate_chars("老板的小搭", 3), "老板的...");
        assert_eq!(truncate_chars("弹幕", 8), "弹幕");
    }
}

#[allow(dead_code)]
fn render_danmaku_menu_overlay(
    font: &FontVec,
    cfg: &DanmakuConfig,
    status: &DanmakuStatus,
) -> Vec<u8> {
    let width = 640u32;
    let height = 520u32;
    let mut pixels = vec![0u8; (width * height * 4) as usize];
    fill_rect(
        &mut pixels,
        width,
        height,
        0,
        0,
        width,
        height,
        [8, 10, 14, 236],
    );
    fill_rect(
        &mut pixels,
        width,
        height,
        0,
        0,
        width,
        58,
        [18, 25, 38, 250],
    );
    fill_rect(
        &mut pixels,
        width,
        height,
        0,
        58,
        width,
        1,
        [56, 189, 248, 190],
    );

    let title = PxScale::from(22.0);
    let normal = PxScale::from(16.0);
    let small = PxScale::from(13.0);
    draw_text_line(
        font,
        &mut pixels,
        width,
        height,
        "VrcDog Menu",
        22.0,
        36.0,
        title,
        [245, 249, 255],
    );
    draw_text_line(
        font,
        &mut pixels,
        width,
        height,
        if status.bili_connected {
            "Bilibili connected"
        } else {
            "Waiting for danmaku / test messages"
        },
        396.0,
        36.0,
        small,
        if status.bili_connected {
            [74, 222, 128]
        } else {
            [148, 163, 184]
        },
    );

    let mode = if cfg.attach_mode == "hand" || cfg.attach_mode == "left_hand" {
        "Left hand"
    } else if cfg.attach_mode == "right_hand" {
        "Right hand"
    } else {
        "HMD"
    };
    let hand = match cfg.toggle_hand.as_str() {
        "right" => "right controller",
        "always_on" => "always on",
        _ => "left controller",
    };
    draw_text_line(
        font,
        &mut pixels,
        width,
        height,
        &format!(
            "Attach: {mode}  |  Toggle: {hand}  |  Room #{}",
            status.room_id
        ),
        22.0,
        84.0,
        normal,
        [226, 232, 240],
    );
    draw_text_line(
        font,
        &mut pixels,
        width,
        height,
        "App menu: show/hide menu    Grip: show/hide danmaku panel",
        22.0,
        112.0,
        small,
        [186, 230, 253],
    );
    draw_text_line(
        font,
        &mut pixels,
        width,
        height,
        "Stick/touchpad: move panel    Hold trigger: distance/size    Tap trigger: keyboard",
        22.0,
        136.0,
        small,
        [186, 230, 253],
    );

    let mut y = 176u32;
    draw_menu_value(font, &mut pixels, width, height, "X", cfg.x, -1.0, 1.0, y);
    y += 38;
    draw_menu_value(font, &mut pixels, width, height, "Y", cfg.y, -0.8, 0.8, y);
    y += 38;
    draw_menu_value(font, &mut pixels, width, height, "Z", cfg.z, -1.5, -0.3, y);
    y += 38;
    draw_menu_value(
        font,
        &mut pixels,
        width,
        height,
        "Pitch",
        cfg.pitch,
        -30.0,
        30.0,
        y,
    );
    y += 38;
    draw_menu_value(
        font,
        &mut pixels,
        width,
        height,
        "Yaw",
        cfg.yaw,
        -30.0,
        30.0,
        y,
    );
    y += 38;
    draw_menu_value(
        font,
        &mut pixels,
        width,
        height,
        "Roll",
        cfg.roll,
        -20.0,
        20.0,
        y,
    );
    y += 38;
    draw_menu_value(
        font,
        &mut pixels,
        width,
        height,
        "Size",
        cfg.overlay_width_m,
        0.15,
        0.8,
        y,
    );
    y += 38;
    draw_menu_value(
        font,
        &mut pixels,
        width,
        height,
        "Background",
        cfg.bg_alpha,
        0.0,
        1.0,
        y,
    );

    pixels
}

fn draw_menu_value(
    font: &FontVec,
    pixels: &mut [u8],
    width: u32,
    height: u32,
    label: &str,
    value: f32,
    min: f32,
    max: f32,
    y: u32,
) {
    let scale = PxScale::from(14.0);
    draw_text_line(
        font,
        pixels,
        width,
        height,
        label,
        28.0,
        y as f32 + 17.0,
        scale,
        [226, 232, 240],
    );
    draw_text_line(
        font,
        pixels,
        width,
        height,
        &format!("{value:.2}"),
        532.0,
        y as f32 + 17.0,
        scale,
        [245, 249, 255],
    );
    let bar_x = 130u32;
    let bar_y = y + 6;
    let bar_w = 370u32;
    fill_rect(
        pixels,
        width,
        height,
        bar_x,
        bar_y,
        bar_w,
        10,
        [39, 45, 58, 230],
    );
    let ratio = ((value - min) / (max - min)).clamp(0.0, 1.0);
    fill_rect(
        pixels,
        width,
        height,
        bar_x,
        bar_y,
        (bar_w as f32 * ratio) as u32,
        10,
        [14, 165, 233, 235],
    );
}

fn should_show_message(cfg: &DanmakuConfig, msg: &DanmakuMessage) -> bool {
    match msg.message_type.as_str() {
        "gift" => cfg.show_gift,
        "enter" => cfg.show_enter,
        "follow" => cfg.show_follow,
        "guard" | "vip_enter" => cfg.show_guard,
        "sc" => cfg.show_sc,
        "warning" => true,
        _ => cfg.show_danmaku,
    }
}

#[allow(dead_code)]
fn message_prefix(msg: &DanmakuMessage) -> &'static str {
    match msg.message_type.as_str() {
        "sc" => "[SC] ",
        "gift" => "[Gift] ",
        "enter" => "[Enter] ",
        "follow" => "[Follow] ",
        "guard" | "vip_enter" => "[Guard] ",
        "warning" => "[Warning] ",
        "osc" => "[OSC] ",
        "input" => "[Input] ",
        _ => "",
    }
}

#[allow(dead_code)]
fn message_color(msg: &DanmakuMessage, default: [u8; 3]) -> [u8; 3] {
    match msg.message_type.as_str() {
        "sc" => [255, 224, 138],
        "gift" => [251, 146, 60],
        "enter" => [74, 222, 128],
        "follow" => [244, 114, 182],
        "guard" | "vip_enter" => [250, 204, 21],
        "warning" => [248, 113, 113],
        "osc" => [125, 211, 252],
        _ => default,
    }
}

fn parse_hex_rgb(hex: &str, fallback: [u8; 3]) -> [u8; 3] {
    let value = hex.trim_start_matches('#');
    if value.len() < 6 {
        return fallback;
    }
    let r = u8::from_str_radix(&value[0..2], 16).unwrap_or(fallback[0]);
    let g = u8::from_str_radix(&value[2..4], 16).unwrap_or(fallback[1]);
    let b = u8::from_str_radix(&value[4..6], 16).unwrap_or(fallback[2]);
    [r, g, b]
}

#[allow(clippy::too_many_arguments)]
fn fill_rect(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    color: [u8; 4],
) {
    let x2 = (x + w).min(width);
    let y2 = (y + h).min(height);
    for py in y..y2 {
        for px in x..x2 {
            let idx = ((py * width + px) * 4) as usize;
            pixels[idx..idx + 4].copy_from_slice(&color);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_wrapped_text(
    font: &FontVec,
    pixels: &mut [u8],
    width: u32,
    height: u32,
    text: &str,
    x: f32,
    y: f32,
    max_width: f32,
    scale: PxScale,
    color: [u8; 3],
    line_height: f32,
) -> f32 {
    let scaled = font.as_scaled(scale);
    let mut cursor_x = x;
    let mut cursor_y = y;
    for ch in text.chars() {
        let glyph_id = font.glyph_id(ch);
        let advance = scaled.h_advance(glyph_id);
        if cursor_x + advance > x + max_width {
            cursor_x = x;
            cursor_y += line_height;
        }
        draw_char(
            font, pixels, width, height, ch, cursor_x, cursor_y, scale, color,
        );
        cursor_x += advance;
    }
    cursor_y + line_height
}

#[allow(clippy::too_many_arguments)]
fn draw_text_line(
    font: &FontVec,
    pixels: &mut [u8],
    width: u32,
    height: u32,
    text: &str,
    x: f32,
    y: f32,
    scale: PxScale,
    color: [u8; 3],
) {
    let scaled = font.as_scaled(scale);
    let mut cursor_x = x;
    for ch in text.chars() {
        let glyph_id = font.glyph_id(ch);
        let advance = scaled.h_advance(glyph_id);
        if cursor_x > width as f32 - 8.0 {
            break;
        }
        draw_char(font, pixels, width, height, ch, cursor_x, y, scale, color);
        cursor_x += advance;
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_char(
    font: &FontVec,
    pixels: &mut [u8],
    width: u32,
    height: u32,
    ch: char,
    x: f32,
    y: f32,
    scale: PxScale,
    color: [u8; 3],
) {
    let glyph = font
        .glyph_id(ch)
        .with_scale_and_position(scale, point(x, y));
    if let Some(outlined) = font.outline_glyph(glyph) {
        let bounds = outlined.px_bounds();
        outlined.draw(|gx, gy, coverage| {
            let px = bounds.min.x as i32 + gx as i32;
            let py = bounds.min.y as i32 + gy as i32;
            if px >= 0 && py >= 0 && (px as u32) < width && (py as u32) < height {
                let idx = ((py as u32 * width + px as u32) * 4) as usize;
                let alpha = (coverage * 255.0) as u8;
                let inv = 255 - alpha;
                pixels[idx] = ((color[0] as u16 * alpha as u16 + pixels[idx] as u16 * inv as u16)
                    / 255) as u8;
                pixels[idx + 1] = ((color[1] as u16 * alpha as u16
                    + pixels[idx + 1] as u16 * inv as u16)
                    / 255) as u8;
                pixels[idx + 2] = ((color[2] as u16 * alpha as u16
                    + pixels[idx + 2] as u16 * inv as u16)
                    / 255) as u8;
                pixels[idx + 3] = pixels[idx + 3].max(alpha);
            }
        });
    }
}
