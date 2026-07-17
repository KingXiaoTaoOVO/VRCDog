use axum::{
    extract::{ws::WebSocketUpgrade, ConnectInfo, Path, State},
    http::StatusCode,
    middleware,
    routing::{get, post},
    Json, Router,
};
use chrono::{Duration, Local, NaiveDateTime};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    env,
    net::SocketAddr,
    path::{Path as FsPath, PathBuf},
    sync::Arc,
};
use tokio::sync::RwLock;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::{info, warn};

mod remote_assist_hub;

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
    roles: HashMap<String, Role>,
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
struct AdminAuthRequest {
    password: String,
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

fn ban_is_expired(ban: &BanInfo) -> bool {
    let Some(hours) = ban.duration_hours else {
        return false;
    };
    let Ok(banned_at) = NaiveDateTime::parse_from_str(&ban.banned_at, "%Y-%m-%d %H:%M:%S")
    else {
        return false;
    };
    Local::now().naive_local() > banned_at + Duration::seconds((hours * 3600.0) as i64)
}

async fn persist(state: &AppState) -> Result<(), String> {
    let snapshot = state.data.read().await.clone();
    let bytes = serde_json::to_vec_pretty(&snapshot).map_err(|error| error.to_string())?;
    if let Some(parent) = state.data_file.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|error| error.to_string())?;
    }
    let temp_file = state.data_file.with_extension("json.tmp");
    tokio::fs::write(&temp_file, bytes)
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
            });
    }
    state.clients.write().await.insert(
        request.user_id.clone(),
        ClientInfo {
            user_id: request.user_id.clone(),
            display_name: request.display_name.clone(),
            avatar_url: request.avatar_url,
            ip_address: address.to_string(),
            connected_at: now.clone(),
            last_heartbeat: now,
        },
    );
    let _ = persist(&state).await;
    info!(user_id = %request.user_id, ip = %address, "client registered");
    Json(json!({ "status": "ok", "message": "registered" }))
}

async fn heartbeat(
    State(state): State<AppState>,
    Json(request): Json<UserIdRequest>,
) -> Json<Value> {
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
    let mut clients = state.clients.write().await;
    let Some(client) = clients.get_mut(&request.user_id) else {
        return Json(json!({ "status": "register_required" }));
    };
    client.last_heartbeat = now_string();
    Json(json!({ "status": "ok" }))
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
    let _ = persist(&state).await;
    Json(json!({ "status": "ok" }))
}

async fn check_status(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
) -> Json<Value> {
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

async fn get_features(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
) -> Json<Value> {
    let data = state.data.read().await;
    let default_role = data.roles.values().find(|role| role.is_default);
    let user_role = data
        .users
        .get(&user_id)
        .and_then(|user| user.role_id.as_ref())
        .and_then(|role_id| data.roles.get(role_id));
    let features = user_role
        .or(default_role)
        .map(|role| role.features.clone())
        .unwrap_or_default();
    Json(json!({
        "menus": features.menus,
        "themes": features.themes,
        "modes": features.modes
    }))
}

async fn admin_auth(Json(request): Json<AdminAuthRequest>) -> impl axum::response::IntoResponse {
    if verify_server_password(&request.password) {
        (StatusCode::OK, Json(json!({ "success": true })))
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
    let _ = persist(&state).await;
    Json(json!({
        "success": existed,
        "message": if existed { "Client kicked" } else { "Client is not online" }
    }))
}

async fn admin_ban(
    State(state): State<AppState>,
    Json(request): Json<BanRequest>,
) -> Json<Value> {
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
    let _ = persist(&state).await;
    Json(json!({ "success": true, "message": "User banned" }))
}

async fn admin_unban(
    State(state): State<AppState>,
    Json(request): Json<UserIdRequest>,
) -> Json<Value> {
    let removed = state.data.write().await.bans.remove(&request.user_id).is_some();
    let _ = persist(&state).await;
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
    let _ = persist(&state).await;
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
    let _ = persist(&state).await;
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
    drop(data);
    let _ = persist(&state).await;
    Json(json!({ "success": true, "message": "User record removed" }))
}

async fn admin_roles(State(state): State<AppState>) -> Json<Value> {
    let mut roles: Vec<Role> = state.data.read().await.roles.values().cloned().collect();
    roles.sort_by(|left, right| left.role_name.cmp(&right.role_name));
    Json(json!({ "roles": roles }))
}

async fn admin_save_role(
    State(state): State<AppState>,
    Json(mut role): Json<Role>,
) -> Json<Value> {
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
    let _ = persist(&state).await;
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
    let _ = persist(&state).await;
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
    let _ = persist(&state).await;
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
    let _ = persist(&state).await;
    Json(json!({ "success": true, "message": "User role updated" }))
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
        .route(
            "/api/admin/roles/set_default",
            post(admin_set_default_role),
        )
        .route("/api/admin/users/set_role", post(admin_set_user_role))
        .route_layer(middleware::from_fn(require_admin_password));

    Router::new()
        .route("/ping", get(ping))
        .route("/api/admin/auth", post(admin_auth))
        .route("/api/client/register", post(register))
        .route("/api/client/heartbeat", post(heartbeat))
        .route("/api/client/disconnect", post(disconnect))
        .route("/api/remote-assist/ws", get(remote_assist_ws))
        .route("/api/client/check-status/{user_id}", get(check_status))
        .route("/api/client/features/{user_id}", get(get_features))
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
        let _ = persist(&state).await;
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
    let state = AppState {
        clients: Arc::new(RwLock::new(HashMap::new())),
        data: Arc::new(RwLock::new(data)),
        data_file: Arc::new(data_file),
        remote_assist: remote_assist_hub::RemoteAssistHub::default(),
    };
    persist(&state).await?;
    tokio::spawn(cleanup_stale_clients(state.clone()));

    let address: SocketAddr = format!("{host}:{port}").parse()?;
    let listener = tokio::net::TcpListener::bind(address).await?;
    info!(%address, "VRCDog standalone server started");
    axum::serve(
        listener,
        router(state).into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async {
        let _ = tokio::signal::ctrl_c().await;
        info!("shutdown signal received");
    })
    .await?;
    Ok(())
}
