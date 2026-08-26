use base64::Engine;
use qrcode::QrCode;
use reqwest::header::{HeaderMap, HeaderValue, COOKIE, REFERER, USER_AGENT};
use serde::Serialize;
use std::io::Cursor;
use std::time::Duration;

pub mod parser;
pub mod queue;

pub use parser::bili_parse_url;

/// Shared User-Agent used by all Bilibili HTTP calls. Centralized so it only
/// needs updating in one place if Bilibili tightens UA filtering.
pub const BILI_USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36";

/// Shared, connection-pooled Bilibili HTTP client with hard connect/read timeouts.
/// Replaces ad-hoc `reqwest::Client::new()` calls that could hang for minutes
/// against an unreachable or filtered endpoint.
pub fn bili_http_client() -> reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT
        .get_or_init(|| {
            reqwest::Client::builder()
                .connect_timeout(Duration::from_secs(5))
                .timeout(Duration::from_secs(15))
                .build()
                .expect("failed to build bilibili http client")
        })
        .clone()
}

pub fn make_headers(sessdata: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        USER_AGENT,
        HeaderValue::from_static(BILI_USER_AGENT),
    );
    headers.insert(
        REFERER,
        HeaderValue::from_static("https://www.bilibili.com"),
    );
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
    let client = bili_http_client();
    let res = client
        .get("https://api.bilibili.com/x/space/myinfo")
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
    let client = bili_http_client();
    let res = client
        .get("https://passport.bilibili.com/x/passport-login/web/qrcode/generate")
        .headers(make_headers(""))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let text = res.text().await.map_err(|e| e.to_string())?;
    let mut body: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    if let Some(url) = body["data"]["url"].as_str() {
        if let Ok(data_url) = build_qr_png_data_url(url) {
            if let Some(data) = body.get_mut("data").and_then(|value| value.as_object_mut()) {
                data.insert(
                    "qr_image_data_url".to_string(),
                    serde_json::Value::String(data_url),
                );
            }
        }
    }
    Ok(body)
}

fn build_qr_png_data_url(text: &str) -> Result<String, String> {
    let code = QrCode::new(text.as_bytes()).map_err(|e| e.to_string())?;
    let image = code
        .render::<image::Luma<u8>>()
        .min_dimensions(240, 240)
        .quiet_zone(true)
        .build();
    let mut png = Vec::new();
    image::DynamicImage::ImageLuma8(image)
        .write_to(&mut Cursor::new(&mut png), image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    Ok(format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(png)
    ))
}

#[tauri::command]
pub async fn bili_get_qr_status(qr_key: String) -> Result<serde_json::Value, String> {
    let client = bili_http_client();
    let url = format!(
        "https://passport.bilibili.com/x/passport-login/web/qrcode/poll?qrcode_key={}",
        qr_key
    );
    let res = client
        .get(&url)
        .headers(make_headers(""))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    // Manually extract cookies returned after QR confirmation.
    let mut sessdata = String::new();
    let mut bili_jct = String::new();
    let mut buvid3 = String::new();
    for cookie in res.headers().get_all(reqwest::header::SET_COOKIE) {
        if let Ok(c_str) = cookie.to_str() {
            if let Some(pair) = c_str.split(';').next() {
                if let Some((name, value)) = pair.split_once('=') {
                    match name.trim().to_ascii_lowercase().as_str() {
                        "sessdata" => sessdata = value.trim().to_string(),
                        "bili_jct" => bili_jct = value.trim().to_string(),
                        "buvid3" => buvid3 = value.trim().to_string(),
                        _ => {}
                    }
                }
            }
        }
    }

    let text = res.text().await.map_err(|e| e.to_string())?;
    let mut body: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;

    if !sessdata.is_empty() || !bili_jct.is_empty() || !buvid3.is_empty() {
        if let Some(obj) = body.as_object_mut() {
            if !sessdata.is_empty() {
                obj.insert(
                    "sessdata_extracted".to_string(),
                    serde_json::Value::String(sessdata),
                );
            }
            if !bili_jct.is_empty() {
                obj.insert(
                    "bili_jct_extracted".to_string(),
                    serde_json::Value::String(bili_jct),
                );
            }
            if !buvid3.is_empty() {
                obj.insert(
                    "buvid3_extracted".to_string(),
                    serde_json::Value::String(buvid3),
                );
            }
        }
    }

    Ok(body)
}

#[tauri::command]
pub async fn bili_get_video_info(
    bvid: String,
    sessdata: String,
) -> Result<serde_json::Value, String> {
    let client = bili_http_client();
    // In a full implementation we should do Wbi signing, but Bilibili allows basic /view without wbi for now,
    // or we can just pass the plain url if they changed it recently.
    let url = format!(
        "https://api.bilibili.com/x/web-interface/view?bvid={}",
        bvid
    );
    let res = client
        .get(&url)
        .headers(make_headers(&sessdata))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let text = res.text().await.map_err(|e| e.to_string())?;
    let body: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    Ok(body)
}

