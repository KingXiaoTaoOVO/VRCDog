use axum::{
    extract::{ws::WebSocketUpgrade, ConnectInfo, Path, Request, State},
    http::{HeaderName, HeaderValue, Method, StatusCode},
    middleware,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::time::{Duration, Instant};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use tower_http::cors::{AllowOrigin, CorsLayer};

use crate::server_survey::{self, Survey, SurveySettings, SurveySubmission};

// ===== Admin brute-force protection =====
// Tracks failed admin-password attempts per client IP and locks the IP out
// for a cooldown window. This complements the static default password and
// mitigates online guessing, addressing the audit's P0 admin-protection gap.
lazy_static! {
    static ref ADMIN_FAILED_ATTEMPTS: StdMutex<HashMap<IpAddr, (u32, Instant)>> =
        StdMutex::new(HashMap::new());
}

const ADMIN_MAX_ATTEMPTS: u32 = 5;
const ADMIN_LOCKOUT_SECS: u64 = 300;

fn admin_is_locked(ip: IpAddr) -> bool {
    let mut map = ADMIN_FAILED_ATTEMPTS.lock().unwrap();
    if let Some((count, first)) = map.get(&ip) {
        if *count >= ADMIN_MAX_ATTEMPTS
            && first.elapsed() < Duration::from_secs(ADMIN_LOCKOUT_SECS)
        {
            return true;
        }
        if first.elapsed() >= Duration::from_secs(ADMIN_LOCKOUT_SECS) {
            map.remove(&ip);
        }
    }
    false
}

fn admin_register_failure(ip: IpAddr) {
    let mut map = ADMIN_FAILED_ATTEMPTS.lock().unwrap();
    let entry = map.entry(ip).or_insert((0u32, Instant::now()));
    entry.0 += 1;
}

fn admin_reset_failures(ip: IpAddr) {
    ADMIN_FAILED_ATTEMPTS.lock().unwrap().remove(&ip);
}

// ===== Data Models =====

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ClientInfo {
    pub user_id: String,
    pub display_name: String,
    pub avatar_url: String,
    pub ip_address: String,
    pub connected_at: String,
    pub last_heartbeat: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct UserRecord {
    pub user_id: String,
    pub display_name: String,
    pub avatar_url: String,
    pub first_seen: String,
    pub last_seen: String,
    pub login_count: u32,
    pub is_online: bool,
    pub role_id: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct BanInfo {
    pub user_id: String,
    pub reason: String,
    pub banned_at: String,
    pub duration_hours: Option<f64>, // None = permanent
    pub expires_at: Option<String>,  // calculated
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct FreezeInfo {
    pub user_id: String,
    pub reason: String,
    pub frozen_at: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct FeatureConfig {
    #[serde(default)]
    pub menus: HashMap<String, bool>, // key -> enabled
    #[serde(default)]
    pub themes: HashMap<String, bool>, // key -> enabled
    #[serde(default)]
    pub modes: HashMap<String, bool>, // key -> enabled (pc, vr)
}

impl Default for FeatureConfig {
    fn default() -> Self {
        let mut menus = HashMap::new();
        let menu_keys = [
            "dashboard",
            "feed",
            "friendlog",
            "locations",
            "charts",
            "playerlist",
            "gallery",
            "social",
            "search",
            "notifications",
            "groups",
            "avatars",
            "favorites",
            "moderation",
            "heatmap",
            "gamelog",
            "notes",
            "presets",
            "tools",
            "translator",
            "ovr",
            "env",
            "export",
            "settings",
        ];
        for key in menu_keys {
            menus.insert(key.to_string(), true);
        }

        let mut themes = HashMap::new();
        for key in ["dog", "cat", "helmet", "mono"] {
            themes.insert(key.to_string(), true);
        }

        let mut modes = HashMap::new();
        modes.insert("pc".to_string(), true);
        modes.insert("vr".to_string(), true);

        FeatureConfig {
            menus,
            themes,
            modes,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Role {
    pub role_id: String,
    pub role_name: String,
    pub is_default: bool,
    pub features: FeatureConfig,
}

#[derive(Clone)]
pub struct SharedState {
    pub app_handle: AppHandle,
    pub clients: Arc<Mutex<HashMap<String, ClientInfo>>>,
    pub users: Arc<Mutex<HashMap<String, UserRecord>>>,
    pub bans: Arc<Mutex<HashMap<String, BanInfo>>>,
    pub frozen: Arc<Mutex<HashMap<String, FreezeInfo>>>,
    pub roles: Arc<Mutex<HashMap<String, Role>>>,
    pub survey_settings: Arc<Mutex<SurveySettings>>,
    pub surveys: Arc<Mutex<HashMap<String, Survey>>>,
    pub survey_submissions: Arc<Mutex<HashMap<String, SurveySubmission>>>,
    pub shutdown: CancellationToken,
    pub remote_assist: crate::remote_assist_hub::RemoteAssistHub,
}

// ===== Request/Response Types =====

#[derive(Serialize)]
struct PingResponse {
    status: String,
    message: String,
}

#[derive(Deserialize)]
struct RegisterRequest {
    user_id: String,
    display_name: String,
    avatar_url: String,
}

#[derive(Serialize)]
struct StatusCheckResponse {
    status: String, // "ok" | "banned" | "frozen"
    reason: Option<String>,
    duration_hours: Option<f64>,
    expires_at: Option<String>,
}

#[derive(Serialize)]
struct FeatureResponse {
    menus: HashMap<String, bool>,
    themes: HashMap<String, bool>,
    modes: HashMap<String, bool>,
}

#[derive(Deserialize)]
struct BanRequest {
    user_id: String,
    reason: String,
    duration_hours: Option<f64>, // None = permanent
}

#[derive(Deserialize)]
struct FreezeRequest {
    user_id: String,
    reason: String,
}

#[derive(Deserialize)]
struct UserIdRequest {
    user_id: String,
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
    answers: HashMap<String, serde_json::Value>,
}

#[derive(Deserialize)]
struct DismissSurveyRequest {
    user_id: String,
    survey_id: String,
    survey_revision: u32,
}

#[derive(Deserialize)]
struct DeleteSubmissionRequest {
    user_id: String,
    submission_id: String,
}

#[derive(Deserialize)]
struct AdminAuthRequest {
    password: String,
}

#[derive(Serialize)]
struct AdminResponse {
    success: bool,
    message: String,
}

#[derive(Serialize)]
struct ClientListResponse {
    clients: Vec<ClientInfo>,
}

#[derive(Serialize)]
struct UserListResponse {
    users: Vec<UserRecord>,
    bans: HashMap<String, BanInfo>,
    frozen: HashMap<String, FreezeInfo>,
}

#[derive(Serialize)]
struct RoleListResponse {
    roles: Vec<Role>,
}

// ===== Helper =====
fn now_str() -> String {
    chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

async fn survey_gate_payload(state: &SharedState, user_id: &str) -> serde_json::Value {
    let settings = state.survey_settings.lock().await.clone();
    let surveys = state.surveys.lock().await.clone();
    let submissions = state.survey_submissions.lock().await.clone();
    let pending = server_survey::pending_surveys(settings.enabled, &surveys, &submissions, user_id);
    let required = pending.iter().any(|survey| survey.required_for_access);
    serde_json::json!({
        "status": if required { "survey_required" } else if pending.is_empty() { "ok" } else { "survey_available" },
        "pending_survey_count": pending.len(),
        "survey_required": required
    })
}

fn is_ban_expired(ban: &BanInfo) -> bool {
    if let Some(hours) = ban.duration_hours {
        if let Ok(banned_at) =
            chrono::NaiveDateTime::parse_from_str(&ban.banned_at, "%Y-%m-%d %H:%M:%S")
        {
            let expire_at = banned_at + chrono::Duration::seconds((hours * 3600.0) as i64);
            let now = chrono::Local::now().naive_local();
            return now > expire_at;
        }
    }
    false // permanent or parse error → not expired
}

// ===== Server Start =====

use std::sync::Mutex as StdMutex;

/// Global shutdown token, set when server starts
static SHUTDOWN_TOKEN: StdMutex<Option<CancellationToken>> = StdMutex::new(None);

pub fn stop_server() {
    if let Ok(mut lock) = SHUTDOWN_TOKEN.lock() {
        if let Some(token) = lock.take() {
            token.cancel();
        }
    }
}

pub fn is_server_running() -> bool {
    if let Ok(lock) = SHUTDOWN_TOKEN.lock() {
        lock.is_some()
    } else {
        false
    }
}

pub async fn start_server(app_handle: AppHandle, host: String, port: u16) -> Result<(), String> {
    let shutdown = CancellationToken::new();
    // Store globally so stop_server can access it
    if let Ok(mut lock) = SHUTDOWN_TOKEN.lock() {
        *lock = Some(shutdown.clone());
    }

    let mut initial_roles = HashMap::new();
    initial_roles.insert(
        "default".to_string(),
        Role {
            role_id: "default".to_string(),
            role_name: "默认用户".to_string(),
            is_default: true,
            features: FeatureConfig::default(),
        },
    );

    let state = SharedState {
        app_handle: app_handle.clone(),
        clients: Arc::new(Mutex::new(HashMap::new())),
        users: Arc::new(Mutex::new(HashMap::new())),
        bans: Arc::new(Mutex::new(HashMap::new())),
        frozen: Arc::new(Mutex::new(HashMap::new())),
        roles: Arc::new(Mutex::new(initial_roles)),
        survey_settings: Arc::new(Mutex::new(SurveySettings::default())),
        surveys: Arc::new(Mutex::new(HashMap::new())),
        survey_submissions: Arc::new(Mutex::new(HashMap::new())),
        shutdown: shutdown.clone(),
        remote_assist: crate::remote_assist_hub::RemoteAssistHub::default(),
    };

    // Only trust loopback / same-machine browser origins by default. The Tauri
    // client reaches the server through the Rust backend (reqwest), which is
    // not subject to CORS, so tightening browser origins here blocks malicious
    // websites from driving the local server via the user's browser without
    // affecting legitimate client/server traffic.
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::predicate(|origin: &HeaderValue, _| {
            match origin.to_str() {
                Ok(s) => {
                    s.starts_with("http://127.0.0.1")
                        || s.starts_with("http://localhost")
                        || s.starts_with("http://[::1]")
                        || s == "null"
                }
                Err(_) => false,
            }
        }))
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([
            HeaderName::from_static("content-type"),
            HeaderName::from_static("x-vrcdog-admin-password"),
        ])
        .allow_private_network(true);

    let admin_routes = Router::new()
        .route("/api/admin/clients", get(handle_admin_clients))
        .route("/api/admin/users", get(handle_admin_users))
        .route("/api/admin/kick", post(handle_admin_kick))
        .route("/api/admin/ban", post(handle_admin_ban))
        .route("/api/admin/unban", post(handle_admin_unban))
        .route("/api/admin/freeze", post(handle_admin_freeze))
        .route("/api/admin/unfreeze", post(handle_admin_unfreeze))
        .route("/api/admin/remove", post(handle_admin_remove))
        .route("/api/admin/roles", get(handle_admin_get_roles))
        .route("/api/admin/roles", post(handle_admin_save_role))
        .route("/api/admin/roles/delete", post(handle_admin_delete_role))
        .route(
            "/api/admin/roles/set_default",
            post(handle_admin_set_default_role),
        )
        .route(
            "/api/admin/users/set_role",
            post(handle_admin_set_user_role),
        )
        .route(
            "/api/admin/survey-settings",
            get(handle_admin_survey_settings).post(handle_admin_save_survey_settings),
        )
        .route(
            "/api/admin/surveys",
            get(handle_admin_surveys).post(handle_admin_save_survey),
        )
        .route(
            "/api/admin/surveys/publish",
            post(handle_admin_publish_survey),
        )
        .route(
            "/api/admin/surveys/resend",
            post(handle_admin_resend_survey),
        )
        .route(
            "/api/admin/surveys/delete",
            post(handle_admin_delete_survey),
        )
        .route(
            "/api/admin/survey-submissions",
            get(handle_admin_survey_submissions),
        )
        .route_layer(middleware::from_fn(require_admin_password));

    let app = Router::new()
        .route("/ping", get(handle_ping))
        .route("/api/admin/auth", post(handle_admin_auth))
        .route("/api/client/register", post(handle_client_register))
        .route("/api/client/heartbeat", post(handle_client_heartbeat))
        .route("/api/client/disconnect", post(handle_client_disconnect))
        .route("/api/remote-assist/ws", get(handle_remote_assist_ws))
        .route(
            "/api/client/check-status/{user_id}",
            get(handle_check_status),
        )
        .route(
            "/api/client/features/{user_id}",
            get(handle_get_features_public),
        )
        .route("/api/client/surveys/{user_id}", get(handle_client_surveys))
        .route(
            "/api/client/surveys/submit",
            post(handle_client_submit_survey),
        )
        .route(
            "/api/client/surveys/dismiss",
            post(handle_client_dismiss_survey),
        )
        .route(
            "/api/client/survey-history/{user_id}",
            get(handle_client_survey_history),
        )
        .route(
            "/api/client/survey-history/delete",
            post(handle_client_delete_submission),
        )
        .merge(admin_routes)
        .layer(cors)
        .with_state(state.clone());

    let addr = format!("{}:{}", host, port);
    let listener = match tokio::net::TcpListener::bind(&addr).await {
        Ok(l) => l,
        Err(e) => {
            let err_msg = format!("绑定端口 {} 失败: {}", port, e);
            let _ = app_handle.emit("server_log", format!("[ERROR] {}", err_msg));
            return Err(err_msg);
        }
    };

    let _ = app_handle.emit(
        "server_log",
        format!("[INFO] 服务端已成功启动，正在监听 {}", addr),
    );

    // Heartbeat cleanup task with shutdown support
    let cleanup_state = state.clone();
    let cleanup_shutdown = shutdown.clone();
    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(15)) => {},
                _ = cleanup_shutdown.cancelled() => break,
            }
            let mut clients = cleanup_state.clients.lock().await;
            let mut users = cleanup_state.users.lock().await;
            let bans = cleanup_state.bans.lock().await;
            let frozen = cleanup_state.frozen.lock().await;
            let now = chrono::Local::now().naive_local();
            let stale_ids: Vec<String> = clients
                .iter()
                .filter_map(|(id, info)| {
                    if let Ok(hb) = chrono::NaiveDateTime::parse_from_str(
                        &info.last_heartbeat,
                        "%Y-%m-%d %H:%M:%S",
                    ) {
                        if (now - hb).num_seconds() > 45 {
                            return Some(id.clone());
                        }
                    }
                    None
                })
                .collect();
            for id in &stale_ids {
                clients.remove(id);
                if !bans.contains_key(id) && !frozen.contains_key(id) {
                    users.remove(id);
                } else if let Some(u) = users.get_mut(id) {
                    u.is_online = false;
                }
                let _ = cleanup_state.app_handle.emit(
                    "server_log",
                    format!("[WARN] 客户端 {} 心跳超时，已自动移除", id),
                );
            }
            drop(bans);
            let mut bans = cleanup_state.bans.lock().await;
            let expired: Vec<String> = bans
                .iter()
                .filter(|(_, b)| is_ban_expired(b))
                .map(|(id, _)| id.clone())
                .collect();
            for id in expired {
                bans.remove(&id);
                if !clients.contains_key(&id) && !frozen.contains_key(&id) {
                    users.remove(&id);
                }
                let _ = cleanup_state.app_handle.emit(
                    "server_log",
                    format!("[INFO] 用户 {} 的封禁已到期，已自动解封", id),
                );
            }
        }
    });

    // Spawn the server with graceful shutdown
    let server_shutdown = shutdown.clone();
    let server_app_handle = app_handle.clone();
    tokio::spawn(async move {
        let serve = axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        );
        tokio::select! {
            res = serve => {
                if let Err(e) = res {
                    let _ = server_app_handle.emit("server_log", format!("[ERROR] 服务端运行异常: {}", e));
                }
            }
            _ = server_shutdown.cancelled() => {
                let _ = server_app_handle.emit("server_log", "[INFO] 服务端主动终止，正在立即释放端口...".to_string());
            }
        }
    });

    Ok(())
}

// ===== Handlers: Public =====

async fn handle_ping(
    State(state): State<SharedState>,
    client_ip: ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    let _ = state.app_handle.emit(
        "server_log",
        format!("[INFO] 收到来自客户端 ({}) 的连接请求 (ping)", client_ip.0),
    );
    Json(PingResponse {
        status: "ok".to_string(),
        message: "Pong from VrcDog Server".to_string(),
    })
}

async fn handle_remote_assist_ws(
    State(state): State<SharedState>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    state.remote_assist.upgrade(ws).await
}

async fn handle_client_register(
    State(state): State<SharedState>,
    client_ip: ConnectInfo<SocketAddr>,
    Json(req): Json<RegisterRequest>,
) -> impl IntoResponse {
    let now = now_str();

    // Check if banned
    {
        let bans = state.bans.lock().await;
        if let Some(ban) = bans.get(&req.user_id) {
            if !is_ban_expired(ban) {
                let _ = state.app_handle.emit(
                    "server_log",
                    format!(
                        "[WARN] 被封禁用户 {} ({}) 尝试注册，已拒绝",
                        req.display_name, req.user_id
                    ),
                );
                return Json(serde_json::json!({
                    "status": "banned",
                    "reason": ban.reason,
                    "duration_hours": ban.duration_hours,
                    "expires_at": ban.expires_at,
                }));
            }
        }
    }

    // Check if frozen
    {
        let frozen = state.frozen.lock().await;
        if let Some(freeze) = frozen.get(&req.user_id) {
            let _ = state.app_handle.emit(
                "server_log",
                format!(
                    "[WARN] 被冻结用户 {} ({}) 尝试注册，已拒绝",
                    req.display_name, req.user_id
                ),
            );
            return Json(serde_json::json!({
                "status": "frozen",
                "reason": freeze.reason,
            }));
        }
    }

    // Add to online clients
    {
        let mut clients = state.clients.lock().await;
        clients.insert(
            req.user_id.clone(),
            ClientInfo {
                user_id: req.user_id.clone(),
                display_name: req.display_name.clone(),
                avatar_url: req.avatar_url.clone(),
                ip_address: client_ip.0.to_string(),
                connected_at: now.clone(),
                last_heartbeat: now.clone(),
            },
        );
    }

    // Update user records
    {
        let mut users = state.users.lock().await;
        if let Some(user) = users.get_mut(&req.user_id) {
            user.display_name = req.display_name.clone();
            user.avatar_url = req.avatar_url.clone();
            user.last_seen = now.clone();
            user.login_count += 1;
            user.is_online = true;
        } else {
            users.insert(
                req.user_id.clone(),
                UserRecord {
                    user_id: req.user_id.clone(),
                    display_name: req.display_name.clone(),
                    avatar_url: req.avatar_url.clone(),
                    first_seen: now.clone(),
                    last_seen: now.clone(),
                    login_count: 1,
                    is_online: true,
                    role_id: None,
                },
            );
        }
    }

    // Emit to dashboard
    let _ = state.app_handle.emit(
        "server_log",
        format!(
            "[INFO] 客户端注册成功: {} ({}) IP: {}",
            req.display_name, req.user_id, client_ip.0
        ),
    );
    let _ = state.app_handle.emit("clients_updated", "");

    let mut response = survey_gate_payload(&state, &req.user_id).await;
    response["message"] = serde_json::json!("registered");
    Json(response)
}

async fn handle_client_heartbeat(
    State(state): State<SharedState>,
    Json(req): Json<UserIdRequest>,
) -> impl IntoResponse {
    let now = now_str();

    // Check ban/freeze
    {
        let bans = state.bans.lock().await;
        if let Some(ban) = bans.get(&req.user_id) {
            if !is_ban_expired(ban) {
                return Json(serde_json::json!({
                    "status": "banned",
                    "reason": ban.reason,
                    "duration_hours": ban.duration_hours,
                    "expires_at": ban.expires_at,
                }));
            }
        }
    }
    {
        let frozen = state.frozen.lock().await;
        if let Some(freeze) = frozen.get(&req.user_id) {
            return Json(serde_json::json!({
                "status": "frozen",
                "reason": freeze.reason,
            }));
        }
    }

    // Update heartbeat
    {
        let mut clients = state.clients.lock().await;
        if let Some(client) = clients.get_mut(&req.user_id) {
            client.last_heartbeat = now;
        } else {
            return Json(serde_json::json!({ "status": "register_required" }));
        }
    }

    Json(survey_gate_payload(&state, &req.user_id).await)
}

async fn handle_client_disconnect(
    State(state): State<SharedState>,
    Json(req): Json<UserIdRequest>,
) -> impl IntoResponse {
    let now = now_str();
    {
        let mut clients = state.clients.lock().await;
        clients.remove(&req.user_id);
    }
    {
        let mut users = state.users.lock().await;
        if let Some(u) = users.get_mut(&req.user_id) {
            u.is_online = false;
            u.last_seen = now;
        }
    }
    let _ = state.app_handle.emit(
        "server_log",
        format!("[INFO] 客户端断开连接: {}", req.user_id),
    );
    let _ = state.app_handle.emit("clients_updated", "");
    Json(serde_json::json!({ "status": "ok" }))
}

async fn handle_check_status(
    State(state): State<SharedState>,
    Path(user_id): Path<String>,
) -> impl IntoResponse {
    // Check ban
    {
        let bans = state.bans.lock().await;
        if let Some(ban) = bans.get(&user_id) {
            if !is_ban_expired(ban) {
                return Json(StatusCheckResponse {
                    status: "banned".to_string(),
                    reason: Some(ban.reason.clone()),
                    duration_hours: ban.duration_hours,
                    expires_at: ban.expires_at.clone(),
                });
            }
        }
    }

    // Check freeze
    {
        let frozen = state.frozen.lock().await;
        if let Some(freeze) = frozen.get(&user_id) {
            return Json(StatusCheckResponse {
                status: "frozen".to_string(),
                reason: Some(freeze.reason.clone()),
                duration_hours: None,
                expires_at: None,
            });
        }
    }

    Json(StatusCheckResponse {
        status: "ok".to_string(),
        reason: None,
        duration_hours: None,
        expires_at: None,
    })
}

async fn handle_get_features_public(
    State(state): State<SharedState>,
    Path(user_id): Path<String>,
) -> impl IntoResponse {
    let users = state.users.lock().await;
    let roles = state.roles.lock().await;

    // Default config fallback
    let mut current_config = FeatureConfig::default();

    // Find default role first
    if let Some(default_role) = roles.values().find(|r| r.is_default) {
        current_config = default_role.features.clone();
    }

    // If user has a specific role, override
    if let Some(user) = users.get(&user_id) {
        if let Some(role_id) = &user.role_id {
            if let Some(role) = roles.get(role_id) {
                current_config = role.features.clone();
            }
        }
    }

    Json(FeatureResponse {
        menus: current_config.menus,
        themes: current_config.themes,
        modes: current_config.modes,
    })
}

async fn handle_client_surveys(
    State(state): State<SharedState>,
    Path(user_id): Path<String>,
) -> impl IntoResponse {
    let enabled = state.survey_settings.lock().await.enabled;
    let surveys = state.surveys.lock().await.clone();
    let submissions = state.survey_submissions.lock().await.clone();
    let pending = server_survey::pending_surveys(enabled, &surveys, &submissions, &user_id);
    Json(serde_json::json!({ "enabled": enabled, "surveys": pending }))
}

async fn handle_client_submit_survey(
    State(state): State<SharedState>,
    Json(req): Json<SubmitSurveyRequest>,
) -> impl IntoResponse {
    let survey = {
        let surveys = state.surveys.lock().await;
        surveys.get(&req.survey_id).cloned()
    };
    let Some(survey) = survey else {
        return Json(serde_json::json!({ "success": false, "message": "Survey not found" }));
    };
    if survey.status != "published" || survey.revision != req.survey_revision {
        return Json(
            serde_json::json!({ "success": false, "message": "Survey version is no longer active" }),
        );
    }
    let evaluation = server_survey::evaluate(&survey, &req.answers);
    let submission_id = server_survey::new_id("submission");
    let status = if evaluation.passed {
        "passed"
    } else {
        "failed"
    };
    state.survey_submissions.lock().await.insert(
        submission_id.clone(),
        SurveySubmission {
            submission_id: submission_id.clone(),
            survey_id: survey.survey_id.clone(),
            survey_revision: survey.revision,
            survey_title: survey.title.clone(),
            user_id: req.user_id,
            submitted_at: now_str(),
            status: status.to_string(),
            passed: evaluation.passed,
            answers: req.answers,
            failed_question_ids: evaluation.failed_question_ids.clone(),
        },
    );
    Json(serde_json::json!({
        "success": true,
        "submission_id": submission_id,
        "passed": evaluation.passed,
        "failed_question_ids": evaluation.failed_question_ids,
        "access_granted": evaluation.passed || !survey.required_for_access
    }))
}

async fn handle_client_dismiss_survey(
    State(state): State<SharedState>,
    Json(req): Json<DismissSurveyRequest>,
) -> impl IntoResponse {
    let survey = {
        let surveys = state.surveys.lock().await;
        surveys.get(&req.survey_id).cloned()
    };
    let Some(survey) = survey else {
        return Json(serde_json::json!({ "success": false, "message": "Survey not found" }));
    };
    if survey.required_for_access {
        return Json(serde_json::json!({ "success": false, "message": "This survey is required" }));
    }
    if survey.status != "published" || survey.revision != req.survey_revision {
        return Json(
            serde_json::json!({ "success": false, "message": "Survey version is no longer active" }),
        );
    }
    let submission_id = server_survey::new_id("submission");
    state.survey_submissions.lock().await.insert(
        submission_id.clone(),
        SurveySubmission {
            submission_id,
            survey_id: survey.survey_id,
            survey_revision: survey.revision,
            survey_title: survey.title,
            user_id: req.user_id,
            submitted_at: now_str(),
            status: "dismissed".to_string(),
            passed: false,
            answers: HashMap::new(),
            failed_question_ids: Vec::new(),
        },
    );
    Json(serde_json::json!({ "success": true, "message": "Survey dismissed" }))
}

async fn handle_client_survey_history(
    State(state): State<SharedState>,
    Path(user_id): Path<String>,
) -> impl IntoResponse {
    let submissions = state.survey_submissions.lock().await;
    let mut history: Vec<SurveySubmission> = submissions
        .values()
        .filter(|submission| submission.user_id == user_id)
        .cloned()
        .collect();
    history.sort_by(|left, right| right.submitted_at.cmp(&left.submitted_at));
    Json(serde_json::json!({ "submissions": history }))
}

async fn handle_client_delete_submission(
    State(state): State<SharedState>,
    Json(req): Json<DeleteSubmissionRequest>,
) -> impl IntoResponse {
    let mut submissions = state.survey_submissions.lock().await;
    let owned = submissions
        .get(&req.submission_id)
        .is_some_and(|submission| submission.user_id == req.user_id);
    if owned {
        submissions.remove(&req.submission_id);
    }
    Json(serde_json::json!({
        "success": owned,
        "message": if owned { "Submission deleted" } else { "Submission not found" }
    }))
}

// ===== Handlers: Admin =====

async fn handle_admin_auth(
    ConnectInfo(client_ip): ConnectInfo<SocketAddr>,
    Json(req): Json<AdminAuthRequest>,
) -> impl IntoResponse {
    let ip = client_ip.ip();
    if admin_is_locked(ip) {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(serde_json::json!({
                "success": false,
                "message": "Too many failed attempts, please try again later"
            })),
        );
    }
    if crate::verify_server_password(&req.password) {
        admin_reset_failures(ip);
        (
            StatusCode::OK,
            Json(serde_json::json!({ "success": true })),
        )
    } else {
        admin_register_failure(ip);
        (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({
                "success": false,
                "message": "Invalid server password"
            })),
        )
    }
}

