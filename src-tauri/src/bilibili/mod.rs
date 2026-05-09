use serde::{Deserialize, Serialize};
use reqwest::header::{HeaderMap, HeaderValue, COOKIE, USER_AGENT, REFERER};

pub mod parser;
pub mod queue;

pub use parser::bili_parse_url;

#[derive(Serialize, Deserialize, Debug)]
pub struct BiliResponse<T> {
    pub code: i32,
    pub message: String,
    pub data: Option<T>,
}

pub fn make_headers(sessdata: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36"));
    headers.insert(REFERER, HeaderValue::from_static("https://www.bilibili.com"));
    if !sessdata.is_empty() {
        let cookie_str = format!("SESSDATA={}", sessdata.trim());
        if let Ok(val) = HeaderValue::from_str(&cookie_str) {
            headers.insert(COOKIE, val);
        }
    }
    headers
}

#[tauri::command]
pub async fn bili_check_login(sessdata: String) -> Result<bool, String> {
    let client = reqwest::Client::new();
    let res = client.get("https://api.bilibili.com/x/space/myinfo")
        .headers(make_headers(&sessdata))
        .send()
        .await
        .map_err(|e| e.to_string())?;
        
    let text = res.text().await.map_err(|e| e.to_string())?;
    let body: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    
    if body["code"].as_i64() == Some(0) {
        Ok(true)
    } else {
        Ok(false)
    }
}

#[tauri::command]
pub async fn bili_new_qr() -> Result<serde_json::Value, String> {
    let client = reqwest::Client::new();
    let res = client.get("https://passport.bilibili.com/x/passport-login/web/qrcode/generate")
        .headers(make_headers(""))
        .send()
        .await
        .map_err(|e| e.to_string())?;
        
    let text = res.text().await.map_err(|e| e.to_string())?;
    let body: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    Ok(body)
}

#[tauri::command]
pub async fn bili_get_qr_status(qr_key: String) -> Result<serde_json::Value, String> {
    let client = reqwest::Client::new();
    let url = format!("https://passport.bilibili.com/x/passport-login/web/qrcode/poll?qrcode_key={}", qr_key);
    let res = client.get(&url)
        .headers(make_headers(""))
        .send()
        .await
        .map_err(|e| e.to_string())?;
        
    // Manually extract cookies to find SESSDATA
    let mut sessdata = String::new();
    for cookie in res.headers().get_all(reqwest::header::SET_COOKIE) {
        if let Ok(c_str) = cookie.to_str() {
            if c_str.starts_with("SESSDATA=") {
                if let Some(val) = c_str.split(';').next() {
                    sessdata = val.replace("SESSDATA=", "");
                }
            }
        }
    }
    
    let text = res.text().await.map_err(|e| e.to_string())?;
    let mut body: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    
    // Inject SESSDATA into response if found
    if !sessdata.is_empty() {
        if let Some(obj) = body.as_object_mut() {
            obj.insert("sessdata_extracted".to_string(), serde_json::Value::String(sessdata));
        }
    }
    
    Ok(body)
}

#[tauri::command]
pub async fn bili_get_video_info(bvid: String, sessdata: String) -> Result<serde_json::Value, String> {
    let client = reqwest::Client::new();
    // In a full implementation we should do Wbi signing, but Bilibili allows basic /view without wbi for now, 
    // or we can just pass the plain url if they changed it recently.
    let url = format!("https://api.bilibili.com/x/web-interface/view?bvid={}", bvid);
    let res = client.get(&url)
        .headers(make_headers(&sessdata))
        .send()
        .await
        .map_err(|e| e.to_string())?;
        
    let text = res.text().await.map_err(|e| e.to_string())?;
    let body: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    Ok(body)
}

#[tauri::command]
pub async fn bili_get_play_info(bvid: String, cid: u64, sessdata: String) -> Result<serde_json::Value, String> {
    let client = reqwest::Client::new();
    let url = format!("https://api.bilibili.com/x/player/playurl?bvid={}&cid={}&fnval=4048&fnver=0&fourk=1", bvid, cid);
    let res = client.get(&url)
        .headers(make_headers(&sessdata))
        .send()
        .await
        .map_err(|e| e.to_string())?;
        
    let text = res.text().await.map_err(|e| e.to_string())?;
    let body: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    Ok(body)
}

#[tauri::command]
pub async fn bili_get_mp4_play_info(bvid: String, cid: u64, sessdata: String) -> Result<serde_json::Value, String> {
    let client = reqwest::Client::new();
    let url = format!("https://api.bilibili.com/x/player/playurl?bvid={}&cid={}&platform=html5&high_quality=1", bvid, cid);
    let res = client.get(&url)
        .headers(make_headers(&sessdata))
        .send()
        .await
        .map_err(|e| e.to_string())?;
        
    let text = res.text().await.map_err(|e| e.to_string())?;
    let body: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    Ok(body)
}

#[derive(Clone, Serialize)]
pub struct BiliTaskProgressPayload {
    pub id: i64,
    pub bvid: String,
    pub status: String,
    pub progress: f64,
    pub detail: String,
}

use tauri::Emitter;
use tokio::sync::Semaphore;
use std::sync::OnceLock;
use std::path::PathBuf;

static DOWNLOAD_SEM: OnceLock<Semaphore> = OnceLock::new();

pub fn get_download_sem() -> &'static Semaphore {
    DOWNLOAD_SEM.get_or_init(|| Semaphore::new(3))
}

