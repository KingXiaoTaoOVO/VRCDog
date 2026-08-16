use crate::AppResult;
use chrono::{Datelike, Local, Timelike, Utc};
use rosc::{OscMessage, OscPacket, OscType};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::net::UdpSocket;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use sysinfo::{Disks, ProcessesToUpdate, System};
use tauri::{AppHandle, Emitter};
use tokio::net::UdpSocket as TokioUdpSocket;

static OSC_MONITOR: OnceLock<Mutex<Option<tauri::async_runtime::JoinHandle<()>>>> = OnceLock::new();
static OSC_AUTOMATION: OnceLock<Mutex<Option<tauri::async_runtime::JoinHandle<()>>>> =
    OnceLock::new();
static OSC_MONITOR_RUNNING: AtomicBool = AtomicBool::new(false);
static OSC_AUTOMATION_RUNNING: AtomicBool = AtomicBool::new(false);
static SYSTEM: OnceLock<Mutex<System>> = OnceLock::new();
static DISKS: OnceLock<Mutex<Disks>> = OnceLock::new();
static HARDWARE_INFO: OnceLock<HardwareInfo> = OnceLock::new();
static GPU_SAMPLE: OnceLock<Mutex<Option<CachedGpuSample>>> = OnceLock::new();

const BYTES_PER_GIB: f32 = 1024.0 * 1024.0 * 1024.0;
const NVIDIA_SAMPLE_INTERVAL: Duration = Duration::from_millis(1500);
const NVIDIA_RETRY_INTERVAL: Duration = Duration::from_secs(60);

#[derive(Debug, Clone)]
struct HardwareInfo {
    cpu_name: String,
    cpu_physical_cores: usize,
    cpu_logical_cores: usize,
    cpu_frequency_mhz: u64,
    os_name: String,
    host_name: String,
}

#[derive(Debug, Clone, Default)]
struct GpuTelemetry {
    name: String,
    usage: Option<f32>,
    memory_used_gb: Option<f32>,
    memory_total_gb: Option<f32>,
    nvidia_available: bool,
}

