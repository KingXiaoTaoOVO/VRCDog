use super::{
    desktop,
    input::InputSimulator,
    tunnel::SecureTunnel,
    types::{
    ChatMessage, ConnectionSession, DeviceInfo, ServerConfig, ServerType, WireMessage, REMOTE_STATE,
    },
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::{collections::HashMap, time::Duration};
use tauri::{AppHandle, Emitter};
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use x25519_dalek::{PublicKey, StaticSecret};

lazy_static::lazy_static! {
    static ref TRANSPORT: Mutex<Option<TransportHandle>> = Mutex::new(None);
}

struct TransportHandle {
    command_tx: mpsc::UnboundedSender<TransportCommand>,
    server_url: String,
    transport_id: String,
}

enum TransportCommand {
    Connect {
        session_id: String,
        peer_id: String,
        password: String,
        reply: oneshot::Sender<Result<ConnectionSession, String>>,
    },
    SendWire {
        session_id: String,
        message: WireMessage,
        reply: oneshot::Sender<Result<(), String>>,
    },
    Disconnect {
        session_id: String,
    },
    SetAccept(bool),
    Shutdown,
}

pub async fn start(
    app_handle: AppHandle,
    server: ServerConfig,
    device: DeviceInfo,
    accepting: bool,
) -> Result<(), String> {
    if server.server_type == ServerType::Official {
        return Err(
            "RustDesk public nodes do not support the VRCDog remote-assist protocol. Select a VRCDog server."
                .into(),
        );
    }
    let server_url = websocket_url(&server)?;

    let mut transport = TRANSPORT.lock().await;
    if transport
        .as_ref()
        .is_some_and(|current| current.server_url == server_url)
    {
        return Ok(());
    }
    if let Some(previous) = transport.take() {
        let _ = previous.command_tx.send(TransportCommand::Shutdown);
    }

    let (command_tx, command_rx) = mpsc::unbounded_channel();
    let (ready_tx, ready_rx) = oneshot::channel();
    let task_url = server_url.clone();
    let transport_id = uuid::Uuid::new_v4().to_string();
    let task_transport_id = transport_id.clone();
    tokio::spawn(run_transport(
        app_handle,
        task_url,
        task_transport_id,
        device,
        accepting,
        command_rx,
        ready_tx,
    ));

    match tokio::time::timeout(Duration::from_secs(12), ready_rx).await {
        Ok(Ok(Ok(()))) => {
            *transport = Some(TransportHandle {
                command_tx,
                server_url,
                transport_id,
            });
            Ok(())
        }
        Ok(Ok(Err(error))) => Err(error),
        Ok(Err(_)) => Err("Remote-assist transport stopped during registration".into()),
        Err(_) => Err("Timed out while registering with the remote-assist server".into()),
    }
}

pub async fn stop() {
    if let Some(handle) = TRANSPORT.lock().await.take() {
        let _ = handle.command_tx.send(TransportCommand::Shutdown);
    }
}

pub async fn is_running() -> bool {
    TRANSPORT.lock().await.is_some()
}

pub async fn connect(
    peer_id: String,
    password: String,
) -> Result<ConnectionSession, String> {
    let session_id = uuid::Uuid::new_v4().to_string();
    let (reply_tx, reply_rx) = oneshot::channel();
    let command_tx = command_sender().await?;
    command_tx
        .send(TransportCommand::Connect {
            session_id,
            peer_id,
            password,
            reply: reply_tx,
        })
        .map_err(|_| "Remote-assist transport is not running".to_string())?;
    match tokio::time::timeout(Duration::from_secs(12), reply_rx).await {
        Ok(Ok(result)) => result,
        Ok(Err(_)) => Err("Remote-assist connection task stopped".into()),
        Err(_) => Err("Timed out waiting for the remote device".into()),
    }
}

pub async fn send_wire(session_id: String, message: WireMessage) -> Result<(), String> {
    let (reply_tx, reply_rx) = oneshot::channel();
    command_sender()
        .await?
        .send(TransportCommand::SendWire {
            session_id,
            message,
            reply: reply_tx,
        })
        .map_err(|_| "Remote-assist transport is not running".to_string())?;
    reply_rx
        .await
        .map_err(|_| "Remote-assist transport stopped".to_string())?
}

pub async fn disconnect(session_id: String) -> Result<(), String> {
    command_sender()
        .await?
        .send(TransportCommand::Disconnect { session_id })
        .map_err(|_| "Remote-assist transport is not running".to_string())
}

pub async fn set_accept(accepting: bool) -> Result<(), String> {
    command_sender()
        .await?
        .send(TransportCommand::SetAccept(accepting))
        .map_err(|_| "Remote-assist transport is not running".to_string())
}

async fn command_sender() -> Result<mpsc::UnboundedSender<TransportCommand>, String> {
    TRANSPORT
        .lock()
        .await
        .as_ref()
        .map(|handle| handle.command_tx.clone())
        .ok_or_else(|| "Start the remote-assist service first".to_string())
}

async fn run_transport(
    app_handle: AppHandle,
    server_url: String,
    transport_id: String,
    device: DeviceInfo,
    accepting: bool,
    mut command_rx: mpsc::UnboundedReceiver<TransportCommand>,
    ready_tx: oneshot::Sender<Result<(), String>>,
) {
    let websocket = match tokio::time::timeout(Duration::from_secs(10), connect_async(&server_url))
        .await
    {
        Ok(Ok((socket, _))) => socket,
        Ok(Err(error)) => {
            let _ = ready_tx.send(Err(format!(
                "Failed to connect to remote-assist server {server_url}: {error}"
            )));
            return;
        }
        Err(_) => {
            let _ = ready_tx.send(Err(format!(
                "Timed out connecting to remote-assist server {server_url}"
            )));
            return;
        }
    };
    let (mut writer, mut reader) = websocket.split();
    let (key_secret, key_public) = SecureTunnel::generate_static_keypair();
    let local_device_id = device.id.clone();
    let register = json!({
        "type": "register",
        "device_id": device.id,
        "password": device.password,
        "hostname": device.hostname,
        "accepting": accepting,
        "public_key": BASE64.encode(key_public.as_bytes())
    });
    if let Err(error) = writer.send(Message::Text(register.to_string())).await {
        let _ = ready_tx.send(Err(format!("Failed to register device: {error}")));
        return;
    }

    let registration = tokio::time::timeout(Duration::from_secs(10), reader.next()).await;
    let registration = match registration {
        Ok(Some(Ok(Message::Text(text)))) => serde_json::from_str::<Value>(&text)
            .map_err(|error| format!("Invalid registration response: {error}")),
        Ok(Some(Ok(_))) => Err("Unexpected registration response".to_string()),
        Ok(Some(Err(error))) => Err(format!("Registration failed: {error}")),
        Ok(None) => Err("Server closed the connection during registration".to_string()),
        Err(_) => Err("Timed out waiting for device registration".to_string()),
    };
    match registration {
        Ok(value) if value.get("type").and_then(Value::as_str) == Some("registered") => {
            let _ = ready_tx.send(Ok(()));
        }
        Ok(value) => {
            let message = value
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("Device registration was rejected");
            let _ = ready_tx.send(Err(message.to_string()));
            return;
        }
        Err(error) => {
            let _ = ready_tx.send(Err(error));
            return;
        }
    }

    let mut pending_connections =
        HashMap::<String, oneshot::Sender<Result<ConnectionSession, String>>>::new();
    let mut tunnels = HashMap::<String, SecureTunnel>::new();
    let mut heartbeat = tokio::time::interval(Duration::from_secs(20));
    heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            _ = heartbeat.tick() => {
                if writer.send(Message::Text(json!({"type": "ping"}).to_string())).await.is_err() {
                    break;
                }
            }
            command = command_rx.recv() => {
                let Some(command) = command else { break };
                let should_stop = handle_command(
                    command,
                    &mut writer,
                    &mut pending_connections,
                    &mut tunnels,
                ).await;
                if should_stop {
                    break;
                }
            }
            incoming = reader.next() => {
                match incoming {
                    Some(Ok(Message::Text(text))) => {
                        match serde_json::from_str::<Value>(&text) {
                            Ok(value) => handle_server_message(
                                &app_handle,
                                value,
                                &mut pending_connections,
                                &key_secret,
                                &local_device_id,
                                &mut tunnels,
                            ).await,
                            Err(error) => emit_transport_error(
                                &app_handle,
                                format!("Invalid remote-assist message: {error}"),
                            ),
                        }
                    }
                    Some(Ok(Message::Ping(payload))) => {
                        if writer.send(Message::Pong(payload)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Err(error)) => {
                        emit_transport_error(&app_handle, format!("Remote-assist connection failed: {error}"));
                        break;
                    }
                    _ => {}
                }
            }
        }
    }

    for (_, reply) in pending_connections {
        let _ = reply.send(Err("Remote-assist server connection was lost".into()));
    }
    {
        let mut state = REMOTE_STATE.write().await;
        state.service_on = false;
        state.accepting = false;
        state.sessions.clear();
    }
    let _ = app_handle.emit(
        "remote_assist_event",
        json!({"event": "service_stopped", "reason": "transport_closed"}),
    );
    let mut current = TRANSPORT.lock().await;
    if current
        .as_ref()
        .is_some_and(|handle| handle.transport_id == transport_id)
    {
        current.take();
    }
}