async fn require_admin_password(
    request: Request,
    next: middleware::Next,
) -> Result<axum::response::Response, StatusCode> {
    let ip = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|c| c.0.ip())
        .unwrap_or_else(|| IpAddr::from([0, 0, 0, 0]));
    if admin_is_locked(ip) {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }
    let password = request
        .headers()
        .get("x-vrcdog-admin-password")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    if !crate::verify_server_password(password) {
        admin_register_failure(ip);
        return Err(StatusCode::UNAUTHORIZED);
    }
    admin_reset_failures(ip);
    Ok(next.run(request).await)
}

async fn handle_admin_clients(State(state): State<SharedState>) -> impl IntoResponse {
    let clients = state.clients.lock().await;
    Json(ClientListResponse {
        clients: clients.values().cloned().collect(),
    })
}

async fn handle_admin_users(State(state): State<SharedState>) -> impl IntoResponse {
    let users = state.users.lock().await;
    let bans = state.bans.lock().await;
    let frozen = state.frozen.lock().await;
    Json(UserListResponse {
        users: users.values().cloned().collect(),
        bans: bans.clone(),
        frozen: frozen.clone(),
    })
}

async fn handle_admin_kick(
    State(state): State<SharedState>,
    Json(req): Json<UserIdRequest>,
) -> impl IntoResponse {
    let is_banned = state.bans.lock().await.contains_key(&req.user_id);
    let is_frozen = state.frozen.lock().await.contains_key(&req.user_id);
    let mut clients = state.clients.lock().await;
    let mut users = state.users.lock().await;
    if clients.remove(&req.user_id).is_some() {
        if !is_banned && !is_frozen {
            users.remove(&req.user_id);
        } else if let Some(u) = users.get_mut(&req.user_id) {
            u.is_online = false;
        }
        let _ = state.app_handle.emit(
            "server_log",
            format!("[WARN] 管理员已踢出用户: {}", req.user_id),
        );
        let _ = state.app_handle.emit("clients_updated", "");
        let _ = state.app_handle.emit(
            "client_kicked",
            serde_json::json!({
                "user_id": req.user_id,
                "reason": "admin_kick",
            }),
        );
        Json(AdminResponse {
            success: true,
            message: "已踢出".to_string(),
        })
    } else {
        Json(AdminResponse {
            success: false,
            message: "用户不在线".to_string(),
        })
    }
}

