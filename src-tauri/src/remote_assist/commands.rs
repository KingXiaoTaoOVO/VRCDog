//! Tauri 命令接口 — 前端调用入口
//!
//! 所有远程协助功能通过这些命令暴露给 Vue 前端。

use super::nat;
use super::types::*;
use tauri::Emitter;

// ─── 设备初始化 ──────────────────────────────────────────────────────────────

/// 初始化远程协助，获取本机设备信息
#[tauri::command]
pub async fn remote_assist_init() -> Result<DeviceInfo, String> {
    let mut state = REMOTE_STATE.write().await;
    if let Some(ref info) = state.device {
        return Ok(info.clone());
    }

    let id = generate_device_id();
    let password = generate_temp_password();
    let hostname = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "Unknown".into());
    let stun = nat::detect_nat();

    let info = DeviceInfo {
        id,
        password,
        hostname,
        platform: "Windows".into(),
        nat_type: stun.nat_type.as_str().into(),
        online: true,
    };

    state.device = Some(info.clone());
    Ok(info)
}

// ─── 服务器管理 ──────────────────────────────────────────────────────────────

/// 获取可用服务器列表（包含官方服务器 + 当前连接的 VrcDog 服务端）
#[tauri::command]
pub async fn remote_assist_get_servers(
    backend_url: Option<String>,
) -> Result<Vec<ServerConfig>, String> {
    let mut servers = default_servers();

    // 如果用户已连接 VrcDog 服务端，将其作为可选服务器加入
    if let Some(url) = backend_url {
        if !url.is_empty() {
            let host = url
                .trim_start_matches("http://")
                .trim_start_matches("https://")
                .trim_end_matches('/');
            servers.insert(
                0,
                ServerConfig {
                    host: host.to_string(),
                    relay: host.to_string(),
                    api: url.clone(),
                    key: String::new(), // VrcDog 服务端使用自己的认证
                    is_official: false,
                    label: format!("VrcDog 服务端 ({})", host),
                    server_type: ServerType::VrcDogBackend,
                },
            );
        }
    }

    Ok(servers)
}

/// 设置当前使用的服务器
#[tauri::command]
pub async fn remote_assist_set_server(server: ServerConfig) -> Result<(), String> {
    let mut state = REMOTE_STATE.write().await;
    state.server = Some(server);
    Ok(())
}

/// 添加自定义服务器（必须是 VrcDog 服务端地址）
#[tauri::command]
pub async fn remote_assist_add_custom_server(
    host: String,
    label: String,
) -> Result<ServerConfig, String> {
    if host.is_empty() {
        return Err("服务器地址不能为空".into());
    }

    let clean_host = host
        .trim_start_matches("http://")
        .trim_start_matches("https://")
        .trim_end_matches('/')
        .to_string();

    Ok(ServerConfig {
        host: clean_host.clone(),
        relay: clean_host.clone(),
        api: if host.starts_with("http") {
            host
        } else {
            format!("http://{}", clean_host)
        },
        key: String::new(),
        is_official: false,
        label: if label.is_empty() {
            format!("自定义 ({})", clean_host)
        } else {
            label
        },
        server_type: ServerType::VrcDogBackend,
    })
}

// ─── 服务控制 ────────────────────────────────────────────────────────────────

/// 启动远程协助服务 (开始接受连接)
#[tauri::command]
pub async fn remote_assist_start_service(app_handle: tauri::AppHandle) -> Result<(), String> {
    let mut state = REMOTE_STATE.write().await;
    if state.service_on {
        return Ok(());
    }
    state.service_on = true;
    state.accepting = true;

    let _ = app_handle.emit(
        "remote_assist_event",
        serde_json::json!({
            "event": "service_started"
        }),
    );
    Ok(())
}

/// 停止远程协助服务
#[tauri::command]
pub async fn remote_assist_stop_service(app_handle: tauri::AppHandle) -> Result<(), String> {
    let mut state = REMOTE_STATE.write().await;
    state.service_on = false;
    state.accepting = false;

    let _ = app_handle.emit(
        "remote_assist_event",
        serde_json::json!({
            "event": "service_stopped"
        }),
    );
    Ok(())
}

// ─── 连接管理 ────────────────────────────────────────────────────────────────

/// 连接到远程设备
#[tauri::command]
pub async fn remote_assist_connect(
    app_handle: tauri::AppHandle,
    peer_id: String,
    _password: String,
) -> Result<ConnectionSession, String> {
    let _ = app_handle.emit(
        "remote_assist_event",
        serde_json::json!({
            "event": "connecting", "peer_id": &peer_id
        }),
    );

    // 1. NAT 探测
    let stun = nat::detect_nat();
    let can_p2p = stun.nat_type.can_hole_punch();

    // 2. 尝试 P2P 或回退到中继
    let conn_type = if can_p2p { "p2p" } else { "relay" };

    let session_id = uuid::Uuid::new_v4().to_string();
    let session = ConnectionSession {
        session_id: session_id.clone(),
        peer_id: peer_id.clone(),
        peer_name: format!("Peer-{}", &peer_id),
        started_at: chrono::Local::now().to_rfc3339(),
        conn_type: conn_type.into(),
        latency_ms: 0,
        status: "connected".into(),
    };

    {
        let mut state = REMOTE_STATE.write().await;
        state.sessions.push(session.clone());
    }

    let _ = app_handle.emit(
        "remote_assist_event",
        serde_json::json!({
            "event": "connected",
            "session_id": &session_id,
            "conn_type": conn_type,
            "peer_id": &peer_id
        }),
    );

    Ok(session)
}

