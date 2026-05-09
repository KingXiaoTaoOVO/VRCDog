use serde::{Deserialize, Serialize};
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT, ACCEPT_LANGUAGE, ACCEPT};
use regex::Regex;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct XhsMedia {
    pub url: String,
    pub format: String, // "image" or "video" or "live"
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct XhsItem {
    pub id: String,
    pub title: String,
    pub owner: String,
    pub cover: String,
    pub type_name: String, // "video" or "normal"
    pub media_list: Vec<XhsMedia>,
}

pub fn make_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    // Modern Chrome User-Agent is required to bypass WAF
    headers.insert(USER_AGENT, HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36"));
    headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("zh-CN,zh;q=0.9,en;q=0.8"));
    headers.insert(ACCEPT, HeaderValue::from_static("text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8"));
    headers
}

#[tauri::command]
pub async fn xhs_parse_url(url: String) -> Result<XhsItem, String> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36")
        .build()
        .map_err(|e| e.to_string())?;

    // 1. Fetch the HTML
    let res = client.get(&url)
        .headers(make_headers())
        .send()
        .await
        .map_err(|e| e.to_string())?;
        
    let html = res.text().await.map_err(|e| e.to_string())?;

    // 2. Extract window.__INITIAL_STATE__
    let re = Regex::new(r"window\.__INITIAL_STATE__=(.*?)</script>").unwrap();
    let state_str = if let Some(caps) = re.captures(&html) {
        caps.get(1).map(|m| m.as_str().to_string()).unwrap_or_default()
    } else {
        return Err("解析失败：未能找到 __INITIAL_STATE__。可能是链接失效或需要登录验证。".into());
    };

    // Replace illegal characters (similar to python's YAML_ILLEGAL.sub)
    let clean_re = Regex::new(r"[\x00-\x08\x0b\x0c\x0e-\x1f\x7f]").unwrap();
    let cleaned_state = clean_re.replace_all(&state_str, "");

    // 3. Parse JSON
    let state: serde_json::Value = serde_json::from_str(&cleaned_state).map_err(|e| format!("JSON解析错误: {}", e))?;

    // 4. Navigate object
    // For PC, it's state["note"]["noteDetailMap"][noteId]["note"]
    let mut note = serde_json::Value::Null;
    if let Some(note_obj) = state.get("note").and_then(|n| n.get("noteDetailMap")).and_then(|n| n.as_object()) {
        // Just get the first/only note in the map
        if let Some((_, v)) = note_obj.iter().next() {
            if let Some(inner) = v.get("note") {
                note = inner.clone();
            }
        }
    }
    
    // For Phone, it's state["noteData"]["data"]["noteData"]
    if note.is_null() {
        if let Some(inner) = state.get("noteData").and_then(|n| n.get("data")).and_then(|n| n.get("noteData")) {
            note = inner.clone();
        }
    }

    if note.is_null() {
        return Err("无法找到作品数据，可能是防爬拦截或帖子已被删除。".into());
    }

    let id = note.get("noteId").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let title = note.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let owner = note.get("user").and_then(|u| u.get("nickname")).and_then(|v| v.as_str()).unwrap_or("").to_string();
    let type_name = note.get("type").and_then(|v| v.as_str()).unwrap_or("normal").to_string();
    
    let mut media_list = Vec::new();
    let mut cover = String::new();

    if let Some(image_list) = note.get("imageList").and_then(|v| v.as_array()) {
        for img in image_list {
            // Get high res image
            let url = if let Some(url_default) = img.get("urlDefault").and_then(|v| v.as_str()) {
                url_default.to_string()
            } else if let Some(info_list) = img.get("infoList").and_then(|v| v.as_array()) {
                // Pick the first one which is usually highest res or specific format
                if let Some(first) = info_list.first() {
                    first.get("url").and_then(|v| v.as_str()).unwrap_or("").to_string()
                } else {
                    "".to_string()
                }
            } else {
                "".to_string()
            };

            if cover.is_empty() && !url.is_empty() {
                cover = url.clone();
            }

            if !url.is_empty() {
                media_list.push(XhsMedia {
                    url,
                    format: "image".to_string(),
                });
            }
            
            // Check for Live Photo
            if let Some(live_photo) = img.get("livePhoto").and_then(|v| v.as_bool()) {
                if live_photo {
                    // Try to extract stream
                    // Sometimes live photo video is in infoList or stream or somewhere else. 
                    // Let's just do a basic fallback or we can add it later
                }
            }
        }
    }

    if let Some(video) = note.get("video") {
        if let Some(media) = video.get("media").and_then(|v| v.get("stream")).and_then(|v| v.get("h264")).and_then(|v| v.as_array()) {
            if let Some(first) = media.first() {
                if let Some(master_url) = first.get("masterUrl").and_then(|v| v.as_str()) {
                    media_list.push(XhsMedia {
                        url: master_url.to_string(),
                        format: "video".to_string(),
                    });
                }
            }
        }
    }

    if cover.is_empty() && !media_list.is_empty() {
        cover = media_list[0].url.clone(); // fallback
    }

    Ok(XhsItem {
        id,
        title,
        owner,
        cover,
        type_name,
        media_list,
    })
}