async fn handle_admin_ban(
    State(state): State<SharedState>,
    Json(req): Json<BanRequest>,
) -> impl IntoResponse {
    let now = now_str();
    let expires_at = req.duration_hours.map(|h| {
        let at = chrono::Local::now() + chrono::Duration::seconds((h * 3600.0) as i64);
        at.format("%Y-%m-%d %H:%M:%S").to_string()
    });

    let ban = BanInfo {
        user_id: req.user_id.clone(),
        reason: req.reason.clone(),
        banned_at: now,
        duration_hours: req.duration_hours,
        expires_at: expires_at.clone(),
    };

    {
        let mut bans = state.bans.lock().await;
        bans.insert(req.user_id.clone(), ban);
    }

    // Kick if online
    {
        let mut clients = state.clients.lock().await;
        clients.remove(&req.user_id);
        let mut users = state.users.lock().await;
        if let Some(u) = users.get_mut(&req.user_id) {
            u.is_online = false;
        }
    }

    let dur_str = match req.duration_hours {
        Some(h) => format!("{:.1}小时", h),
        None => "永久".to_string(),
    };
    let _ = state.app_handle.emit(
        "server_log",
        format!(
            "[WARN] 管理员封禁用户 {}, 原因: {}, 时长: {}",
            req.user_id, req.reason, dur_str
        ),
    );
    let _ = state.app_handle.emit("clients_updated", "");
    let _ = state.app_handle.emit(
        "client_banned",
        serde_json::json!({
            "user_id": req.user_id.clone(),
            "reason": req.reason.clone(),
            "duration_hours": req.duration_hours,
        }),
    );

    Json(AdminResponse {
        success: true,
        message: format!("已封禁 ({})", dur_str),
    })
}

