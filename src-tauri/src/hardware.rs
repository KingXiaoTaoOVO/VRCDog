use crate::AppResult;
use discord_rich_presence::{activity, DiscordIpc, DiscordIpcClient};
use rosc::{OscMessage, OscPacket, OscType};
use serde::Deserialize;
use std::net::UdpSocket;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;
use sysinfo::System;
use tauri::{AppHandle, Emitter};

static DISCORD_CLIENT: OnceLock<Mutex<DiscordIpcClient>> = OnceLock::new();
static OSC_AUTOMATION: OnceLock<Mutex<Option<tauri::async_runtime::JoinHandle<()>>>> =
    OnceLock::new();
static AUTO_LAUNCH_APPS: OnceLock<Mutex<Vec<std::process::Child>>> = OnceLock::new();

/// 自动启动项允许的可执行文件类型。
///
/// 该列表刻意只包含「双击即可运行」的文件类型，从而让 `sys_start_auto_launch_apps`
/// 只具备「启动一个已存在的程序」的能力，而不是一个通用的任意命令执行原语。
const AUTO_LAUNCH_ALLOWED_EXT: &[&str] = &["exe", "com", "lnk", "msc", "appref-ms"];

/// 把一个自动启动项拆成 `(可执行文件路径, 参数字符串)`。
/// 支持 `"C:\Program Files\App\app.exe" --flag` 与 `C:\App\app.exe --flag` 两种写法。
fn split_auto_launch_entry(raw: &str) -> (String, String) {
    let trimmed = raw.trim();
    if let Some(stripped) = trimmed.strip_prefix('"') {
        if let Some(end) = stripped.find('"') {
            return (
                stripped[..end].to_string(),
                stripped[end + 1..].trim().to_string(),
            );
        }
    }
    let lower = trimmed.to_ascii_lowercase();
    for ext in AUTO_LAUNCH_ALLOWED_EXT {
        let needle = format!(".{ext} ");
        if let Some(idx) = lower.find(&needle) {
            let cut = idx + ext.len() + 1;
            return (trimmed[..cut].to_string(), trimmed[cut..].trim().to_string());
        }
    }
    (trimmed.to_string(), String::new())
}

/// 校验自动启动项的可执行文件路径。
///
/// 修复点：原实现把前端传入的字符串直接交给 `Command::new(cmd).args(args)`，
/// 于是 `"cmd.exe /c ..."`、`"powershell -Command ..."` 这类输入可以在本机执行任意命令
/// （WebView 一旦被注入脚本即可升级为 RCE）。现在要求：
/// 1. 必须是绝对路径，且是真实存在的普通文件；
/// 2. 扩展名在 `AUTO_LAUNCH_ALLOWED_EXT` 白名单内；
/// 3. 路径不含 shell 元字符，避免任何解释器层面的二次解析。
fn validate_auto_launch_target(raw: &str) -> Result<std::path::PathBuf, String> {
    let trimmed = raw.trim().trim_matches('"').trim();
    if trimmed.is_empty() {
        return Err("自动启动项为空".to_string());
    }
    if trimmed
        .chars()
        .any(|c| "&|<>^%!`$;()\n\r\t".contains(c) || c == '\0')
    {
        return Err(format!("自动启动项包含非法字符，已拒绝: {trimmed}"));
    }
    let path = std::path::PathBuf::from(trimmed);
    if !path.is_absolute() {
        return Err(format!("自动启动项必须是绝对路径: {trimmed}"));
    }
    if !path.is_file() {
        return Err(format!("自动启动项不存在或不是文件: {trimmed}"));
    }
    let ext = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if !AUTO_LAUNCH_ALLOWED_EXT.contains(&ext.as_str()) {
        return Err(format!("不支持的自动启动文件类型: .{ext}"));
    }
    Ok(path)
}

#[tauri::command]
pub fn sys_start_auto_launch_apps(apps: Vec<String>) -> AppResult<()> {
    let mut procs = AUTO_LAUNCH_APPS
        .get_or_init(|| Mutex::new(Vec::new()))
        .lock()
        .unwrap();
    if !procs.is_empty() {
        return Ok(());
    } // Already launched

    for app in apps {
        if app.trim().is_empty() {
            continue;
        }

        let (executable, args_str) = split_auto_launch_entry(&app);
        // 先做安全校验：只允许启动「已存在的、白名单扩展名的绝对路径可执行文件」。
        // 校验失败不再静默忽略，而是把原因回报给调用方，方便用户修正配置。
        let executable = validate_auto_launch_target(&executable)?;

        // 参数以独立 argv 元素传入，不经过 shell 解释
        let args: Vec<&str> = if args_str.is_empty() {
            Vec::new()
        } else {
            args_str.split_whitespace().collect()
        };

        // Launch and save actual child process so we can kill it later
        if let Ok(child) = Command::new(&executable).args(args).spawn() {
            procs.push(child);
        }
    }
    Ok(())
}

