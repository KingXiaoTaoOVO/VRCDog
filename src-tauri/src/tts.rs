use crate::{AppError, AppResult};
use base64::Engine;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use tauri::{AppHandle, Manager};
use uuid::Uuid;

const PRESET_FILE: &str = "tts-voice-presets.json";

/// 合成缓存的保留上限：条数 + 存活时间。
/// VRCDog 是常驻工具，通知 TTS 会持续合成 `cache/tts/{uuid}.wav`，
/// 而仓库里原本没有任何删除逻辑 —— 目录只会单调变大直到占满磁盘且毫无提示。
const TTS_CACHE_MAX_FILES: usize = 200;
const TTS_CACHE_MAX_AGE: Duration = Duration::from_secs(7 * 24 * 60 * 60);

/// 计算需要删除的文件（纯函数，便于测试）。
///
/// 输入是 `(路径, 修改时间)` 列表，按「新 → 旧」排序后：
/// 超出 `max_files` 的部分删除；比 `now - max_age` 更老的也删除。
/// 刚写完的那个文件永远是最新的，一定落在保留区间内，不会把正在播放的音频删掉。
fn cache_entries_to_remove(
    mut entries: Vec<(PathBuf, SystemTime)>,
    now: SystemTime,
    max_files: usize,
    max_age: Duration,
) -> Vec<PathBuf> {
    entries.sort_by(|left, right| right.1.cmp(&left.1));
    entries
        .into_iter()
        .enumerate()
        .filter_map(|(index, (path, modified))| {
            let over_cap = index >= max_files;
            // 用传入的 `now` 做比较而不是 `modified.elapsed()`，保证函数对同一输入是确定的；
            // `duration_since` 在 mtime 超前于 now 时报 Err（文件系统时间被往回调过 /
            // NTP 跳变），此时保守地当作未过期保留。
            let expired = matches!(now.duration_since(modified), Ok(age) if age > max_age);
            if over_cap || expired {
                Some(path)
            } else {
                None
            }
        })
        .collect()
}

/// 清理合成缓存。失败一律忽略：缓存清理绝不能反过来让语音合成报错。
async fn prune_tts_cache(root: &Path, max_files: usize, max_age: Duration) {
    let Ok(mut read_dir) = tokio::fs::read_dir(root).await else {
        return;
    };
    let mut entries: Vec<(PathBuf, SystemTime)> = Vec::new();
    loop {
        match read_dir.next_entry().await {
            Ok(Some(entry)) => {
                if let Ok(meta) = entry.metadata().await {
                    if meta.is_file() {
                        let modified = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
                        entries.push((entry.path(), modified));
                    }
                }
            }
            Ok(None) | Err(_) => break,
        }
    }
    let now = SystemTime::now();
    for path in cache_entries_to_remove(entries, now, max_files, max_age) {
        let _ = tokio::fs::remove_file(&path).await;
    }
}

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
        "openai" | "openai_tts" | "siliconflow" => Some("v1/audio/speech"),
        _ => None,
    }
}