async fn handle_command<S>(
    command: TransportCommand,
    writer: &mut S,
    pending: &mut HashMap<String, oneshot::Sender<Result<ConnectionSession, String>>>,
    tunnels: &mut HashMap<String, SecureTunnel>,
) -> bool
where
    S: futures_util::Sink<Message> + Unpin,
    S::Error: std::fmt::Display,
{
    let (value, failure_reply): (
        Option<Value>,
        Option<oneshot::Sender<Result<ConnectionSession, String>>>,
    ) = match command {
        TransportCommand::Connect {
            session_id,
            peer_id,
            password,
            reply,
        } => {
            let value = json!({
                "type": "connect",
                "session_id": session_id,
                "target_id": peer_id,
                "password": password
            });
            pending.insert(session_id, reply);
            (Some(value), None)
        }
        TransportCommand::SendWire {
            session_id,
            message,
            reply,
        } => {
            let plaintext = match serde_json::to_vec(&message) {
                Ok(payload) => payload,
                Err(error) => {
                    let _ = reply.send(Err(error.to_string()));
                    return false;
                }
            };
            let Some(tunnel) = tunnels.get(&session_id) else {
                let _ = reply.send(Err("Secure session has not been established".into()));
                return false;
            };
            let (nonce, ciphertext) = match tunnel.encrypt_packet(&plaintext) {
                Ok(packet) => packet,
                Err(error) => {
                    let _ = reply.send(Err(error));
                    return false;
                }
            };
            let result = writer
                .send(Message::Text(
                    json!({
                        "type": "relay",
                        "session_id": session_id,
                        "payload": {
                            "encrypted": true,
                            "nonce": nonce,
                            "ciphertext": BASE64.encode(ciphertext)
                        }
                    })
                    .to_string(),
                ))
                .await
                .map_err(|error| error.to_string());
            let failed = result.is_err();
            let _ = reply.send(result);
            return failed;
        }
        TransportCommand::Disconnect { session_id } => (
            Some(json!({"type": "disconnect", "session_id": session_id})),
            None,
        ),
        TransportCommand::SetAccept(accepting) => (
            Some(json!({"type": "set_accept", "accepting": accepting})),
            None,
        ),
        TransportCommand::Shutdown => {
            let _ = writer.send(Message::Close(None)).await;
            return true;
        }
    };

    if let Some(value) = value {
        if let Err(error) = writer.send(Message::Text(value.to_string())).await {
            if let Some(reply) = failure_reply {
                let _ = reply.send(Err(error.to_string()));
            }
            return true;
        }
    }
    false
}

