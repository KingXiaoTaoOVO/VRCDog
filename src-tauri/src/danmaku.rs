use ab_glyph::{point, Font, FontVec, PxScale, ScaleFont};
use brotli::Decompressor as BrotliDecompressor;
use flate2::read::ZlibDecoder;
use futures_util::{SinkExt, StreamExt};
use rosc::{OscMessage, OscPacket, OscType};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
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

const DEFAULT_BILI_WS_HOST: &str = "broadcastlv.chat.bilibili.com";
const BILI_OP_HEARTBEAT: u32 = 2;
const BILI_OP_HEARTBEAT_REPLY: u32 = 3;
const BILI_OP_MESSAGE: u32 = 5;
const BILI_OP_AUTH: u32 = 7;
const BILI_OP_AUTH_REPLY: u32 = 8;

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
    pub room_id: u64,
    pub online: u64,
    pub message_count: usize,
    pub last_error: String,
    pub last_event: String,
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
            status.room_id = config.room_id;
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
                room_id: config.room_id,
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
    }
    {
        if let Ok(mut status) = state.status.lock() {
            status.overlay_visible = visible;
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

async fn run_bilibili_source(runtime: DanmakuRuntime, tx: mpsc::UnboundedSender<DanmakuMessage>) {
    let mut reconnect_count = 0u32;

    while !runtime.stop.load(Ordering::Acquire) {
        match run_bilibili_once(runtime.clone(), tx.clone()).await {
            Ok(()) => {
                // 连接正常关闭（非错误），但未被要求停止
                // 需要短暂等待并标记断开，避免立即重连造成空转
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
                emit_log(&runtime.app, &format!("Bilibili source error: {err}"));
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
    let real_room_id = resolve_bili_room_id(&client, cfg.room_id, &cfg.bili_sessdata).await?;
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
        .map_err(|e| e.to_string())?;
    let (mut writer, mut reader) = stream.split();

    let auth_body = serde_json::json!({
        "uid": 0,
        "roomid": real_room_id,
        "protover": 3,
        "platform": "web",
        "type": 2,
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
        status.bili_connected = true;
        status.last_error.clear();
        status.last_event = "bilibili_connected".to_string();
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
                            handle_bili_frame(&runtime, &tx, frame).await;
                        }
                    }
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
                            handle_bili_value(&runtime, &tx, value).await;
                        }
                    }
                    Some(Ok(Message::Close(_))) => return Err("bilibili websocket closed".to_string()),
                    Some(Ok(_)) => {}
                    Some(Err(err)) => return Err(err.to_string()),
                    None => return Err("bilibili websocket ended".to_string()),
                }
            }
        }
    }

    Ok(())
}

async fn resolve_bili_room_id(
    client: &reqwest::Client,
    room_id: u64,
    sessdata: &str,
) -> Result<u64, String> {
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

    Ok(body["data"]["room_id"].as_u64().unwrap_or(room_id))
}

