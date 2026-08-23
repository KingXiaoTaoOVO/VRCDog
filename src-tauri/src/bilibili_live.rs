use reqwest::header::{HeaderMap, HeaderValue, COOKIE, ORIGIN, REFERER, USER_AGENT};
use serde::{Deserialize, Serialize};
use serde_json::Value;

const LIVE_API: &str = "https://api.live.bilibili.com";
const MAIN_API: &str = "https://api.bilibili.com";

#[derive(Clone, Debug, Default, Deserialize)]
pub struct BiliLiveSession {
    pub sessdata: String,
    pub bili_jct: String,
    #[serde(default)]
    pub buvid3: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct LiveRoomInfo {
    pub room_id: u64,
    pub uid: u64,
    pub title: String,
    pub area_id: u64,
    pub area_name: String,
    pub parent_area_id: u64,
    pub parent_area_name: String,
    pub live_status: u64,
    pub online: u64,
    pub cover: String,
    pub announcement: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct LiveArea {
    pub id: u64,
    pub name: String,
    pub parent_id: u64,
    pub parent_name: String,
    pub pic: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct StreamEndpoint {
    pub protocol: String,
    pub address: String,
    pub stream_key: String,
    pub provider: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct LiveStartResult {
    pub live: bool,
    pub requires_face_auth: bool,
    pub face_auth_url: Option<String>,
    pub message: String,
    pub endpoints: Vec<StreamEndpoint>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ContributionRankItem {
    pub uid: u64,
    pub name: String,
    pub face: String,
    pub rank: u64,
    pub score: u64,
}

fn session_headers(session: &BiliLiveSession, room_id: Option<u64>) -> Result<HeaderMap, String> {
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36"));
    headers.insert(
        ORIGIN,
        HeaderValue::from_static("https://live.bilibili.com"),
    );
    let referer = room_id
        .map(|id| format!("https://live.bilibili.com/{id}"))
        .unwrap_or_else(|| "https://live.bilibili.com/".to_string());
    headers.insert(
        REFERER,
        HeaderValue::from_str(&referer).map_err(|e| e.to_string())?,
    );

    let mut cookies = Vec::new();
    if !session.sessdata.trim().is_empty() {
        cookies.push(format!("SESSDATA={}", session.sessdata.trim()));
    }
    if !session.bili_jct.trim().is_empty() {
        cookies.push(format!("bili_jct={}", session.bili_jct.trim()));
    }
    if !session.buvid3.trim().is_empty() {
        cookies.push(format!("buvid3={}", session.buvid3.trim()));
    }
    if !cookies.is_empty() {
        headers.insert(
            COOKIE,
            HeaderValue::from_str(&cookies.join("; ")).map_err(|e| e.to_string())?,
        );
    }
    Ok(headers)
}

fn api_error(body: &Value, action: &str) -> String {
    let code = body["code"].as_i64().unwrap_or(-1);
    let message = body["message"]
        .as_str()
        .or_else(|| body["msg"].as_str())
        .unwrap_or("未知错误");
    format!("{action}失败（{code}）：{message}")
}

fn ensure_success(body: &Value, action: &str) -> Result<(), String> {
    if body["code"].as_i64() == Some(0) {
        Ok(())
    } else {
        Err(api_error(body, action))
    }
}

async fn get_json(
    url: String,
    session: &BiliLiveSession,
    room_id: Option<u64>,
) -> Result<Value, String> {
    crate::bilibili::bili_http_client()
        .get(url)
        .headers(session_headers(session, room_id)?)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<Value>()
        .await
        .map_err(|e| e.to_string())
}

async fn post_form(
    url: &str,
    session: &BiliLiveSession,
    room_id: u64,
    form: Vec<(&str, String)>,
) -> Result<Value, String> {
    crate::bilibili::bili_http_client()
        .post(url)
        .headers(session_headers(session, Some(room_id))?)
        .form(&form)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<Value>()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn bili_live_get_room_info(
    session: BiliLiveSession,
    room_id: u64,
) -> Result<LiveRoomInfo, String> {
    if room_id == 0 {
        return Err("直播间号不能为空".to_string());
    }
    let body = get_json(
        format!("{LIVE_API}/room/v1/Room/get_info?room_id={room_id}"),
        &session,
        Some(room_id),
    )
    .await?;
    ensure_success(&body, "获取直播间信息")?;
    let data = &body["data"];
    let uid = data["uid"].as_u64().unwrap_or(0);
    let announcement = if uid > 0 {
        get_json(
            format!("{LIVE_API}/xlive/app-blink/v1/index/getRoomNews?room_id={room_id}&uid={uid}"),
            &session,
            Some(room_id),
        )
        .await
        .ok()
        .filter(|news| news["code"].as_i64() == Some(0))
        .and_then(|news| news["data"]["content"].as_str().map(str::to_string))
        .unwrap_or_default()
    } else {
        String::new()
    };
    Ok(LiveRoomInfo {
        room_id: data["room_id"].as_u64().unwrap_or(room_id),
        uid,
        title: data["title"].as_str().unwrap_or_default().to_string(),
        area_id: data["area_id"].as_u64().unwrap_or(0),
        area_name: data["area_name"].as_str().unwrap_or_default().to_string(),
        parent_area_id: data["parent_area_id"].as_u64().unwrap_or(0),
        parent_area_name: data["parent_area_name"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        live_status: data["live_status"].as_u64().unwrap_or(0),
        online: data["online"].as_u64().unwrap_or(0),
        cover: data["user_cover"]
            .as_str()
            .or_else(|| data["keyframe"].as_str())
            .unwrap_or_default()
            .to_string(),
        announcement,
    })
}

#[tauri::command]
pub async fn bili_live_get_own_room(session: BiliLiveSession) -> Result<LiveRoomInfo, String> {
    let nav = get_json(format!("{MAIN_API}/x/web-interface/nav"), &session, None).await?;
    ensure_success(&nav, "获取 Bilibili 账号")?;
    let uid = nav["data"]["mid"]
        .as_u64()
        .ok_or_else(|| "账号 UID 缺失".to_string())?;
    let room = get_json(
        format!("{LIVE_API}/room/v2/Room/room_id_by_uid?uid={uid}"),
        &session,
        None,
    )
    .await?;
    ensure_success(&room, "查找账号直播间")?;
    let room_id = room["data"]["room_id"].as_u64().unwrap_or(0);
    if room_id == 0 {
        return Err("当前账号尚未开通 Bilibili 直播间".to_string());
    }
    bili_live_get_room_info(session, room_id).await
}

#[tauri::command]
pub async fn bili_live_get_areas() -> Result<Vec<LiveArea>, String> {
    let body = get_json(
        format!("{LIVE_API}/room/v1/Area/getList?show_pinyin=1"),
        &BiliLiveSession::default(),
        None,
    )
    .await?;
    ensure_success(&body, "获取直播分区")?;
    let mut areas = Vec::new();
    for parent in body["data"].as_array().into_iter().flatten() {
        let parent_id = parent["id"].as_u64().unwrap_or(0);
        let parent_name = parent["name"].as_str().unwrap_or_default().to_string();
        for child in parent["list"].as_array().into_iter().flatten() {
            areas.push(LiveArea {
                id: child["id"].as_u64().unwrap_or(0),
                name: child["name"].as_str().unwrap_or_default().to_string(),
                parent_id,
                parent_name: parent_name.clone(),
                pic: child["pic"].as_str().unwrap_or_default().to_string(),
            });
        }
    }
    Ok(areas)
}

#[tauri::command]
pub async fn bili_live_update_title(
    session: BiliLiveSession,
    room_id: u64,
    title: String,
) -> Result<(), String> {
    let title = title.trim();
    if title.is_empty() {
        return Err("直播标题不能为空".to_string());
    }
    let body = post_form(
        &format!("{LIVE_API}/room/v1/Room/update"),
        &session,
        room_id,
        vec![
            ("room_id", room_id.to_string()),
            ("title", title.to_string()),
            ("platform", "pc_link".to_string()),
            ("csrf", session.bili_jct.clone()),
            ("csrf_token", session.bili_jct.clone()),
        ],
    )
    .await?;
    ensure_success(&body, "更新直播标题")
}

#[tauri::command]
pub async fn bili_live_update_area(
    session: BiliLiveSession,
    room_id: u64,
    area_id: u64,
) -> Result<(), String> {
    let body = post_form(
        &format!("{LIVE_API}/room/v1/Room/update"),
        &session,
        room_id,
        vec![
            ("room_id", room_id.to_string()),
            ("area_id", area_id.to_string()),
            ("platform", "pc_link".to_string()),
            ("csrf", session.bili_jct.clone()),
            ("csrf_token", session.bili_jct.clone()),
        ],
    )
    .await?;
    ensure_success(&body, "更新直播分区")
}

#[tauri::command]
pub async fn bili_live_update_announcement(
    session: BiliLiveSession,
    room_id: u64,
    uid: u64,
    announcement: String,
) -> Result<(), String> {
    if uid == 0 {
        return Err("直播间主播 UID 无效".to_string());
    }
    let body = post_form(
        &format!("{LIVE_API}/xlive/app-blink/v1/index/updateRoomNews"),
        &session,
        room_id,
        vec![
            ("room_id", room_id.to_string()),
            ("uid", uid.to_string()),
            ("content", announcement.trim().to_string()),
            ("csrf", session.bili_jct.clone()),
            ("csrf_token", session.bili_jct.clone()),
        ],
    )
    .await?;
    ensure_success(&body, "更新主播公告")
}

#[tauri::command]
pub async fn bili_live_start(
    session: BiliLiveSession,
    room_id: u64,
    area_id: u64,
) -> Result<LiveStartResult, String> {
    let body = post_form(
        &format!("{LIVE_API}/room/v1/Room/startLive"),
        &session,
        room_id,
        vec![
            ("room_id", room_id.to_string()),
            ("area_v2", area_id.to_string()),
            ("platform", "pc_link".to_string()),
            ("backup_stream", "0".to_string()),
            ("csrf", session.bili_jct.clone()),
            ("csrf_token", session.bili_jct.clone()),
        ],
    )
    .await?;

    let code = body["code"].as_i64().unwrap_or(-1);
    let message = body["message"]
        .as_str()
        .or_else(|| body["msg"].as_str())
        .unwrap_or_default()
        .to_string();
    if code == 60043 {
        return Ok(LiveStartResult {
            live: false,
            requires_face_auth: true,
            face_auth_url: body["data"]["qr"].as_str().map(str::to_string),
            message,
            endpoints: Vec::new(),
        });
    }
    ensure_success(&body, "开始直播")?;

    let mut endpoints = Vec::new();
    let rtmp = &body["data"]["rtmp"];
    if let (Some(address), Some(key)) = (rtmp["addr"].as_str(), rtmp["code"].as_str()) {
        endpoints.push(StreamEndpoint {
            protocol: "RTMP".to_string(),
            address: address.to_string(),
            stream_key: key.to_string(),
            provider: String::new(),
        });
    }
    for item in body["data"]["protocols"].as_array().into_iter().flatten() {
        let address = item["addr"]
            .as_str()
            .or_else(|| item["new_link"].as_str())
            .unwrap_or_default();
        let key = item["code"].as_str().unwrap_or_default();
        if !address.is_empty() && !key.is_empty() {
            endpoints.push(StreamEndpoint {
                protocol: item["protocol"].as_str().unwrap_or("STREAM").to_uppercase(),
                address: address.to_string(),
                stream_key: key.to_string(),
                provider: item["provider"].as_str().unwrap_or_default().to_string(),
            });
        }
    }
    endpoints.dedup_by(|a, b| {
        a.protocol == b.protocol && a.address == b.address && a.stream_key == b.stream_key
    });

    Ok(LiveStartResult {
        live: true,
        requires_face_auth: false,
        face_auth_url: None,
        message,
        endpoints,
    })
}

#[tauri::command]
pub async fn bili_live_stop(session: BiliLiveSession, room_id: u64) -> Result<(), String> {
    let body = post_form(
        &format!("{LIVE_API}/room/v1/Room/stopLive"),
        &session,
        room_id,
        vec![
            ("room_id", room_id.to_string()),
            ("platform", "pc_link".to_string()),
            ("csrf", session.bili_jct.clone()),
            ("csrf_token", session.bili_jct.clone()),
        ],
    )
    .await?;
    ensure_success(&body, "停止直播")
}

#[tauri::command]
pub async fn bili_live_send_danmaku(
    session: BiliLiveSession,
    room_id: u64,
    message: String,
) -> Result<(), String> {
    let message = message.trim();
    if message.is_empty() {
        return Err("弹幕内容不能为空".to_string());
    }
    if message.chars().count() > 30 {
        return Err("弹幕内容不能超过 30 个字符".to_string());
    }
    let rnd = chrono::Utc::now().timestamp().to_string();
    let body = post_form(
        &format!("{LIVE_API}/msg/send"),
        &session,
        room_id,
        vec![
            ("msg", message.to_string()),
            ("color", "16777215".to_string()),
            ("fontsize", "25".to_string()),
            ("mode", "1".to_string()),
            ("rnd", rnd),
            ("roomid", room_id.to_string()),
            ("csrf", session.bili_jct.clone()),
            ("csrf_token", session.bili_jct.clone()),
        ],
    )
    .await?;
    ensure_success(&body, "发送弹幕")
}

#[tauri::command]
pub async fn bili_live_get_contribution_rank(
    session: BiliLiveSession,
    room_id: u64,
) -> Result<Vec<ContributionRankItem>, String> {
    // NOTE: Bilibili removed the legacy `rankdb/v1/RoomRank/webTop` action
    // (returned "方法 webTop 未在控制器 ...RoomRank 中找到"). The current
    // endpoint lives under `xlive/general-interface/v1/rank/getOnlineGoldRank`
    // and additionally requires the broadcaster's uid (`ruid`), not just the room id.
    // 贡献榜依赖主播 uid（ruid）。若 room_init 解析失败或返回 0，
    // 整个命令会硬失败并让面板报错；这里降级为空列表，避免级联错误。
    let ruid = match get_bili_uid(&session, room_id).await {
        Ok(ruid) if ruid != 0 => ruid,
        _ => return Ok(Vec::new()),
    };
    let body = get_json(
        format!(
            "{LIVE_API}/xlive/general-interface/v1/rank/getOnlineGoldRank?roomId={room_id}&ruid={ruid}&page=1&pageSize=20"
        ),
        &session,
        Some(room_id),
    )
    .await?;
    ensure_success(&body, "获取在线贡献榜")?;
    let list = body["data"]["OnlineRankItem"]
        .as_array()
        .or_else(|| body["data"]["list"].as_array())
        .or_else(|| body["data"]["item"].as_array())
        .cloned()
        .unwrap_or_default();
    Ok(list
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let uid = item["uid"].as_u64().unwrap_or(0);
            let name = item["name"]
                .as_str()
                .or_else(|| item["uname"].as_str())
                .or_else(|| item["uinfo"]["base"]["name"].as_str())
                .unwrap_or("观众")
                .to_string();
            let face = item["face"]
                .as_str()
                .or_else(|| item["uinfo"]["base"]["face"].as_str())
                .unwrap_or_default()
                .to_string();
            let rank = item["userRank"]
                .as_u64()
                .or_else(|| item["rank"].as_u64())
                .unwrap_or((index + 1) as u64);
            let score = item["score"].as_u64().unwrap_or(0);
            ContributionRankItem {
                uid,
                name,
                face,
                rank,
                score,
            }
        })
        .collect())
}

/// Resolve the broadcaster uid for a room id. The contribution-rank endpoint
/// requires `ruid` (the streamer's uid) rather than just `roomId`.
async fn get_bili_uid(session: &BiliLiveSession, room_id: u64) -> Result<u64, String> {
    let body = get_json(
        format!("{LIVE_API}/room/v1/Room/room_init?id={room_id}"),
        session,
        Some(room_id),
    )
    .await?;
    if body["code"].as_i64() != Some(0) {
        return Err(body["message"].as_str().unwrap_or("获取主播 UID 失败").to_string());
    }
    Ok(body["data"]["uid"].as_u64().unwrap_or(0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_cookie_header_without_debugging_secrets() {
        let headers = session_headers(
            &BiliLiveSession {
                sessdata: "session".into(),
                bili_jct: "csrf".into(),
                buvid3: "device".into(),
            },
            Some(123),
        )
        .unwrap();
        let cookie = headers.get(COOKIE).unwrap().to_str().unwrap();
        assert!(cookie.contains("SESSDATA=session"));
        assert!(cookie.contains("bili_jct=csrf"));
        assert!(cookie.contains("buvid3=device"));
    }

    #[test]
    fn rejects_platform_errors() {
        let body = serde_json::json!({ "code": -101, "message": "账号未登录" });
        let error = ensure_success(&body, "测试").unwrap_err();
        assert!(error.contains("-101"));
        assert!(error.contains("账号未登录"));
    }
}