async fn handle_server_message(
    app_handle: &AppHandle,
    value: Value,
    pending: &mut HashMap<String, oneshot::Sender<Result<ConnectionSession, String>>>,
    key_secret: &StaticSecret,
    local_device_id: &str,
    tunnels: &mut HashMap<String, SecureTunnel>,
) {
    match value.get("type").and_then(Value::as_str).unwrap_or_default() {
        "connected" | "incoming_connected" => {
            let session_id = string_field(&value, "session_id");
            let peer_id = string_field(&value, "peer_id");
            if session_id.is_empty() || peer_id.is_empty() {
                return;
            }
            let peer_hostname = string_field(&value, "peer_hostname");
            let peer_public_key = match decode_public_key(&value) {
                Ok(key) => key,
                Err(error) => {
                    if let Some(reply) = pending.remove(&session_id) {
                        let _ = reply.send(Err(error.clone()));
                    }
                    emit_transport_error(app_handle, error);
                    return;
                }
            };
            let shared_secret = key_secret.diffie_hellman(&peer_public_key);
            let (send_prefix, receive_prefix) = if local_device_id < peer_id.as_str() {
                (0, 1)
            } else {
                (1, 0)
            };
            tunnels.insert(
                session_id.clone(),
                SecureTunnel::from_shared_secret_with_prefixes(
                    shared_secret.as_bytes(),
                    send_prefix,
                    receive_prefix,
                ),
            );
            let session = ConnectionSession {
                session_id: session_id.clone(),
                peer_id: peer_id.clone(),
                peer_name: if peer_hostname.is_empty() {
                    format!("Peer-{peer_id}")
                } else {
                    peer_hostname
                },
                started_at: chrono::Local::now().to_rfc3339(),
                conn_type: "relay".into(),
                latency_ms: 0,
                status: "connected".into(),
            };
            {
                let mut state = REMOTE_STATE.write().await;
                state.sessions.retain(|current| current.session_id != session_id);
                state.sessions.push(session.clone());
            }
            if let Some(reply) = pending.remove(&session_id) {
                let _ = reply.send(Ok(session.clone()));
            }
            let _ = app_handle.emit(
                "remote_assist_event",
                json!({
                    "event": "connected",
                    "session_id": session_id,
                    "peer_id": peer_id,
                    "conn_type": "relay",
                    "incoming": value.get("type").and_then(Value::as_str) == Some("incoming_connected")
                }),
            );
        }
        "connect_error" => {
            let session_id = string_field(&value, "session_id");
            let message = value
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("Remote connection was rejected")
                .to_string();
            if let Some(reply) = pending.remove(&session_id) {
                let _ = reply.send(Err(message.clone()));
            }
            emit_transport_error(app_handle, message);
        }
        "relay" => {
            let session_id = string_field(&value, "session_id");
            if let Some(payload) = value.get("payload") {
                match decrypt_wire_message(&session_id, payload, tunnels) {
                    Ok(message) => handle_wire_message(app_handle, &session_id, message).await,
                    Err(error) => emit_transport_error(
                        app_handle,
                        format!("Invalid relayed message: {error}"),
                    ),
                }
            }
        }
        "peer_disconnected" => {
            let session_id = string_field(&value, "session_id");
            tunnels.remove(&session_id);
            remove_session(app_handle, &session_id).await;
        }
        "replaced" => {
            emit_transport_error(
                app_handle,
                "This device ID was registered from another application instance".into(),
            );
        }
        "error" => {
            emit_transport_error(
                app_handle,
                value
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("Remote-assist server error")
                    .to_string(),
            );
        }
        _ => {}
    }
}