#[derive(Debug, Clone)]
struct CachedGpuSample {
    sampled_at: Instant,
    telemetry: GpuTelemetry,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OscArgument {
    pub value_type: String,
    pub value: Value,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OscMonitorEvent {
    pub address: String,
    pub args: Vec<OscArgument>,
    pub sender: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OscRuntimeStatus {
    pub monitor_running: bool,
    pub automation_running: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OscSystemSnapshot {
    pub cpu_usage: f32,
    pub cpu_name: String,
    pub cpu_physical_cores: usize,
    pub cpu_logical_cores: usize,
    pub cpu_frequency_mhz: u64,
    pub ram_usage: f32,
    pub memory_used_gb: f32,
    pub memory_total_gb: f32,
    pub gpu_name: String,
    pub gpu_usage: Option<f32>,
    pub gpu_memory_used_gb: Option<f32>,
    pub gpu_memory_total_gb: Option<f32>,
    pub disk_usage: f32,
    pub disk_used_gb: f32,
    pub disk_total_gb: f32,
    pub os_name: String,
    pub host_name: String,
    pub system_uptime_seconds: u64,
    pub idle_seconds: u64,
    pub active_window: String,
    pub local_time: String,
    pub local_date: String,
    pub vrc_running: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OscAutomationConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_output_port")]
    pub port: u16,
    #[serde(default = "default_interval")]
    pub interval_ms: u64,
    #[serde(default)]
    pub mappings: Vec<OscAutomationMapping>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OscAutomationMapping {
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub address: String,
    pub source: String,
    #[serde(default = "default_scale")]
    pub scale: f64,
    #[serde(default)]
    pub offset: f64,
    pub min: Option<f64>,
    pub max: Option<f64>,
    #[serde(default = "default_float_type")]
    pub value_type: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OscRouteRule {
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub source_address: String,
    #[serde(default = "default_host")]
    pub target_host: String,
    #[serde(default = "default_output_port")]
    pub target_port: u16,
    #[serde(default)]
    pub target_address: String,
}

fn default_host() -> String {
    "127.0.0.1".to_string()
}

fn default_output_port() -> u16 {
    9000
}

fn default_interval() -> u64 {
    1500
}

fn default_true() -> bool {
    true
}

fn default_scale() -> f64 {
    1.0
}

fn default_float_type() -> String {
    "float".to_string()
}

fn app_error(error: impl ToString) -> crate::AppError {
    crate::AppError::from(error.to_string())
}

fn validate_endpoint(host: &str, port: u16) -> AppResult<String> {
    let host = host.trim();
    if host.is_empty() {
        return Err(app_error("OSC host cannot be empty"));
    }
    if port == 0 {
        return Err(app_error("OSC port must be between 1 and 65535"));
    }
    Ok(format!("{host}:{port}"))
}

fn validate_address(address: &str) -> AppResult<String> {
    let address = address.trim();
    if !address.starts_with('/') || address.contains(char::is_whitespace) {
        return Err(app_error(
            "OSC address must start with '/' and contain no spaces",
        ));
    }
    Ok(address.to_string())
}

fn value_to_osc(value_type: &str, value: Value) -> AppResult<Vec<OscType>> {
    let value_type = value_type.trim().to_ascii_lowercase();
    let arg = match value_type.as_str() {
        "float" => OscType::Float(
            value
                .as_f64()
                .or_else(|| value.as_str().and_then(|v| v.parse::<f64>().ok()))
                .ok_or_else(|| app_error("Invalid float value"))? as f32,
        ),
        "double" => OscType::Double(
            value
                .as_f64()
                .or_else(|| value.as_str().and_then(|v| v.parse::<f64>().ok()))
                .ok_or_else(|| app_error("Invalid double value"))?,
        ),
        "int" => OscType::Int(
            value
                .as_i64()
                .or_else(|| value.as_f64().map(|v| v.round() as i64))
                .or_else(|| value.as_str().and_then(|v| v.parse::<i64>().ok()))
                .ok_or_else(|| app_error("Invalid integer value"))?
                .clamp(i32::MIN as i64, i32::MAX as i64) as i32,
        ),
        "long" => OscType::Long(
            value
                .as_i64()
                .or_else(|| value.as_f64().map(|v| v.round() as i64))
                .or_else(|| value.as_str().and_then(|v| v.parse::<i64>().ok()))
                .ok_or_else(|| app_error("Invalid long value"))?,
        ),
        "bool" => OscType::Bool(match value {
            Value::Bool(value) => value,
            Value::Number(value) => value.as_f64().unwrap_or_default() != 0.0,
            Value::String(value) => matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "true" | "1" | "yes" | "on"
            ),
            _ => false,
        }),
        "string" => OscType::String(match value {
            Value::String(value) => value,
            other => other.to_string(),
        }),
        "impulse" | "none" => return Ok(Vec::new()),
        _ => {
            return Err(app_error(format!(
                "Unsupported OSC value type: {value_type}"
            )))
        }
    };
    Ok(vec![arg])
}

fn send_message_internal(
    host: &str,
    port: u16,
    address: &str,
    value_type: &str,
    value: Value,
) -> AppResult<()> {
    let endpoint = validate_endpoint(host, port)?;
    let address = validate_address(address)?;
    let packet = OscPacket::Message(OscMessage {
        addr: address,
        args: value_to_osc(value_type, value)?,
    });
    let bytes = rosc::encoder::encode(&packet).map_err(app_error)?;
    let socket = UdpSocket::bind("0.0.0.0:0").map_err(app_error)?;
    socket.send_to(&bytes, endpoint).map_err(app_error)?;
    Ok(())
}

#[tauri::command]
pub fn osc_send_message(
    host: String,
    port: u16,
    address: String,
    value_type: String,
    value: Value,
) -> AppResult<()> {
    send_message_internal(&host, port, &address, &value_type, value)
}

#[tauri::command]
pub fn osc_send_chatbox(
    host: String,
    port: u16,
    text: String,
    send: bool,
    notify: bool,
) -> AppResult<()> {
    let endpoint = validate_endpoint(&host, port)?;
    let packet = OscPacket::Message(OscMessage {
        addr: "/chatbox/input".to_string(),
        args: vec![
            OscType::String(text),
            OscType::Bool(send),
            OscType::Bool(notify),
        ],
    });
    let bytes = rosc::encoder::encode(&packet).map_err(app_error)?;
    let socket = UdpSocket::bind("0.0.0.0:0").map_err(app_error)?;
    socket.send_to(&bytes, endpoint).map_err(app_error)?;
    Ok(())
}

fn osc_argument(arg: OscType) -> OscArgument {
    match arg {
        OscType::Float(value) => OscArgument {
            value_type: "float".to_string(),
            value: json!(value),
        },
        OscType::Double(value) => OscArgument {
            value_type: "double".to_string(),
            value: json!(value),
        },
        OscType::Int(value) => OscArgument {
            value_type: "int".to_string(),
            value: json!(value),
        },
        OscType::Long(value) => OscArgument {
            value_type: "long".to_string(),
            value: json!(value),
        },
        OscType::Bool(value) => OscArgument {
            value_type: "bool".to_string(),
            value: json!(value),
        },
        OscType::String(value) => OscArgument {
            value_type: "string".to_string(),
            value: json!(value),
        },
        OscType::Char(value) => OscArgument {
            value_type: "char".to_string(),
            value: json!(value.to_string()),
        },
        OscType::Blob(value) => OscArgument {
            value_type: "blob".to_string(),
            value: json!(format!("{} bytes", value.len())),
        },
        OscType::Nil => OscArgument {
            value_type: "nil".to_string(),
            value: Value::Null,
        },
        OscType::Inf => OscArgument {
            value_type: "inf".to_string(),
            value: json!("Infinity"),
        },
        other => OscArgument {
            value_type: "other".to_string(),
            value: json!(format!("{other:?}")),
        },
    }
}

fn address_matches(pattern: &str, address: &str) -> bool {
    let pattern = pattern.trim();
    if pattern == "*" {
        return true;
    }
    if let Some(prefix) = pattern.strip_suffix('*') {
        return address.starts_with(prefix);
    }
    pattern == address
}

fn route_message(rules: &[OscRouteRule], listen_endpoint: &str, message: &OscMessage) {
    for rule in rules.iter().filter(|rule| rule.enabled) {
        if !address_matches(&rule.source_address, &message.addr) {
            continue;
        }
        let Ok(target_endpoint) = validate_endpoint(&rule.target_host, rule.target_port) else {
            continue;
        };
        let target_address = if rule.target_address.trim().is_empty() {
            message.addr.clone()
        } else {
            match validate_address(&rule.target_address) {
                Ok(address) => address,
                Err(_) => continue,
            }
        };
        if target_endpoint == listen_endpoint && target_address == message.addr {
            continue;
        }
        let packet = OscPacket::Message(OscMessage {
            addr: target_address,
            args: message.args.clone(),
        });
        let Ok(bytes) = rosc::encoder::encode(&packet) else {
            continue;
        };
        if let Ok(socket) = UdpSocket::bind("0.0.0.0:0") {
            let _ = socket.send_to(&bytes, target_endpoint);
        }
    }
}

fn emit_packet(
    app: &AppHandle,
    sender: &str,
    packet: OscPacket,
    routes: &[OscRouteRule],
    listen_endpoint: &str,
) {
    match packet {
        OscPacket::Message(message) => {
            route_message(routes, listen_endpoint, &message);
            let event = OscMonitorEvent {
                address: message.addr,
                args: message.args.into_iter().map(osc_argument).collect(),
                sender: sender.to_string(),
                timestamp: Local::now().to_rfc3339(),
            };
            let _ = app.emit("osc-monitor-event", event);
        }
        OscPacket::Bundle(bundle) => {
            for packet in bundle.content {
                emit_packet(app, sender, packet, routes, listen_endpoint);
            }
        }
    }
}

/// Bind the listening UDP socket used by the OSC monitor.
///
/// Extracted into its own function so the monitor loop can re-create the socket
/// after a fatal transport error (e.g. the network interface went down) instead
/// of dying and forcing the user to manually restart the listener.
fn bind_monitor_socket(endpoint: &str) -> AppResult<TokioUdpSocket> {
    let std_socket = UdpSocket::bind(endpoint).map_err(|err| {
        // Surface a precise, user-actionable message instead of a raw OS error.
        let hint = if err.kind() == std::io::ErrorKind::AddrInUse {
            format!(
                "OSC 监听端口 {endpoint} 已被占用（可能 VRChat 正占用该端口，或上一次监听未完全释放）。可稍候重试或更换端口。"
            )
        } else if err.kind() == std::io::ErrorKind::PermissionDenied {
            format!("没有权限绑定 {endpoint}，请尝试更高权限或更换端口。")
        } else {
            format!("无法绑定 OSC 监听端口 {endpoint}: {err}")
        };
        app_error(hint)
    })?;
    std_socket.set_nonblocking(true).map_err(app_error)?;
    TokioUdpSocket::from_std(std_socket).map_err(app_error)
}

/// Attempt to transparently rebind the OSC monitor socket after a fatal
/// transport error. Tries up to `MAX_REBIND_ATTEMPTS` times with a short
/// backoff between attempts, so transient interface resets recover on their
/// own. Returns `Some(socket)` on success, `None` if every attempt failed.
async fn try_rebind_monitor_socket(
    app_handle: &AppHandle,
    endpoint: &str,
) -> Option<TokioUdpSocket> {
    const MAX_REBIND_ATTEMPTS: u32 = 5;
    const REBIND_BACKOFF: Duration = Duration::from_secs(1);
    for attempt in 1..=MAX_REBIND_ATTEMPTS {
        // Yield so the runtime can service other tasks while we wait for the
        // interface to recover.
        tokio::time::sleep(REBIND_BACKOFF).await;
        match bind_monitor_socket(endpoint) {
            Ok(socket) => return Some(socket),
            Err(err) => {
                let _ = app_handle.emit(
                    "osc-monitor-warning",
                    format!("OSC 监听 socket 重绑失败 (第 {attempt} 次): {}", err.message),
                );
            }
        }
    }
    None
}

#[tauri::command]
pub fn osc_start_monitor(
    app: AppHandle,
    host: String,
    port: u16,
    routes: Option<Vec<OscRouteRule>>,
) -> AppResult<()> {
    let endpoint = validate_endpoint(&host, port)?;

    // Validate all routes up-front so we don't bind a port only to surface a
    // half-started listener to the user.
    let routes_vec = routes.unwrap_or_default();
    for rule in routes_vec.iter().filter(|rule| rule.enabled) {
        validate_endpoint(&rule.target_host, rule.target_port)?;
        if !rule.target_address.trim().is_empty() {
            validate_address(&rule.target_address)?;
        }
    }

    osc_stop_monitor()?;

    let mut socket = bind_monitor_socket(&endpoint)?;

    let app_handle = app.clone();
    let listen_endpoint = endpoint.clone();
    let rebind_endpoint = endpoint.clone();

    let handle = tauri::async_runtime::spawn(async move {
        use futures::FutureExt;
        use std::panic::AssertUnwindSafe;

        OSC_MONITOR_RUNNING.store(true, Ordering::SeqCst);
        let _ = app_handle.emit("osc-monitor-status", true);
        let mut buffer = vec![0_u8; 65_535];

        let outcome = AssertUnwindSafe(async {
            let mut consecutive_errors: u32 = 0;
            loop {
                match socket.recv_from(&mut buffer).await {
                    Ok((0, _)) => continue,
                    Ok((size, sender)) => {
                        consecutive_errors = 0;
                        match rosc::decoder::decode_udp(&buffer[..size]) {
                            Ok((_, packet)) => {
                                emit_packet(
                                    &app_handle,
                                    &sender.to_string(),
                                    packet,
                                    &routes_vec,
                                    &listen_endpoint,
                                );
                            }
                            Err(decode_err) => {
                                // Malformed packets are dropped silently (avoids spam),
                                // but the warning event lets the UI show a counter.
                                let _ = app_handle.emit(
                                    "osc-monitor-warning",
                                    format!("解码失败: {decode_err}"),
                                );
                            }
                        }
                    }
                    Err(error) => {
                        if matches!(error.kind(), std::io::ErrorKind::Interrupted) {
                            consecutive_errors = 0;
                            continue;
                        }
                        consecutive_errors += 1;
                        // Persistent transport errors mean the socket is likely dead
                        // (e.g. the network interface went down). Surface the count, then
                        // try to transparently rebind the socket so monitoring survives
                        // transient outages without user intervention. Only give up (clean
                        // stop) once rebind itself has repeatedly failed.
                        let _ = app_handle.emit(
                            "osc-monitor-warning",
                            format!("socket 错误 (连续第 {consecutive_errors} 次): {error}"),
                        );
                        if consecutive_errors >= 100 {
                            match try_rebind_monitor_socket(&app_handle, &rebind_endpoint).await {
                                Some(new_socket) => {
                                    socket = new_socket;
                                    consecutive_errors = 0;
                                    let _ = app_handle.emit(
                                        "osc-monitor-warning",
                                        "OSC 监听 socket 已自动重绑，继续监听。".to_string(),
                                    );
                                }
                                None => {
                                    let _ = app_handle.emit(
                                        "osc-monitor-error",
                                        "OSC 监听持续出错且重绑失败，已自动停止。请检查网络或端口占用后重新启动监听。".to_string(),
                                    );
                                    break;
                                }
                            }
                        }
                        tokio::time::sleep(Duration::from_millis(50)).await;
                    }
                }
            }
            #[allow(unreachable_code)]
            Ok::<(), String>(())
        })
        .catch_unwind()
        .await;

        OSC_MONITOR_RUNNING.store(false, Ordering::SeqCst);
        let _ = app_handle.emit("osc-monitor-status", false);

        match outcome {
            Ok(Ok(())) => {}
            Ok(Err(error)) => {
                let _ = app_handle.emit("osc-monitor-error", error);
            }
            Err(panic) => {
                // Catch_unwind ensures any internal panic (decode error,
                // emitter failure, etc.) is converted into a user-visible
                // error event instead of silently terminating the task
                // while leaving the listener UI stuck on "monitoring".
                let panic_msg = panic
                    .downcast_ref::<String>()
                    .cloned()
                    .or_else(|| panic.downcast_ref::<&str>().map(|s| (*s).to_string()))
                    .unwrap_or_else(|| "未知 panic".to_string());
                let _ = app_handle.emit(
                    "osc-monitor-error",
                    format!("OSC 监听任务异常终止: {panic_msg}"),
                );
            }
        }
    });

    *OSC_MONITOR
        .get_or_init(|| Mutex::new(None))
        .lock()
        .map_err(app_error)? = Some(handle);
    Ok(())
}

#[tauri::command]
pub fn osc_stop_monitor() -> AppResult<()> {
    if let Some(handle) = OSC_MONITOR
        .get_or_init(|| Mutex::new(None))
        .lock()
        .map_err(app_error)?
        .take()
    {
        handle.abort();
    }
    OSC_MONITOR_RUNNING.store(false, Ordering::SeqCst);
    Ok(())
}

#[cfg(target_os = "windows")]
fn active_window_title() -> String {
    use windows::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowTextLengthW, GetWindowTextW,
    };

    unsafe {
        let window = GetForegroundWindow();
        if window.0.is_null() {
            return String::new();
        }
        let length = GetWindowTextLengthW(window);
        if length <= 0 {
            return String::new();
        }
        let mut buffer = vec![0_u16; length as usize + 1];
        let copied = GetWindowTextW(window, &mut buffer);
        String::from_utf16_lossy(&buffer[..copied.max(0) as usize])
    }
}

#[cfg(not(target_os = "windows"))]
fn active_window_title() -> String {
    String::new()
}

#[cfg(target_os = "windows")]
fn idle_seconds() -> u64 {
    use std::mem::size_of;
    use windows::Win32::System::SystemInformation::GetTickCount64;
    use windows::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};

    unsafe {
        let mut info = LASTINPUTINFO {
            cbSize: size_of::<LASTINPUTINFO>() as u32,
            ..Default::default()
        };
        if GetLastInputInfo(&mut info).as_bool() {
            return GetTickCount64().saturating_sub(info.dwTime as u64) / 1000;
        }
    }
    0
}

#[cfg(not(target_os = "windows"))]
fn idle_seconds() -> u64 {
    0
}

#[cfg(target_os = "windows")]
fn hidden_command(program: &str) -> Command {
    use std::os::windows::process::CommandExt;
    use windows::Win32::System::Threading::CREATE_NO_WINDOW;
    let mut command = Command::new(program);
    command.creation_flags(CREATE_NO_WINDOW.0);
    command
}

#[cfg(not(target_os = "windows"))]
fn hidden_command(program: &str) -> Command {
    Command::new(program)
}

fn parse_nvidia_snapshot(line: &str) -> Option<GpuTelemetry> {
    let mut values = line.split(',').map(str::trim);
    let name = values.next()?.to_string();
    let usage = values.next()?.parse::<f32>().ok()?;
    let memory_used_mb = values.next()?.parse::<f32>().ok()?;
    let memory_total_mb = values.next()?.parse::<f32>().ok()?;
    if name.is_empty() {
        return None;
    }
    Some(GpuTelemetry {
        name,
        usage: Some(usage.clamp(0.0, 100.0)),
        memory_used_gb: Some(memory_used_mb / 1024.0),
        memory_total_gb: Some(memory_total_mb / 1024.0),
        nvidia_available: true,
    })
}

fn query_nvidia_snapshot() -> Option<GpuTelemetry> {
    let output = hidden_command("nvidia-smi")
        .args([
            "--query-gpu=name,utilization.gpu,memory.used,memory.total",
            "--format=csv,noheader,nounits",
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let line = String::from_utf8_lossy(&output.stdout)
        .lines()
        .next()
        .unwrap_or_default()
        .to_string();
    parse_nvidia_snapshot(&line)
}

#[cfg(target_os = "windows")]
fn fallback_gpu_info() -> (String, Option<f32>) {
    use windows::Win32::Graphics::Dxgi::{
        CreateDXGIFactory1, IDXGIFactory1, DXGI_ADAPTER_FLAG_SOFTWARE,
    };

    unsafe {
        let Ok(factory) = CreateDXGIFactory1::<IDXGIFactory1>() else {
            return (String::new(), None);
        };
        let mut index = 0;
        let mut best: Option<(String, usize)> = None;
        while let Ok(adapter) = factory.EnumAdapters1(index) {
            index += 1;
            let Ok(description) = adapter.GetDesc1() else {
                continue;
            };
            if description.Flags & DXGI_ADAPTER_FLAG_SOFTWARE.0 as u32 != 0 {
                continue;
            }
            let end = description
                .Description
                .iter()
                .position(|value| *value == 0)
                .unwrap_or(description.Description.len());
            let name = String::from_utf16_lossy(&description.Description[..end])
                .trim()
                .to_string();
            if name.is_empty() {
                continue;
            }
            let memory = description.DedicatedVideoMemory;
            if best.as_ref().is_none_or(|(_, current)| memory > *current) {
                best = Some((name, memory));
            }
        }
        best.map(|(name, memory)| (name, Some(memory as f32 / BYTES_PER_GIB)))
            .unwrap_or_default()
    }
}

#[cfg(not(target_os = "windows"))]
fn fallback_gpu_info() -> (String, Option<f32>) {
    (String::new(), None)
}

fn gpu_snapshot() -> GpuTelemetry {
    let now = Instant::now();
    let cache = GPU_SAMPLE.get_or_init(|| Mutex::new(None));
    let mut cached = cache
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(sample) = cached.as_ref() {
        let interval = if sample.telemetry.nvidia_available {
            NVIDIA_SAMPLE_INTERVAL
        } else {
            NVIDIA_RETRY_INTERVAL
        };
        if now.duration_since(sample.sampled_at) < interval {
            return sample.telemetry.clone();
        }
    }

    let telemetry = query_nvidia_snapshot().unwrap_or_else(|| {
        let (name, memory_total_gb) = fallback_gpu_info();
        GpuTelemetry {
            name,
            memory_total_gb,
            ..Default::default()
        }
    });
    *cached = Some(CachedGpuSample {
        sampled_at: now,
        telemetry: telemetry.clone(),
    });
    telemetry
}

fn hardware_info(system: &System) -> HardwareInfo {
    let cpu = system.cpus().first();
    HardwareInfo {
        cpu_name: cpu
            .map(|value| value.brand().trim().to_string())
            .unwrap_or_default(),
        cpu_physical_cores: System::physical_core_count().unwrap_or_default(),
        cpu_logical_cores: system.cpus().len(),
        cpu_frequency_mhz: cpu.map(|value| value.frequency()).unwrap_or_default(),
        os_name: System::long_os_version()
            .or_else(System::name)
            .unwrap_or_default(),
        host_name: System::host_name().unwrap_or_default(),
    }
}

fn disk_snapshot() -> (f32, f32, f32) {
    let mut disks = DISKS
        .get_or_init(|| Mutex::new(Disks::new_with_refreshed_list()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    disks.refresh(true);
    let (total, available) = disks
        .list()
        .iter()
        .filter(|disk| !disk.is_removable() && disk.total_space() > 0)
        .fold((0_u64, 0_u64), |(total, available), disk| {
            (
                total.saturating_add(disk.total_space()),
                available.saturating_add(disk.available_space()),
            )
        });
    let used = total.saturating_sub(available);
    let usage = if total > 0 {
        used as f32 / total as f32 * 100.0
    } else {
        0.0
    };
    (
        usage,
        used as f32 / BYTES_PER_GIB,
        total as f32 / BYTES_PER_GIB,
    )
}

pub(crate) fn system_snapshot(include_gpu: bool) -> OscSystemSnapshot {
    let mut system = SYSTEM
        .get_or_init(|| Mutex::new(System::new_all()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    system.refresh_cpu_usage();
    system.refresh_memory();
    system.refresh_processes(ProcessesToUpdate::All, true);

    let total_memory = system.total_memory();
    let used_memory = system.used_memory();
    let vrc_running = system.processes().values().any(|process| {
        process
            .name()
            .to_string_lossy()
            .to_ascii_lowercase()
            .contains("vrchat")
    });
    let hardware = HARDWARE_INFO.get_or_init(|| hardware_info(&system)).clone();
    let gpu = if include_gpu {
        gpu_snapshot()
    } else {
        GpuTelemetry::default()
    };
    let (disk_usage, disk_used_gb, disk_total_gb) = disk_snapshot();
    let now = Local::now();

    OscSystemSnapshot {
        cpu_usage: system.global_cpu_usage(),
        cpu_name: hardware.cpu_name,
        cpu_physical_cores: hardware.cpu_physical_cores,
        cpu_logical_cores: hardware.cpu_logical_cores,
        cpu_frequency_mhz: hardware.cpu_frequency_mhz,
        ram_usage: if total_memory > 0 {
            used_memory as f32 / total_memory as f32 * 100.0
        } else {
            0.0
        },
        memory_used_gb: used_memory as f32 / BYTES_PER_GIB,
        memory_total_gb: total_memory as f32 / BYTES_PER_GIB,
        gpu_name: gpu.name,
        gpu_usage: gpu.usage,
        gpu_memory_used_gb: gpu.memory_used_gb,
        gpu_memory_total_gb: gpu.memory_total_gb,
        disk_usage,
        disk_used_gb,
        disk_total_gb,
        os_name: hardware.os_name,
        host_name: hardware.host_name,
        system_uptime_seconds: System::uptime(),
        idle_seconds: idle_seconds(),
        active_window: active_window_title(),
        local_time: now.format("%H:%M:%S").to_string(),
        local_date: now.format("%Y-%m-%d").to_string(),
        vrc_running,
    }
}

#[tauri::command]
pub fn osc_get_system_snapshot() -> AppResult<OscSystemSnapshot> {
    Ok(system_snapshot(true))
}

fn source_value(source: &str, snapshot: &OscSystemSnapshot) -> Option<f64> {
    let now = Local::now();
    let utc = Utc::now();
    match source.trim().to_ascii_lowercase().as_str() {
        "cpu" | "cpu_usage" => Some(snapshot.cpu_usage as f64 / 100.0),
        "ram" | "ram_usage" => Some(snapshot.ram_usage as f64 / 100.0),
        "gpu" | "gpu_usage" => snapshot.gpu_usage.map(|value| value as f64 / 100.0),
        "memory_used_gb" => Some(snapshot.memory_used_gb as f64),
        "memory_total_gb" => Some(snapshot.memory_total_gb as f64),
        "gpu_memory_used_gb" => snapshot.gpu_memory_used_gb.map(|value| value as f64),
        "gpu_memory_total_gb" => snapshot.gpu_memory_total_gb.map(|value| value as f64),
        "disk" | "disk_usage" => Some(snapshot.disk_usage as f64 / 100.0),
        "disk_used_gb" => Some(snapshot.disk_used_gb as f64),
        "disk_total_gb" => Some(snapshot.disk_total_gb as f64),
        "cpu_physical_cores" => Some(snapshot.cpu_physical_cores as f64),
        "cpu_logical_cores" => Some(snapshot.cpu_logical_cores as f64),
        "cpu_frequency_mhz" => Some(snapshot.cpu_frequency_mhz as f64),
        "system_uptime_seconds" => Some(snapshot.system_uptime_seconds as f64),
        "idle_seconds" => Some(snapshot.idle_seconds as f64),
        "vrc_running" => Some(if snapshot.vrc_running { 1.0 } else { 0.0 }),
        "local_year" => Some(now.year() as f64),
        "local_month" => Some(now.month() as f64),
        "local_day" => Some(now.day() as f64),
        "local_day_of_week" => Some(now.weekday().num_days_from_sunday() as f64),
        "local_hour" => Some(now.hour() as f64),
        "local_minute" => Some(now.minute() as f64),
        "local_second" => Some(now.second() as f64),
        "local_time_of_day" => Some(now.num_seconds_from_midnight() as f64 / 86_400.0),
        "local_timestamp" => Some(now.timestamp_millis() as f64 / 1000.0),
        "utc_hour" => Some(utc.hour() as f64),
        "utc_minute" => Some(utc.minute() as f64),
        "utc_second" => Some(utc.second() as f64),
        "utc_time_of_day" => Some(utc.num_seconds_from_midnight() as f64 / 86_400.0),
        "random" => Some(now.timestamp_subsec_millis() as f64 / 1000.0),
        _ => None,
    }
}

fn transform_value(mut value: f64, mapping: &OscAutomationMapping) -> f64 {
    value = value * mapping.scale + mapping.offset;
    if let Some(min) = mapping.min {
        value = value.max(min);
    }
    if let Some(max) = mapping.max {
        value = value.min(max);
    }
    value
}

#[tauri::command]
pub fn osc_start_automation(app: AppHandle, config: OscAutomationConfig) -> AppResult<()> {
    osc_stop_automation()?;
    validate_endpoint(&config.host, config.port)?;
    for mapping in config.mappings.iter().filter(|mapping| mapping.enabled) {
        validate_address(&mapping.address)?;
    }

    let include_gpu = config
        .mappings
        .iter()
        .any(|mapping| mapping.enabled && mapping.source.to_ascii_lowercase().contains("gpu"));
    let interval_ms = config.interval_ms.clamp(250, 60_000);
    let app_handle = app.clone();

    let handle = tauri::async_runtime::spawn(async move {
        OSC_AUTOMATION_RUNNING.store(true, Ordering::SeqCst);
        let _ = app_handle.emit("osc-automation-status", true);
        let mut ticker = tokio::time::interval(tokio::time::Duration::from_millis(interval_ms));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            ticker.tick().await;
            let snapshot = system_snapshot(include_gpu);
            for mapping in config.mappings.iter().filter(|mapping| mapping.enabled) {
                let Some(value) = source_value(&mapping.source, &snapshot) else {
                    continue;
                };
                let value = transform_value(value, mapping);
                let json_value = match mapping.value_type.as_str() {
                    "bool" => json!(value >= 0.5),
                    "int" | "long" => json!(value.round() as i64),
                    _ => json!(value),
                };
                if let Err(error) = send_message_internal(
                    &config.host,
                    config.port,
                    &mapping.address,
                    &mapping.value_type,
                    json_value,
                ) {
                    let _ = app_handle.emit("osc-automation-error", error.message);
                }
            }
            let _ = app_handle.emit("osc-system-snapshot", &snapshot);
        }
    });

    *OSC_AUTOMATION
        .get_or_init(|| Mutex::new(None))
        .lock()
        .map_err(app_error)? = Some(handle);
    Ok(())
}

#[tauri::command]
pub fn osc_stop_automation() -> AppResult<()> {
    if let Some(handle) = OSC_AUTOMATION
        .get_or_init(|| Mutex::new(None))
        .lock()
        .map_err(app_error)?
        .take()
    {
        handle.abort();
    }
    OSC_AUTOMATION_RUNNING.store(false, Ordering::SeqCst);
    Ok(())
}

#[tauri::command]
pub fn osc_get_status() -> AppResult<OscRuntimeStatus> {
    Ok(OscRuntimeStatus {
        monitor_running: OSC_MONITOR_RUNNING.load(Ordering::SeqCst),
        automation_running: OSC_AUTOMATION_RUNNING.load(Ordering::SeqCst),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nvidia_telemetry_with_device_name() {
        let telemetry = parse_nvidia_snapshot("NVIDIA GeForce GTX 1060, 36, 3893, 6144")
            .expect("valid NVIDIA telemetry");
        assert_eq!(telemetry.name, "NVIDIA GeForce GTX 1060");
        assert_eq!(telemetry.usage, Some(36.0));
        assert!((telemetry.memory_used_gb.unwrap() - 3.801_757_8).abs() < 0.001);
        assert_eq!(telemetry.memory_total_gb, Some(6.0));
    }

    #[test]
    fn rejects_incomplete_nvidia_telemetry() {
        assert!(parse_nvidia_snapshot("NVIDIA GeForce RTX 4070, 25").is_none());
        assert!(parse_nvidia_snapshot("").is_none());
    }

    #[test]
    fn normalizes_system_usage_sources_for_avatar_parameters() {
        let snapshot = OscSystemSnapshot {
            cpu_usage: 25.0,
            cpu_name: String::new(),
            cpu_physical_cores: 0,
            cpu_logical_cores: 0,
            cpu_frequency_mhz: 0,
            ram_usage: 75.0,
            memory_used_gb: 0.0,
            memory_total_gb: 0.0,
            gpu_name: String::new(),
            gpu_usage: Some(50.0),
            gpu_memory_used_gb: None,
            gpu_memory_total_gb: None,
            disk_usage: 0.0,
            disk_used_gb: 0.0,
            disk_total_gb: 0.0,
            os_name: String::new(),
            host_name: String::new(),
            system_uptime_seconds: 0,
            idle_seconds: 0,
            active_window: String::new(),
            local_time: String::new(),
            local_date: String::new(),
            vrc_running: false,
        };
        assert_eq!(source_value("cpu_usage", &snapshot), Some(0.25));
        assert_eq!(source_value("ram_usage", &snapshot), Some(0.75));
        assert_eq!(source_value("gpu_usage", &snapshot), Some(0.5));
    }
}
