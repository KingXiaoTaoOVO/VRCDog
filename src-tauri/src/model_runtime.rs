use crate::{AppError, AppResult};
use reqwest::Client;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager};

const SILERO_URL: &str = "https://github.com/snakers4/silero-vad/raw/v4.0/files/silero_vad.onnx";
const SILERO_SHA256: &str = "6b99cbfd39246b6706f98ec13c7c50c6b299181f2474fa05cbc8046acc274396";

/// U²-Netp 抠图权重（HF `BritishWerewolf/U-2-Netp`，Apache-2.0，4,574,861 字节）。
/// 2026-11-06 用 `.scratch/tmp-probe-u2netp.py` 实测：静态输入 `input.1` [1,3,320,320]，
/// CPU 单张约 400ms。SHA-256 是下载文件后本地算出来的，不是抄文档的。
const MATING_URL: &str = "https://huggingface.co/BritishWerewolf/U-2-Netp/resolve/main/onnx/model.onnx";
const MATING_SHA256: &str = "309c8469258dda742793dce0ebea8e6dd393174f89934733ecc8b14c76f4ddd8";
const MATING_FILE: &str = "u2netp.onnx";

#[derive(Debug, Clone, Serialize)]
pub struct ModelStatus {
    pub name: String,
    pub installed: bool,
    pub valid: bool,
    pub path: String,
    pub sha256: Option<String>,
    pub size: u64,
}

fn model_path(app: &AppHandle) -> AppResult<PathBuf> {
    let root = app.path().app_data_dir().map_err(|error| AppError::from(error.to_string()))?;
    Ok(root.join("models").join("silero_vad.onnx"))
}

async fn sha256(path: &PathBuf) -> Result<String, String> {
    let bytes = tokio::fs::read(path).await.map_err(|error| error.to_string())?;
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    Ok(format!("{:x}", hasher.finalize()))
}

async fn status_for(app: &AppHandle) -> AppResult<ModelStatus> {
    let path = model_path(app)?;
    if !path.is_file() {
        if let Ok(resource_dir) = app.path().resource_dir() {
            for bundled in [resource_dir.join("models").join("silero_vad.onnx"), resource_dir.join("silero_vad.onnx")] {
                if bundled.is_file() {
                    let digest = sha256(&bundled).await.ok();
                    let size = tokio::fs::metadata(&bundled).await.map(|meta| meta.len()).unwrap_or(0);
                    return Ok(ModelStatus { name: "silero-vad".into(), installed: true, valid: digest.as_deref() == Some(SILERO_SHA256), path: bundled.to_string_lossy().into_owned(), sha256: digest, size });
                }
            }
        }
    }
    let installed = path.is_file();
    let digest = if installed { sha256(&path).await.ok() } else { None };
    let valid = digest.as_deref() == Some(SILERO_SHA256);
    let size = tokio::fs::metadata(&path).await.map(|meta| meta.len()).unwrap_or(0);
    Ok(ModelStatus { name: "silero-vad".into(), installed, valid, path: path.to_string_lossy().into_owned(), sha256: digest, size })
}

#[tauri::command]
pub async fn model_get_status(app: AppHandle) -> AppResult<ModelStatus> {
    status_for(&app).await
}

#[tauri::command]
pub async fn model_download_silero(app: AppHandle) -> AppResult<ModelStatus> {
    let path = model_path(&app)?;
    if let Some(parent) = path.parent() { tokio::fs::create_dir_all(parent).await.map_err(|error| AppError::from(error.to_string()))?; }
    if let Ok(current) = status_for(&app).await { if current.valid { return Ok(current); } }
    let response = Client::builder().timeout(std::time::Duration::from_secs(120)).build().map_err(|error| AppError::from(error.to_string()))?
        .get(SILERO_URL).send().await.map_err(|error| AppError::from(format!("Silero download failed: {error}")))?;
    if !response.status().is_success() { return Err(AppError::from(format!("Silero download returned HTTP {}", response.status()))); }
    let bytes = response.bytes().await.map_err(|error| AppError::from(error.to_string()))?;
    let temporary = path.with_extension("onnx.download");
    tokio::fs::write(&temporary, &bytes).await.map_err(|error| AppError::from(error.to_string()))?;
    let digest = sha256(&temporary).await.map_err(AppError::from)?;
    if digest != SILERO_SHA256 { let _ = tokio::fs::remove_file(&temporary).await; return Err(AppError::from("Silero SHA-256 校验失败")); }
    tokio::fs::rename(&temporary, &path).await.map_err(|error| AppError::from(error.to_string()))?;
    let result = status_for(&app).await?;
    let _ = app.emit("model-runtime-event", &result);
    Ok(result)
}

// ── U²-Netp 抠图模型（按需下载，不打进安装包）────────────────────────

fn matting_model_path(app: &AppHandle) -> AppResult<PathBuf> {
    let root = app.path().app_data_dir().map_err(|error| AppError::from(error.to_string()))?;
    Ok(root.join("models").join(MATING_FILE))
}