async fn handle_wire_message(app_handle: &AppHandle, session_id: &str, message: WireMessage) {
    match message {
        WireMessage::Frame {
            seq,
            w,
            h,
            data,
            keyframe,
        } => {
            let _ = app_handle.emit(
                "remote_assist_frame",
                json!({
                    "session_id": session_id,
                    "seq": seq,
                    "width": w,
                    "height": h,
                    "data": data,
                    "keyframe": keyframe
                }),
            );
        }
        WireMessage::Input(event) => {
            InputSimulator::new().handle_event(&event);
        }
        WireMessage::StartScreen => {
            if let Err(error) = desktop::start_screen_share(session_id.to_string()).await {
                emit_transport_error(app_handle, error);
            }
        }
        WireMessage::StopScreen => {
            desktop::stop_screen_share(session_id).await;
        }
        WireMessage::Chat { text } => {
            let message = ChatMessage {
                id: uuid::Uuid::new_v4().to_string(),
                from: "remote".into(),
                text,
                time: chrono::Local::now().to_rfc3339(),
            };
            REMOTE_STATE.write().await.messages.push(message.clone());
            let _ = app_handle.emit(
                "remote_assist_chat",
                json!({"session_id": session_id, "message": message}),
            );
        }
        WireMessage::Disconnect { .. } => remove_session(app_handle, session_id).await,
        WireMessage::Ping => {
            let _ = send_wire(session_id.to_string(), WireMessage::Pong).await;
        }
        _ => {
            let _ = app_handle.emit(
                "remote_assist_wire",
                json!({"session_id": session_id, "message": message}),
            );
        }
    }
}