async fn get_bili_danmaku_endpoint(
    client: &reqwest::Client,
    room_id: u64,
    sessdata: &str,
) -> Result<(String, String, u64), String> {
    let url = format!(
        "https://api.live.bilibili.com/xlive/web-room/v1/index/getDanmuInfo?id={room_id}&type=0"
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
        let primary_error = body["message"]
            .as_str()
            .unwrap_or("getDanmuInfo failed")
            .to_string();
        return get_bili_legacy_danmaku_endpoint(client, room_id, sessdata)
            .await
            .map_err(|fallback_error| {
                format!("getDanmuInfo failed: {primary_error}; getConf fallback failed: {fallback_error}")
            });
    }

    extract_bili_endpoint(&body["data"], "host_list")
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
        if let Some(first) = hosts.first() {
            host = first["host"]
                .as_str()
                .unwrap_or(DEFAULT_BILI_WS_HOST)
                .to_string();
            port = first["wss_port"]
                .as_u64()
                .or_else(|| first["ws_port"].as_u64())
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
) {
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
        BILI_OP_AUTH_REPLY => {
            set_status(runtime, |status| {
                status.bili_connected = true;
                status.last_event = "bilibili_auth_ok".to_string();
            });
            emit_status(&runtime.app, &runtime.status);
        }
        BILI_OP_MESSAGE => {
            if frame.version == 0 || frame.version == 1 {
                if let Ok(value) = serde_json::from_slice::<serde_json::Value>(&frame.body) {
                    handle_bili_value(runtime, tx, value).await;
                }
            }
        }
        _ => {}
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
                let _ = tx.send(make_message(
                    runtime,
                    "bilibili",
                    "vip_enter",
                    "Captain",
                    &text,
                ));
            }
        }
        "WARNING" | "CUT_OFF" | "ROOM_LOCK" => {
            let text = value["data"]["data"]["msg"]
                .as_str()
                .or_else(|| value["data"]["msg"].as_str())
                .unwrap_or(cmd);
            let _ = tx.send(make_message(runtime, "bilibili", "warning", "System", text));
        }
        "GUARD_BUY" => {
            let data = &value["data"]["data"];
            let user = data["username"].as_str().unwrap_or("???");
            let gift = data["gift_name"].as_str().unwrap_or("Guard");
            let _ = tx.send(make_message(runtime, "bilibili", "guard", user, gift));
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
        .unwrap_or("???")
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
    let user = data["uname"].as_str().unwrap_or("???");
    let gift = data["giftName"].as_str().unwrap_or("Gift");
    let count = data["num"].as_u64().unwrap_or(1) as u32;
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
    let user = data["user_info"]["uname"].as_str().unwrap_or("???");
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
            custom_text.unwrap_or("VRDanmaku is now integrated into VrcDog."),
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
                .map(|price| format!(" ¥{price:.0}"))
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
            // 检测 OpenVR 是否已被 OVR 翻译器初始化（同一进程只能 init 一次）
            let msg = if err_str.to_lowercase().contains("init") && !err_str.to_lowercase().contains("not")
                || err_str.to_lowercase().contains("already")
            {
                "SteamVR overlay is already initialized by OVR Translator; release and retry".to_string()
            } else {
                format!("OpenVR init failed: {err_str}")
            };
            set_status(&runtime, |status| {
                status.vr_initialized = false;
                status.last_error = msg;
            });
            emit_log(&runtime.app, &format!("[VR Overlay] OpenVR init error: {err_str}"));
            emit_status(&runtime.app, &runtime.status);
            return;
        }
        Err(_) => {
            set_status(&runtime, |status| {
                status.vr_initialized = false;
                status.last_error =
                    "SteamVR overlay is already initialized by OVR Translator; release and retry".to_string();
            });
            emit_log(&runtime.app, "[VR Overlay] OpenVR already initialized (panic path)");
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

    let handle = match overlay.create_overlay("vrcdog.danmaku\0", "VrcDog Danmaku\0") {
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

    set_status(&runtime, |status| {
        status.vr_initialized = true;
        status.last_error.clear();
        status.last_event = "vr_overlay_initialized".to_string();
    });
    emit_status(&runtime.app, &runtime.status);
    emit_log(&runtime.app, "SteamVR danmaku overlay initialized");

    let mut previous_pressed = 0u64;
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
            let pressed = controller_pressed_for_toggle(&sys, &cfg);
            let grip_mask = 1u64 << openvr::button_id::GRIP;
            if pressed & grip_mask != 0 && previous_pressed & grip_mask == 0 {
                let next_visible = !cfg.overlay_visible;
                if let Ok(mut current) = runtime.config.lock() {
                    current.overlay_visible = next_visible;
                }
                set_status(&runtime, |status| {
                    status.overlay_visible = next_visible;
                    status.last_event = "overlay_toggled_by_controller".to_string();
                });
                emit_status(&runtime.app, &runtime.status);
            }
            previous_pressed = pressed;
        }

        let visible = runtime
            .config
            .lock()
            .map(|cfg| cfg.overlay_visible || cfg.toggle_hand == "always_on")
            .unwrap_or(true);

        let pixels = render_danmaku_overlay(&font, &messages, &cfg, &status_snapshot);
        let _ = overlay.set_raw_data(handle, &pixels, 640, 420, 4);
        let _ = overlay.set_visibility(handle, visible);
        set_status(&runtime, |status| {
            status.overlay_visible = visible;
        });

        std::thread::sleep(Duration::from_millis(80));
    }

    let _ = overlay.set_visibility(handle, false);
    set_status(&runtime, |status| {
        status.vr_initialized = false;
        status.overlay_visible = false;
    });
    emit_status(&runtime.app, &runtime.status);
}