#[tauri::command]
pub fn sys_kill_auto_launch_apps() -> AppResult<()> {
    let mut procs = AUTO_LAUNCH_APPS
        .get_or_init(|| Mutex::new(Vec::new()))
        .lock()
        .unwrap();
    for mut child in procs.drain(..) {
        let _ = child.kill();
        let _ = child.wait();
    }
    Ok(())
}

#[tauri::command]
pub fn sys_is_vrchat_running() -> AppResult<bool> {
    let mut sys = System::new();
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

    for process in sys.processes().values() {
        if process
            .name()
            .to_string_lossy()
            .to_lowercase()
            .contains("vrchat")
        {
            return Ok(true);
        }
    }
    Ok(false)
}

#[tauri::command]
pub fn sys_launch_vrchat(launch_args: Option<String>) -> AppResult<()> {
    // 仅允许安全的 URI 片段字符，拒绝 shell 元字符（& | && > < 等）注入
    if let Some(ref la) = launch_args {
        if !la
            .chars()
            .all(|c| c.is_alphanumeric() || "-=/.:_%+?".contains(c))
        {
            return Err(crate::AppError::from("launch_args 含非法字符"));
        }
    }

    let url = format!(
        "steam://rungameid/438100{}",
        launch_args
            .map(|la| format!("//{}", la))
            .unwrap_or_default()
    );

    // 不用 cmd 解释器执行拼好的字符串：以双引号包裹 URL 抑制 cmd 元字符，
    // 并用空标题参数("")避免把 URL 误判为 start 的标题
    Command::new("cmd")
        .args(["/C", "start", "", &format!("\"{}\"", url)])
        .spawn()
        .map_err(|e| crate::AppError::from(e.to_string()))?;

    Ok(())
}

#[tauri::command]
pub fn sys_kill_vrchat() -> AppResult<()> {
    // Windows only: kill VRChat.exe
    let _ = Command::new("taskkill")
        .args(["/F", "/IM", "VRChat.exe"])
        .output();
    Ok(())
}

#[tauri::command]
pub fn sys_send_osc_param(address: String, value: f32) -> AppResult<()> {
    let socket = UdpSocket::bind("0.0.0.0:0").map_err(|e| crate::AppError::from(e.to_string()))?;

    let msg = OscMessage {
        addr: address,
        args: vec![OscType::Float(value)],
    };

    let packet = OscPacket::Message(msg);
    let msg_buf =
        rosc::encoder::encode(&packet).map_err(|e| crate::AppError::from(e.to_string()))?;

    socket
        .send_to(&msg_buf, "127.0.0.1:9000")
        .map_err(|e| crate::AppError::from(e.to_string()))?;

    Ok(())
}

#[tauri::command]
pub fn sys_send_osc_chatbox(text: String, complete: bool, delay_secs: Option<f64>, notification: Option<bool>) -> AppResult<()> {
    if let Some(delay) = delay_secs {
        if delay > 0.0 {
            let ms = (delay * 1000.0).round().max(1.0) as u64;
            std::thread::sleep(Duration::from_millis(ms));
        }
    }
    send_osc_chatbox_raw(text, complete, notification.unwrap_or(false))
}

#[tauri::command]
pub fn sys_send_osc_chatbox_multi(
    text: String,
    complete: bool,
    delay_secs: Option<f64>,
    notification: Option<bool>,
    secondary: Option<(String, u16, bool)>,
    tertiary: Option<(String, u16, bool)>,
) -> AppResult<()> {
    if let Some(delay) = delay_secs {
        if delay > 0.0 {
            let ms = (delay * 1000.0).round().max(1.0) as u64;
            std::thread::sleep(Duration::from_millis(ms));
        }
    }
    let notif = notification.unwrap_or(false);
    send_osc_chatbox_raw(text.clone(), complete, notif)?;
    if let Some((host, port, enabled)) = secondary {
        if enabled {
            send_osc_chatbox_to(&text, complete, notif, &host, port)?;
        }
    }
    if let Some((host, port, enabled)) = tertiary {
        if enabled {
            send_osc_chatbox_to(&text, complete, notif, &host, port)?;
        }
    }
    Ok(())
}

