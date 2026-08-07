use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use lazy_static::lazy_static;
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::Mutex as AsyncMutex;
use rustls::pki_types::ServerName;
use tokio_rustls::client::TlsStream;
use tokio_rustls::TlsConnector;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::header::{ORIGIN, USER_AGENT};
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{client_async, WebSocketStream};

use crate::vrc_api::VrcState;

const DEFAULT_PIPELINE_URL: &str = "wss://pipeline.vrchat.cloud";
const VRC_USER_AGENT: &str = concat!("VRCDog/", env!("CARGO_PKG_VERSION"));

const MAX_RECONNECT_ATTEMPTS: u32 = 5;
const RECONNECT_BASE_DELAY_MS: u64 = 2000;
const RECONNECT_MAX_DELAY_MS: u64 = 60000;
const HANDSHAKE_TIMEOUT_MS: u64 = 15000;
const PING_INTERVAL_SECS: u64 = 30;

/// Parse a user-supplied pipeline WebSocket URL into (host, port).
/// Accepts `ws://` or `wss://` schemes; defaults port to 443 when omitted.
fn parse_pipeline_url(url: Option<&str>) -> Result<(String, u16), String> {
    let raw = url
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(DEFAULT_PIPELINE_URL);

    let scheme_sep = raw
        .find("://")
        .ok_or_else(|| format!("URL 必须以 ws:// 或 wss:// 开头：{raw}"))?;
    let scheme = &raw[..scheme_sep];
    if scheme != "ws" && scheme != "wss" {
        return Err(format!("URL scheme 必须是 ws 或 wss：{raw}"));
    }

    let rest = &raw[scheme_sep + 3..];
    let host_port = rest
        .split('/')
        .next()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| format!("URL 缺少 host：{raw}"))?;
    if host_port.contains(' ') || host_port.contains('\n') {
        return Err(format!("URL host 非法：{raw}"));
    }

    let (host, port) = match host_port.rsplit_once(':') {
        Some((h, p)) => {
            let port = p
                .parse::<u16>()
                .map_err(|_| format!("URL 端口非法：{raw}"))?;
            (h.to_string(), port)
        }
        None => (host_port.to_string(), 443u16),
    };

    if host.is_empty() {
        return Err(format!("URL host 为空：{raw}"));
    }

    Ok((host, port))
}

/// Mirrors the frontend `wsState` shape so the UI can render pipeline status.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineStatus {
    pub phase: String,
    pub connected: bool,
    pub message_count: u64,
    pub last_error: String,
    pub reconnect_attempts: u32,
}

struct PipelineRun {
    stop: Arc<AtomicBool>,
    handle: tokio::task::JoinHandle<()>,
}

lazy_static! {
    static ref RUN: AsyncMutex<Option<PipelineRun>> = AsyncMutex::new(None);
    static ref MESSAGE_COUNT: AtomicU64 = AtomicU64::new(0);
}

#[tauri::command]
pub async fn start_pipeline_ws(
    app: AppHandle,
    state: State<'_, VrcState>,
    auth_token: String,
    pipeline_url: Option<String>,
) -> Result<(), String> {
    // Stop any previous run before starting a new one (idempotent on re-login).
    stop_internal().await;

    let (host, port) = parse_pipeline_url(pipeline_url.as_deref())?;
    let proxy_url = state.proxy_url.read().await.clone();
    let stop = Arc::new(AtomicBool::new(false));

    let app_for_task = app.clone();
    let token = auth_token;
    let stop_for_task = stop.clone();

    let handle = tokio::spawn(async move {
        run_pipeline(app_for_task, token, host, port, proxy_url, stop_for_task).await;
    });

    let mut run = RUN.lock().await;
    *run = Some(PipelineRun { stop, handle });
    Ok(())
}

#[tauri::command]
pub async fn stop_pipeline_ws() -> Result<(), String> {
    stop_internal().await;
    Ok(())
}

