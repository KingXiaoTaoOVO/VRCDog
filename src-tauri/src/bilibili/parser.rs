use serde::{Deserialize, Serialize};
use reqwest::header::{HeaderMap, HeaderValue, COOKIE, USER_AGENT, REFERER};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ParsedBiliItem {
    pub bvid: String,
    pub cid: u64,
    pub title: String,
    pub owner: String,
    pub cover: String,
    pub duration: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ParserResult {
    pub collection_name: String,
    pub items: Vec<ParsedBiliItem>,
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

// 单个 BVID 解析
pub async fn parse_bvid(bvid: &str, sessdata: &str) -> Result<ParserResult, String> {
    let client = reqwest::Client::new();
    let url = format!("https://api.bilibili.com/x/web-interface/view?bvid={}", bvid);
    let res = client.get(&url).headers(make_headers(sessdata)).send().await.map_err(|e| e.to_string())?;
    let body: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
    
    if body["code"].as_i64() != Some(0) {
        return Err(body["message"].as_str().unwrap_or("Unknown error").to_string());
    }
    
    let data = &body["data"];
    let title = data["title"].as_str().unwrap_or("").to_string();
    let owner = data["owner"]["name"].as_str().unwrap_or("").to_string();
    let cover = data["pic"].as_str().unwrap_or("").to_string();
    let cid = data["cid"].as_u64().unwrap_or(0);
    let duration = data["duration"].as_u64().unwrap_or(0);
    
    // Support multi-part videos (P1, P2...)
    let mut items = Vec::new();
    if let Some(pages) = data["pages"].as_array() {
        for page in pages {
            let part_title = page["part"].as_str().unwrap_or("").to_string();
            let page_cid = page["cid"].as_u64().unwrap_or(0);
            let page_duration = page["duration"].as_u64().unwrap_or(0);
            
            let full_title = if pages.len() > 1 {
                format!("{} - {}", title, part_title)
            } else {
                title.clone()
            };
            
            items.push(ParsedBiliItem {
                bvid: bvid.to_string(),
                cid: page_cid,
                title: full_title,
                owner: owner.clone(),
                cover: cover.clone(),
                duration: page_duration,
            });
        }
    } else {
        items.push(ParsedBiliItem {
            bvid: bvid.to_string(),
            cid, title: title.clone(), owner, cover, duration
        });
    }
    
    Ok(ParserResult {
        collection_name: title,
        items,
    })
}

// 番剧解析 (Bangumi)
pub async fn parse_bangumi(ep_id: Option<&str>, season_id: Option<&str>, sessdata: &str) -> Result<ParserResult, String> {
    let client = reqwest::Client::new();
    let mut url = String::from("https://api.bilibili.com/pgc/view/web/season?");
    if let Some(ep) = ep_id {
        url.push_str(&format!("ep_id={}", ep));
    } else if let Some(ss) = season_id {
        url.push_str(&format!("season_id={}", ss));
    } else {
        return Err("必须提供 ep_id 或 season_id".to_string());
    }

    let res = client.get(&url).headers(make_headers(sessdata)).send().await.map_err(|e| e.to_string())?;
    let body: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
    
    if body["code"].as_i64() != Some(0) {
        return Err(body["message"].as_str().unwrap_or("Unknown error").to_string());
    }
    
    let result = &body["result"];
    let series_title = result["title"].as_str().unwrap_or("").to_string();
    let cover = result["cover"].as_str().unwrap_or("").to_string();
    let owner = result["up_info"]["uname"].as_str().unwrap_or("Bilibili 番剧").to_string();
    
    let mut items = Vec::new();
    if let Some(episodes) = result["episodes"].as_array() {
        for ep in episodes {
            let ep_title = ep["long_title"].as_str().unwrap_or(ep["title"].as_str().unwrap_or(""));
            let full_title = format!("{} - {}", series_title, ep_title);
            let bvid = ep["bvid"].as_str().unwrap_or("").to_string();
            let cid = ep["cid"].as_u64().unwrap_or(0);
            
            items.push(ParsedBiliItem {
                bvid,
                cid,
                title: full_title,
                owner: owner.clone(),
                cover: cover.clone(),
                duration: 0, // Bangumi list usually lacks direct duration in this API without an extra call
            });
        }
    }
    
    Ok(ParserResult {
        collection_name: series_title,
        items,
    })
}

// 收藏夹解析 (Favlist)
pub async fn parse_favlist(media_id: &str, sessdata: &str) -> Result<ParserResult, String> {
    let client = reqwest::Client::new();
    let mut items = Vec::new();
    let mut page = 1;
    let mut collection_title = String::from("收藏夹");

    loop {
        let url = format!(
            "https://api.bilibili.com/x/v3/fav/resource/list?media_id={}&pn={}&ps=20&order=mtime&type=0&tid=0&platform=web",
            media_id, page
        );
        let res = client.get(&url).headers(make_headers(sessdata)).send().await.map_err(|e| e.to_string())?;
        let body: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;

        if body["code"].as_i64() != Some(0) {
            return Err(body["message"].as_str().unwrap_or("Unknown error").to_string());
        }

        let data = &body["data"];
        if page == 1 {
            collection_title = data["info"]["title"].as_str().unwrap_or("收藏夹").to_string();
        }

        if let Some(medias) = data["medias"].as_array() {
            for m in medias {
                // If the video is not valid (e.g. deleted), skip it
                if m["title"].as_str().unwrap_or("") == "已失效视频" {
                    continue;
                }
                let bvid = m["bvid"].as_str().unwrap_or("").to_string();
                let title = m["title"].as_str().unwrap_or("").to_string();
                let cover = m["cover"].as_str().unwrap_or("").to_string();
                let owner = m["upper"]["name"].as_str().unwrap_or("").to_string();
                let duration = m["duration"].as_u64().unwrap_or(0);
                
                // For favlist, we get the default part (cid might require an extra call, but let's default to 0 for lazy fetch)
                // However, Bilibili API often includes cid in some endpoints, but not favlist natively.
                // Our downloader can fetch play info using just BVID if we lazily resolve cid later, but we can also just put 0.
                items.push(ParsedBiliItem {
                    bvid,
                    cid: 0, 
                    title,
                    owner,
                    cover,
                    duration,
                });
            }
        }

        let has_more = data["has_more"].as_bool().unwrap_or(false);
        if !has_more {
            break;
        }
        page += 1;
    }

    Ok(ParserResult {
        collection_name: collection_title,
        items,
    })
}

// UP主视频合集解析 (Collection / Seasons Archives)
pub async fn parse_collection(mid: &str, season_id: &str, sessdata: &str) -> Result<ParserResult, String> {
    let client = reqwest::Client::new();
    let mut items = Vec::new();
    let mut page = 1;
    let mut collection_title = String::from("视频合集");

    loop {
        let url = format!(
            "https://api.bilibili.com/x/polymer/web-space/seasons_archives_list?mid={}&season_id={}&page_num={}&page_size=30",
            mid, season_id, page
        );
        let res = client.get(&url).headers(make_headers(sessdata)).send().await.map_err(|e| e.to_string())?;
        let body: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;

        if body["code"].as_i64() != Some(0) {
            return Err(body["message"].as_str().unwrap_or("Unknown error").to_string());
        }

        let data = &body["data"];
        
        if let Some(meta) = data["meta"].as_object() {
            collection_title = meta.get("name").and_then(|n| n.as_str()).unwrap_or("视频合集").to_string();
        }

        if let Some(archives) = data["archives"].as_array() {
            if archives.is_empty() {
                break;
            }
            for arc in archives {
                let bvid = arc["bvid"].as_str().unwrap_or("").to_string();
                let title = arc["title"].as_str().unwrap_or("").to_string();
                let cover = arc["pic"].as_str().unwrap_or("").to_string();
                let owner = arc["owner"]["name"].as_str().unwrap_or("").to_string();
                let duration = arc["duration"].as_u64().unwrap_or(0);
                
                items.push(ParsedBiliItem {
                    bvid,
                    cid: 0,
                    title,
                    owner,
                    cover,
                    duration,
                });
            }
        } else {
            break;
        }
        
        let page_info = &data["page"];
        let total = page_info["total"].as_u64().unwrap_or(0);
        let page_size = page_info["page_size"].as_u64().unwrap_or(30);
        let current_page = page_info["page_num"].as_u64().unwrap_or(page);
        
        if current_page * page_size >= total {
            break;
        }
        page += 1;
    }

    Ok(ParserResult {
        collection_name: collection_title,
        items,
    })
}

// 统一路由入口
#[tauri::command]
pub async fn bili_parse_url(url: String, sessdata: String) -> Result<ParserResult, String> {
    // Basic regex matching
    if url.contains("/video/BV") {
        let bvid = url.split("/video/").nth(1).unwrap_or("").split(&['/', '?'][..]).next().unwrap_or("");
        if !bvid.is_empty() {
            return parse_bvid(bvid, &sessdata).await;
        }
    } else if url.contains("/bangumi/play/ep") {
        let ep = url.split("/ep").nth(1).unwrap_or("").split(&['/', '?'][..]).next().unwrap_or("");
        if !ep.is_empty() {
            return parse_bangumi(Some(ep), None, &sessdata).await;
        }
    } else if url.contains("/bangumi/play/ss") {
        let ss = url.split("/ss").nth(1).unwrap_or("").split(&['/', '?'][..]).next().unwrap_or("");
        if !ss.is_empty() {
            return parse_bangumi(None, Some(ss), &sessdata).await;
        }
    } else if url.contains("favlist?fid=") || url.contains("ml") {
        // Example: https://space.bilibili.com/1176277996/favlist?fid=1234122612
        let fid = if url.contains("fid=") {
            url.split("fid=").nth(1).unwrap_or("").split('&').next().unwrap_or("")
        } else if url.contains("ml") {
            // ml1234122612
            let ml_part = url.split("ml").nth(1).unwrap_or("").split(&['/', '?'][..]).next().unwrap_or("");
            ml_part
        } else { "" };
        if !fid.is_empty() {
            return parse_favlist(fid, &sessdata).await;
        }
    } else if url.contains("collectiondetail?sid=") {
        // Example: https://space.bilibili.com/282565107/channel/collectiondetail?sid=1427135
        let sid = url.split("sid=").nth(1).unwrap_or("").split('&').next().unwrap_or("");
        let mid = url.split("space.bilibili.com/").nth(1).unwrap_or("").split('/').next().unwrap_or("");
        if !sid.is_empty() && !mid.is_empty() {
            return parse_collection(mid, sid, &sessdata).await;
        }
    }
    
    Err("不支持的链接格式或暂未实现".to_string())
}