fn send_osc_chatbox_to(text: &str, complete: bool, notification: bool, host: &str, port: u16) -> AppResult<()> {
    let socket = UdpSocket::bind("0.0.0.0:0").map_err(|e| crate::AppError::from(e.to_string()))?;
    let msg = OscMessage {
        addr: "/chatbox/input".to_string(),
        args: vec![
            OscType::String(text.to_string()),
            OscType::Bool(complete),
            OscType::Bool(notification),
        ],
    };
    let packet = OscPacket::Message(msg);
    let buf = rosc::encoder::encode(&packet).map_err(|e| crate::AppError::from(e.to_string()))?;
    let endpoint = format!("{}:{}", host, port);
    socket.send_to(&buf, endpoint).map_err(|e| crate::AppError::from(e.to_string()))?;
    Ok(())
}

#[tauri::command]
pub fn sys_send_osc_typing(_text: String, typing: bool) -> AppResult<()> {
    let socket = UdpSocket::bind("0.0.0.0:0").map_err(|e| crate::AppError::from(e.to_string()))?;
    let msg = OscMessage {
        addr: "/chatbox/typing".to_string(),
        args: vec![OscType::Bool(typing)],
    };
    let packet = OscPacket::Message(msg);
    let buf = rosc::encoder::encode(&packet).map_err(|e| crate::AppError::from(e.to_string()))?;
    socket.send_to(&buf, "127.0.0.1:9000").map_err(|e| crate::AppError::from(e.to_string()))?;
    Ok(())
}

#[tauri::command]
pub fn sys_chatbox_keepalive_start(app: AppHandle) -> AppResult<()> {
    if CHATBOX_KEEPALIVE_RUNNING.swap(true, Ordering::SeqCst) {
        return Ok(());
    }
    let handle = app.clone();
    let _ = tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(8));
        loop {
            interval.tick().await;
            if !CHATBOX_KEEPALIVE_RUNNING.load(Ordering::SeqCst) {
                break;
            }
            let _ = send_osc_chatbox_raw(String::new(), true, false);
            let _ = handle.emit("chatbox-keepalive-tick", ());
        }
    });
    Ok(())
}

#[tauri::command]
pub fn sys_chatbox_keepalive_stop() -> AppResult<()> {
    CHATBOX_KEEPALIVE_RUNNING.store(false, Ordering::SeqCst);
    Ok(())
}

fn send_osc_chatbox_raw(text: String, complete: bool, notification: bool) -> AppResult<()> {
    let socket = UdpSocket::bind("0.0.0.0:0").map_err(|e| crate::AppError::from(e.to_string()))?;
    let msg = OscMessage {
        addr: "/chatbox/input".to_string(),
        args: vec![
            OscType::String(text),
            OscType::Bool(complete),
            OscType::Bool(notification),
        ],
    };
    let packet = OscPacket::Message(msg);
    let buf = rosc::encoder::encode(&packet).map_err(|e| crate::AppError::from(e.to_string()))?;
    socket.send_to(&buf, "127.0.0.1:9000").map_err(|e| crate::AppError::from(e.to_string()))?;
    Ok(())
}

static CHATBOX_KEEPALIVE_RUNNING: AtomicBool = AtomicBool::new(false);

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscordRpcRequest {
    pub details: String,
    pub state: String,
    #[serde(default)]
    pub show_world_thumbnail: bool,
    #[serde(default)]
    pub show_join_button: bool,
}

#[tauri::command]
pub fn sys_set_discord_rpc(req: DiscordRpcRequest) -> AppResult<()> {
    let client_mutex = DISCORD_CLIENT.get_or_init(|| {
        let mut client = DiscordIpcClient::new("112233445566778899");
        let _ = client.connect();
        Mutex::new(client)
    });

    if let Ok(mut client) = client_mutex.lock() {
        let mut activity = activity::Activity::new()
            .details(&req.details)
            .state(&req.state);
        if req.show_world_thumbnail {
            // Best-effort: only renders if the configured Discord application
            // registers the "vrchat" asset key. Harmless otherwise.
            activity = activity.assets(activity::Assets::default().large_image("vrchat"));
        }
        if req.show_join_button {
            activity = activity.buttons(vec![activity::Button::new(
                "Join VRChat",
                "https://vrchat.com/home",
            )]);
        }
        let _ = client.set_activity(activity);
    }

    Ok(())
}