fn controller_pressed_for_toggle(sys: &openvr::System, cfg: &DanmakuConfig) -> u64 {
    let role = match cfg.toggle_hand.as_str() {
        "right" => openvr::TrackedControllerRole::RightHand,
        "always_on" => return 0,
        _ => openvr::TrackedControllerRole::LeftHand,
    };
    sys.tracked_device_index_for_controller_role(role)
        .and_then(|index| sys.controller_state(index))
        .map(|state| state.button_pressed)
        .unwrap_or(0)
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

fn render_danmaku_overlay(
    font: &FontVec,
    messages: &[DanmakuMessage],
    cfg: &DanmakuConfig,
    status: &DanmakuStatus,
) -> Vec<u8> {
    let width = 640u32;
    let height = 420u32;
    let mut pixels = vec![0u8; (width * height * 4) as usize];
    let bg = parse_hex_rgb(&cfg.bg_color, [16, 20, 31]);
    let text = parse_hex_rgb(&cfg.text_color, [255, 255, 255]);
    fill_rect(
        &mut pixels,
        width,
        height,
        0,
        0,
        width,
        height,
        [
            bg[0],
            bg[1],
            bg[2],
            (cfg.bg_alpha.clamp(0.0, 1.0) * 255.0) as u8,
        ],
    );
    fill_rect(
        &mut pixels,
        width,
        height,
        0,
        0,
        width,
        44,
        [24, 30, 45, 230],
    );

    let scale = PxScale::from(cfg.font_size.clamp(12.0, 34.0));
    let small = PxScale::from((cfg.font_size * 0.72).clamp(10.0, 22.0));
    let header = format!(
        "VrcDog 直播弹幕  房间 #{}  观众 {}",
        status.room_id, status.online
    );
    draw_text_line(
        font,
        &mut pixels,
        width,
        height,
        &header,
        16.0,
        29.0,
        small,
        [225, 231, 255],
    );

    let state = if status.bili_connected {
        "B站已连接"
    } else if status.osc_input_running {
        "OSC"
    } else {
        "待机"
    };
    draw_text_line(
        font,
        &mut pixels,
        width,
        height,
        state,
        560.0,
        29.0,
        small,
        if status.bili_connected || status.osc_input_running {
            [74, 222, 128]
        } else {
            [148, 163, 184]
        },
    );

    let mut y = 62.0f32;
    let line_height = (cfg.font_size * 1.42).clamp(24.0, 46.0);
    let filtered = messages
        .iter()
        .rev()
        .filter(|msg| should_show_message(cfg, msg))
        .take(12)
        .cloned()
        .collect::<Vec<_>>();

    if filtered.is_empty() {
        let hint = if status.running {
            "弹幕窗口已就绪，等待 Bilibili / OSC 消息..."
        } else {
            "弹幕服务未启动。启动后可在这里显示直播弹幕。"
        };
        draw_text_line(
            font,
            &mut pixels,
            width,
            height,
            hint,
            20.0,
            y + 20.0,
            scale,
            [148, 163, 184],
        );
        draw_text_line(
            font,
            &mut pixels,
            width,
            height,
            "可点击桌面端“测试弹幕 / 测试 SC / 测试礼物”确认 VR 内显示。",
            20.0,
            y + 54.0,
            small,
            [196, 181, 253],
        );
        return pixels;
    }

    for msg in filtered.into_iter().rev() {
        if y > height as f32 - line_height {
            break;
        }

        let color = message_color(&msg, text);
        if msg.message_type == "sc" {
            fill_rect(
                &mut pixels,
                width,
                height,
                12,
                y as u32 - 6,
                width - 24,
                line_height as u32 + 14,
                [74, 54, 20, 210],
            );
        }

        let prefix = message_prefix(&msg);
        let line = format!("{}{}: {}", prefix, msg.user, msg.text);
        y = draw_wrapped_text(
            font,
            &mut pixels,
            width,
            height,
            &line,
            20.0,
            y,
            width as f32 - 40.0,
            scale,
            color,
            line_height,
        );
        y += 4.0;
    }

    pixels
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

fn message_prefix(msg: &DanmakuMessage) -> &'static str {
    match msg.message_type.as_str() {
        "sc" => "[SC] ",
        "gift" => "[礼物] ",
        "enter" => "[进入] ",
        "follow" => "[关注] ",
        "guard" | "vip_enter" => "[舰长] ",
        "warning" => "[警告] ",
        "osc" => "[OSC] ",
        _ => "",
    }
}

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
