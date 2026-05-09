use tauri_plugin_shell::ShellExt;
use tauri::AppHandle;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use std::path::PathBuf;
use reqwest::header::{HeaderMap, HeaderValue, REFERER, USER_AGENT};
use futures_util::StreamExt;
use crate::bilibili::BiliTaskProgressPayload;
use tauri::Emitter;

pub async fn download_stream(
    app: AppHandle,
    id: i64,
    bvid: String,
    url: String,
    dest: PathBuf,
    referer: &str,
    stream_type: &str,
) -> Result<(), String> {
    let client = reqwest::Client::new();
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static("Mozilla/5.0"));
    headers.insert(REFERER, HeaderValue::from_str(referer).unwrap());
    
    let res = client.get(&url).headers(headers).send().await.map_err(|e| e.to_string())?;
    let total_size = res.content_length().unwrap_or(0);
    let mut file = fs::File::create(&dest).await.map_err(|e| e.to_string())?;
    let mut downloaded = 0;
    
    let mut stream = res.bytes_stream();
    let mut last_progress = 0.0;
    
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        file.write_all(&chunk).await.map_err(|e| e.to_string())?;
        downloaded += chunk.len() as u64;
        
        if total_size > 0 {
            let progress = (downloaded as f64 / total_size as f64) * 100.0;
            if progress - last_progress > 1.0 || downloaded == total_size {
                last_progress = progress;
                let detail = format!("下载{}中: {:.1}%", stream_type, progress);
                let _ = app.emit("bili_task_progress", BiliTaskProgressPayload {
                    id, bvid: bvid.clone(), status: "running".into(), progress, detail
                });
            }
        }
    }
    file.flush().await.map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn merge_media(
    app: AppHandle,
    video_dest: PathBuf,
    audio_dest: PathBuf,
    final_dest: PathBuf,
) -> Result<(), String> {
    let sidecar = app.shell().sidecar("ffmpeg")
        .map_err(|e| format!("无法初始化 FFmpeg Sidecar: {}", e))?;
    
    // Tauri's sidecar allows executing the bundled binary safely
    let (mut rx, mut _child) = sidecar
        .args(["-y", "-i", video_dest.to_str().unwrap(), "-i", audio_dest.to_str().unwrap(), "-c:v", "copy", "-c:a", "copy", final_dest.to_str().unwrap()])
        .spawn()
        .map_err(|e| format!("FFmpeg 启动失败: {}", e))?;
    
    // We wait for the command to finish. Sidecar in Tauri v2 returns (Receiver<CommandEvent>, Child).
    // We need to loop over rx to wait until it finishes.
    let mut success = false;
    while let Some(event) = rx.recv().await {
        match event {
            tauri_plugin_shell::process::CommandEvent::Terminated(payload) => {
                if payload.code == Some(0) {
                    success = true;
                }
            }
            _ => {}
        }
    }
    
    if success {
        Ok(())
    } else {
        Err("FFmpeg 合并进程返回非零退出码".to_string())
    }
}