#[tauri::command]
pub async fn bili_download_video(
    app: tauri::AppHandle,
    state: tauri::State<'_, crate::db::DbState>,
    bvid: String,
    cid: u64,
    title: String,
    owner: String,
    cover: String,
    sessdata: String,
) -> Result<i64, String> {
    let app_data = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| "C:\\".to_string());
    let download_dir = PathBuf::from(&app_data).join("VrcDog").join("bilidown");
    std::fs::create_dir_all(&download_dir).map_err(|e| e.to_string())?;
    
    // Add task to DB
    let id = crate::db::db_bili_add_task(
        state.clone(), bvid.clone(), cid as i64, 0, title.clone(), owner.clone(), cover.clone(), 
        "waiting".into(), download_dir.to_string_lossy().to_string(), 0, "video".into()
    )?;
    
    let bvid_c = bvid.clone();
    let sessdata_c = sessdata.clone();
    let db_conn = state.conn.clone();
    let app_c = app.clone();
    
    tokio::spawn(async move {
        let update_status = |status: &str| {
            if let Ok(conn) = db_conn.lock() {
                let _ = conn.execute("UPDATE bili_tasks SET status = ?1 WHERE id = ?2", rusqlite::params![status, id]);
            }
        };
        
        update_status("running");
        let _ = app_c.emit("bili_task_progress", BiliTaskProgressPayload {
            id, bvid: bvid_c.clone(), status: "running".into(), progress: 0.0, detail: "获取解析地址...".into()
        });
        
        let play_info = match bili_get_play_info(bvid_c.clone(), cid, sessdata_c).await {
            Ok(info) => info,
            Err(e) => {
                update_status("error");
                let _ = app_c.emit("bili_task_progress", BiliTaskProgressPayload {
                    id, bvid: bvid_c.clone(), status: "error".into(), progress: 0.0, detail: e
                });
                return;
            }
        };
        
        let dash = play_info["data"]["dash"].clone();
        if dash.is_null() {
            update_status("error");
            let _ = app_c.emit("bili_task_progress", BiliTaskProgressPayload {
                id, bvid: bvid_c.clone(), status: "error".into(), progress: 0.0, detail: "获取的视频不是 DASH 格式，暂不支持".into()
            });
            return;
        }
        
        let video_url = dash["video"][0]["baseUrl"].as_str().unwrap_or("").to_string();
        let audio_url = dash["audio"][0]["baseUrl"].as_str().unwrap_or("").to_string();
        
        if video_url.is_empty() || audio_url.is_empty() {
            update_status("error");
            let _ = app_c.emit("bili_task_progress", BiliTaskProgressPayload {
                id, bvid: bvid_c.clone(), status: "error".into(), progress: 0.0, detail: "无法提取视频流地址".into()
            });
            return;
        }
        
        let referer = format!("https://www.bilibili.com/video/{}", bvid_c);
        let video_dest = download_dir.join(format!("{}_video.m4s", id));
        let audio_dest = download_dir.join(format!("{}_audio.m4s", id));
        let safe_title = title.replace(|c: char| !c.is_alphanumeric() && !c.is_whitespace() && c != '-' && c != '_' && c != '【' && c != '】', "_");
        let final_dest = download_dir.join(format!("{} {}.mp4", safe_title, bvid_c));
        
        let _permit = get_download_sem().acquire().await.unwrap();
        
        // Download Video using queue module
        if let Err(e) = queue::download_stream(app_c.clone(), id, bvid_c.clone(), video_url, video_dest.clone(), &referer, "视频流").await {
            update_status("error");
            let _ = app_c.emit("bili_task_progress", BiliTaskProgressPayload { id, bvid: bvid_c.clone(), status: "error".into(), progress: 0.0, detail: format!("视频流下载失败: {}", e) });
            return;
        }
        
        // Download Audio using queue module
        if let Err(e) = queue::download_stream(app_c.clone(), id, bvid_c.clone(), audio_url, audio_dest.clone(), &referer, "音频流").await {
            update_status("error");
            let _ = app_c.emit("bili_task_progress", BiliTaskProgressPayload { id, bvid: bvid_c.clone(), status: "error".into(), progress: 0.0, detail: format!("音频流下载失败: {}", e) });
            return;
        }
        
        // Merge with FFmpeg Sidecar
        let _ = app_c.emit("bili_task_progress", BiliTaskProgressPayload {
            id, bvid: bvid_c.clone(), status: "running".into(), progress: 100.0, detail: "正在调用内置引擎合并音视频...".into()
        });
        
        let merge_result = queue::merge_media(app_c.clone(), video_dest.clone(), audio_dest.clone(), final_dest.clone()).await;
            
        // Cleanup temp files
        let _ = tokio::fs::remove_file(video_dest).await;
        let _ = tokio::fs::remove_file(audio_dest).await;
        
        match merge_result {
            Ok(_) => {
                update_status("done");
                let _ = app_c.emit("bili_task_progress", BiliTaskProgressPayload {
                    id, bvid: bvid_c.clone(), status: "done".into(), progress: 100.0, detail: "下载完成".into()
                });
            }
            Err(e) => {
                update_status("error");
                let _ = app_c.emit("bili_task_progress", BiliTaskProgressPayload {
                    id, bvid: bvid_c.clone(), status: "error".into(), progress: 0.0, detail: e
                });
            }
        }
    });
    
    Ok(id)
}
