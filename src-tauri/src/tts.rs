use crate::{AppError, AppResult};
use base64::Engine;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};
use uuid::Uuid;

const PRESET_FILE: &str = "tts-voice-presets.json";

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TtsSynthesisRequest {
    pub provider: String,
    pub base_url: String,
    #[serde(default)]
    pub api_key: String,
    pub text: String,
    #[serde(default)]
    pub language: String,
    #[serde(default)]
    pub voice: String,
    #[serde(default = "default_speed")]
    pub speed: f32,
    #[serde(default = "default_volume")]
    pub volume: f32,
    #[serde(default)]
    pub reference_audio: String,
    #[serde(default)]
    pub reference_text: String,
    #[serde(default)]
    pub instruct: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TtsSynthesisResult {
    pub provider: String,
    pub voice: String,
    pub text: String,
    pub output_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TtsVoicePreset {
    pub id: String,
    pub name: String,
    pub provider: String,
    #[serde(default)]
    pub voice: String,
    #[serde(default)]
    pub language: String,
    #[serde(default)]
    pub reference_audio: String,
    #[serde(default)]
    pub reference_text: String,
    #[serde(default)]
    pub instruct: String,
}

async fn preset_file(app: &AppHandle) -> AppResult<PathBuf> {
    Ok(app.path().app_data_dir().map_err(|error| AppError::from(error.to_string()))?.join(PRESET_FILE))
}

async fn read_presets(app: &AppHandle) -> AppResult<Vec<TtsVoicePreset>> {
    let path = preset_file(app).await?;
    let Ok(content) = tokio::fs::read_to_string(path).await else { return Ok(Vec::new()); };
    serde_json::from_str(&content).map_err(|error| AppError::from(format!("TTS 预设文件无效: {error}")))
}

async fn write_presets(app: &AppHandle, presets: &[TtsVoicePreset]) -> AppResult<()> {
    let path = preset_file(app).await?;
    if let Some(parent) = path.parent() { tokio::fs::create_dir_all(parent).await.map_err(|error| AppError::from(error.to_string()))?; }
    let content = serde_json::to_vec_pretty(presets).map_err(|error| AppError::from(error.to_string()))?;
    tokio::fs::write(path, content).await.map_err(|error| AppError::from(error.to_string()))
}

#[tauri::command]
pub async fn tts_list_presets(app: AppHandle) -> AppResult<Vec<TtsVoicePreset>> { read_presets(&app).await }

#[tauri::command]
pub async fn tts_save_preset(app: AppHandle, mut preset: TtsVoicePreset) -> AppResult<Vec<TtsVoicePreset>> {
    preset.name = preset.name.trim().to_string();
    preset.provider = preset.provider.trim().to_ascii_lowercase();
    if preset.name.is_empty() || preset.provider.is_empty() { return Err(AppError::from("TTS 预设名称和 provider 不能为空")); }
    if preset.id.trim().is_empty() { preset.id = Uuid::new_v4().to_string(); }
    let mut presets = read_presets(&app).await?;
    presets.retain(|item| item.id != preset.id);
    presets.push(preset);
    write_presets(&app, &presets).await?;
    Ok(presets)
}

#[tauri::command]
pub async fn tts_delete_preset(app: AppHandle, id: String) -> AppResult<Vec<TtsVoicePreset>> {
    let mut presets = read_presets(&app).await?;
    presets.retain(|item| item.id != id);
    write_presets(&app, &presets).await?;
    Ok(presets)
}

#[tauri::command]
pub async fn tts_export_presets(app: AppHandle, path: String) -> AppResult<()> {
    let destination = PathBuf::from(path.trim());
    if destination.as_os_str().is_empty() { return Err(AppError::from("导出路径不能为空")); }
    if let Some(parent) = destination.parent() { tokio::fs::create_dir_all(parent).await.map_err(|error| AppError::from(error.to_string()))?; }
    let content = serde_json::to_vec_pretty(&read_presets(&app).await?).map_err(|error| AppError::from(error.to_string()))?;
    tokio::fs::write(destination, content).await.map_err(|error| AppError::from(error.to_string()))
}

#[tauri::command]
pub async fn tts_import_presets(app: AppHandle, path: String) -> AppResult<Vec<TtsVoicePreset>> {
    let content = tokio::fs::read_to_string(PathBuf::from(path.trim())).await.map_err(|error| AppError::from(format!("无法读取 TTS 预设: {error}")))?;
    let imported: Vec<TtsVoicePreset> = serde_json::from_str(&content).map_err(|error| AppError::from(format!("TTS 预设格式无效: {error}")))?;
    let mut presets = read_presets(&app).await?;
    for preset in imported {
        if preset.name.trim().is_empty() || preset.provider.trim().is_empty() { continue; }
        presets.retain(|item| item.id != preset.id);
        presets.push(preset);
    }
    write_presets(&app, &presets).await?;
    Ok(presets)
}

fn default_speed() -> f32 { 1.0 }
fn default_volume() -> f32 { 1.0 }

fn provider_path(provider: &str) -> Option<&'static str> {
    match provider.trim().to_ascii_lowercase().as_str() {
        "qwen" => Some("tts/qwen"),
        "moss" | "moss-tts" => Some("tts/moss"),
        "omnivoice" | "omni" | "omni-voice" => Some("tts/omnivoice"),
        "edge" => Some("tts/edge"),
        _ => None,
    }
}

fn resolve_endpoint(provider: &str, base_url: &str) -> Result<String, String> {
    let base = base_url.trim().trim_end_matches('/');
    if !(base.starts_with("http://") || base.starts_with("https://")) {
        return Err("TTS 服务地址必须以 http:// 或 https:// 开头".into());
    }
    if base.ends_with("/tts") || base.contains("/tts/") || base.ends_with("/speech") || base.contains("/speech/") || base.ends_with("/completions") {
        return Ok(base.to_string());
    }
    Ok(format!("{}/{}", base, provider_path(provider).unwrap_or("tts")))
}

#[cfg(test)]
mod tests {
    use super::resolve_endpoint;

    #[test]
    fn provider_endpoint_is_not_appended_twice() {
        assert_eq!(resolve_endpoint("qwen", "https://example.test/tts/qwen").unwrap(), "https://example.test/tts/qwen");
        assert_eq!(resolve_endpoint("moss", "https://example.test/api").unwrap(), "https://example.test/api/tts/moss");
    }
}

async fn response_audio(response: reqwest::Response) -> Result<(String, Vec<u8>), String> {
    let content_type = response.headers().get(reqwest::header::CONTENT_TYPE).and_then(|value| value.to_str().ok()).unwrap_or("audio/wav").split(';').next().unwrap_or("audio/wav").to_ascii_lowercase();
    let bytes = response.bytes().await.map_err(|error| error.to_string())?.to_vec();
    if content_type.contains("json") {
        let value: serde_json::Value = serde_json::from_slice(&bytes).map_err(|error| format!("TTS JSON 响应无效: {error}"))?;
        if let Some(url) = value.get("audio_url").or_else(|| value.get("url")).and_then(serde_json::Value::as_str) {
            let fetched = Client::new().get(url).send().await.map_err(|error| error.to_string())?;
            let fetched_content_type = fetched.headers().get(reqwest::header::CONTENT_TYPE).and_then(|value| value.to_str().ok()).unwrap_or("audio/wav").split(';').next().unwrap_or("audio/wav").to_string();
            let fetched_bytes = fetched.bytes().await.map_err(|error| error.to_string())?.to_vec();
            return Ok((fetched_content_type, fetched_bytes));
        }
        if let Some(encoded) = value.get("audio").or_else(|| value.get("data")).and_then(serde_json::Value::as_str) {
            let encoded = encoded.strip_prefix("data:audio/wav;base64,").unwrap_or(encoded);
            return base64::engine::general_purpose::STANDARD.decode(encoded).map(|audio| ("audio/wav".into(), audio)).map_err(|error| format!("TTS 音频 base64 无效: {error}"));
        }
        return Err("TTS JSON 响应未包含音频".into());
    }
    if bytes.is_empty() { return Err("TTS 返回空音频".into()); }
    Ok((content_type, bytes))
}

#[tauri::command]
pub async fn translation_tts_synthesize(app: AppHandle, request: TtsSynthesisRequest) -> AppResult<TtsSynthesisResult> {
    let provider = request.provider.trim().to_ascii_lowercase();
    if request.text.trim().is_empty() { return Err(AppError::from("TTS 文本不能为空")); }
    let endpoint = resolve_endpoint(&provider, &request.base_url).map_err(AppError::from)?;
    let client = Client::builder().timeout(std::time::Duration::from_secs(120)).build().map_err(|error| AppError::from(error.to_string()))?;
    let mut builder = client.post(endpoint).header("Content-Type", "application/json");
    if !request.api_key.trim().is_empty() { builder = builder.bearer_auth(request.api_key.trim()); }
    let payload = serde_json::json!({
        "input": request.text.trim(),
        "text": request.text.trim(),
        "language": request.language,
        "voice": request.voice,
        "speed": request.speed.clamp(0.5, 2.0),
        "volume": request.volume.clamp(0.0, 1.0),
        "response_format": "wav",
        "stream": false,
        "ref_audio": request.reference_audio,
        "ref_text": request.reference_text,
        "instruct": request.instruct,
    });
    let response = builder.json(&payload).send().await.map_err(|error| AppError::from(format!("TTS 请求失败: {error}")))?;
    if !response.status().is_success() { let status = response.status(); let body = response.text().await.unwrap_or_default(); return Err(AppError::from(format!("TTS 服务返回 HTTP {status}: {body}"))); }
    let (content_type, audio) = response_audio(response).await.map_err(AppError::from)?;
    let extension = if content_type.contains("mpeg") || content_type.contains("mp3") { "mp3" } else { "wav" };
    let root: PathBuf = app.path().app_cache_dir().map_err(|error| AppError::from(error.to_string()))?.join("tts");
    tokio::fs::create_dir_all(&root).await.map_err(|error| AppError::from(error.to_string()))?;
    let output_path = root.join(format!("{}.{extension}", Uuid::new_v4()));
    tokio::fs::write(&output_path, audio).await.map_err(|error| AppError::from(error.to_string()))?;
    Ok(TtsSynthesisResult { provider, voice: request.voice, text: request.text, output_path: output_path.to_string_lossy().into_owned() })
}