async fn handle_admin_unban(
    State(state): State<SharedState>,
    Json(req): Json<UserIdRequest>,
) -> impl IntoResponse {
    let mut bans = state.bans.lock().await;
    if bans.remove(&req.user_id).is_some() {
        let is_online = state.clients.lock().await.contains_key(&req.user_id);
        let is_frozen = state.frozen.lock().await.contains_key(&req.user_id);
        if !is_online && !is_frozen {
            let mut users = state.users.lock().await;
            users.remove(&req.user_id);
        }
        let _ = state.app_handle.emit(
            "server_log",
            format!("[INFO] 管理员解封用户: {}", req.user_id),
        );
        Json(AdminResponse {
            success: true,
            message: "已解封".to_string(),
        })
    } else {
        Json(AdminResponse {
            success: false,
            message: "用户未被封禁".to_string(),
        })
    }
}

async fn handle_admin_freeze(
    State(state): State<SharedState>,
    Json(req): Json<FreezeRequest>,
) -> impl IntoResponse {
    let now = now_str();
    let freeze = FreezeInfo {
        user_id: req.user_id.clone(),
        reason: req.reason.clone(),
        frozen_at: now,
    };

    {
        let mut frozen = state.frozen.lock().await;
        frozen.insert(req.user_id.clone(), freeze);
    }

    // Kick if online
    {
        let mut clients = state.clients.lock().await;
        clients.remove(&req.user_id);
        let mut users = state.users.lock().await;
        if let Some(u) = users.get_mut(&req.user_id) {
            u.is_online = false;
        }
    }

    let _ = state.app_handle.emit(
        "server_log",
        format!(
            "[WARN] 管理员冻结用户 {}, 原因: {}",
            req.user_id, req.reason
        ),
    );
    let _ = state.app_handle.emit("clients_updated", "");
    let _ = state.app_handle.emit(
        "client_frozen",
        serde_json::json!({
            "user_id": req.user_id.clone(),
            "reason": req.reason.clone(),
        }),
    );

    Json(AdminResponse {
        success: true,
        message: "已冻结".to_string(),
    })
}