#[tauri::command]
pub async fn bili_get_play_info(
    bvid: String,
    cid: u64,
    sessdata: String,
) -> Result<serde_json::Value, String> {
    let client = bili_http_client();
    let url = format!(
        "https://api.bilibili.com/x/player/playurl?bvid={}&cid={}&fnval=4048&fnver=0&fourk=1",
        bvid, cid
    );
    let res = client
        .get(&url)
        .headers(make_headers(&sessdata))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let text = res.text().await.map_err(|e| e.to_string())?;
    let body: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    Ok(body)
}

#[tauri::command]
pub async fn bili_get_mp4_play_info(
    bvid: String,
    cid: u64,
    sessdata: String,
) -> Result<serde_json::Value, String> {
    let client = bili_http_client();
    let url = format!(
        "https://api.bilibili.com/x/player/playurl?bvid={}&cid={}&platform=html5&high_quality=1",
        bvid, cid
    );
    let res = client
        .get(&url)
        .headers(make_headers(&sessdata))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let text = res.text().await.map_err(|e| e.to_string())?;
    let body: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    Ok(body)
}

fn extract_mp4_url(play_info: &serde_json::Value) -> Option<String> {
    play_info["data"]["durl"]
        .as_array()
        .and_then(|items| items.first())
        .and_then(|item| {
            item["url"].as_str().or_else(|| {
                item["backup_url"]
                    .as_array()
                    .and_then(|urls| urls.first())
                    .and_then(|url| url.as_str())
            })
        })
        .map(str::to_string)
}

#[derive(Clone, Serialize)]
pub struct BiliTaskProgressPayload {
    pub id: i64,
    pub bvid: String,
    pub status: String,
    pub progress: f64,
    pub detail: String,
}

