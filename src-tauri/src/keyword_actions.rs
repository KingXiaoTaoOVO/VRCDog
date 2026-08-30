use crate::{AppError, AppResult};
use rosc::{OscMessage, OscPacket, OscType};
use serde::{Deserialize, Serialize};
use std::net::UdpSocket;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeywordAction {
    pub keyword: String,
    pub address: String,
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_value")]
    pub value: f32,
    #[serde(default)]
    pub value_type: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub cooldown_ms: u64,
}

fn default_host() -> String { "127.0.0.1".into() }
fn default_port() -> u16 { 9000 }
fn default_value() -> f32 { 1.0 }
fn default_true() -> bool { true }

#[derive(Debug, Clone, Serialize)]
pub struct KeywordActionResult { pub matched: Vec<String> }

static LAST_TRIGGERED: OnceLock<Mutex<HashMap<String, Instant>>> = OnceLock::new();

fn send(action: &KeywordAction) -> AppResult<()> {
    if action.keyword.trim().is_empty() || !action.address.starts_with('/') || action.port == 0 { return Err(AppError::from("关键词动作配置无效")); }
    let argument = match action.value_type.trim().to_ascii_lowercase().as_str() {
        "bool" => OscType::Bool(action.value != 0.0),
        "int" => OscType::Int(action.value.round() as i32),
        "double" => OscType::Double(action.value as f64),
        _ => OscType::Float(action.value),
    };
    let packet = OscPacket::Message(OscMessage { addr: action.address.clone(), args: vec![argument] });
    let bytes = rosc::encoder::encode(&packet).map_err(|error| AppError::from(error.to_string()))?;
    UdpSocket::bind("0.0.0.0:0").map_err(|error| AppError::from(error.to_string()))?.send_to(&bytes, format!("{}:{}", action.host, action.port)).map_err(|error| AppError::from(error.to_string()))?;
    Ok(())
}

#[tauri::command]
pub fn keyword_actions_trigger(text: String, actions: Vec<KeywordAction>) -> AppResult<KeywordActionResult> {
    let haystack = text.to_lowercase();
    let mut matched = Vec::new();
    let now = Instant::now();
    let state = LAST_TRIGGERED.get_or_init(|| Mutex::new(HashMap::new()));
    let mut last_triggered = state.lock().map_err(|_| AppError::from("关键词动作状态不可用"))?;
    for action in actions.into_iter().filter(|action| action.enabled && !action.keyword.trim().is_empty()) {
        if !haystack.contains(&action.keyword.to_lowercase()) { continue; }
        let key = format!("{}\0{}\0{}", action.keyword.to_lowercase(), action.address, action.host);
        if action.cooldown_ms > 0 && last_triggered.get(&key).is_some_and(|previous| now.duration_since(*previous).as_millis() < action.cooldown_ms as u128) { continue; }
        send(&action)?;
        last_triggered.insert(key, now);
        matched.push(action.keyword);
    }
    last_triggered.retain(|_, previous| now.duration_since(*previous).as_secs() < 3600);
    Ok(KeywordActionResult { matched })
}

#[cfg(test)]
mod tests {
    use super::{keyword_actions_trigger, KeywordAction};

    #[test]
    fn invalid_keyword_action_is_rejected_before_network_send() {
        let result = keyword_actions_trigger("hello".into(), vec![KeywordAction { keyword: "hello".into(), address: "bad".into(), host: "127.0.0.1".into(), port: 9000, value: 1.0, value_type: "float".into(), enabled: true, cooldown_ms: 0 }]);
        assert!(result.is_err());
    }
}