async fn handle_admin_unfreeze(
    State(state): State<SharedState>,
    Json(req): Json<UserIdRequest>,
) -> impl IntoResponse {
    let mut frozen = state.frozen.lock().await;
    if frozen.remove(&req.user_id).is_some() {
        let is_online = state.clients.lock().await.contains_key(&req.user_id);
        let is_banned = state.bans.lock().await.contains_key(&req.user_id);
        if !is_online && !is_banned {
            let mut users = state.users.lock().await;
            users.remove(&req.user_id);
        }
        let _ = state.app_handle.emit(
            "server_log",
            format!("[INFO] 管理员解冻用户: {}", req.user_id),
        );
        Json(AdminResponse {
            success: true,
            message: "已解冻".to_string(),
        })
    } else {
        Json(AdminResponse {
            success: false,
            message: "用户未被冻结".to_string(),
        })
    }
}

async fn handle_admin_remove(
    State(state): State<SharedState>,
    Json(req): Json<UserIdRequest>,
) -> impl IntoResponse {
    {
        let mut clients = state.clients.lock().await;
        clients.remove(&req.user_id);
    }
    {
        let mut users = state.users.lock().await;
        users.remove(&req.user_id);
    }
    // Also clear bans/freezes for this user
    {
        let mut bans = state.bans.lock().await;
        bans.remove(&req.user_id);
    }
    {
        let mut frozen = state.frozen.lock().await;
        frozen.remove(&req.user_id);
    }

    let _ = state.app_handle.emit(
        "server_log",
        format!(
            "[WARN] 管理员移除用户记录: {} (用户重新登录后将自动重新注册)",
            req.user_id
        ),
    );
    let _ = state.app_handle.emit("clients_updated", "");
    let _ = state.app_handle.emit(
        "client_removed",
        serde_json::json!({
            "user_id": req.user_id,
            "reason": "admin_remove",
        }),
    );

    Json(AdminResponse {
        success: true,
        message: "已移除用户记录".to_string(),
    })
}