/// 同步查找可用的抠图权重：先看用户已下载的 app_data 版本，再看随包 resources/models。
/// 绘画管线在 `spawn_blocking` 里调用它，所以这里不做哈希（下载时已经校验过），
/// 只确认文件存在且不是半个空壳。
pub fn matting_model_file(app: &AppHandle) -> Option<PathBuf> {
    if let Ok(path) = matting_model_path(app) {
        if path.is_file() && path.metadata().map(|meta| meta.len()).unwrap_or(0) > 1_000_000 {
            return Some(path);
        }
    }
    let resource_dir = app.path().resource_dir().ok()?;
    for candidate in [
        resource_dir.join("models").join(MATING_FILE),
        resource_dir.join(MATING_FILE),
    ] {
        if candidate.is_file() && candidate.metadata().map(|meta| meta.len()).unwrap_or(0) > 1_000_000 {
            return Some(candidate);
        }
    }
    None
}

async fn matting_status_for(app: &AppHandle) -> AppResult<ModelStatus> {
    let path = matting_model_path(app)?;
    let installed = path.is_file();
    let digest = if installed { sha256(&path).await.ok() } else { None };
    let valid = digest.as_deref() == Some(MATING_SHA256);
    let size = if installed { tokio::fs::metadata(&path).await.map(|meta| meta.len()).unwrap_or(0) } else { 0 };
    if !valid {
        if let Some(bundled) = matting_model_file(app) {
            let digest = sha256(&bundled).await.ok();
            let size = tokio::fs::metadata(&bundled).await.map(|meta| meta.len()).unwrap_or(0);
            return Ok(ModelStatus {
                name: "u2netp-matting".into(),
                installed: true,
                valid: digest.as_deref() == Some(MATING_SHA256),
                path: bundled.to_string_lossy().into_owned(),
                sha256: digest,
                size,
            });
        }
    }
    Ok(ModelStatus { name: "u2netp-matting".into(), installed, valid, path: path.to_string_lossy().into_owned(), sha256: digest, size })
}

#[tauri::command]
pub async fn model_get_matting_status(app: AppHandle) -> AppResult<ModelStatus> {
    matting_status_for(&app).await
}

#[tauri::command]
pub async fn model_download_matting(app: AppHandle) -> AppResult<ModelStatus> {
    let path = matting_model_path(&app)?;
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await.map_err(|error| AppError::from(error.to_string()))?;
    }
    if let Ok(current) = matting_status_for(&app).await {
        if current.valid { return Ok(current); }
    }
    let response = Client::builder()
        .timeout(std::time::Duration::from_secs(180))
        .build()
        .map_err(|error| AppError::from(error.to_string()))?
        .get(MATING_URL)
        .send()
        .await
        .map_err(|error| AppError::from(format!("Matting download failed: {error}")))?;
    if !response.status().is_success() {
        return Err(AppError::from(format!("Matting download returned HTTP {}", response.status())));
    }
    let bytes = response.bytes().await.map_err(|error| AppError::from(error.to_string()))?;
    let temporary = path.with_extension("onnx.download");
    tokio::fs::write(&temporary, &bytes).await.map_err(|error| AppError::from(error.to_string()))?;
    let digest = sha256(&temporary).await.map_err(AppError::from)?;
    if digest != MATING_SHA256 {
        let _ = tokio::fs::remove_file(&temporary).await;
        return Err(AppError::from("Matting model SHA-256 校验失败"));
    }
    tokio::fs::rename(&temporary, &path).await.map_err(|error| AppError::from(error.to_string()))?;
    let result = matting_status_for(&app).await?;
    let _ = app.emit("model-runtime-event", &result);
    Ok(result)
}

#[derive(Debug, Clone, Serialize)]
pub struct VadCalibrationResult {    pub source: String,
    pub suggested_threshold: u32,
    pub min_threshold: u32,
    pub max_threshold: u32,
    pub guidance: String,
}

#[tauri::command]
pub fn model_calibrate_vad(source: String, observed_levels: Vec<u32>) -> AppResult<VadCalibrationResult> {
    if !matches!(source.as_str(), "mic" | "speaker") { return Err(AppError::from("VAD source must be mic or speaker")); }
    if observed_levels.is_empty() { return Err(AppError::from("至少需要一组音频电平样本")); }
    let mut sorted = observed_levels;
    sorted.sort_unstable();
    let noise = sorted[sorted.len() / 2];
    let peak = *sorted.last().unwrap_or(&noise);
    let suggested = noise.saturating_add(((peak.saturating_sub(noise) as f32) * 0.28) as u32).clamp(80, 6000);
    Ok(VadCalibrationResult { source, suggested_threshold: suggested, min_threshold: noise.saturating_add(20), max_threshold: peak.max(suggested), guidance: "保持环境安静采样后，再说一句完整句子；建议值会留出语音与噪声的安全间隔。".into() })
}
