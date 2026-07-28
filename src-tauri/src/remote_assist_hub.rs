use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::Response,
};
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{mpsc, RwLock};

const MAX_RELAY_TEXT_BYTES: usize = 2 * 1024 * 1024;

#[derive(Clone, Default)]
pub struct RemoteAssistHub {
    inner: Arc<RwLock<HubState>>,
}

#[derive(Default)]
struct HubState {
    peers: HashMap<String, Peer>,
    sessions: HashMap<String, Session>,
}

#[derive(Clone)]
struct Peer {
    sender: mpsc::UnboundedSender<Message>,
    password_hash: [u8; 32],
    hostname: String,
    public_key: String,
    accepting: bool,
}

#[derive(Clone)]
struct Session {
    left: String,
    right: String,
}

impl RemoteAssistHub {
    pub async fn upgrade(self, ws: WebSocketUpgrade) -> Response {
        ws.on_upgrade(move |socket| self.handle_socket(socket))
    }

    async fn handle_socket(self, socket: WebSocket) {
        let (mut socket_tx, mut socket_rx) = socket.split();
        let (outgoing_tx, mut outgoing_rx) = mpsc::unbounded_channel::<Message>();
        let writer = tokio::spawn(async move {
            while let Some(message) = outgoing_rx.recv().await {
                if socket_tx.send(message).await.is_err() {
                    break;
                }
            }
        });

        let mut registered_id: Option<String> = None;
        while let Some(Ok(message)) = socket_rx.next().await {
            match message {
                Message::Text(text) => {
                    if text.len() > MAX_RELAY_TEXT_BYTES {
                        let _ = send_json(
                            &outgoing_tx,
                            json!({"type": "error", "message": "Message is too large"}),
                        );
                        continue;
                    }
                    let Ok(value) = serde_json::from_str::<Value>(&text) else {
                        let _ = send_json(
                            &outgoing_tx,
                            json!({"type": "error", "message": "Invalid JSON message"}),
                        );
                        continue;
                    };
                    if registered_id.is_none() {
                        match self.register_peer(value, outgoing_tx.clone()).await {
                            Ok(device_id) => registered_id = Some(device_id),
                            Err(message) => {
                                let _ = send_json(
                                    &outgoing_tx,
                                    json!({"type": "register_error", "message": message}),
                                );
                            }
                        }
                        continue;
                    }
                    self.handle_peer_message(registered_id.as_deref().unwrap_or_default(), value)
                        .await;
                }
                Message::Ping(payload) => {
                    let _ = outgoing_tx.send(Message::Pong(payload));
                }
                Message::Close(_) => break,
                _ => {}
            }
        }

        if let Some(device_id) = registered_id {
            self.remove_peer(&device_id).await;
        }
        writer.abort();
    }

    async fn register_peer(
        &self,
        value: Value,
        sender: mpsc::UnboundedSender<Message>,
    ) -> Result<String, String> {
        if value.get("type").and_then(Value::as_str) != Some("register") {
            return Err("The first message must register the device".into());
        }
        let device_id = required_string(&value, "device_id")?;
        let password = required_string(&value, "password")?;
        let hostname = value
            .get("hostname")
            .and_then(Value::as_str)
            .unwrap_or("Unknown")
            .trim()
            .to_string();
        let public_key = required_string(&value, "public_key")?;
        if device_id.len() > 64
            || password.len() > 128
            || hostname.len() > 255
            || public_key.len() > 128
        {
            return Err("Registration field is too long".into());
        }

        let peer = Peer {
            sender: sender.clone(),
            password_hash: hash_password(&password),
            hostname,
            public_key,
            accepting: value
                .get("accepting")
                .and_then(Value::as_bool)
                .unwrap_or(true),
        };
        let replaced = self
            .inner
            .write()
            .await
            .peers
            .insert(device_id.clone(), peer);
        if let Some(previous) = replaced {
            let _ = send_json(
                &previous.sender,
                json!({"type": "replaced", "message": "Device registered from another connection"}),
            );
        }
        send_json(
            &sender,
            json!({"type": "registered", "device_id": device_id}),
        )?;
        Ok(device_id)
    }

    async fn handle_peer_message(&self, device_id: &str, value: Value) {
        match value
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default()
        {
            "set_accept" => {
                if let Some(peer) = self.inner.write().await.peers.get_mut(device_id) {
                    peer.accepting = value
                        .get("accepting")
                        .and_then(Value::as_bool)
                        .unwrap_or(false);
                }
            }
            "connect" => self.connect_peer(device_id, &value).await,
            "relay" => self.relay_message(device_id, &value).await,
            "disconnect" => self.disconnect_session(device_id, &value).await,
            "ping" => {
                if let Some(peer) = self.inner.read().await.peers.get(device_id) {
                    let _ = send_json(&peer.sender, json!({"type": "pong"}));
                }
            }
            _ => {
                if let Some(peer) = self.inner.read().await.peers.get(device_id) {
                    let _ = send_json(
                        &peer.sender,
                        json!({"type": "error", "message": "Unsupported remote-assist message"}),
                    );
                }
            }
        }
    }