async fn handle_admin_get_roles(State(state): State<SharedState>) -> impl IntoResponse {
    let roles = state.roles.lock().await;
    let mut role_list: Vec<Role> = roles.values().cloned().collect();
    role_list.sort_by(|a, b| a.role_name.cmp(&b.role_name)); // sort by name
    Json(RoleListResponse { roles: role_list })
}

async fn handle_admin_save_role(
    State(state): State<SharedState>,
    Json(mut role): Json<Role>,
) -> impl IntoResponse {
    let mut roles = state.roles.lock().await;
    if roles.is_empty() {
        role.is_default = true;
    }
    if role.is_default {
        // ensure others are not default
        for r in roles.values_mut() {
            r.is_default = false;
        }
    }
    roles.insert(role.role_id.clone(), role);
    let _ = state
        .app_handle
        .emit("server_log", "[INFO] 管理员保存了角色配置".to_string());
    Json(AdminResponse {
        success: true,
        message: "角色配置已保存".to_string(),
    })
}

async fn handle_admin_delete_role(
    State(state): State<SharedState>,
    Json(req): Json<RoleIdRequest>,
) -> impl IntoResponse {
    let mut roles = state.roles.lock().await;
    if let Some(role) = roles.get(&req.role_id) {
        if role.is_default {
            return Json(AdminResponse {
                success: false,
                message: "默认角色无法删除".to_string(),
            });
        }
    }
    if roles.remove(&req.role_id).is_some() {
        // Also clear this role from users
        let mut users = state.users.lock().await;
        for u in users.values_mut() {
            if u.role_id == Some(req.role_id.clone()) {
                u.role_id = None;
            }
        }
        let _ = state
            .app_handle
            .emit("server_log", "[INFO] 管理员删除了角色".to_string());
        Json(AdminResponse {
            success: true,
            message: "角色已删除".to_string(),
        })
    } else {
        Json(AdminResponse {
            success: false,
            message: "角色不存在".to_string(),
        })
    }
}