fn resolve_endpoint(provider: &str, base_url: &str) -> Result<String, String> {
    let base = if base_url.trim().is_empty() && (provider == "openai" || provider == "openai_tts") {
        "https://api.openai.com"
    } else {
        base_url.trim().trim_end_matches('/')
    };
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

/// 单次 TTS 响应上限。既防止恶意/故障服务端把整块内存吃满，也避免把
/// HTTP 错误页当成音频缓存下来。
const MAX_TTS_AUDIO_BYTES: usize = 32 * 1024 * 1024;

async fn response_audio(response: reqwest::Response) -> Result<(String, Vec<u8>), String> {
    let content_type = response.headers().get(reqwest::header::CONTENT_TYPE).and_then(|value| value.to_str().ok()).unwrap_or("audio/wav").split(';').next().unwrap_or("audio/wav").to_ascii_lowercase();
    let bytes = response.bytes().await.map_err(|error| error.to_string())?.to_vec();
    if bytes.len() > MAX_TTS_AUDIO_BYTES {
        return Err(format!("TTS 响应过大 ({} 字节)", bytes.len()));
    }
    if content_type.contains("json") {
        let value: serde_json::Value = serde_json::from_slice(&bytes).map_err(|error| format!("TTS JSON 响应无效: {error}"))?;
        if let Some(url) = value.get("audio_url").or_else(|| value.get("url")).and_then(serde_json::Value::as_str) {
            let fetched = Client::new().get(url).send().await.map_err(|error| error.to_string())?;
            if !fetched.status().is_success() {
                return Err(format!("TTS 音频下载失败: HTTP {}", fetched.status()));
            }
            let fetched_content_type = fetched.headers().get(reqwest::header::CONTENT_TYPE).and_then(|value| value.to_str().ok()).unwrap_or("audio/wav").split(';').next().unwrap_or("audio/wav").to_string();
            let fetched_bytes = fetched.bytes().await.map_err(|error| error.to_string())?.to_vec();
            if fetched_bytes.len() > MAX_TTS_AUDIO_BYTES {
                return Err(format!("TTS 音频过大 ({} 字节)", fetched_bytes.len()));
            }
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
    let is_openai = provider == "openai" || provider == "openai_tts";
    let payload = if is_openai {
        let voice = if request.voice.is_empty() { "alloy" } else { request.voice.as_str() };
        serde_json::json!({
            "model": "tts-1",
            "input": request.text.trim(),
            "voice": voice,
            "speed": request.speed.clamp(0.5, 2.0),
        })
    } else {
        serde_json::json!({
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
        })
    };
    let response = builder.json(&payload).send().await.map_err(|error| AppError::from(format!("TTS 请求失败: {error}")))?;
    if !response.status().is_success() { let status = response.status(); let body = response.text().await.unwrap_or_default(); return Err(AppError::from(format!("TTS 服务返回 HTTP {status}: {body}"))); }
    let (content_type, audio) = response_audio(response).await.map_err(AppError::from)?;
    let extension = if content_type.contains("mpeg") || content_type.contains("mp3") { "mp3" } else { "wav" };
    let root: PathBuf = app.path().app_cache_dir().map_err(|error| AppError::from(error.to_string()))?.join("tts");
    tokio::fs::create_dir_all(&root).await.map_err(|error| AppError::from(error.to_string()))?;
    let output_path = root.join(format!("{}.{extension}", Uuid::new_v4()));
    tokio::fs::write(&output_path, audio).await.map_err(|error| AppError::from(error.to_string()))?;
    // 写完再清理：刚生成的文件时间戳最新，必然落在保留区间内，
    // 不会出现「前端还没播完就被自己删掉」。清理失败不影响本次合成。
    prune_tts_cache(&root, TTS_CACHE_MAX_FILES, TTS_CACHE_MAX_AGE).await;
    Ok(TtsSynthesisResult { provider, voice: request.voice, text: request.text, output_path: output_path.to_string_lossy().into_owned() })
}

#[cfg(test)]
mod cache_tests {
    use super::cache_entries_to_remove;
    use std::path::PathBuf;
    use std::time::{Duration, SystemTime};

    fn file_at(name: &str, age: Duration) -> (PathBuf, SystemTime) {
        (
            PathBuf::from(name),
            SystemTime::now() - age,
        )
    }

    #[test]
    fn keeps_the_newest_files_and_drops_the_overflow() {
        // 全部都没过期，只按条数上限裁剪
        let entries = vec![
            file_at("old1.wav", Duration::from_secs(600)),
            file_at("new1.wav", Duration::from_secs(10)),
            file_at("old2.wav", Duration::from_secs(900)),
            file_at("mid.wav", Duration::from_secs(300)),
        ];
        let removed = cache_entries_to_remove(entries, SystemTime::now(), 2, Duration::from_secs(3600));
        let names: Vec<String> = removed
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        // 按「新 → 旧」排序后是 new1, mid, old1, old2；上限 2 => 删掉 old1、old2
        assert_eq!(
            names,
            vec!["old1.wav", "old2.wav"],
            "应删最旧的、超出上限的部分"
        );
    }

    #[test]
    fn drops_expired_files_even_when_under_the_count_cap() {
        let entries = vec![
            file_at("fresh.wav", Duration::from_secs(5)),
            file_at("stale.wav", Duration::from_secs(8 * 24 * 3600)),
        ];
        let removed = cache_entries_to_remove(entries, SystemTime::now(), 200, Duration::from_secs(7 * 24 * 3600));
        assert_eq!(removed.len(), 1);
        assert_eq!(removed[0].file_name().unwrap(), "stale.wav");
    }

    #[test]
    fn never_removes_the_freshest_file() {
        // 刚写完的那个文件如果被删掉，前端就播不出声音了
        let entries = vec![file_at("just-written.wav", Duration::from_millis(50))];
        let removed = cache_entries_to_remove(entries, SystemTime::now(), 1, Duration::from_secs(3600));
        assert!(removed.is_empty(), "单个新鲜文件绝不能被清理掉");
    }

    #[test]
    fn empty_cache_is_fine() {
        let removed = cache_entries_to_remove(vec![], SystemTime::now(), 10, Duration::from_secs(60));
        assert!(removed.is_empty());
    }

    #[test]
    fn clock_skew_forward_is_not_treated_as_expired() {
        // 文件系统时间被往回调过（或 NTP 跳变）时，mtime 可能「在未来」：
        // `elapsed()` 会 Err，此时应当保守地当成未过期保留，而不是删掉。
        let future = (
            PathBuf::from("future.wav"),
            SystemTime::now() + Duration::from_secs(3600),
        );
        let removed = cache_entries_to_remove(vec![future], SystemTime::now(), 1, Duration::from_secs(60));
        assert!(removed.is_empty(), "时间戳超前的文件不应被判定为过期");
    }
}
