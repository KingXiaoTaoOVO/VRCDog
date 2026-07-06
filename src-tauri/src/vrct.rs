use crate::ovr::OvrState;
use crate::translate::{translate, TranslateRequest};
use rosc::{OscMessage, OscPacket, OscType};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::net::UdpSocket;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;

const HISTORY_LIMIT: usize = 80;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VrctMessageSource {
    Chat,
    Mic,
    Speaker,
}

impl Default for VrctMessageSource {
    fn default() -> Self {
        Self::Chat
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VrctProcessRequest {
    pub text: String,
    #[serde(default)]
    pub source: VrctMessageSource,
    #[serde(default = "default_source_lang")]
    pub source_lang: String,
    #[serde(default = "default_target_lang")]
    pub target_lang: String,
    #[serde(default = "default_service")]
    pub service: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub prompt: String,
    #[serde(default)]
    pub custom_api_url: String,
    #[serde(default)]
    pub send_osc: bool,
    #[serde(default = "default_true")]
    pub complete: bool,
    #[serde(default)]
    pub notification: bool,
    #[serde(default = "default_true")]
    pub update_overlay: bool,
    #[serde(default)]
    pub show_original_in_osc: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VrctMessageRecord {
    pub id: u64,
    pub source: VrctMessageSource,
    pub original: String,
    pub translated: String,
    pub source_lang: String,
    pub target_lang: String,
    pub service: String,
    pub sent_osc: bool,
    pub overlay_updated: bool,
    pub timestamp: String,
}

pub struct VrctState {
    history: Arc<Mutex<VecDeque<VrctMessageRecord>>>,
    next_id: Arc<Mutex<u64>>,
}

impl Default for VrctState {
    fn default() -> Self {
        Self::new()
    }
}

impl VrctState {
    pub fn new() -> Self {
        Self {
            history: Arc::new(Mutex::new(VecDeque::with_capacity(HISTORY_LIMIT))),
            next_id: Arc::new(Mutex::new(1)),
        }
    }
}

fn default_source_lang() -> String {
    "auto".into()
}

fn default_target_lang() -> String {
    "zh-CN".into()
}

fn default_service() -> String {
    "google_free".into()
}

fn default_true() -> bool {
    true
}

fn normalize_lang(code: &str) -> String {
    match code {
        "" => "auto".into(),
        "google" => "google_free".into(),
        "bing" => "microsoft".into(),
        "lm_studio" => "lmstudio".into(),
        "zh" => "zh-CN".into(),
        other => other.into(),
    }
}

fn osc_text(req: &VrctProcessRequest, translated: &str) -> String {
    if req.show_original_in_osc && !req.text.trim().is_empty() {
        format!("{} ({})", translated, req.text.trim())
    } else {
        translated.to_string()
    }
}

fn send_osc_chatbox(text: String, complete: bool, notification: bool) -> Result<(), String> {
    let socket = UdpSocket::bind("0.0.0.0:0").map_err(|e| e.to_string())?;
    let msg = OscMessage {
        addr: "/chatbox/input".to_string(),
        args: vec![
            OscType::String(text),
            OscType::Bool(complete),
            OscType::Bool(notification),
        ],
    };
    let packet = OscPacket::Message(msg);
    let msg_buf = rosc::encoder::encode(&packet).map_err(|e| e.to_string())?;
    socket
        .send_to(&msg_buf, "127.0.0.1:9000")
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn vrct_process_message(
    app_handle: AppHandle,
    state: tauri::State<'_, VrctState>,
    ovr_state: tauri::State<'_, OvrState>,
    req: VrctProcessRequest,
) -> crate::AppResult<VrctMessageRecord> {
    let text = req.text.trim().to_string();
    if text.is_empty() {
        return Err("VRCT message text is empty".into());
    }

    let ovr_config = ovr_state.config.lock().await.clone();
    let source_lang = if req.source_lang.trim().is_empty() {
        ovr_config.trans_source_lang
    } else {
        req.source_lang.clone()
    };
    let target_lang = if req.target_lang.trim().is_empty() {
        ovr_config.trans_target_lang
    } else {
        req.target_lang.clone()
    };
    let service = if req.service.trim().is_empty() {
        ovr_config.trans_service
    } else {
        req.service.clone()
    };
    let api_key = if req.api_key.trim().is_empty() {
        ovr_config.trans_api_key
    } else {
        req.api_key.clone()
    };
    let model = if req.model.trim().is_empty() {
        ovr_config.trans_llm_model
    } else {
        req.model.clone()
    };
    let prompt = if req.prompt.trim().is_empty() {
        ovr_config.trans_llm_prompt
    } else {
        req.prompt.clone()
    };
    let custom_api_url = if req.custom_api_url.trim().is_empty() {
        ovr_config.custom_api_url
    } else {
        req.custom_api_url.clone()
    };

    let translate_req = TranslateRequest {
        text: text.clone(),
        source_lang: normalize_lang(&source_lang),
        target_lang: normalize_lang(&target_lang),
        service: normalize_lang(&service),
        api_key,
        model,
        prompt,
        custom_api_url,
    };

    let translated = translate(&translate_req).await?.translated;

    let mut sent_osc = false;
    if req.send_osc {
        send_osc_chatbox(osc_text(&req, &translated), req.complete, req.notification)
            .map_err(crate::AppError::from)?;
        sent_osc = true;
    }

    let mut overlay_updated = false;
    if req.update_overlay {
        crate::ovr::update_overlay_text(&app_handle, &ovr_state, text.clone(), translated.clone())
            .await?;
        overlay_updated = true;
    }

    let mut next_id = state.next_id.lock().await;
    let record = VrctMessageRecord {
        id: *next_id,
        source: req.source.clone(),
        original: text,
        translated,
        source_lang: translate_req.source_lang,
        target_lang: translate_req.target_lang,
        service: translate_req.service,
        sent_osc,
        overlay_updated,
        timestamp: chrono::Local::now().to_rfc3339(),
    };
    *next_id += 1;
    drop(next_id);

    let mut history = state.history.lock().await;
    if history.len() >= HISTORY_LIMIT {
        history.pop_front();
    }
    history.push_back(record.clone());
    drop(history);

    let _ = app_handle.emit("vrct_translation_event", &record);
    Ok(record)
}

#[tauri::command]
pub async fn vrct_get_history(
    state: tauri::State<'_, VrctState>,
) -> crate::AppResult<Vec<VrctMessageRecord>> {
    Ok(state.history.lock().await.iter().cloned().collect())
}

#[tauri::command]
pub async fn vrct_clear_history(state: tauri::State<'_, VrctState>) -> crate::AppResult<()> {
    state.history.lock().await.clear();
    Ok(())
}