use std::path::PathBuf;
use std::sync::OnceLock;
use tauri::Emitter;
use tokio::sync::Semaphore;

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
    let mut resolved_cid = cid;
    if resolved_cid == 0 {
        let info = bili_get_video_info(bvid.clone(), sessdata.clone()).await?;
        resolved_cid = info["data"]["cid"]
            .as_u64()
            .ok_or_else(|| "unable to resolve cid for Bilibili video".to_string())?;
    }

    let app_data = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| "C:\\".to_string());
    let download_dir = PathBuf::from(&app_data).join("VrcDog").join("bilidown");
    std::fs::create_dir_all(&download_dir).map_err(|e| e.to_string())?;

    // Add task to DB
    let id = crate::db::db_bili_add_task(
        state.clone(),
        bvid.clone(),
        resolved_cid as i64,
        0,
        title.clone(),
        owner.clone(),
        cover.clone(),
        "waiting".into(),
        download_dir.to_string_lossy().to_string(),
        0,
        "video".into(),
    )?;

    let bvid_c = bvid.clone();
    let sessdata_c = sessdata.clone();
    let db_conn = state.conn.clone();
    let app_c = app.clone();

    tokio::spawn(async move {
        let update_status = |status: &str| {
            if let Ok(conn) = db_conn.lock() {
                let _ = conn.execute(
                    "UPDATE bili_tasks SET status = ?1 WHERE id = ?2",
                    rusqlite::params![status, id],
                );
            }
        };

        update_status("running");
        let _ = app_c.emit(
            "bili_task_progress",
            BiliTaskProgressPayload {
                id,
                bvid: bvid_c.clone(),
                status: "running".into(),
                progress: 0.0,
                detail: "获取解析地址...".into(),
            },
        );

        let play_info =
            match bili_get_play_info(bvid_c.clone(), resolved_cid, sessdata_c.clone()).await {
                Ok(info) => info,
                Err(e) => {
                    update_status("error");
                    let _ = app_c.emit(
                        "bili_task_progress",
                        BiliTaskProgressPayload {
                            id,
                            bvid: bvid_c.clone(),
                            status: "error".into(),
                            progress: 0.0,
                            detail: e,
                        },
                    );
                    return;
                }
            };

        let dash = play_info["data"]["dash"].clone();
        if dash.is_null() {
            let mp4_play_info = if extract_mp4_url(&play_info).is_some() {
                play_info.clone()
            } else {
                match bili_get_mp4_play_info(bvid_c.clone(), resolved_cid, sessdata_c.clone()).await
                {
                    Ok(info) => info,
                    Err(e) => {
                        update_status("error");
                        let _ = app_c.emit(
                            "bili_task_progress",
                            BiliTaskProgressPayload {
                                id,
                                bvid: bvid_c.clone(),
                                status: "error".into(),
                                progress: 0.0,
                                detail: format!("获取 MP4 播放地址失败: {}", e),
                            },
                        );
                        return;
                    }
                }
            };

            let Some(mp4_url) = extract_mp4_url(&mp4_play_info) else {
                update_status("error");
                let _ = app_c.emit(
                    "bili_task_progress",
                    BiliTaskProgressPayload {
                        id,
                        bvid: bvid_c.clone(),
                        status: "error".into(),
                        progress: 0.0,
                        detail: "无法提取 MP4 下载地址".into(),
                    },
                );
                return;
            };

            let referer = format!("https://www.bilibili.com/video/{}", bvid_c);
            let safe_title = title.replace(
                |c: char| {
                    !c.is_alphanumeric()
                        && !c.is_whitespace()
                        && c != '-'
                        && c != '_'
                        && c != '【'
                        && c != '】'
                },
                "_",
            );
            let final_dest = download_dir.join(format!("{} {}.mp4", safe_title, bvid_c));
            let _permit = get_download_sem().acquire().await.unwrap();

            if let Err(e) = queue::download_stream(
                app_c.clone(),
                id,
                bvid_c.clone(),
                mp4_url,
                final_dest,
                &referer,
                "MP4",
            )
            .await
            {
                update_status("error");
                let _ = app_c.emit(
                    "bili_task_progress",
                    BiliTaskProgressPayload {
                        id,
                        bvid: bvid_c.clone(),
                        status: "error".into(),
                        progress: 0.0,
                        detail: format!("MP4 下载失败: {}", e),
                    },
                );
                return;
            }

            update_status("done");
            let _ = app_c.emit(
                "bili_task_progress",
                BiliTaskProgressPayload {
                    id,
                    bvid: bvid_c.clone(),
                    status: "done".into(),
                    progress: 100.0,
                    detail: "下载完成".into(),
                },
            );
            return;
        }

        let video_url = dash["video"][0]["baseUrl"]
            .as_str()
            .unwrap_or("")
            .to_string();
        let audio_url = dash["audio"][0]["baseUrl"]
            .as_str()
            .unwrap_or("")
            .to_string();

        if video_url.is_empty() || audio_url.is_empty() {
            update_status("error");
            let _ = app_c.emit(
                "bili_task_progress",
                BiliTaskProgressPayload {
                    id,
                    bvid: bvid_c.clone(),
                    status: "error".into(),
                    progress: 0.0,
                    detail: "无法提取视频流地址".into(),
                },
            );
            return;
        }

        let referer = format!("https://www.bilibili.com/video/{}", bvid_c);
        let video_dest = download_dir.join(format!("{}_video.m4s", id));
        let audio_dest = download_dir.join(format!("{}_audio.m4s", id));
        let safe_title = title.replace(
            |c: char| {
                !c.is_alphanumeric()
                    && !c.is_whitespace()
                    && c != '-'
                    && c != '_'
                    && c != '【'
                    && c != '】'
            },
            "_",
        );
        let final_dest = download_dir.join(format!("{} {}.mp4", safe_title, bvid_c));

        let _permit = get_download_sem().acquire().await.unwrap();

        // Download Video using queue module
        if let Err(e) = queue::download_stream(
            app_c.clone(),
            id,
            bvid_c.clone(),
            video_url,
            video_dest.clone(),
            &referer,
            "视频流",
        )
        .await
        {
            update_status("error");
            let _ = app_c.emit(
                "bili_task_progress",
                BiliTaskProgressPayload {
                    id,
                    bvid: bvid_c.clone(),
                    status: "error".into(),
                    progress: 0.0,
                    detail: format!("视频流下载失败: {}", e),
                },
            );
            return;
        }

        // Download Audio using queue module
        if let Err(e) = queue::download_stream(
            app_c.clone(),
            id,
            bvid_c.clone(),
            audio_url,
            audio_dest.clone(),
            &referer,
            "音频流",
        )
        .await
        {
            update_status("error");
            let _ = app_c.emit(
                "bili_task_progress",
                BiliTaskProgressPayload {
                    id,
                    bvid: bvid_c.clone(),
                    status: "error".into(),
                    progress: 0.0,
                    detail: format!("音频流下载失败: {}", e),
                },
            );
            return;
        }

        // Merge with FFmpeg Sidecar
        let _ = app_c.emit(
            "bili_task_progress",
            BiliTaskProgressPayload {
                id,
                bvid: bvid_c.clone(),
                status: "running".into(),
                progress: 100.0,
                detail: "正在调用内置引擎合并音视频...".into(),
            },
        );

        let merge_result = queue::merge_media(
            app_c.clone(),
            video_dest.clone(),
            audio_dest.clone(),
            final_dest.clone(),
        )
        .await;

        // Cleanup temp files
        let _ = tokio::fs::remove_file(video_dest).await;
        let _ = tokio::fs::remove_file(audio_dest).await;

        match merge_result {
            Ok(_) => {
                update_status("done");
                let _ = app_c.emit(
                    "bili_task_progress",
                    BiliTaskProgressPayload {
                        id,
                        bvid: bvid_c.clone(),
                        status: "done".into(),
                        progress: 100.0,
                        detail: "下载完成".into(),
                    },
                );
            }
            Err(e) => {
                update_status("error");
                let _ = app_c.emit(
                    "bili_task_progress",
                    BiliTaskProgressPayload {
                        id,
                        bvid: bvid_c.clone(),
                        status: "error".into(),
                        progress: 0.0,
                        detail: e,
                    },
                );
            }
        }
    });

    Ok(id)
}