    async fn connect_peer(&self, device_id: &str, value: &Value) {
        let session_id = match required_string(value, "session_id") {
            Ok(value) => value,
            Err(message) => {
                self.send_connect_error(device_id, "", &message).await;
                return;
            }
        };
        let target_id = match required_string(value, "target_id") {
            Ok(value) => value,
            Err(message) => {
                self.send_connect_error(device_id, &session_id, &message)
                    .await;
                return;
            }
        };
        let password = value
            .get("password")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if target_id == device_id {
            self.send_connect_error(
                device_id,
                &session_id,
                "Cannot connect to this device itself",
            )
            .await;
            return;
        }

        let mut state = self.inner.write().await;
        let Some(source) = state.peers.get(device_id).cloned() else {
            return;
        };
        let Some(target) = state.peers.get(&target_id).cloned() else {
            drop(state);
            self.send_connect_error(device_id, &session_id, "Remote device is offline")
                .await;
            return;
        };
        if !target.accepting {
            drop(state);
            self.send_connect_error(
                device_id,
                &session_id,
                "Remote device is not accepting connections",
            )
            .await;
            return;
        }
        if target.password_hash != hash_password(password) {
            drop(state);
            self.send_connect_error(device_id, &session_id, "Incorrect temporary password")
                .await;
            return;
        }

        state.sessions.insert(
            session_id.clone(),
            Session {
                left: device_id.to_string(),
                right: target_id.clone(),
            },
        );
        drop(state);

        let _ = send_json(
            &source.sender,
            json!({
                "type": "connected",
                "session_id": session_id,
                "peer_id": target_id,
                "peer_hostname": target.hostname,
                "peer_public_key": target.public_key
            }),
        );
        let _ = send_json(
            &target.sender,
            json!({
                "type": "incoming_connected",
                "session_id": session_id,
                "peer_id": device_id,
                "peer_hostname": source.hostname,
                "peer_public_key": source.public_key
            }),
        );
    }

    async fn relay_message(&self, device_id: &str, value: &Value) {
        let Ok(session_id) = required_string(value, "session_id") else {
            return;
        };
        let payload = value.get("payload").cloned().unwrap_or(Value::Null);
        let state = self.inner.read().await;
        let Some(session) = state.sessions.get(&session_id) else {
            return;
        };
        let target_id = if session.left == device_id {
            &session.right
        } else if session.right == device_id {
            &session.left
        } else {
            return;
        };
        if let Some(target) = state.peers.get(target_id) {
            let _ = send_json(
                &target.sender,
                json!({
                    "type": "relay",
                    "session_id": session_id,
                    "from_id": device_id,
                    "payload": payload
                }),
            );
        }
    }

    async fn disconnect_session(&self, device_id: &str, value: &Value) {
        let Ok(session_id) = required_string(value, "session_id") else {
            return;
        };
        let mut state = self.inner.write().await;
        let Some(session) = state.sessions.remove(&session_id) else {
            return;
        };
        let target_id = if session.left == device_id {
            session.right
        } else if session.right == device_id {
            session.left
        } else {
            return;
        };
        if let Some(target) = state.peers.get(&target_id) {
            let _ = send_json(
                &target.sender,
                json!({"type": "peer_disconnected", "session_id": session_id}),
            );
        }
    }

    async fn send_connect_error(&self, device_id: &str, session_id: &str, message: &str) {
        if let Some(peer) = self.inner.read().await.peers.get(device_id) {
            let _ = send_json(
                &peer.sender,
                json!({
                    "type": "connect_error",
                    "session_id": session_id,
                    "message": message
                }),
            );
        }
    }

    async fn remove_peer(&self, device_id: &str) {
        let mut state = self.inner.write().await;
        state.peers.remove(device_id);
        let affected: Vec<(String, String)> = state
            .sessions
            .iter()
            .filter_map(|(session_id, session)| {
                if session.left == device_id {
                    Some((session_id.clone(), session.right.clone()))
                } else if session.right == device_id {
                    Some((session_id.clone(), session.left.clone()))
                } else {
                    None
                }
            })
            .collect();
        for (session_id, target_id) in affected {
            state.sessions.remove(&session_id);
            if let Some(target) = state.peers.get(&target_id) {
                let _ = send_json(
                    &target.sender,
                    json!({"type": "peer_disconnected", "session_id": session_id}),
                );
            }
        }
    }
}

fn required_string(value: &Value, key: &str) -> Result<String, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| format!("Missing {key}"))
}

fn hash_password(password: &str) -> [u8; 32] {
    Sha256::digest(password.as_bytes()).into()
}

fn send_json(sender: &mpsc::UnboundedSender<Message>, value: Value) -> Result<(), String> {
    sender
        .send(Message::Text(value.to_string().into()))
        .map_err(|_| "WebSocket connection is closed".to_string())
}