async fn remove_session(app_handle: &AppHandle, session_id: &str) {
    desktop::stop_screen_share(session_id).await;
    REMOTE_STATE
        .write()
        .await
        .sessions
        .retain(|session| session.session_id != session_id);
    let _ = app_handle.emit(
        "remote_assist_event",
        json!({"event": "disconnected", "session_id": session_id}),
    );
}

fn emit_transport_error(app_handle: &AppHandle, message: String) {
    let _ = app_handle.emit(
        "remote_assist_event",
        json!({"event": "transport_error", "message": message}),
    );
}

fn websocket_url(server: &ServerConfig) -> Result<String, String> {
    let base = server.api.trim().trim_end_matches('/');
    if base.is_empty() {
        return Err("Remote-assist server URL is empty".into());
    }
    let websocket_base = if let Some(rest) = base.strip_prefix("https://") {
        format!("wss://{rest}")
    } else if let Some(rest) = base.strip_prefix("http://") {
        format!("ws://{rest}")
    } else if base.starts_with("ws://") || base.starts_with("wss://") {
        base.to_string()
    } else {
        format!("ws://{base}")
    };
    Ok(format!("{websocket_base}/api/remote-assist/ws"))
}

fn string_field(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn decode_public_key(value: &Value) -> Result<PublicKey, String> {
    let encoded = value
        .get("peer_public_key")
        .and_then(Value::as_str)
        .ok_or_else(|| "Remote device did not provide an encryption key".to_string())?;
    let bytes = BASE64
        .decode(encoded)
        .map_err(|error| format!("Invalid remote encryption key: {error}"))?;
    let bytes: [u8; 32] = bytes
        .try_into()
        .map_err(|_| "Remote encryption key has an invalid length".to_string())?;
    Ok(PublicKey::from(bytes))
}

fn decrypt_wire_message(
    session_id: &str,
    payload: &Value,
    tunnels: &HashMap<String, SecureTunnel>,
) -> Result<WireMessage, String> {
    if payload.get("encrypted").and_then(Value::as_bool) != Some(true) {
        return Err("Rejected an unencrypted remote-assist payload".into());
    }
    let nonce = payload
        .get("nonce")
        .and_then(Value::as_u64)
        .ok_or_else(|| "Encrypted payload is missing its nonce".to_string())?;
    let ciphertext = payload
        .get("ciphertext")
        .and_then(Value::as_str)
        .ok_or_else(|| "Encrypted payload is missing ciphertext".to_string())?;
    let ciphertext = BASE64
        .decode(ciphertext)
        .map_err(|error| format!("Invalid encrypted payload: {error}"))?;
    let tunnel = tunnels
        .get(session_id)
        .ok_or_else(|| "Secure session has not been established".to_string())?;
    let plaintext = tunnel.decrypt(&ciphertext, nonce)?;
    serde_json::from_slice(&plaintext)
        .map_err(|error| format!("Invalid decrypted remote-assist message: {error}"))
}