async fn handle_admin_set_default_role(
    State(state): State<SharedState>,
    Json(req): Json<RoleIdRequest>,
) -> impl IntoResponse {
    let mut roles = state.roles.lock().await;
    if !roles.contains_key(&req.role_id) {
        return Json(AdminResponse {
            success: false,
            message: "角色不存在".to_string(),
        });
    }
    for r in roles.values_mut() {
        r.is_default = r.role_id == req.role_id;
    }
    let _ = state
        .app_handle
        .emit("server_log", "[INFO] 管理员设置了新的默认角色".to_string());
    Json(AdminResponse {
        success: true,
        message: "默认角色已更新".to_string(),
    })
}

async fn handle_admin_set_user_role(
    State(state): State<SharedState>,
    Json(req): Json<SetUserRoleRequest>,
) -> impl IntoResponse {
    let mut users = state.users.lock().await;
    if let Some(u) = users.get_mut(&req.user_id) {
        u.role_id = req.role_id;
        let _ = state.app_handle.emit(
            "server_log",
            format!("[INFO] 管理员更新了用户角色: {}", req.user_id),
        );
        Json(AdminResponse {
            success: true,
            message: "用户角色已更新".to_string(),
        })
    } else {
        Json(AdminResponse {
            success: false,
            message: "用户不存在".to_string(),
        })
    }
}

