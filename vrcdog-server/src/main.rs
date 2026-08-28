use axum::{
    body::{Body, Bytes},
    extract::{ws::WebSocketUpgrade, ConnectInfo, Path, Query, State},
    http::{header::{CONTENT_TYPE, HeaderMap, HeaderValue}, StatusCode},
    middleware,
    response::{Html, IntoResponse, Response},
    routing::{get, on, post, MethodFilter},
    Json, Router,
};
use chrono::{Duration, Local, NaiveDateTime};
use hex;
use rand;
use reqwest::{Client as ReqwestClient, ClientBuilder, Method};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    env,
    net::SocketAddr,
    path::{Path as FsPath, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
    time::Duration as StdDuration,
};
use tokio::sync::{Notify, RwLock};
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::{info, warn};

mod remote_assist_hub;
mod survey;

use survey::{Survey, SurveyAnswerFile, SurveyClickEvent, SurveySettings, SurveySubmission};

#[derive(Clone, Serialize, Deserialize, Debug)]
struct ClientInfo {
    user_id: String,
    display_name: String,
    avatar_url: String,
    ip_address: String,
    connected_at: String,
    last_heartbeat: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct UserRecord {
    user_id: String,
    display_name: String,
    avatar_url: String,
    first_seen: String,
    last_seen: String,
    login_count: u32,
    is_online: bool,
    role_id: Option<String>,
    #[serde(default)]
    role_expires_at: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct BanInfo {
    user_id: String,
    reason: String,
    banned_at: String,
    duration_hours: Option<f64>,
    expires_at: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct FreezeInfo {
    user_id: String,
    reason: String,
    frozen_at: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct FeatureConfig {
    #[serde(default)]
    menus: HashMap<String, bool>,
    #[serde(default)]
    themes: HashMap<String, bool>,
    #[serde(default)]
    modes: HashMap<String, bool>,
}

impl Default for FeatureConfig {
    fn default() -> Self {
        let menus = [
            "dashboard",
            "feed",
            "locations",
            "charts",
            "playerlist",
            "gallery",
            "social",
            "friendslist",
            "moderation",
            "search",
            "notifications",
            "groups",
            "avatars",
            "favorites",
            "heatmap",
            "notes",
            "presets",
            "tools",
            "vrpiano",
            "drawing",
            "bilidown",
            "danmaku",
            "translator",
            "ovr",
            "remote",
            "env",
            "export",
            "settings",
        ]
        .into_iter()
        .map(|key| (key.to_string(), true))
        .collect();
        let themes = ["dog", "cat", "helmet", "mono"]
            .into_iter()
            .map(|key| (key.to_string(), true))
            .collect();
        let modes = ["pc", "vr"]
            .into_iter()
            .map(|key| (key.to_string(), true))
            .collect();
        Self {
            menus,
            themes,
            modes,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct Role {
    role_id: String,
    role_name: String,
    is_default: bool,
    features: FeatureConfig,
}

#[derive(Clone, Serialize, Deserialize, Default)]
struct PersistentData {
    #[serde(default)]
    users: HashMap<String, UserRecord>,
    #[serde(default)]
    bans: HashMap<String, BanInfo>,
    #[serde(default)]
    frozen: HashMap<String, FreezeInfo>,
    #[serde(default)]
    kicked: HashMap<String, String>,
    #[serde(default)]
    roles: HashMap<String, Role>,
    #[serde(default)]
    survey_settings: SurveySettings,
    #[serde(default)]
    surveys: HashMap<String, Survey>,
    #[serde(default)]
    survey_submissions: HashMap<String, SurveySubmission>,
    /// Raw per-option click events reported by clients while filling surveys.
    #[serde(default)]
    survey_click_events: Vec<SurveyClickEvent>,
}

impl PersistentData {
    fn ensure_defaults(&mut self) {
        if self.roles.is_empty() {
            self.roles.insert(
                "default".to_string(),
                Role {
                    role_id: "default".to_string(),
                    role_name: "Default User".to_string(),
                    is_default: true,
                    features: FeatureConfig::default(),
                },
            );
        }
        for user in self.users.values_mut() {
            user.is_online = false;
        }
    }
}

#[derive(Clone)]
struct AppState {
    clients: Arc<RwLock<HashMap<String, ClientInfo>>>,
    data: Arc<RwLock<PersistentData>>,
    data_file: Arc<PathBuf>,
    remote_assist: remote_assist_hub::RemoteAssistHub,
    persist_dirty: Arc<AtomicBool>,
    persist_notify: Arc<Notify>,
    admin_sessions: Arc<RwLock<HashMap<String, AdminSession>>>,
    http_client: Arc<ReqwestClient>,
}

#[derive(Deserialize)]
struct RegisterRequest {
    user_id: String,
    display_name: String,
    #[serde(default)]
    avatar_url: String,
}

#[derive(Deserialize)]
struct UserIdRequest {
    user_id: String,
}

#[derive(Deserialize)]
struct BanRequest {
    user_id: String,
    reason: String,
    duration_hours: Option<f64>,
}

#[derive(Deserialize)]
struct FreezeRequest {
    user_id: String,
    reason: String,
}

#[derive(Deserialize)]
struct RoleIdRequest {
    role_id: String,
}

#[derive(Deserialize)]
struct SetUserRoleRequest {
    user_id: String,
    role_id: Option<String>,
}

#[derive(Deserialize)]
struct SurveyIdRequest {
    survey_id: String,
}

#[derive(Deserialize)]
struct SubmitSurveyRequest {
    user_id: String,
    survey_id: String,
    survey_revision: u32,
    #[serde(default)]
    answers: HashMap<String, Value>,
    /// Per-question file attachments uploaded by the respondent.
    #[serde(default)]
    answer_files: Option<HashMap<String, Vec<SurveyAnswerFile>>>,
}

#[derive(Deserialize)]
struct DismissSurveyRequest {
    user_id: String,
    survey_id: String,
    survey_revision: u32,
}

/// A single click/interaction reported by the client while the respondent is
/// filling a survey. Labels sent by the client are only used as a fallback when
/// the server-side survey snapshot cannot resolve them.
#[derive(Deserialize)]
struct ClickSurveyRequest {
    user_id: String,
    survey_id: String,
    #[serde(default)]
    survey_revision: u32,
    question_id: String,
    #[serde(default)]
    option_id: String,
    #[serde(default)]
    option_label: String,
    #[serde(default)]
    question_title: String,
    /// "select" | "deselect" | "input"
    #[serde(default)]
    action: String,
    #[serde(default)]
    text_value: String,
}

/// Upper bound for stored raw click events; oldest events are dropped first so
/// the persistence file cannot grow without limit.
const MAX_SURVEY_CLICK_EVENTS: usize = 20_000;

#[derive(Deserialize)]
struct DeleteSubmissionRequest {
    user_id: String,
    submission_id: String,
}

#[derive(Deserialize)]
struct AdminAuthRequest {
    password: String,
}

#[derive(Clone)]
struct AdminSession {
    token: String,
    created_at: String,
}

const DEFAULT_SERVER_PASSWORD_BCRYPT: &str =
    "$2b$12$go9qphFk80mBGkPx9AiayObfu.gfsSvKCAL0sBMnTBYreWAGYDBiK";

fn server_password_hash() -> String {
    env::var("VRCDOG_SERVER_PASSWORD_BCRYPT")
        .ok()
        .map(|hash| hash.trim().to_string())
        .filter(|hash| !hash.is_empty())
        .unwrap_or_else(|| DEFAULT_SERVER_PASSWORD_BCRYPT.to_string())
}

fn verify_server_password(password: &str) -> bool {
    bcrypt::verify(password, &server_password_hash()).unwrap_or(false)
}

fn now_string() -> String {
    Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

fn survey_gate_payload(data: &PersistentData, user_id: &str) -> Value {
    let pending = survey::pending_surveys(
        data.survey_settings.enabled,
        &data.surveys,
        &data.survey_submissions,
        user_id,
    );
    let required = pending.iter().any(|survey| survey.required_for_access);
    json!({
        "status": if required { "survey_required" } else if pending.is_empty() { "ok" } else { "survey_available" },
        "pending_survey_count": pending.len(),
        "survey_required": required
    })
}

fn ban_is_expired(ban: &BanInfo) -> bool {
    let Some(hours) = ban.duration_hours else {
        return false;
    };
    let Ok(banned_at) = NaiveDateTime::parse_from_str(&ban.banned_at, "%Y-%m-%d %H:%M:%S") else {
        return false;
    };
    Local::now().naive_local() > banned_at + Duration::seconds((hours * 3600.0) as i64)
}

/// True when the user's granted role has an expiry that is already in the past.
fn role_expired(user: &UserRecord) -> bool {
    let Some(expires_at) = &user.role_expires_at else {
        return false;
    };
    let Ok(parsed) = NaiveDateTime::parse_from_str(expires_at, "%Y-%m-%d %H:%M:%S") else {
        return false;
    };
    Local::now().naive_local() > parsed
}

/// Revoke a user's granted role once it has expired. Uses a read check first so
/// healthy heartbeats (the common case) never take a write lock.
async fn revoke_expired_user_role(state: &AppState, user_id: &str) {
    let expired = {
        let data = state.data.read().await;
        data.users.get(user_id).is_some_and(role_expired)
    };
    if expired {
        let mut data = state.data.write().await;
        if let Some(user) = data.users.get_mut(user_id) {
            if role_expired(user) {
                user.role_id = None;
                user.role_expires_at = None;
            }
        }
        drop(data);
        schedule_persist(state);
    }
}

// Mutations mark state dirty instead of blocking on a full serialize+write.
// The background writer coalesces bursts into a single disk write within this
// window, which slashes IOPS and the transient 2x memory peak from cloning.
const PERSIST_DEBOUNCE_MS: u64 = 800;

async fn persist_inner(state: &AppState) -> Result<(), String> {
    // Serialize in place under a read lock (no full clone of PersistentData),
    // then drop the lock before touching the disk.
    let bytes = {
        let data = state.data.read().await;
        serde_json::to_vec_pretty(&*data).map_err(|error| error.to_string())?
    };
    if let Some(parent) = state.data_file.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|error| error.to_string())?;
    }
    let temp_file = state.data_file.with_extension("json.tmp");
    tokio::fs::write(&temp_file, &bytes)
        .await
        .map_err(|error| error.to_string())?;
    if tokio::fs::try_exists(state.data_file.as_ref())
        .await
        .unwrap_or(false)
    {
        let _ = tokio::fs::remove_file(state.data_file.as_ref()).await;
    }
    tokio::fs::rename(temp_file, state.data_file.as_ref())
        .await
        .map_err(|error| error.to_string())
}

fn schedule_persist(state: &AppState) {
    state.persist_dirty.store(true, Ordering::SeqCst);
    state.persist_notify.notify_one();
}

async fn persist_worker(state: AppState) {
    loop {
        state.persist_notify.notified().await;
        // Quiet window: any mutations within it coalesce into one write.
        tokio::time::sleep(StdDuration::from_millis(PERSIST_DEBOUNCE_MS)).await;
        if state.persist_dirty.swap(false, Ordering::SeqCst) {
            if let Err(error) = persist_inner(&state).await {
                warn!("persist failed: {error}");
                state.persist_dirty.store(true, Ordering::SeqCst);
            }
        }
    }
}

async fn load_data(path: &FsPath) -> PersistentData {
    let mut data = match tokio::fs::read(path).await {
        Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_default(),
        Err(_) => PersistentData::default(),
    };
    data.ensure_defaults();
    data
}

async fn ping() -> Json<Value> {
    Json(json!({
        "status": "ok",
        "message": "Pong from VRCDog Standalone Server"
    }))
}

// Self-contained (no external assets) browser status page shown at the server
// root. Rendered dark/glassmorphism so it looks good behind any reverse proxy.
const STATUS_PAGE_HTML: &str = r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="utf-8" />
<meta name="viewport" content="width=device-width, initial-scale=1" />
<title>VRCDog 服务端</title>
<style>
  :root { color-scheme: dark; }
  * { box-sizing: border-box; margin: 0; padding: 0; }
  body {
    min-height: 100vh;
    display: flex; align-items: center; justify-content: center;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", "PingFang SC", "Microsoft YaHei", sans-serif;
    background: radial-gradient(1200px 600px at 50% -10%, #1b2a4a 0%, #0b1020 55%, #070a14 100%);
    color: #e8ecf4; padding: 24px;
  }
  .card {
    width: min(520px, 92vw);
    background: linear-gradient(180deg, rgba(255,255,255,0.06), rgba(255,255,255,0.02));
    border: 1px solid rgba(255,255,255,0.10); border-radius: 22px; padding: 40px 36px;
    text-align: center; backdrop-filter: blur(8px); box-shadow: 0 20px 60px rgba(0,0,0,0.45);
  }
  .badge {
    width: 84px; height: 84px; margin: 0 auto 22px; border-radius: 50%;
    display: grid; place-items: center;
    background: linear-gradient(135deg, #2bd47f, #18a0fb);
    animation: pulse 2.4s ease-in-out infinite;
  }
  .badge svg { width: 42px; height: 42px; stroke: #06121f; }
  @keyframes pulse {
    0%,100% { box-shadow: 0 0 0 8px rgba(43,212,127,0.12), 0 10px 30px rgba(24,160,251,0.35); }
    50% { box-shadow: 0 0 0 14px rgba(43,212,127,0.06), 0 10px 36px rgba(24,160,251,0.45); }
  }
  .title {
    font-size: 26px; font-weight: 800; letter-spacing: .5px;
    background: linear-gradient(90deg, #6fe3ff, #18a0fb);
    -webkit-background-clip: text; background-clip: text; color: transparent;
  }
  .status { margin-top: 10px; font-size: 17px; font-weight: 700; color: #d6e2ff; }
  .hint { margin-top: 6px; font-size: 14px; color: #9fb0cc; }
  .meta { margin-top: 26px; display: flex; gap: 10px; justify-content: center; flex-wrap: wrap; }
  .chip {
    font-size: 12px; color: #aab8d4; background: rgba(255,255,255,0.06);
    border: 1px solid rgba(255,255,255,0.08); padding: 6px 12px; border-radius: 999px;
  }
  .chip b { color: #e8ecf4; font-weight: 700; }
  .foot { margin-top: 22px; font-size: 11px; color: #6b7a99; }
</style>
</head>
<body>
  <div class="card">
    <div class="badge">
      <svg viewBox="0 0 24 24" fill="none" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><path d="M5 13l4 4L19 7"/></svg>
    </div>
    <div class="title">VRCDog 服务端</div>
    <div class="status">服务已成功运行</div>
    <div class="hint">请用客户端连接！</div>
    <div class="meta">
      <span class="chip">版本 <b>{{VERSION}}</b></span>
      <span class="chip">端口 <b>11451</b></span>
      <span class="chip">启动于 <b>{{NOW}}</b></span>
    </div>
    <div class="foot">VRCDog Standalone Server · 此页面仅用于部署状态确认</div>
  </div>
</body>
</html>"#;

async fn status_page() -> Html<String> {
    let html = STATUS_PAGE_HTML
        .replace("{{VERSION}}", env!("CARGO_PKG_VERSION"))
        .replace("{{NOW}}", &now_string());
    Html(html)
}

#[derive(Serialize)]
struct VersionInfo {
    version: &'static str,
    name: &'static str,
    published_at: String,
    body: &'static str,
    html_url: &'static str,
}

async fn api_version() -> Json<VersionInfo> {
    Json(VersionInfo {
        version: env!("CARGO_PKG_VERSION"),
        name: "VRCDog Server",
        published_at: now_string(),
        body: "VRCDog standalone server",
        html_url: "https://github.com/KingXiaoTaoOVO/vrcdog-releases",
    })
}

async fn remote_assist_ws(
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
) -> impl axum::response::IntoResponse {
    state.remote_assist.upgrade(ws).await
}

async fn register(
    State(state): State<AppState>,
    ConnectInfo(address): ConnectInfo<SocketAddr>,
    Json(request): Json<RegisterRequest>,
) -> Json<Value> {
    let now = now_string();
    let was_kicked = state.data.read().await.kicked.contains_key(&request.user_id);
    if was_kicked {
        let mut data = state.data.write().await;
        data.kicked.remove(&request.user_id);
        drop(data);
        schedule_persist(&state);
        return Json(json!({ "status": "kicked", "reason": "Removed by administrator" }));
    }
    {
        let mut data = state.data.write().await;
        if let Some(ban) = data.bans.get(&request.user_id) {
            if !ban_is_expired(ban) {
                return Json(json!({
                    "status": "banned",
                    "reason": ban.reason,
                    "duration_hours": ban.duration_hours,
                    "expires_at": ban.expires_at
                }));
            }
        }
        data.bans.retain(|_, ban| !ban_is_expired(ban));
        if let Some(freeze) = data.frozen.get(&request.user_id) {
            return Json(json!({ "status": "frozen", "reason": freeze.reason }));
        }
        data.users
            .entry(request.user_id.clone())
            .and_modify(|user| {
                user.display_name = request.display_name.clone();
                user.avatar_url = request.avatar_url.clone();
                user.last_seen = now.clone();
                user.login_count += 1;
                user.is_online = true;
            })
            .or_insert_with(|| UserRecord {
                user_id: request.user_id.clone(),
                display_name: request.display_name.clone(),
                avatar_url: request.avatar_url.clone(),
                first_seen: now.clone(),
                last_seen: now.clone(),
                login_count: 1,
                is_online: true,
                role_id: None,
                role_expires_at: None,
            });
        // Drop an expired incentive role on (re)connection so the default role applies.
        if let Some(user) = data.users.get_mut(&request.user_id) {
            if role_expired(user) {
                user.role_id = None;
                user.role_expires_at = None;
            }
        }
    }
    state.clients.write().await.insert(
        request.user_id.clone(),
        ClientInfo {
            user_id: request.user_id.clone(),
            display_name: request.display_name.clone(),
            avatar_url: request.avatar_url.clone(),
            ip_address: address.to_string(),
            connected_at: now.clone(),
            last_heartbeat: now,
        },
    );
    schedule_persist(&state);
    info!(user_id = %request.user_id, ip = %address, "client registered");
    let data = state.data.read().await;
    let mut response = survey_gate_payload(&data, &request.user_id);
    response["message"] = json!("registered");
    Json(response)
}

async fn heartbeat(
    State(state): State<AppState>,
    Json(request): Json<UserIdRequest>,
) -> Json<Value> {
    let was_kicked = state.data.read().await.kicked.contains_key(&request.user_id);
    if was_kicked {
        let mut data = state.data.write().await;
        data.kicked.remove(&request.user_id);
        drop(data);
        schedule_persist(&state);
        return Json(json!({ "status": "kicked", "reason": "Removed by administrator" }));
    }
    {
        let data = state.data.read().await;
        if let Some(ban) = data.bans.get(&request.user_id) {
            if !ban_is_expired(ban) {
                return Json(json!({
                    "status": "banned",
                    "reason": ban.reason,
                    "duration_hours": ban.duration_hours,
                    "expires_at": ban.expires_at
                }));
            }
        }
        if let Some(freeze) = data.frozen.get(&request.user_id) {
            return Json(json!({ "status": "frozen", "reason": freeze.reason }));
        }
    }
    revoke_expired_user_role(&state, &request.user_id).await;
    let mut clients = state.clients.write().await;
    let Some(client) = clients.get_mut(&request.user_id) else {
        return Json(json!({ "status": "register_required" }));
    };
    client.last_heartbeat = now_string();
    drop(clients);
    let data = state.data.read().await;
    Json(survey_gate_payload(&data, &request.user_id))
}

async fn disconnect(
    State(state): State<AppState>,
    Json(request): Json<UserIdRequest>,
) -> Json<Value> {
    state.clients.write().await.remove(&request.user_id);
    if let Some(user) = state.data.write().await.users.get_mut(&request.user_id) {
        user.is_online = false;
        user.last_seen = now_string();
    }
    schedule_persist(&state);
    Json(json!({ "status": "ok" }))
}

async fn check_status(State(state): State<AppState>, Path(user_id): Path<String>) -> Json<Value> {
    let data = state.data.read().await;
    if let Some(ban) = data.bans.get(&user_id) {
        if !ban_is_expired(ban) {
            return Json(json!({
                "status": "banned",
                "reason": ban.reason,
                "duration_hours": ban.duration_hours,
                "expires_at": ban.expires_at
            }));
        }
    }
    if let Some(freeze) = data.frozen.get(&user_id) {
        return Json(json!({ "status": "frozen", "reason": freeze.reason }));
    }
    Json(json!({ "status": "ok", "reason": null }))
}

async fn get_features(State(state): State<AppState>, Path(user_id): Path<String>) -> Json<Value> {
    let data = state.data.read().await;
    let default_role = data.roles.values().find(|role| role.is_default);
    let user_role = data
        .users
        .get(&user_id)
        .and_then(|user| {
            if role_expired(user) {
                None
            } else {
                user.role_id.as_ref()
            }
        })
        .and_then(|role_id| data.roles.get(role_id));
    let features = user_role
        .or(default_role)
        .map(|role| role.features.clone())
        .unwrap_or_default();
    let (role_id, role_expires_at, role_expired_flag) = data
        .users
        .get(&user_id)
        .map(|user| {
            (
                user.role_id.clone(),
                user.role_expires_at.clone(),
                role_expired(user),
            )
        })
        .unwrap_or((None, None, true));
    Json(json!({
        "menus": features.menus,
        "themes": features.themes,
        "modes": features.modes,
        "role_id": role_id,
        "role_expires_at": role_expires_at,
        "role_expired": role_expired_flag
    }))
}

async fn client_surveys(State(state): State<AppState>, Path(user_id): Path<String>) -> Json<Value> {
    let data = state.data.read().await;
    let surveys = survey::pending_surveys(
        data.survey_settings.enabled,
        &data.surveys,
        &data.survey_submissions,
        &user_id,
    );
    Json(json!({
        "enabled": data.survey_settings.enabled,
        "surveys": surveys
    }))
}

const MAX_UPLOAD_BYTES: u64 = 10 * 1024 * 1024; // 10 MiB

static UPLOAD_SEQ: AtomicU64 = AtomicU64::new(1);

/// Directory that stores respondent-uploaded files; derived from the state file location.
fn uploads_dir(state: &AppState) -> PathBuf {
    let parent = state
        .data_file
        .parent()
        .unwrap_or_else(|| FsPath::new("."));
    parent.join("uploads")
}

fn to_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

fn safe_extension(name: &str) -> bool {
    !name.is_empty() && name.len() <= 12 && name.chars().all(|c| c.is_ascii_alphanumeric())
}

fn extension_for(mime: &str, file_name: Option<&str>) -> String {
    if let Some(name) = file_name {
        if let Some((_, ext)) = name.rsplit_once('.') {
            let ext = ext.to_lowercase();
            if safe_extension(&ext) {
                return ext;
            }
        }
    }
    match mime {
        "image/png" => "png",
        "image/jpeg" => "jpg",
        "image/gif" => "gif",
        "image/webp" => "webp",
        "image/bmp" => "bmp",
        "image/svg+xml" => "svg",
        "application/pdf" => "pdf",
        "text/plain" => "txt",
        "application/msword" => "doc",
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document" => "docx",
        _ => "bin",
    }
    .to_string()
}

fn mime_for(name: &str) -> &'static str {
    match name.rsplit_once('.').map(|(_, e)| e.to_lowercase()).as_deref() {
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("bmp") => "image/bmp",
        Some("svg") => "image/svg+xml",
        Some("pdf") => "application/pdf",
        Some("txt") => "text/plain",
        Some("doc") => "application/msword",
        Some("docx") => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        _ => "application/octet-stream",
    }
}

#[derive(Deserialize)]
struct UploadQuery {
    user_id: String,
    survey_id: String,
    question_id: String,
    #[serde(default)]
    file_name: Option<String>,
}

async fn client_upload_file(
    State(state): State<AppState>,
    Query(params): Query<UploadQuery>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if body.is_empty() {
        return (StatusCode::BAD_REQUEST, "empty file").into_response();
    }
    if body.len() as u64 > MAX_UPLOAD_BYTES {
        return (
            StatusCode::PAYLOAD_TOO_LARGE,
            "file exceeds size limit (10 MiB)",
        )
            .into_response();
    }
    let uploads = uploads_dir(&state);
    if let Err(error) = std::fs::create_dir_all(&uploads) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("failed to create uploads dir: {error}"),
        )
            .into_response();
    }
    let mime = headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("application/octet-stream")
        .to_string();
    let ext = extension_for(&mime, params.file_name.as_deref());
    let mut hasher = Sha256::new();
    hasher.update(now_string().as_bytes());
    hasher.update(UPLOAD_SEQ.fetch_add(1, Ordering::Relaxed).to_le_bytes());
    hasher.update(&body);
    let file_id = to_hex(&hasher.finalize());
    let stored_name = format!("{file_id}.{ext}");
    let path = uploads.join(&stored_name);
    if let Err(error) = std::fs::write(&path, &body) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("failed to write file: {error}"),
        )
            .into_response();
    }
    let file_name = params.file_name.unwrap_or_else(|| stored_name.clone());
    let payload = SurveyAnswerFile {
        file_id,
        file_name,
        mime_type: mime,
        size: body.len() as u64,
        url: format!("/api/client/uploads/{stored_name}"),
    };
    info!(
        user_id = %params.user_id,
        survey_id = %params.survey_id,
        question_id = %params.question_id,
        "respondent uploaded a survey answer file"
    );
    Json(payload).into_response()
}

async fn client_get_upload(State(state): State<AppState>, Path(raw): Path<String>) -> Response {
    if raw.is_empty()
        || raw.contains('/')
        || raw.contains('\\')
        || raw.contains("..")
        || raw.contains('\0')
    {
        return (StatusCode::BAD_REQUEST, "invalid file id").into_response();
    }
    let uploads = uploads_dir(&state);
    let _ = std::fs::create_dir_all(&uploads);
    let Ok(root) = std::fs::canonicalize(&uploads) else {
        return (StatusCode::NOT_FOUND, "not found").into_response();
    };
    let path = uploads.join(&raw);
    let Ok(target) = std::fs::canonicalize(&path) else {
        return (StatusCode::NOT_FOUND, "not found").into_response();
    };
    if !target.starts_with(&root) {
        return (StatusCode::FORBIDDEN, "forbidden").into_response();
    }
    match tokio::fs::read(&path).await {
        Ok(bytes) => {
            let mut response = Response::new(Body::from(bytes));
            if let Ok(value) = HeaderValue::from_str(mime_for(&raw)) {
                response.headers_mut().insert(CONTENT_TYPE, value);
            }
            response
        }
        Err(_) => (StatusCode::NOT_FOUND, "not found").into_response(),
    }
}

async fn client_click_survey(
    State(state): State<AppState>,
    Json(request): Json<ClickSurveyRequest>,
) -> Json<Value> {
    let mut data = state.data.write().await;
    // Snapshot question/option/survey labels server-side so the click log stays
    // readable even after the survey is edited or deleted.
    let (survey_title, revision, question_title, option_label) =
        match data.surveys.get(&request.survey_id) {
            Some(survey) => {
                let question = survey
                    .questions
                    .iter()
                    .find(|question| question.question_id == request.question_id);
                let question_title = question
                    .map(|question| question.title.clone())
                    .filter(|title| !title.is_empty())
                    .unwrap_or_else(|| request.question_title.clone());
                let option_label = question
                    .and_then(|question| {
                        question
                            .options
                            .iter()
                            .find(|option| option.option_id == request.option_id)
                    })
                    .map(|option| option.label.clone())
                    .filter(|label| !label.is_empty())
                    .unwrap_or_else(|| request.option_label.clone());
                (
                    survey.title.clone(),
                    if request.survey_revision > 0 {
                        request.survey_revision
                    } else {
                        survey.revision
                    },
                    question_title,
                    option_label,
                )
            }
            None => (
                String::new(),
                request.survey_revision,
                request.question_title.clone(),
                request.option_label.clone(),
            ),
        };

    let action = match request.action.as_str() {
        "deselect" | "input" => request.action.clone(),
        _ => "select".to_string(),
    };
    let event = SurveyClickEvent {
        event_id: survey::new_id("click"),
        survey_id: request.survey_id,
        survey_revision: revision,
        survey_title,
        user_id: request.user_id,
        question_id: request.question_id,
        question_title,
        option_id: request.option_id,
        option_label,
        action,
        text_value: request.text_value,
        clicked_at: now_string(),
        submission_id: String::new(),
    };
    data.survey_click_events.push(event);
    let overflow = data
        .survey_click_events
        .len()
        .saturating_sub(MAX_SURVEY_CLICK_EVENTS);
    if overflow > 0 {
        data.survey_click_events.drain(0..overflow);
    }
    drop(data);
    schedule_persist(&state);
    Json(json!({ "success": true }))
}

async fn client_submit_survey(
    State(state): State<AppState>,
    Json(request): Json<SubmitSurveyRequest>,
) -> Json<Value> {
    let mut data = state.data.write().await;
    let Some(survey) = data.surveys.get(&request.survey_id).cloned() else {
        return Json(json!({ "success": false, "message": "Survey not found" }));
    };
    if survey.status != "published" || survey.revision != request.survey_revision {
        return Json(json!({ "success": false, "message": "Survey version is no longer active" }));
    }
    let evaluation = survey::evaluate(&survey, &request.answers);
    let submission_id = survey::new_id("submission");
    let status = if evaluation.passed {
        "passed"
    } else {
        "failed"
    };
    // Link this user's unclaimed clicks for the same survey + revision to the
    // submission so the admin panel can show a per-submission click timeline.
    let click_events: Vec<SurveyClickEvent> = data
        .survey_click_events
        .iter_mut()
        .filter(|event| {
            event.submission_id.is_empty()
                && event.user_id == request.user_id
                && event.survey_id == request.survey_id
                && event.survey_revision == survey.revision
        })
        .map(|event| {
            event.submission_id = submission_id.clone();
            event.clone()
        })
        .collect();
    data.survey_submissions.insert(
        submission_id.clone(),
        SurveySubmission {
            submission_id: submission_id.clone(),
            survey_id: survey.survey_id.clone(),
            survey_revision: survey.revision,
            survey_title: survey.title.clone(),
            user_id: request.user_id.clone(),
            submitted_at: now_string(),
            status: status.to_string(),
            passed: evaluation.passed,
            answers: request.answers,
            answer_files: request.answer_files.unwrap_or_default(),
            failed_question_ids: evaluation.failed_question_ids.clone(),
            click_events,
        },
    );

    // Incentive: when the submission passes and the survey defines a reward, grant
    // the configured role (temporary or permanent) to the submitting user.
    let mut reward_payload: Option<Value> = None;
    if evaluation.passed {
        if let Some(grant) = &survey.reward {
            if let Some(user) = data.users.get_mut(&request.user_id) {
                user.role_id = Some(grant.role_id.clone());
                let expires_at = grant.duration_value.and_then(|value| {
                    let seconds = match grant.duration_unit.as_str() {
                        "day" => value * 86400.0,
                        "month" => value * 2_592_000.0,
                        "year" => value * 31_536_000.0,
                        _ => value * 3600.0, // hour (default)
                    };
                    let secs = seconds as i64;
                    if secs <= 0 {
                        return None;
                    }
                    Some(
                        (Local::now() + Duration::seconds(secs))
                            .format("%Y-%m-%d %H:%M:%S")
                            .to_string(),
                    )
                });
                user.role_expires_at = expires_at.clone();
                reward_payload = Some(json!({
                    "role_id": grant.role_id,
                    "role_name": data.roles.get(&grant.role_id).map(|role| role.role_name.clone()),
                    "duration_value": grant.duration_value,
                    "duration_unit": grant.duration_unit,
                    "expires_at": expires_at,
                    "permanent": grant.duration_value.is_none()
                }));
            }
        }
    }

    drop(data);
    schedule_persist(&state);
    let mut response = json!({
        "success": true,
        "submission_id": submission_id,
        "passed": evaluation.passed,
        "failed_question_ids": evaluation.failed_question_ids,
        "access_granted": evaluation.passed || !survey.required_for_access
    });
    if let Some(reward) = reward_payload {
        response["reward"] = reward;
    }
    Json(response)
}

async fn client_dismiss_survey(
    State(state): State<AppState>,
    Json(request): Json<DismissSurveyRequest>,
) -> Json<Value> {
    let mut data = state.data.write().await;
    let Some(survey) = data.surveys.get(&request.survey_id).cloned() else {
        return Json(json!({ "success": false, "message": "Survey not found" }));
    };
    if survey.required_for_access {
        return Json(json!({ "success": false, "message": "This survey is required" }));
    }
    if survey.status != "published" || survey.revision != request.survey_revision {
        return Json(json!({ "success": false, "message": "Survey version is no longer active" }));
    }
    let submission_id = survey::new_id("submission");
    data.survey_submissions.insert(
        submission_id.clone(),
        SurveySubmission {
            submission_id,
            survey_id: survey.survey_id,
            survey_revision: survey.revision,
            survey_title: survey.title,
            user_id: request.user_id,
            submitted_at: now_string(),
            status: "dismissed".to_string(),
            passed: false,
            answers: HashMap::new(),
            answer_files: HashMap::new(),
            failed_question_ids: Vec::new(),
            click_events: Vec::new(),
        },
    );
    drop(data);
    schedule_persist(&state);
    Json(json!({ "success": true, "message": "Survey dismissed" }))
}

async fn client_survey_history(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
) -> Json<Value> {
    let data = state.data.read().await;
    let mut submissions: Vec<SurveySubmission> = data
        .survey_submissions
        .values()
        .filter(|submission| submission.user_id == user_id)
        .cloned()
        .collect();
    submissions.sort_by(|left, right| right.submitted_at.cmp(&left.submitted_at));
    Json(json!({ "submissions": submissions }))
}

async fn client_delete_submission(
    State(state): State<AppState>,
    Json(request): Json<DeleteSubmissionRequest>,
) -> Json<Value> {
    let mut data = state.data.write().await;
    let owned = data
        .survey_submissions
        .get(&request.submission_id)
        .is_some_and(|submission| submission.user_id == request.user_id);
    if owned {
        data.survey_submissions.remove(&request.submission_id);
    }
    drop(data);
    if owned {
        schedule_persist(&state);
    }
    Json(json!({
        "success": owned,
        "message": if owned { "Submission deleted" } else { "Submission not found" }
    }))
}

async fn admin_auth(State(state): State<AppState>, Json(request): Json<AdminAuthRequest>) -> impl axum::response::IntoResponse {
    if verify_server_password(&request.password) {
        let token = hex::encode(rand::random::<[u8; 32]>());
        let session = AdminSession {
            token: token.clone(),
            created_at: now_string(),
        };
        state.admin_sessions.write().await.insert(token.clone(), session);
        (
            StatusCode::OK,
            Json(json!({ "success": true, "token": token })),
        )
    } else {
        (
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "success": false,
                "message": "Invalid server password"
            })),
        )
    }
}

async fn require_admin_password(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Result<axum::response::Response, StatusCode> {
    let password = request
        .headers()
        .get("x-vrcdog-admin-password")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    if !verify_server_password(password) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    Ok(next.run(request).await)
}

#[derive(Deserialize)]
struct VrchatProxyQuery {
    path: String,
}

async fn require_admin_session(
    mut request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Result<axum::response::Response, StatusCode> {
    let token = request
        .headers()
        .get("x-vrcdog-admin-token")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    let state = request
        .extensions()
        .get::<AppState>()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let sessions = state.admin_sessions.read().await;
    if !sessions.contains_key(token) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    drop(sessions);
    Ok(next.run(request).await)
}

async fn vrchat_api_proxy(
    State(state): State<AppState>,
    method: Method,
    Query(params): Query<VrchatProxyQuery>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    let target = format!("https://api.vrchat.cloud/api/1/{}", params.path.trim_start_matches('/'));
    let mut builder = state.http_client.request(method.clone(), &target);
    for (key, value) in headers.iter() {
        if key == "x-vrcdog-admin-token" || key == "host" || key == "connection" || key == "content-length" {
            continue;
        }
        if let Ok(v) = value.to_str() {
            builder = builder.header(key.as_str(), v);
        }
    }
    if method != Method::GET && method != Method::HEAD && !body.is_empty() {
        builder = builder.body(body);
    }
    match builder.send().await {
        Ok(resp) => {
            let status = StatusCode::from_u16(resp.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
            let mut response_builder = Response::builder().status(status);
            for (key, value) in resp.headers().iter() {
                if key == "content-encoding" || key == "content-length" {
                    continue;
                }
                response_builder = response_builder.header(key.as_str(), value.to_str().unwrap_or(""));
            }
            match resp.bytes().await {
                Ok(body) => response_builder.body(Body::from(body)).unwrap_or_else(|_| {
                    (StatusCode::BAD_GATEWAY, "failed to build response").into_response()
                }),
                Err(_) => (StatusCode::BAD_GATEWAY, "failed to read upstream response").into_response(),
            }
        }
        Err(_) => (StatusCode::BAD_GATEWAY, "failed to reach upstream").into_response(),
    }
}

async fn admin_clients(State(state): State<AppState>) -> Json<Value> {
    let clients: Vec<ClientInfo> = state.clients.read().await.values().cloned().collect();
    Json(json!({ "clients": clients }))
}

async fn admin_users(State(state): State<AppState>) -> Json<Value> {
    let data = state.data.read().await;
    let users: Vec<UserRecord> = data.users.values().cloned().collect();
    Json(json!({
        "users": users,
        "bans": data.bans,
        "frozen": data.frozen
    }))
}

async fn mark_offline(state: &AppState, user_id: &str) {
    state.clients.write().await.remove(user_id);
    if let Some(user) = state.data.write().await.users.get_mut(user_id) {
        user.is_online = false;
        user.last_seen = now_string();
    }
}

async fn admin_kick(
    State(state): State<AppState>,
    Json(request): Json<UserIdRequest>,
) -> Json<Value> {
    let existed = state.clients.read().await.contains_key(&request.user_id);
    mark_offline(&state, &request.user_id).await;
    if existed {
        state
            .data
            .write()
            .await
            .kicked
            .insert(request.user_id.clone(), now_string());
    }
    schedule_persist(&state);
    Json(json!({
        "success": existed,
        "message": if existed { "Client kicked" } else { "Client is not online" }
    }))
}

async fn admin_ban(State(state): State<AppState>, Json(request): Json<BanRequest>) -> Json<Value> {
    let now = now_string();
    let expires_at = request.duration_hours.map(|hours| {
        (Local::now() + Duration::seconds((hours * 3600.0) as i64))
            .format("%Y-%m-%d %H:%M:%S")
            .to_string()
    });
    state.data.write().await.bans.insert(
        request.user_id.clone(),
        BanInfo {
            user_id: request.user_id.clone(),
            reason: request.reason,
            banned_at: now,
            duration_hours: request.duration_hours,
            expires_at,
        },
    );
    mark_offline(&state, &request.user_id).await;
    schedule_persist(&state);
    Json(json!({ "success": true, "message": "User banned" }))
}

async fn admin_unban(
    State(state): State<AppState>,
    Json(request): Json<UserIdRequest>,
) -> Json<Value> {
    let removed = state
        .data
        .write()
        .await
        .bans
        .remove(&request.user_id)
        .is_some();
    schedule_persist(&state);
    Json(json!({
        "success": removed,
        "message": if removed { "User unbanned" } else { "User was not banned" }
    }))
}

async fn admin_freeze(
    State(state): State<AppState>,
    Json(request): Json<FreezeRequest>,
) -> Json<Value> {
    state.data.write().await.frozen.insert(
        request.user_id.clone(),
        FreezeInfo {
            user_id: request.user_id.clone(),
            reason: request.reason,
            frozen_at: now_string(),
        },
    );
    mark_offline(&state, &request.user_id).await;
    schedule_persist(&state);
    Json(json!({ "success": true, "message": "User frozen" }))
}

async fn admin_unfreeze(
    State(state): State<AppState>,
    Json(request): Json<UserIdRequest>,
) -> Json<Value> {
    let removed = state
        .data
        .write()
        .await
        .frozen
        .remove(&request.user_id)
        .is_some();
    schedule_persist(&state);
    Json(json!({
        "success": removed,
        "message": if removed { "User unfrozen" } else { "User was not frozen" }
    }))
}

async fn admin_remove(
    State(state): State<AppState>,
    Json(request): Json<UserIdRequest>,
) -> Json<Value> {
    state.clients.write().await.remove(&request.user_id);
    let mut data = state.data.write().await;
    data.users.remove(&request.user_id);
    data.bans.remove(&request.user_id);
    data.frozen.remove(&request.user_id);
    data.kicked.remove(&request.user_id);
    // Cascade: the player's own survey submissions are the same records the client
    // shows, so removing the user must also purge their submissions.
    data.survey_submissions
        .retain(|_, submission| submission.user_id != request.user_id);
    data.survey_click_events
        .retain(|event| event.user_id != request.user_id);
    drop(data);
    schedule_persist(&state);
    Json(json!({ "success": true, "message": "User record removed" }))
}

async fn admin_roles(State(state): State<AppState>) -> Json<Value> {
    let mut roles: Vec<Role> = state.data.read().await.roles.values().cloned().collect();
    roles.sort_by(|left, right| left.role_name.cmp(&right.role_name));
    Json(json!({ "roles": roles }))
}

async fn admin_save_role(State(state): State<AppState>, Json(mut role): Json<Role>) -> Json<Value> {
    let mut data = state.data.write().await;
    if data.roles.is_empty() {
        role.is_default = true;
    }
    if role.is_default {
        for current in data.roles.values_mut() {
            current.is_default = false;
        }
    }
    data.roles.insert(role.role_id.clone(), role);
    drop(data);
    schedule_persist(&state);
    Json(json!({ "success": true, "message": "Role saved" }))
}

async fn admin_delete_role(
    State(state): State<AppState>,
    Json(request): Json<RoleIdRequest>,
) -> Json<Value> {
    let mut data = state.data.write().await;
    if data
        .roles
        .get(&request.role_id)
        .is_some_and(|role| role.is_default)
    {
        return Json(json!({ "success": false, "message": "Default role cannot be deleted" }));
    }
    let removed = data.roles.remove(&request.role_id).is_some();
    if removed {
        for user in data.users.values_mut() {
            if user.role_id.as_deref() == Some(request.role_id.as_str()) {
                user.role_id = None;
            }
        }
    }
    drop(data);
    schedule_persist(&state);
    Json(json!({
        "success": removed,
        "message": if removed { "Role deleted" } else { "Role not found" }
    }))
}

async fn admin_set_default_role(
    State(state): State<AppState>,
    Json(request): Json<RoleIdRequest>,
) -> Json<Value> {
    let mut data = state.data.write().await;
    if !data.roles.contains_key(&request.role_id) {
        return Json(json!({ "success": false, "message": "Role not found" }));
    }
    for role in data.roles.values_mut() {
        role.is_default = role.role_id == request.role_id;
    }
    drop(data);
    schedule_persist(&state);
    Json(json!({ "success": true, "message": "Default role updated" }))
}

async fn admin_set_user_role(
    State(state): State<AppState>,
    Json(request): Json<SetUserRoleRequest>,
) -> Json<Value> {
    let mut data = state.data.write().await;
    let role_exists = request
        .role_id
        .as_ref()
        .is_none_or(|role_id| data.roles.contains_key(role_id));
    if !role_exists {
        return Json(json!({ "success": false, "message": "Role not found" }));
    }
    let Some(user) = data.users.get_mut(&request.user_id) else {
        return Json(json!({ "success": false, "message": "User not found" }));
    };
    user.role_id = request.role_id;
    drop(data);
    schedule_persist(&state);
    Json(json!({ "success": true, "message": "User role updated" }))
}

async fn admin_survey_settings(State(state): State<AppState>) -> Json<Value> {
    let data = state.data.read().await;
    Json(json!(data.survey_settings))
}

async fn admin_save_survey_settings(
    State(state): State<AppState>,
    Json(settings): Json<SurveySettings>,
) -> Json<Value> {
    state.data.write().await.survey_settings = settings.clone();
    schedule_persist(&state);
    Json(json!({
        "success": true,
        "enabled": settings.enabled,
        "message": "Survey settings saved"
    }))
}

async fn admin_surveys(State(state): State<AppState>) -> Json<Value> {
    let data = state.data.read().await;
    let mut surveys: Vec<Survey> = data.surveys.values().cloned().collect();
    surveys.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
    Json(json!({ "surveys": surveys }))
}

async fn admin_save_survey(
    State(state): State<AppState>,
    Json(mut incoming): Json<Survey>,
) -> Json<Value> {
    if let Err(message) = survey::validate_survey(&mut incoming) {
        return Json(json!({ "success": false, "message": message }));
    }
    let now = now_string();
    let mut data = state.data.write().await;
    if let Some(reward) = &incoming.reward {
        if !data.roles.contains_key(&reward.role_id) {
            return Json(json!({
                "success": false,
                "message": "奖励角色不存在，请选择一个有效角色"
            }));
        }
    }
    if let Some(existing) = data.surveys.get(&incoming.survey_id) {
        incoming.created_at = existing.created_at.clone();
        incoming.revision = existing.revision.max(1);
        if existing.status == "published" {
            let content_changed = existing.title != incoming.title
                || existing.description != incoming.description
                || existing.required_for_access != incoming.required_for_access
                || existing.questions != incoming.questions;
            if content_changed {
                incoming.revision += 1;
                incoming.status = "published".to_string();
                incoming.published_at = Some(now.clone());
            } else {
                incoming.published_at = existing.published_at.clone();
            }
        }
    } else {
        incoming.created_at = now.clone();
        incoming.revision = 1;
        if incoming.status == "published" {
            incoming.published_at = Some(now.clone());
        }
    }
    incoming.updated_at = now;
    let survey_id = incoming.survey_id.clone();
    let revision = incoming.revision;
    data.surveys.insert(survey_id.clone(), incoming);
    drop(data);
    schedule_persist(&state);
    Json(json!({
        "success": true,
        "survey_id": survey_id,
        "revision": revision,
        "message": "Survey saved"
    }))
}

async fn admin_publish_survey(
    State(state): State<AppState>,
    Json(request): Json<SurveyIdRequest>,
) -> Json<Value> {
    let mut data = state.data.write().await;
    let Some(survey) = data.surveys.get_mut(&request.survey_id) else {
        return Json(json!({ "success": false, "message": "Survey not found" }));
    };
    let now = now_string();
    if survey.status == "published" {
        survey.revision += 1;
    }
    survey.status = "published".to_string();
    survey.published_at = Some(now.clone());
    survey.updated_at = now;
    let revision = survey.revision;
    drop(data);
    schedule_persist(&state);
    Json(json!({
        "success": true,
        "revision": revision,
        "message": "Survey published"
    }))
}

async fn admin_resend_survey(
    State(state): State<AppState>,
    Json(request): Json<SurveyIdRequest>,
) -> Json<Value> {
    let mut data = state.data.write().await;
    let Some(survey) = data.surveys.get_mut(&request.survey_id) else {
        return Json(json!({ "success": false, "message": "Survey not found" }));
    };
    if survey.status != "published" {
        return Json(json!({ "success": false, "message": "Publish the survey before resending" }));
    }
    survey.revision += 1;
    survey.published_at = Some(now_string());
    survey.updated_at = now_string();
    let revision = survey.revision;
    drop(data);
    schedule_persist(&state);
    Json(json!({
        "success": true,
        "revision": revision,
        "message": "Survey resent to all users"
    }))
}

async fn admin_delete_survey(
    State(state): State<AppState>,
    Json(request): Json<SurveyIdRequest>,
) -> Json<Value> {
    let mut data = state.data.write().await;
    let removed = data.surveys.remove(&request.survey_id).is_some();
    if removed {
        data.survey_submissions
            .retain(|_, submission| submission.survey_id != request.survey_id);
    }
    drop(data);
    if removed {
        schedule_persist(&state);
    }
    Json(json!({
        "success": removed,
        "message": if removed { "Survey and its submissions deleted" } else { "Survey not found" }
    }))
}

async fn admin_survey_submissions(State(state): State<AppState>) -> Json<Value> {
    let data = state.data.read().await;
    let mut submissions: Vec<SurveySubmission> =
        data.survey_submissions.values().cloned().collect();
    submissions.sort_by(|left, right| right.submitted_at.cmp(&left.submitted_at));
    Json(json!({ "submissions": submissions }))
}

/// Raw click log for the admin panel. Clicks keep server-side snapshots of the
/// survey/question/option labels, so they remain readable even for deleted or
/// revised surveys. Optional `survey_id` query parameter filters the output.
#[derive(Deserialize)]
struct SurveyClicksQuery {
    survey_id: Option<String>,
}

async fn admin_survey_clicks(
    State(state): State<AppState>,
    Query(query): Query<SurveyClicksQuery>,
) -> Json<Value> {
    let data = state.data.read().await;
    let mut clicks: Vec<SurveyClickEvent> = data
        .survey_click_events
        .iter()
        .filter(|event| {
            query
                .survey_id
                .as_deref()
                .map_or(true, |survey_id| event.survey_id == survey_id)
        })
        .cloned()
        .collect();
    clicks.sort_by(|left, right| right.clicked_at.cmp(&left.clicked_at));
    Json(json!({ "clicks": clicks }))
}

async fn admin_delete_submission(
    State(state): State<AppState>,
    Json(request): Json<DeleteSubmissionRequest>,
) -> Json<Value> {
    let mut data = state.data.write().await;
    let removed = data
        .survey_submissions
        .remove(&request.submission_id)
        .is_some();
    drop(data);
    if removed {
        schedule_persist(&state);
    }
    Json(json!({
        "success": removed,
        "message": if removed { "Submission deleted" } else { "Submission not found" }
    }))
}

fn router(state: AppState) -> Router {
    let admin_routes = Router::new()
        .route("/api/admin/clients", get(admin_clients))
        .route("/api/admin/users", get(admin_users))
        .route("/api/admin/kick", post(admin_kick))
        .route("/api/admin/ban", post(admin_ban))
        .route("/api/admin/unban", post(admin_unban))
        .route("/api/admin/freeze", post(admin_freeze))
        .route("/api/admin/unfreeze", post(admin_unfreeze))
        .route("/api/admin/remove", post(admin_remove))
        .route("/api/admin/roles", get(admin_roles).post(admin_save_role))
        .route("/api/admin/roles/delete", post(admin_delete_role))
        .route("/api/admin/roles/set_default", post(admin_set_default_role))
        .route("/api/admin/users/set_role", post(admin_set_user_role))
        .route(
            "/api/admin/survey-settings",
            get(admin_survey_settings).post(admin_save_survey_settings),
        )
        .route(
            "/api/admin/surveys",
            get(admin_surveys).post(admin_save_survey),
        )
        .route("/api/admin/surveys/publish", post(admin_publish_survey))
        .route("/api/admin/surveys/resend", post(admin_resend_survey))
        .route("/api/admin/surveys/delete", post(admin_delete_survey))
        .route(
            "/api/admin/survey-submissions",
            get(admin_survey_submissions),
        )
        .route(
            "/api/admin/survey-submissions/delete",
            post(admin_delete_submission),
        )
        .route("/api/admin/survey-clicks", get(admin_survey_clicks))
        .route_layer(middleware::from_fn_with_state(state.clone(), require_admin_session));

    Router::new()
        .route("/", get(status_page))
        .route("/ping", get(ping))
        .route("/api/version", get(api_version))
        .route("/api/admin/auth", post(admin_auth))
        .route(
            "/api/vrchat-proxy",
            on(
                MethodFilter::GET
                    .or(MethodFilter::POST)
                    .or(MethodFilter::PUT)
                    .or(MethodFilter::DELETE)
                    .or(MethodFilter::PATCH)
                    .or(MethodFilter::OPTIONS),
                vrchat_api_proxy,
            ),
        )
        .route("/api/client/register", post(register))
        .route("/api/client/heartbeat", post(heartbeat))
        .route("/api/client/disconnect", post(disconnect))
        .route("/api/remote-assist/ws", get(remote_assist_ws))
        .route("/api/client/check-status/{user_id}", get(check_status))
        .route("/api/client/features/{user_id}", get(get_features))
        .route("/api/client/surveys/{user_id}", get(client_surveys))
        .route("/api/client/surveys/submit", post(client_submit_survey))
        .route("/api/client/surveys/click", post(client_click_survey))
        .route("/api/client/surveys/dismiss", post(client_dismiss_survey))
        .route("/api/client/surveys/upload", post(client_upload_file))
        .route("/api/client/uploads/{file_id}", get(client_get_upload))
        .route(
            "/api/client/survey-history/{user_id}",
            get(client_survey_history),
        )
        .route(
            "/api/client/survey-history/delete",
            post(client_delete_submission),
        )
        .merge(admin_routes)
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any)
                .allow_private_network(true),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn cleanup_stale_clients(state: AppState) {
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(15)).await;
        let cutoff = Local::now().naive_local() - Duration::seconds(45);
        let stale_ids: Vec<String> = state
            .clients
            .read()
            .await
            .iter()
            .filter_map(|(user_id, client)| {
                NaiveDateTime::parse_from_str(&client.last_heartbeat, "%Y-%m-%d %H:%M:%S")
                    .ok()
                    .filter(|heartbeat| *heartbeat < cutoff)
                    .map(|_| user_id.clone())
            })
            .collect();
        if stale_ids.is_empty() {
            continue;
        }
        for user_id in stale_ids {
            warn!(%user_id, "heartbeat timed out");
            mark_offline(&state, &user_id).await;
        }
        schedule_persist(&state);
    }
}

async fn cleanup_expired_sessions(state: AppState) {
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(300)).await;
        let cutoff = Local::now().naive_local() - Duration::hours(24);
        let mut sessions = state.admin_sessions.write().await;
        sessions.retain(|_, session| {
            NaiveDateTime::parse_from_str(&session.created_at, "%Y-%m-%d %H:%M:%S")
                .map(|created| created > cutoff)
                .unwrap_or(false)
        });
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "vrcdog_server=info,tower_http=info".into()),
        )
        .init();

    let host = env::var("VRCDOG_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = env::var("VRCDOG_PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(11451);
    let data_file = PathBuf::from(
        env::var("VRCDOG_DATA_FILE").unwrap_or_else(|_| "./data/server-state.json".to_string()),
    );
    let data = load_data(&data_file).await;
    let http_client = Arc::new(
        ClientBuilder::new()
            .cookie_store(true)
            .user_agent("VRCDog/5.1.3 (https://vrcdog.pcb.im; vrcdog@pcb.im)")
            .build()?,
    );
    let state = AppState {
        clients: Arc::new(RwLock::new(HashMap::new())),
        data: Arc::new(RwLock::new(data)),
        data_file: Arc::new(data_file),
        remote_assist: remote_assist_hub::RemoteAssistHub::default(),
        persist_dirty: Arc::new(AtomicBool::new(false)),
        persist_notify: Arc::new(Notify::new()),
        admin_sessions: Arc::new(RwLock::new(HashMap::new())),
        http_client,
    };
    if let Err(error) = persist_inner(&state).await {
        return Err(format!("initial persist failed: {error}").into());
    }
    tokio::spawn(persist_worker(state.clone()));
    tokio::spawn(cleanup_stale_clients(state.clone()));
    tokio::spawn(cleanup_expired_sessions(state.clone()));

    let address: SocketAddr = format!("{host}:{port}").parse()?;
    let listener = tokio::net::TcpListener::bind(address).await?;
    info!(%address, "VRCDog standalone server started");
    axum::serve(
        listener,
        router(state.clone()).into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async move {
        let _ = tokio::signal::ctrl_c().await;
        info!("shutdown signal received");
        let _ = persist_inner(&state).await;
    })
    .await?;
    Ok(())
}
