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
        let app = app.trim();
        if app.is_empty() {
            continue;
        }

        // Simple manual parsing to handle quotes and arguments
        let (cmd, args) = if app.starts_with('"') {
            if let Some(end_idx) = app[1..].find('"') {
                let cmd = &app[1..=end_idx];
                let args_str = app[end_idx + 2..].trim();
                let args = args_str.split_whitespace().collect::<Vec<_>>();
                (cmd, args)
            } else {
                (app.trim_matches('"'), vec![])
            }
        } else if let Some(exe_idx) = app.to_lowercase().find(".exe ") {
            let cmd = &app[..exe_idx + 4];
            let args_str = app[exe_idx + 5..].trim();
            let args = args_str.split_whitespace().collect::<Vec<_>>();
            (cmd, args)
        } else {
            let parts: Vec<&str> = app.split_whitespace().collect();
            if parts.len() > 1 && !app.to_lowercase().contains(".exe") {
                (parts[0], parts[1..].to_vec())
            } else {
                (app, vec![])
            }
        };

        // Launch and save actual child process so we can kill it later
        if let Ok(child) = Command::new(cmd).args(args).spawn() {
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
    let mut args = String::from("steam://rungameid/438100");
    if let Some(la) = launch_args {
        args.push_str("//");
        args.push_str(&la);
    }

    Command::new("cmd")
        .args(["/C", "start", &args])
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
pub fn sys_send_osc_chatbox(text: String, complete: bool, delay_secs: Option<f64>) -> AppResult<()> {
    if let Some(delay) = delay_secs {
        if delay > 0.0 {
            let ms = (delay * 1000.0).round().max(1.0) as u64;
            std::thread::sleep(Duration::from_millis(ms));
        }
    }
    send_osc_chatbox_raw(text, complete, false)
}

#[tauri::command]
pub fn sys_send_osc_typing(text: String, typing: bool) -> AppResult<()> {
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
