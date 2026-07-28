use super::{nat, transport, types::*};
use tauri::Emitter;

#[tauri::command]
pub async fn remote_assist_init() -> Result<DeviceInfo, String> {
    let mut state = REMOTE_STATE.write().await;
    if let Some(info) = &state.device {
        return Ok(info.clone());
    }

    let hostname = hostname::get()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|_| "Unknown".into());
    let stun = nat::detect_nat();
    let info = DeviceInfo {
        id: generate_device_id(),
        password: generate_temp_password(),
        hostname,
        platform: std::env::consts::OS.to_string(),
        nat_type: stun.nat_type.as_str().into(),
        online: true,
    };
    state.device = Some(info.clone());
    Ok(info)
}

#[tauri::command]
pub async fn remote_assist_get_servers(
    backend_url: Option<String>,
) -> Result<Vec<ServerConfig>, String> {
    let mut servers = Vec::new();
    if let Some(url) = backend_url
        .map(|value| value.trim().trim_end_matches('/').to_string())
        .filter(|value| !value.is_empty())
    {
        let host = display_host(&url);
        servers.push(ServerConfig {
            host: host.clone(),
            relay: host.clone(),
            api: url,
            key: String::new(),
            is_official: false,
            label: format!("VRCDog Server ({host})"),
            server_type: ServerType::VrcDogBackend,
        });
    }
    Ok(servers)
}

#[tauri::command]
pub async fn remote_assist_set_server(
    app_handle: tauri::AppHandle,
    server: ServerConfig,
) -> Result<(), String> {
    let should_restart = {
        let mut state = REMOTE_STATE.write().await;
        state.server = Some(server);
        state.service_on
    };
    if should_restart {
        transport::stop().await;
        ensure_transport(app_handle).await?;
    }
    Ok(())
}

#[tauri::command]
pub async fn remote_assist_add_custom_server(
    host: String,
    label: String,
) -> Result<ServerConfig, String> {
    let api = normalize_server_url(&host)?;
    let clean_host = display_host(&api);
    Ok(ServerConfig {
        host: clean_host.clone(),
        relay: clean_host.clone(),
        api,
        key: String::new(),
        is_official: false,
        label: if label.trim().is_empty() {
            format!("Custom VRCDog Server ({clean_host})")
        } else {
            label.trim().to_string()
        },
        server_type: ServerType::VrcDogBackend,
    })
}

#[tauri::command]
pub async fn remote_assist_start_service(app_handle: tauri::AppHandle) -> Result<(), String> {
    if transport::is_running().await {
        return Ok(());
    }
    ensure_transport(app_handle).await
}

#[tauri::command]
pub async fn remote_assist_stop_service(app_handle: tauri::AppHandle) -> Result<(), String> {
    transport::stop().await;
    {
        let mut state = REMOTE_STATE.write().await;
        state.service_on = false;
        state.accepting = false;
        state.sessions.clear();
    }
    let _ = app_handle.emit(
        "remote_assist_event",
        serde_json::json!({"event": "service_stopped"}),
    );
    Ok(())
}

#[tauri::command]
pub async fn remote_assist_connect(
    app_handle: tauri::AppHandle,
    peer_id: String,
    password: String,
) -> Result<ConnectionSession, String> {
    let peer_id = peer_id.trim().to_string();
    if peer_id.is_empty() {
        return Err("Remote device ID is required".into());
    }
    if password.is_empty() {
        return Err("Temporary password is required".into());
    }
    ensure_transport(app_handle.clone()).await?;
    let _ = app_handle.emit(
        "remote_assist_event",
        serde_json::json!({"event": "connecting", "peer_id": peer_id}),
    );
    transport::connect(peer_id, password).await
}

#[tauri::command]
pub async fn remote_assist_disconnect(
    app_handle: tauri::AppHandle,
    session_id: String,
) -> Result<(), String> {
    transport::disconnect(session_id.clone()).await?;
    REMOTE_STATE
        .write()
        .await
        .sessions
        .retain(|session| session.session_id != session_id);
    let _ = app_handle.emit(
        "remote_assist_event",
        serde_json::json!({"event": "disconnected", "session_id": session_id}),
    );
    Ok(())
}

#[tauri::command]
pub async fn remote_assist_start_view(session_id: String) -> Result<(), String> {
    require_connected_session(&session_id).await?;
    transport::send_wire(session_id, WireMessage::StartScreen).await
}

#[tauri::command]
pub async fn remote_assist_stop_view(session_id: String) -> Result<(), String> {
    require_connected_session(&session_id).await?;
    transport::send_wire(session_id, WireMessage::StopScreen).await
}

