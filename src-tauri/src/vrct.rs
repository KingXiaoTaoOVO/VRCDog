use crate::ovr::OvrState;
use crate::translate::{translate, TranslateRequest};
use rosc::{OscMessage, OscPacket, OscType};
use serde::{Deserialize, Serialize};
use std::collections::{HashSet, VecDeque};
use std::net::UdpSocket;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;

const HISTORY_LIMIT: usize = 80;
const VRC_CHATBOX_LIMIT: usize = 144;

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
    #[serde(default)]
    pub target_langs: Vec<String>,
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
    #[serde(default)]
    pub osc_port: Option<u16>,
    #[serde(default)]
    pub osc_host: Option<String>,
    #[serde(default)]
    pub send_typing: bool,
    #[serde(default)]
    pub message_prefix: String,
    #[serde(default)]
    pub message_suffix: String,
    #[serde(default)]
    pub translation_prefix: String,
    #[serde(default)]
    pub translation_suffix: String,
    #[serde(default = "default_separator")]
    pub separator: String,
    #[serde(default)]
    pub translation_first: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VrctTranslation {
    pub target_lang: String,
    pub translated: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VrctMessageRecord {
    pub id: u64,
    pub source: VrctMessageSource,
    pub original: String,
    pub translated: String,
    pub source_lang: String,
    pub target_lang: String,
    #[serde(default)]
    pub translations: Vec<VrctTranslation>,
    pub service: String,
    pub sent_osc: bool,
    pub overlay_updated: bool,
    pub timestamp: String,
}

pub struct VrctState {
    history: Arc<Mutex<VecDeque<VrctMessageRecord>>>,
    next_id: Arc<Mutex<u64>>,
    history_path: Option<PathBuf>,
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
            history_path: None,
        }
    }

    pub fn with_history_path(history_path: PathBuf) -> Self {
        let mut records = std::fs::read_to_string(&history_path)
            .ok()
            .and_then(|content| serde_json::from_str::<VecDeque<VrctMessageRecord>>(&content).ok())
            .unwrap_or_default();
        while records.len() > HISTORY_LIMIT {
            records.pop_front();
        }
        let next_id = records
            .back()
            .map_or(1, |record| record.id.saturating_add(1));
        Self {
            history: Arc::new(Mutex::new(records)),
            next_id: Arc::new(Mutex::new(next_id)),
            history_path: Some(history_path),
        }
    }

    fn persist_history(&self, records: &VecDeque<VrctMessageRecord>) {
        let Some(path) = self.history_path.as_ref() else {
            return;
        };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(content) = serde_json::to_vec(records) {
            let _ = std::fs::write(path, content);
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

fn default_separator() -> String {
    " ".into()
}

fn normalize_lang(code: &str) -> String {
    let normalized = code.trim().replace('_', "-");
    match normalized.as_str() {
        "" => "auto".into(),
        "google" => "google_free".into(),
        "bing" => "microsoft".into(),
        "lm-studio" | "lm_studio" => "lmstudio".into(),
        "zh" | "zh-Hans" | "zh-Hans-CN" => "zh-CN".into(),
        "zh-TW" | "zh-Hant" | "zh-Hant-TW" => "zh-TW".into(),
        "en-US" | "en-GB" | "en-AU" | "en-CA" => "en".into(),
        "ja-JP" => "ja".into(),
        "ko-KR" => "ko".into(),
        "fr-FR" => "fr".into(),
        "de-DE" => "de".into(),
        "es-ES" => "es".into(),
        "ru-RU" => "ru".into(),
        "pt-PT" => "pt".into(),
        "th-TH" => "th".into(),
        "vi-VN" => "vi".into(),
        other => other.into(),
    }
}

fn format_part(prefix: &str, text: &str, suffix: &str) -> String {
    format!("{}{}{}", prefix, text, suffix)
}

fn osc_text_single(req: &VrctProcessRequest, translated: &str) -> String {
    let message = req.text.trim();
    let translation = translated.trim();
    let translation_part = format_part(
        &req.translation_prefix,
        translation,
        &req.translation_suffix,
    );

    if !req.show_original_in_osc || message.is_empty() {
        return translation_part;
    }

    if req.message_prefix.is_empty()
        && req.message_suffix.is_empty()
        && req.translation_prefix.is_empty()
        && req.translation_suffix.is_empty()
        && req.separator.trim().is_empty()
    {
        return format!("{} ({})", translation, message);
    }

    let message_part = format_part(&req.message_prefix, message, &req.message_suffix);
    if req.translation_first {
        format!("{}{}{}", translation_part, req.separator, message_part)
    } else {
        format!("{}{}{}", message_part, req.separator, translation_part)
    }
}

fn trim_for_chatbox(text: String) -> String {
    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.chars().count() <= VRC_CHATBOX_LIMIT {
        return normalized;
    }

    let mut clipped: String = normalized
        .chars()
        .take(VRC_CHATBOX_LIMIT.saturating_sub(1))
        .collect();
    clipped.push('…');
    clipped
}

fn send_osc_packet(host: &str, port: u16, msg: OscMessage) -> Result<(), String> {
    let socket = UdpSocket::bind("0.0.0.0:0").map_err(|e| e.to_string())?;
    let packet = OscPacket::Message(msg);
    let msg_buf = rosc::encoder::encode(&packet).map_err(|e| e.to_string())?;
    socket
        .send_to(&msg_buf, format!("{}:{}", host, port))
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn send_osc_typing(host: &str, port: u16, flag: bool) -> Result<(), String> {
    let msg = OscMessage {
        addr: "/chatbox/typing".to_string(),
        args: vec![OscType::Bool(flag)],
    };
    send_osc_packet(host, port, msg)
}

fn send_osc_chatbox(
    host: &str,
    port: u16,
    text: String,
    complete: bool,
    notification: bool,
) -> Result<(), String> {
    let msg = OscMessage {
        addr: "/chatbox/input".to_string(),
        args: vec![
            OscType::String(text),
            OscType::Bool(complete),
            OscType::Bool(notification),
        ],
    };
    send_osc_packet(host, port, msg)
}

fn osc_target(req: &VrctProcessRequest) -> (String, u16) {
    let host = req
        .osc_host
        .as_deref()
        .map(str::trim)
        .filter(|host| !host.is_empty())
        .unwrap_or("127.0.0.1")
        .to_string();
    let port = req.osc_port.unwrap_or(9000).max(1);
    (host, port)
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
        return Err("VrcDog translation message text is empty".into());
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

    let source_lang = normalize_lang(&source_lang);
    let service = normalize_lang(&service);
    let target_languages = collect_target_languages(&target_lang, &req.target_langs);
    if target_languages.is_empty() {
        return Err("VrcDog translation target language is empty".into());
    }

    let osc_destination = req.send_osc.then(|| osc_target(&req));
    if req.send_typing {
        if let Some((host, port)) = osc_destination.as_ref() {
            send_osc_typing(host, *port, true).map_err(crate::AppError::from)?;
        }
    }

    let mut translations = Vec::with_capacity(target_languages.len());
    for target_language in &target_languages {
        let translate_req = TranslateRequest {
            text: text.clone(),
            source_lang: source_lang.clone(),
            target_lang: target_language.clone(),
            service: service.clone(),
            api_key: api_key.clone(),
            model: model.clone(),
            prompt: prompt.clone(),
            custom_api_url: custom_api_url.clone(),
        };
        match translate(&translate_req).await {
            Ok(result) => translations.push(VrctTranslation {
                target_lang: target_language.clone(),
                translated: result.translated,
            }),
            Err(error) => {
                if req.send_typing {
                    if let Some((host, port)) = osc_destination.as_ref() {
                        let _ = send_osc_typing(host, *port, false);
                    }
                }
                return Err(error.into());
            }
        }
    }
    let translated = translations
        .first()
        .map(|item| item.translated.clone())
        .unwrap_or_default();

    let mut sent_osc = false;
    if req.send_osc {
        let (osc_host, osc_port) =
            osc_destination.expect("OSC destination exists when send_osc is true");
        let send_result = send_osc_chatbox(
            &osc_host,
            osc_port,
            trim_for_chatbox(osc_text(&req, &translations)),
            req.complete,
            req.notification,
        );
        if req.send_typing {
            let _ = send_osc_typing(&osc_host, osc_port, false);
        }
        send_result.map_err(crate::AppError::from)?;
        sent_osc = true;
    }

    let mut overlay_updated = false;
    if req.update_overlay {
        overlay_updated = crate::ovr::update_overlay_text(
            &app_handle,
            &ovr_state,
            text.clone(),
            overlay_translation_text(&translations),
        )
        .await?;
    }

    let mut next_id = state.next_id.lock().await;
    let record = VrctMessageRecord {
        id: *next_id,
        source: req.source.clone(),
        original: text,
        translated,
        source_lang,
        target_lang: target_languages[0].clone(),
        translations,
        service,
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
    state.persist_history(&history);
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
    let mut history = state.history.lock().await;
    history.clear();
    state.persist_history(&history);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> VrctProcessRequest {
        VrctProcessRequest {
            text: "hello".into(),
            source: VrctMessageSource::Mic,
            source_lang: "en-US".into(),
            target_lang: "zh-CN".into(),
            target_langs: vec![],
            service: "google_free".into(),
            api_key: String::new(),
            model: String::new(),
            prompt: String::new(),
            custom_api_url: String::new(),
            send_osc: true,
            complete: true,
            notification: false,
            update_overlay: true,
            show_original_in_osc: true,
            osc_port: None,
            osc_host: None,
            send_typing: false,
            message_prefix: String::new(),
            message_suffix: String::new(),
            translation_prefix: String::new(),
            translation_suffix: String::new(),
            separator: " ".into(),
            translation_first: false,
        }
    }

    #[test]
    fn target_languages_are_normalized_deduplicated_and_bounded() {
        let languages = collect_target_languages(
            "zh-Hans",
            &[
                "zh-CN".into(),
                "ja-JP".into(),
                "ko-KR".into(),
                "fr-FR".into(),
                "de-DE".into(),
            ],
        );
        assert_eq!(languages, vec!["zh-CN", "ja", "ko", "fr"]);
    }

    #[test]
    fn multilingual_osc_contains_source_and_all_translations_in_one_message() {
        let translations = vec![
            VrctTranslation {
                target_lang: "zh-CN".into(),
                translated: "你好".into(),
            },
            VrctTranslation {
                target_lang: "ja".into(),
                translated: "こんにちは".into(),
            },
        ];
        let text = osc_text(&request(), &translations);
        assert_eq!(text, "[en] hello | [zh-CN] 你好 | [ja] こんにちは");
    }

    #[test]
    fn chatbox_clipping_is_unicode_safe() {
        let clipped = trim_for_chatbox("你".repeat(200));
        assert_eq!(clipped.chars().count(), VRC_CHATBOX_LIMIT);
        assert!(clipped.ends_with('…'));
    }
}

fn collect_target_languages(primary: &str, additional: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    std::iter::once(primary)
        .chain(additional.iter().map(String::as_str))
        .map(normalize_lang)
        .filter(|language| language != "auto" && seen.insert(language.to_ascii_lowercase()))
        .take(4)
        .collect()
}

fn osc_text(req: &VrctProcessRequest, translations: &[VrctTranslation]) -> String {
    if translations.len() <= 1 {
        return osc_text_single(
            req,
            translations
                .first()
                .map(|item| item.translated.as_str())
                .unwrap_or_default(),
        );
    }

    let translated = translations
        .iter()
        .map(|item| format!("[{}] {}", item.target_lang, item.translated.trim()))
        .collect::<Vec<_>>()
        .join(" | ");
    if !req.show_original_in_osc || req.text.trim().is_empty() {
        return translated;
    }

    let original = format!("[{}] {}", normalize_lang(&req.source_lang), req.text.trim());
    if req.translation_first {
        format!("{} | {}", translated, original)
    } else {
        format!("{} | {}", original, translated)
    }
}

fn overlay_translation_text(translations: &[VrctTranslation]) -> String {
    if translations.len() <= 1 {
        return translations
            .first()
            .map(|item| item.translated.clone())
            .unwrap_or_default();
    }
    translations
        .iter()
        .map(|item| format!("[{}] {}", item.target_lang, item.translated.trim()))
        .collect::<Vec<_>>()
        .join("\n")
}