async fn stop_internal() {
    let mut run = RUN.lock().await;
    if let Some(r) = run.take() {
        r.stop.store(true, Ordering::SeqCst);
        r.handle.abort();
    }
}

fn status_payload(phase: &str, connected: bool, last_error: &str, attempts: u32) -> PipelineStatus {
    PipelineStatus {
        phase: phase.to_string(),
        connected,
        message_count: MESSAGE_COUNT.load(Ordering::SeqCst),
        last_error: last_error.to_string(),
        reconnect_attempts: attempts,
    }
}

fn emit_status(app: &AppHandle, phase: &str, connected: bool, last_error: &str, attempts: u32) {
    let _ = app.emit("pipeline_ws_status", status_payload(phase, connected, last_error, attempts));
}

async fn run_pipeline(
    app: AppHandle,
    token: String,
    host: String,
    port: u16,
    proxy: Option<String>,
    stop: Arc<AtomicBool>,
) {
    // Make sure a rustls crypto provider is installed (ring is enabled via Cargo features).
    let _ = rustls::crypto::CryptoProvider::install_default(rustls::crypto::ring::default_provider());

    MESSAGE_COUNT.store(0, Ordering::SeqCst);
    let mut attempts: u32 = 0;

    loop {
        if stop.load(Ordering::SeqCst) {
            return;
        }

        emit_status(&app, "authenticating", false, "", attempts);

        match connect(host.clone(), port, &token, &proxy).await {
            Ok(ws) => {
                attempts = 0;
                emit_status(&app, "connected", true, "", 0);

                // Read messages until the connection drops (or we are asked to stop).
                let result = pump_messages(&app, ws, &stop).await;
                if stop.load(Ordering::SeqCst) {
                    return;
                }
                let err = result
                    .err()
                    .map(|e| e.to_string())
                    .unwrap_or_else(|| "连接已断开".to_string());
                emit_status(&app, "waiting", false, &err, attempts);
            }
            Err(e) => {
                emit_status(&app, "waiting", false, &e.to_string(), attempts);
            }
        }

        if stop.load(Ordering::SeqCst) {
            return;
        }

        attempts += 1;
        if attempts >= MAX_RECONNECT_ATTEMPTS {
            emit_status(
                &app,
                "failed",
                false,
                "无法连接 VRChat 实时推送管道（请检查网络或代理设置）",
                attempts,
            );
            return;
        }

        let delay = reconnect_delay(attempts);
        emit_status(&app, "waiting", false, "", attempts);

        // Wait for `delay`, bailing out early if we are asked to stop.
        let started = tokio::time::Instant::now();
        while started.elapsed() < Duration::from_millis(delay) {
            if stop.load(Ordering::SeqCst) {
                return;
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    }
}

fn reconnect_delay(attempts: u32) -> u64 {
    let exponent = (attempts.saturating_sub(1)) as f64;
    let base = (RECONNECT_BASE_DELAY_MS as f64 * 1.5f64.powf(exponent))
        .min(RECONNECT_MAX_DELAY_MS as f64);
    let jitter = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_millis() as f64 / 1000.0)
        .unwrap_or(0.0))
        * 0.25
        * base;
    (base + jitter) as u64
}