async fn handle_admin_survey_settings(State(state): State<SharedState>) -> impl IntoResponse {
    Json(state.survey_settings.lock().await.clone())
}

async fn handle_admin_save_survey_settings(
    State(state): State<SharedState>,
    Json(settings): Json<SurveySettings>,
) -> impl IntoResponse {
    *state.survey_settings.lock().await = settings.clone();
    Json(serde_json::json!({
        "success": true,
        "enabled": settings.enabled,
        "message": "Survey settings saved"
    }))
}

async fn handle_admin_surveys(State(state): State<SharedState>) -> impl IntoResponse {
    let surveys = state.surveys.lock().await;
    let mut list: Vec<Survey> = surveys.values().cloned().collect();
    list.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
    Json(serde_json::json!({ "surveys": list }))
}

async fn handle_admin_save_survey(
    State(state): State<SharedState>,
    Json(mut incoming): Json<Survey>,
) -> impl IntoResponse {
    if let Err(message) = server_survey::validate_survey(&mut incoming) {
        return Json(serde_json::json!({ "success": false, "message": message }));
    }
    let now = now_str();
    let mut surveys = state.surveys.lock().await;
    if let Some(existing) = surveys.get(&incoming.survey_id) {
        incoming.created_at = existing.created_at.clone();
        incoming.revision = existing.revision.max(1);
        if existing.status == "published" {
            let content_changed = existing.title != incoming.title
                || existing.description != incoming.description
                || existing.required_for_access != incoming.required_for_access
                || existing.questions != incoming.questions;
            if content_changed {
                incoming.revision += 1;
                incoming.status = "published".into();
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
    surveys.insert(survey_id.clone(), incoming);
    Json(serde_json::json!({
        "success": true,
        "survey_id": survey_id,
        "revision": revision,
        "message": "Survey saved"
    }))
}

async fn handle_admin_publish_survey(
    State(state): State<SharedState>,
    Json(req): Json<SurveyIdRequest>,
) -> impl IntoResponse {
    let mut surveys = state.surveys.lock().await;
    let Some(survey) = surveys.get_mut(&req.survey_id) else {
        return Json(serde_json::json!({ "success": false, "message": "Survey not found" }));
    };
    if survey.status == "published" {
        survey.revision += 1;
    }
    survey.status = "published".into();
    survey.published_at = Some(now_str());
    survey.updated_at = now_str();
    Json(
        serde_json::json!({ "success": true, "revision": survey.revision, "message": "Survey published" }),
    )
}

async fn handle_admin_resend_survey(
    State(state): State<SharedState>,
    Json(req): Json<SurveyIdRequest>,
) -> impl IntoResponse {
    let mut surveys = state.surveys.lock().await;
    let Some(survey) = surveys.get_mut(&req.survey_id) else {
        return Json(serde_json::json!({ "success": false, "message": "Survey not found" }));
    };
    if survey.status != "published" {
        return Json(
            serde_json::json!({ "success": false, "message": "Publish the survey before resending" }),
        );
    }
    survey.revision += 1;
    survey.published_at = Some(now_str());
    survey.updated_at = now_str();
    Json(
        serde_json::json!({ "success": true, "revision": survey.revision, "message": "Survey resent to all users" }),
    )
}

async fn handle_admin_delete_survey(
    State(state): State<SharedState>,
    Json(req): Json<SurveyIdRequest>,
) -> impl IntoResponse {
    let removed = state.surveys.lock().await.remove(&req.survey_id).is_some();
    if removed {
        state
            .survey_submissions
            .lock()
            .await
            .retain(|_, submission| submission.survey_id != req.survey_id);
    }
    Json(serde_json::json!({
        "success": removed,
        "message": if removed { "Survey and its submissions deleted" } else { "Survey not found" }
    }))
}

async fn handle_admin_survey_submissions(State(state): State<SharedState>) -> impl IntoResponse {
    let submissions = state.survey_submissions.lock().await;
    let mut list: Vec<SurveySubmission> = submissions.values().cloned().collect();
    list.sort_by(|left, right| right.submitted_at.cmp(&left.submitted_at));
    Json(serde_json::json!({ "submissions": list }))
}
