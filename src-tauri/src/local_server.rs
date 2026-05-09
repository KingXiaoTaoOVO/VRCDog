use axum::{
    extract::{ConnectInfo, Path, State},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use tower_http::cors::{Any, CorsLayer};

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
    pub duration_hours: Option<f64>,  // None = permanent
    pub expires_at: Option<String>,   // calculated
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
    pub menus: HashMap<String, bool>,   // key -> enabled
    #[serde(default)]
    pub themes: HashMap<String, bool>,  // key -> enabled
    #[serde(default)]
    pub modes: HashMap<String, bool>,   // key -> enabled (pc, vr)
}

impl Default for FeatureConfig {
    fn default() -> Self {
        let mut menus = HashMap::new();
        let menu_keys = [
            "dashboard", "feed", "friendlog", "locations", "charts",
            "playerlist", "gallery", "social", "search", "notifications",
            "groups", "avatars", "favorites", "moderation", "heatmap",
            "gamelog", "notes", "presets", "tools", "translator",
            "ovr", "env", "export", "settings",
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

        FeatureConfig { menus, themes, modes }
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
    pub shutdown: CancellationToken,
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
    status: String,       // "ok" | "banned" | "frozen"
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
    duration_hours: Option<f64>,  // None = permanent
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

fn is_ban_expired(ban: &BanInfo) -> bool {
    if let Some(hours) = ban.duration_hours {
        if let Ok(banned_at) = chrono::NaiveDateTime::parse_from_str(&ban.banned_at, "%Y-%m-%d %H:%M:%S") {
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
    initial_roles.insert("default".to_string(), Role {
        role_id: "default".to_string(),
        role_name: "默认用户".to_string(),
        is_default: true,
        features: FeatureConfig::default(),
    });

    let state = SharedState {
        app_handle: app_handle.clone(),
        clients: Arc::new(Mutex::new(HashMap::new())),
        users: Arc::new(Mutex::new(HashMap::new())),
        bans: Arc::new(Mutex::new(HashMap::new())),
        frozen: Arc::new(Mutex::new(HashMap::new())),
        roles: Arc::new(Mutex::new(initial_roles)),
        shutdown: shutdown.clone(),
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any)
        .allow_private_network(true);

    let app = Router::new()
        // Public endpoints (clients use these)
        .route("/ping", get(handle_ping))
        .route("/api/client/register", post(handle_client_register))
        .route("/api/client/heartbeat", post(handle_client_heartbeat))
        .route("/api/client/disconnect", post(handle_client_disconnect))
        .route("/api/client/check-status/{user_id}", get(handle_check_status))
        .route("/api/client/features/{user_id}", get(handle_get_features_public))
        // Admin endpoints (dashboard uses these)
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
        .route("/api/admin/roles/set_default", post(handle_admin_set_default_role))
        .route("/api/admin/users/set_role", post(handle_admin_set_user_role))
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

    let _ = app_handle.emit("server_log", format!("[INFO] 服务端已成功启动，正在监听 {}", addr));

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
                    if let Ok(hb) = chrono::NaiveDateTime::parse_from_str(&info.last_heartbeat, "%Y-%m-%d %H:%M:%S") {
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
        let serve = axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>());
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
                    format!("[WARN] 被封禁用户 {} ({}) 尝试注册，已拒绝", req.display_name, req.user_id),
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
                format!("[WARN] 被冻结用户 {} ({}) 尝试注册，已拒绝", req.display_name, req.user_id),
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
        clients.insert(req.user_id.clone(), ClientInfo {
            user_id: req.user_id.clone(),
            display_name: req.display_name.clone(),
            avatar_url: req.avatar_url.clone(),
            ip_address: client_ip.0.to_string(),
            connected_at: now.clone(),
            last_heartbeat: now.clone(),
        });
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
            users.insert(req.user_id.clone(), UserRecord {
                user_id: req.user_id.clone(),
                display_name: req.display_name.clone(),
                avatar_url: req.avatar_url.clone(),
                first_seen: now.clone(),
                last_seen: now.clone(),
                login_count: 1,
                is_online: true,
                role_id: None,
            });
        }
    }

    // Emit to dashboard
    let _ = state.app_handle.emit(
        "server_log",
        format!("[INFO] 客户端注册成功: {} ({}) IP: {}", req.display_name, req.user_id, client_ip.0),
    );
    let _ = state.app_handle.emit("clients_updated", "");

    Json(serde_json::json!({ "status": "ok", "message": "注册成功" }))
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
            return Json(serde_json::json!({ "status": "kicked" }));
        }
    }

    Json(serde_json::json!({ "status": "ok" }))
}

async fn handle_client_disconnect(
    State(state): State<SharedState>,
    Json(req): Json<UserIdRequest>,
) -> impl IntoResponse {
    let is_banned = state.bans.lock().await.contains_key(&req.user_id);
    let is_frozen = state.frozen.lock().await.contains_key(&req.user_id);
    {
        let mut clients = state.clients.lock().await;
        clients.remove(&req.user_id);
    }
    {
        let mut users = state.users.lock().await;
        if !is_banned && !is_frozen {
            users.remove(&req.user_id);
        } else if let Some(u) = users.get_mut(&req.user_id) {
            u.is_online = false;
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

// ===== Handlers: Admin =====

async fn handle_admin_clients(
    State(state): State<SharedState>,
) -> impl IntoResponse {
    let clients = state.clients.lock().await;
    Json(ClientListResponse {
        clients: clients.values().cloned().collect(),
    })
}

async fn handle_admin_users(
    State(state): State<SharedState>,
) -> impl IntoResponse {
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
        let _ = state.app_handle.emit("client_kicked", req.user_id.clone());
        Json(AdminResponse { success: true, message: "已踢出".to_string() })
    } else {
        Json(AdminResponse { success: false, message: "用户不在线".to_string() })
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
        format!("[WARN] 管理员封禁用户 {}, 原因: {}, 时长: {}", req.user_id, req.reason, dur_str),
    );
    let _ = state.app_handle.emit("clients_updated", "");
    let _ = state.app_handle.emit("client_banned", serde_json::json!({
        "user_id": req.user_id.clone(),
        "reason": req.reason.clone(),
        "duration_hours": req.duration_hours,
    }));
    
    Json(AdminResponse { success: true, message: format!("已封禁 ({})", dur_str) })
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
        Json(AdminResponse { success: true, message: "已解封".to_string() })
    } else {
        Json(AdminResponse { success: false, message: "用户未被封禁".to_string() })
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
        format!("[WARN] 管理员冻结用户 {}, 原因: {}", req.user_id, req.reason),
    );
    let _ = state.app_handle.emit("clients_updated", "");
    let _ = state.app_handle.emit("client_frozen", serde_json::json!({
        "user_id": req.user_id.clone(),
        "reason": req.reason.clone(),
    }));
    
    Json(AdminResponse { success: true, message: "已冻结".to_string() })
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
        Json(AdminResponse { success: true, message: "已解冻".to_string() })
    } else {
        Json(AdminResponse { success: false, message: "用户未被冻结".to_string() })
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
        format!("[WARN] 管理员移除用户记录: {} (用户重新登录后将自动重新注册)", req.user_id),
    );
    let _ = state.app_handle.emit("clients_updated", "");
    let _ = state.app_handle.emit("client_kicked", req.user_id.clone()); // remove acts like kick for active client
    
    Json(AdminResponse { success: true, message: "已移除用户记录".to_string() })
}

async fn handle_admin_get_roles(
    State(state): State<SharedState>,
) -> impl IntoResponse {
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
    let _ = state.app_handle.emit("server_log", "[INFO] 管理员保存了角色配置".to_string());
    Json(AdminResponse { success: true, message: "角色配置已保存".to_string() })
}

async fn handle_admin_delete_role(
    State(state): State<SharedState>,
    Json(req): Json<RoleIdRequest>,
) -> impl IntoResponse {
    let mut roles = state.roles.lock().await;
    if let Some(role) = roles.get(&req.role_id) {
        if role.is_default {
            return Json(AdminResponse { success: false, message: "默认角色无法删除".to_string() });
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
        let _ = state.app_handle.emit("server_log", "[INFO] 管理员删除了角色".to_string());
        Json(AdminResponse { success: true, message: "角色已删除".to_string() })
    } else {
        Json(AdminResponse { success: false, message: "角色不存在".to_string() })
    }
}

async fn handle_admin_set_default_role(
    State(state): State<SharedState>,
    Json(req): Json<RoleIdRequest>,
) -> impl IntoResponse {
    let mut roles = state.roles.lock().await;
    if !roles.contains_key(&req.role_id) {
        return Json(AdminResponse { success: false, message: "角色不存在".to_string() });
    }
    for r in roles.values_mut() {
        r.is_default = r.role_id == req.role_id;
    }
    let _ = state.app_handle.emit("server_log", "[INFO] 管理员设置了新的默认角色".to_string());
    Json(AdminResponse { success: true, message: "默认角色已更新".to_string() })
}

async fn handle_admin_set_user_role(
    State(state): State<SharedState>,
    Json(req): Json<SetUserRoleRequest>,
) -> impl IntoResponse {
    let mut users = state.users.lock().await;
    if let Some(u) = users.get_mut(&req.user_id) {
        u.role_id = req.role_id;
        let _ = state.app_handle.emit("server_log", format!("[INFO] 管理员更新了用户角色: {}", req.user_id));
        Json(AdminResponse { success: true, message: "用户角色已更新".to_string() })
    } else {
        Json(AdminResponse { success: false, message: "用户不存在".to_string() })
    }
}