/// 断开连接
#[tauri::command]
pub async fn remote_assist_disconnect(
    app_handle: tauri::AppHandle,
    session_id: String,
) -> Result<(), String> {
    let mut state = REMOTE_STATE.write().await;
    state.sessions.retain(|s| s.session_id != session_id);

    let _ = app_handle.emit(
        "remote_assist_event",
        serde_json::json!({
            "event": "disconnected", "session_id": &session_id
        }),
    );
    Ok(())
}

// ─── 密码与会话 ──────────────────────────────────────────────────────────────

/// 刷新临时密码
#[tauri::command]
pub async fn remote_assist_refresh_password() -> Result<String, String> {
    let new_pwd = generate_temp_password();
    let mut state = REMOTE_STATE.write().await;
    if let Some(ref mut d) = state.device {
        d.password = new_pwd.clone();
    }
    Ok(new_pwd)
}

/// 获取当前会话列表
#[tauri::command]
pub async fn remote_assist_get_sessions() -> Result<Vec<ConnectionSession>, String> {
    let state = REMOTE_STATE.read().await;
    Ok(state.sessions.clone())
}

// ─── 聊天 ────────────────────────────────────────────────────────────────────

/// 发送聊天消息
#[tauri::command]
pub async fn remote_assist_send_chat(
    app_handle: tauri::AppHandle,
    session_id: String,
    message: String,
) -> Result<ChatMessage, String> {
    let msg = ChatMessage {
        id: uuid::Uuid::new_v4().to_string(),
        from: "local".into(),
        text: message,
        time: chrono::Local::now().to_rfc3339(),
    };
    let mut state = REMOTE_STATE.write().await;
    state.messages.push(msg.clone());

    let _ = app_handle.emit(
        "remote_assist_chat",
        serde_json::json!({
            "session_id": &session_id, "message": &msg
        }),
    );
    Ok(msg)
}

/// 获取聊天记录
#[tauri::command]
pub async fn remote_assist_get_chat() -> Result<Vec<ChatMessage>, String> {
    let state = REMOTE_STATE.read().await;
    Ok(state.messages.clone())
}

// ─── 状态查询 ────────────────────────────────────────────────────────────────

/// 获取完整状态
#[tauri::command]
pub async fn remote_assist_get_state() -> Result<serde_json::Value, String> {
    let state = REMOTE_STATE.read().await;
    Ok(serde_json::json!({
        "service_on": state.service_on,
        "accepting": state.accepting,
        "device": state.device,
        "server": state.server,
        "sessions": state.sessions,
    }))
}

/// 切换是否接受连接
#[tauri::command]
pub async fn remote_assist_toggle_accept(accept: bool) -> Result<(), String> {
    let mut state = REMOTE_STATE.write().await;
    state.accepting = accept;
    Ok(())
}

// ─── 内部工具函数 ────────────────────────────────────────────────────────────

fn generate_device_id() -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let machine_id = get_machine_guid().unwrap_or_else(|| "vrcdog-fallback".into());
    let mut hasher = DefaultHasher::new();
    machine_id.hash(&mut hasher);
    format!("{:09}", hasher.finish() % 1_000_000_000)
}

fn generate_temp_password() -> String {
    use std::time::SystemTime;
    let seed = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let chars: Vec<char> = "abcdefghjkmnpqrstuvwxyz23456789".chars().collect();
    let mut pwd = String::with_capacity(8);
    let mut s = seed;
    for _ in 0..8 {
        pwd.push(chars[(s % chars.len() as u128) as usize]);
        s = s
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
    }
    pwd
}

fn get_machine_guid() -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        use winreg::enums::*;
        use winreg::RegKey;
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        if let Ok(key) = hklm.open_subkey("SOFTWARE\\Microsoft\\Cryptography") {
            if let Ok(guid) = key.get_value::<String, _>("MachineGuid") {
                return Some(guid);
            }
        }
    }
    None
}

fn default_servers() -> Vec<ServerConfig> {
    vec![
        ServerConfig {
            host: "rs-ny.rustdesk.com".into(),
            relay: "rs-ny.rustdesk.com".into(),
            api: "https://admin.rustdesk.com".into(),
            key: "OeVuKk5nlHiXp+APNn0Y3pC1Iwpwn44JGqrQCsWqmBw=".into(),
            is_official: true,
            label: "官方节点 (纽约)".into(),
            server_type: ServerType::Official,
        },
        ServerConfig {
            host: "rs-sg.rustdesk.com".into(),
            relay: "rs-sg.rustdesk.com".into(),
            api: "https://admin.rustdesk.com".into(),
            key: "OeVuKk5nlHiXp+APNn0Y3pC1Iwpwn44JGqrQCsWqmBw=".into(),
            is_official: true,
            label: "官方节点 (新加坡)".into(),
            server_type: ServerType::Official,
        },
        ServerConfig {
            host: "rs-cn.rustdesk.com".into(),
            relay: "rs-cn.rustdesk.com".into(),
            api: "https://admin.rustdesk.com".into(),
            key: "OeVuKk5nlHiXp+APNn0Y3pC1Iwpwn44JGqrQCsWqmBw=".into(),
            is_official: true,
            label: "官方节点 (中国)".into(),
            server_type: ServerType::Official,
        },
    ]
}
