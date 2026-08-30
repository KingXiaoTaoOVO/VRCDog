use crate::{AppError, AppResult};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

const DEFAULT_CAPABILITIES_URL: &str = "https://raw.githubusercontent.com/KingXiaoTaoOVO/vrcdog-releases/main/translation-capabilities.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationRuntimeCapabilities {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub languages: Vec<String>,
    #[serde(default)]
    pub engines: Vec<String>,
    #[serde(default)]
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TranslationRuntimeUpdateResult {
    pub capabilities: TranslationRuntimeCapabilities,
    pub sha256: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RealtimeAsrValidationRequest {
    pub provider: String,
    #[serde(default)]
    pub app_id: String,
    #[serde(default)]
    pub secret_id: String,
    #[serde(default)]
    pub secret_key: String,
    #[serde(default)]
    pub app_key: String,
    #[serde(default)]
    pub access_token: String,
    #[serde(default)]
    pub model: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RealtimeAsrValidationResult { pub provider: String, pub valid: bool, pub message: String }

fn default_schema_version() -> u32 { 1 }
fn capabilities_path(app: &AppHandle) -> AppResult<PathBuf> {
    Ok(app.path().app_data_dir().map_err(|error| AppError::from(error.to_string()))?.join("translation-capabilities.json"))
}

#[tauri::command]
pub async fn translation_runtime_get(app: AppHandle) -> AppResult<TranslationRuntimeCapabilities> {
    let path = capabilities_path(&app)?;
    if let Ok(content) = tokio::fs::read_to_string(path).await {
        if let Ok(capabilities) = serde_json::from_str(&content) { return Ok(capabilities); }
    }
    Ok(TranslationRuntimeCapabilities { schema_version: 1, version: "builtin".into(), languages: vec!["auto".into(), "zh-CN".into(), "en-US".into(), "ja-JP".into(), "ko-KR".into()], engines: vec!["google_free".into(), "google_cloud".into(), "deepl".into(), "openai".into(), "deepseek".into(), "ollama".into()], updated_at: String::new() })
}

#[tauri::command]
pub async fn translation_runtime_update(app: AppHandle, url: Option<String>, expected_sha256: Option<String>) -> AppResult<TranslationRuntimeUpdateResult> {
    let endpoint = url.filter(|value| value.starts_with("https://") || value.starts_with("http://")).unwrap_or_else(|| DEFAULT_CAPABILITIES_URL.into());
    let response = Client::builder().timeout(std::time::Duration::from_secs(20)).build().map_err(|error| AppError::from(error.to_string()))?.get(endpoint).send().await.map_err(|error| AppError::from(format!("翻译能力更新失败: {error}")))?;
    if !response.status().is_success() { return Err(AppError::from(format!("翻译能力更新返回 HTTP {}", response.status()))); }
    let bytes = response.bytes().await.map_err(|error| AppError::from(error.to_string()))?;
    if let Some(expected) = expected_sha256.filter(|value| !value.trim().is_empty()) {
        let mut hasher = Sha256::new(); hasher.update(&bytes);
        if format!("{:x}", hasher.finalize()).eq_ignore_ascii_case(expected.trim()) == false { return Err(AppError::from("翻译能力清单 SHA-256 校验失败")); }
    }
    let capabilities: TranslationRuntimeCapabilities = serde_json::from_slice(&bytes).map_err(|error| AppError::from(format!("翻译能力清单格式无效: {error}")))?;
    let path = capabilities_path(&app)?;
    if let Some(parent) = path.parent() { tokio::fs::create_dir_all(parent).await.map_err(|error| AppError::from(error.to_string()))?; }
    tokio::fs::write(path, &bytes).await.map_err(|error| AppError::from(error.to_string()))?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Ok(TranslationRuntimeUpdateResult { capabilities, sha256: format!("{:x}", hasher.finalize()) })
}

#[tauri::command]
pub async fn realtime_asr_validate(request: RealtimeAsrValidationRequest) -> AppResult<RealtimeAsrValidationResult> {
    let provider = request.provider.trim().to_ascii_lowercase();
    let (valid, message) = match provider.as_str() {
        "tencent_realtime" => ( !request.app_id.trim().is_empty() && !request.secret_id.trim().is_empty() && !request.secret_key.trim().is_empty() && !request.model.trim().is_empty(), "腾讯云实时 ASR 配置字段完整；启动监听时会进行 WebSocket 握手。"),
        "aliyun_realtime" => ( !request.app_key.trim().is_empty() && !request.access_token.trim().is_empty(), "阿里云 NLS 配置字段完整；启动监听时会进行 WebSocket 握手。"),
        _ => (false, "未知的实时 ASR provider"),
    };
    if !valid { return Ok(RealtimeAsrValidationResult { provider, valid, message: message.into() }); }
    Ok(RealtimeAsrValidationResult { provider, valid, message: message.into() })
}