/// Establish the WebSocket connection, optionally tunnelling through an HTTP proxy
/// so the connection honours the app's proxy setting (the WebView WebSocket does not).
async fn connect(
    host: String,
    port: u16,
    token: &str,
    proxy: &Option<String>,
) -> Result<WebSocketStream<TlsStream<TcpStream>>, Box<dyn std::error::Error + Send + Sync + 'static>> {
    let tcp = match proxy {
        Some(p) if !p.trim().is_empty() => tunnel_tcp(p, &host, port).await?,
        _ => TcpStream::connect((host.as_str(), port)).await?,
    };

    // TLS handshake over the (proxied or direct) TCP stream.
    let mut root_store = rustls::RootCertStore::empty();
    root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let config = rustls::ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();
    let connector = TlsConnector::from(Arc::new(config));
    // `host` must be owned `String` so we get the `TryFrom<String>` impl that
    // yields `ServerName<'static>`; passing `&str` would require `'static` and
    // escape the function body (E0521). Clone so we can reuse `host` below.
    let server_name = ServerName::try_from(host.clone())?;
    let tls = connector.connect(server_name, tcp).await?;

    // WebSocket handshake.
    let url = format!(
        "wss://{}:{}/?authToken={}",
        host,
        port,
        urlencoding::encode(token)
    );
    let mut request = url.into_client_request()?;
    request
        .headers_mut()
        .insert(USER_AGENT, HeaderValue::from_static(VRC_USER_AGENT));
    request
        .headers_mut()
        .insert(ORIGIN, HeaderValue::from_static("https://vrchat.com"));

    let (ws, _resp) = tokio::time::timeout(
        Duration::from_millis(HANDSHAKE_TIMEOUT_MS),
        client_async(request, tls),
    )
    .await
    .map_err(|_| "握手超时")??;

    Ok(ws)
}

/// Open a TCP tunnel through an HTTP CONNECT proxy (the proxy resolves the
/// target host, which also avoids local DNS poisoning).
async fn tunnel_tcp(
    proxy_url: &str,
    target_host: &str,
    target_port: u16,
) -> Result<TcpStream, Box<dyn std::error::Error + Send + Sync + 'static>> {
    let trimmed = proxy_url.trim();
    let without_scheme = trimmed
        .trim_start_matches("http://")
        .trim_start_matches("https://")
        .trim_start_matches("socks5://");
    let (host, port) = match without_scheme.rsplit_once(':') {
        Some((h, p)) => (h, p.parse::<u16>().unwrap_or(1080)),
        None => (without_scheme, 1080),
    };

    let mut stream = TcpStream::connect((host, port)).await?;

    let connect_req = format!(
        "CONNECT {}:{} HTTP/1.1\r\nHost: {}:{}\r\nProxy-Connection: Keep-Alive\r\n\r\n",
        target_host, target_port, target_host, target_port
    );
    stream.write_all(connect_req.as_bytes()).await?;

    // Read the proxy response status line + headers until the blank line.
    let mut buf: Vec<u8> = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        let n = stream.read(&mut byte).await?;
        if n == 0 {
            break;
        }
        buf.push(byte[0]);
        if buf.len() >= 4 && &buf[buf.len() - 4..] == b"\r\n\r\n" {
            break;
        }
        if buf.len() > 4096 {
            break;
        }
    }

    let response = String::from_utf8_lossy(&buf);
    if !response.starts_with("HTTP/") || !response.contains("200") {
        return Err(format!(
            "代理 CONNECT 失败: {}",
            response.lines().next().unwrap_or("")
        )
        .into());
    }

    Ok(stream)
}

async fn pump_messages(
    app: &AppHandle,
    ws_stream: WebSocketStream<TlsStream<TcpStream>>,
    stop: &Arc<AtomicBool>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let (mut write, mut read) = ws_stream.split();
    let mut ping_interval = tokio::time::interval(Duration::from_secs(PING_INTERVAL_SECS));
    // Consume the immediate first tick so the first real ping fires after the interval.
    ping_interval.tick().await;

    loop {
        tokio::select! {
            _ = ping_interval.tick() => {
                if write.send(Message::Ping(Vec::new())).await.is_err() {
                    break;
                }
            }
            msg = read.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        MESSAGE_COUNT.fetch_add(1, Ordering::SeqCst);
                        let _ = app.emit("pipeline_ws_message", text.to_string());
                        emit_status(app, "connected", true, "", 0);
                    }
                    Some(Ok(Message::Ping(_))) => {
                        if write.send(Message::Pong(Vec::new())).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Message::Close(_))) => break,
                    Some(Ok(_)) => {}
                    Some(Err(e)) => return Err(Box::new(e)),
                    None => break,
                }
            }
        }

        if stop.load(Ordering::SeqCst) {
            break;
        }
    }

    Ok(())
}