#[tauri::command]
pub async fn remote_assist_send_input(session_id: String, event: InputEvent) -> Result<(), String> {
    require_connected_session(&session_id).await?;
    transport::send_wire(session_id, WireMessage::Input(event)).await
}

#[tauri::command]
pub async fn remote_assist_refresh_password(
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    let new_password = generate_temp_password();
    let should_restart = {
        let mut state = REMOTE_STATE.write().await;
        let device = state
            .device
            .as_mut()
            .ok_or_else(|| "Remote-assist device is not initialized".to_string())?;
        device.password = new_password.clone();
        state.service_on
    };
    if should_restart {
        transport::stop().await;
        ensure_transport(app_handle).await?;
    }
    Ok(new_password)
}

#[tauri::command]
pub async fn remote_assist_get_sessions() -> Result<Vec<ConnectionSession>, String> {
    Ok(REMOTE_STATE.read().await.sessions.clone())
}

#[tauri::command]
pub async fn remote_assist_send_chat(
    app_handle: tauri::AppHandle,
    session_id: String,
    message: String,
) -> Result<ChatMessage, String> {
    let text = message.trim().to_string();
    if text.is_empty() {
        return Err("Chat message cannot be empty".into());
    }
    {
        let state = REMOTE_STATE.read().await;
        if !state
            .sessions
            .iter()
            .any(|session| session.session_id == session_id && session.status == "connected")
        {
            return Err("The remote-assist session is not connected".into());
        }
    }
    transport::send_wire(session_id.clone(), WireMessage::Chat { text: text.clone() }).await?;

    let message = ChatMessage {
        id: uuid::Uuid::new_v4().to_string(),
        from: "local".into(),
        text,
        time: chrono::Local::now().to_rfc3339(),
    };
    REMOTE_STATE.write().await.messages.push(message.clone());
    let _ = app_handle.emit(
        "remote_assist_chat",
        serde_json::json!({"session_id": session_id, "message": message}),
    );
    Ok(message)
}

#[tauri::command]
pub async fn remote_assist_get_chat() -> Result<Vec<ChatMessage>, String> {
    Ok(REMOTE_STATE.read().await.messages.clone())
}

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

#[tauri::command]
pub async fn remote_assist_toggle_accept(accept: bool) -> Result<(), String> {
    if REMOTE_STATE.read().await.service_on {
        transport::set_accept(accept).await?;
    }
    REMOTE_STATE.write().await.accepting = accept;
    Ok(())
}

async fn ensure_transport(app_handle: tauri::AppHandle) -> Result<(), String> {
    if transport::is_running().await {
        return Ok(());
    }
    let (server, device, accepting) = {
        let state = REMOTE_STATE.read().await;
        (
            state
                .server
                .clone()
                .ok_or_else(|| "Select a VRCDog remote-assist server first".to_string())?,
            state
                .device
                .clone()
                .ok_or_else(|| "Remote-assist device is not initialized".to_string())?,
            if state.service_on {
                state.accepting
            } else {
                true
            },
        )
    };
    transport::start(app_handle.clone(), server, device, accepting).await?;
    let mut state = REMOTE_STATE.write().await;
    state.service_on = true;
    state.accepting = accepting;
    drop(state);
    let _ = app_handle.emit(
        "remote_assist_event",
        serde_json::json!({"event": "service_started"}),
    );
    Ok(())
}

async fn require_connected_session(session_id: &str) -> Result<(), String> {
    if REMOTE_STATE
        .read()
        .await
        .sessions
        .iter()
        .any(|session| session.session_id == session_id && session.status == "connected")
    {
        Ok(())
    } else {
        Err("The remote-assist session is not connected".into())
    }
}

fn normalize_server_url(value: &str) -> Result<String, String> {
    let value = value.trim().trim_end_matches('/');
    if value.is_empty() {
        return Err("Server URL cannot be empty".into());
    }
    if value.starts_with("http://")
        || value.starts_with("https://")
        || value.starts_with("ws://")
        || value.starts_with("wss://")
    {
        Ok(value.to_string())
    } else {
        Ok(format!("http://{value}"))
    }
}

fn display_host(value: &str) -> String {
    value
        .trim_start_matches("http://")
        .trim_start_matches("https://")
        .trim_start_matches("ws://")
        .trim_start_matches("wss://")
        .trim_end_matches('/')
        .to_string()
}

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
    let mut password = String::with_capacity(8);
    let mut state = seed;
    for _ in 0..8 {
        password.push(chars[(state % chars.len() as u128) as usize]);
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
    }
    password
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