#[tauri::command]
pub fn sys_start_osc_automation() -> AppResult<()> {
    let mut automation_lock = OSC_AUTOMATION
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap();
    if automation_lock.is_some() {
        return Ok(());
    }

    let handle = tauri::async_runtime::spawn(async move {
        let socket = match UdpSocket::bind("0.0.0.0:0") {
            Ok(s) => s,
            Err(_) => return,
        };

        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;
            let snapshot = crate::osc::system_snapshot(false);

            // Send CPU usage (0.0 to 1.0)
            let cpu_usage = snapshot.cpu_usage / 100.0;
            let msg_cpu = OscMessage {
                addr: "/avatar/parameters/SystemCPU".to_string(),
                args: vec![OscType::Float(cpu_usage)],
            };

            // Send RAM usage (0.0 to 1.0)
            let ram_usage = snapshot.ram_usage / 100.0;
            let msg_ram = OscMessage {
                addr: "/avatar/parameters/SystemRAM".to_string(),
                args: vec![OscType::Float(ram_usage)],
            };

            if let Ok(cpu_buf) = rosc::encoder::encode(&OscPacket::Message(msg_cpu)) {
                let _ = socket.send_to(&cpu_buf, "127.0.0.1:9000");
            }
            if let Ok(ram_buf) = rosc::encoder::encode(&OscPacket::Message(msg_ram)) {
                let _ = socket.send_to(&ram_buf, "127.0.0.1:9000");
            }
        }
    });

    *automation_lock = Some(handle);
    Ok(())
}

#[tauri::command]
pub fn sys_stop_osc_automation() -> AppResult<()> {
    let mut automation_lock = OSC_AUTOMATION
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap();
    if let Some(handle) = automation_lock.take() {
        handle.abort();
    }
    Ok(())
}

#[tauri::command]
pub fn sys_show_in_explorer(path: String) -> AppResult<()> {
    Command::new("explorer")
        .args(["/select,", &path])
        .spawn()
        .map_err(|e| crate::AppError::from(e.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod auto_launch_tests {
    use super::{split_auto_launch_entry, validate_auto_launch_target};

    /// 在临时目录造一个白名单扩展名的真实文件，避免依赖测试二进制自身的后缀。
    fn temp_executable(name: &str) -> std::path::PathBuf {
        let mut path = std::env::temp_dir();
        path.push(name);
        std::fs::write(&path, b"stub").expect("create temp file");
        path
    }

    #[test]
    fn splits_quoted_entry_with_arguments() {
        let (exe, args) = split_auto_launch_entry("\"C:\\Program Files\\App\\app.exe\" --flag 1");
        assert_eq!(exe, "C:\\Program Files\\App\\app.exe");
        assert_eq!(args, "--flag 1");
    }

    #[test]
    fn splits_unquoted_entry_with_arguments() {
        let (exe, args) = split_auto_launch_entry("C:\\Tools\\app.exe --flag");
        assert_eq!(exe, "C:\\Tools\\app.exe");
        assert_eq!(args, "--flag");
    }

    #[test]
    fn accepts_existing_absolute_executable() {
        let path = temp_executable("vrcdog-autolaunch-ok.exe");
        assert!(validate_auto_launch_target(&path.to_string_lossy()).is_ok());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn rejects_shell_command_smuggling() {
        // 修复前：`cmd.exe /c calc.exe` 会被当作可执行文件 + 参数直接 spawn
        assert!(validate_auto_launch_target("cmd.exe /c calc.exe").is_err());
        assert!(validate_auto_launch_target("C:\\Windows\\System32\\cmd.exe /c calc").is_err());
        assert!(validate_auto_launch_target("C:\\Windows\\System32\\cmd.exe & whoami").is_err());
        assert!(validate_auto_launch_target("C:\\Windows\\System32\\cmd.exe|calc").is_err());
    }

    #[test]
    fn rejects_relative_and_missing_paths() {
        assert!(validate_auto_launch_target("app.exe").is_err());
        assert!(validate_auto_launch_target("C:\\definitely\\missing\\app.exe").is_err());
    }

    #[test]
    fn rejects_non_executable_extension() {
        let path = temp_executable("vrcdog-autolaunch-reject.txt");
        assert!(validate_auto_launch_target(&path.to_string_lossy()).is_err());
        let _ = std::fs::remove_file(&path);
    }
}
