use base64::Engine;
use reqwest::{header, Client};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tauri::State;
use tokio::sync::RwLock;

const VRC_USER_AGENT: &str = concat!("VRCDog/", env!("CARGO_PKG_VERSION"));

pub struct VrcState {
    pub client: RwLock<Client>,
    pub proxy_url: RwLock<Option<String>>,
    pub cookie_jar: RwLock<Arc<reqwest::cookie::Jar>>,
}

impl Default for VrcState {
    fn default() -> Self {
        Self::new()
    }
}

impl VrcState {
    pub fn new() -> Self {
        let jar = Arc::new(reqwest::cookie::Jar::default());
        let client = build_vrc_client(None, None, jar.clone()).unwrap_or_else(|error| {
            eprintln!("[VrcApi] {error}");
            Client::new()
        });
        Self {
            client: RwLock::new(client),
            proxy_url: RwLock::new(None),
            cookie_jar: RwLock::new(jar),
        }
    }
}

fn build_vrc_client(
    proxy_url: Option<String>,
    auth_cookie: Option<String>,
    jar: Arc<reqwest::cookie::Jar>,
) -> Result<Client, String> {
    let mut headers = header::HeaderMap::new();
    headers.insert(
        header::USER_AGENT,
        header::HeaderValue::from_static(VRC_USER_AGENT),
    );
    headers.insert(
        header::REFERER,
        header::HeaderValue::from_static("https://vrchat.com/"),
    );

    if let Some(cv) = auth_cookie {
        if let Ok(url) = "https://api.vrchat.cloud".parse::<reqwest::Url>() {
            for cookie in parse_auth_cookies(&cv) {
                jar.add_cookie_str(&cookie, &url);
            }
        }
    }

    let mut builder = Client::builder()
        .cookie_provider(jar)
        .default_headers(headers)
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(15))
        .tcp_keepalive(Duration::from_secs(30))
        .pool_max_idle_per_host(10);

    if let Some(url) = proxy_url {
        if !url.is_empty() {
            let proxy =
                reqwest::Proxy::all(&url).map_err(|error| format!("Invalid proxy URL: {error}"))?;
            builder = builder.proxy(proxy);
        }
    }

    builder
        .build()
        .map_err(|error| format!("Unable to create HTTP client: {error}"))
}

#[tauri::command]
pub async fn vrc_set_proxy(
    state: tauri::State<'_, VrcState>,
    proxy_url: Option<String>,
    auth_cookie: Option<String>,
) -> Result<(), String> {
    let jar = Arc::new(reqwest::cookie::Jar::default());
    let next_client = build_vrc_client(proxy_url.clone(), auth_cookie, jar.clone())?;

    let mut proxy_lock = state.proxy_url.write().await;
    *proxy_lock = proxy_url.clone();

    let mut jar_lock = state.cookie_jar.write().await;
    *jar_lock = jar.clone();

    let mut client = state.client.write().await;
    *client = next_client;
    Ok(())
}

#[tauri::command]
pub async fn vrc_clear_cookies(state: tauri::State<'_, VrcState>) -> Result<(), String> {
    let proxy_url = state.proxy_url.read().await.clone();

    let jar = Arc::new(reqwest::cookie::Jar::default());
    let next_client = build_vrc_client(proxy_url, None, jar.clone())?;
    let mut jar_lock = state.cookie_jar.write().await;
    *jar_lock = jar.clone();

    let mut client = state.client.write().await;
    *client = next_client;
    Ok(())
}

#[tauri::command]
pub async fn vrc_apply_auth_cookie(
    state: tauri::State<'_, VrcState>,
    auth_cookie: String,
) -> Result<(), String> {
    let jar = state.cookie_jar.read().await.clone();
    let url = "https://api.vrchat.cloud"
        .parse::<reqwest::Url>()
        .map_err(|e| e.to_string())?;
    for cookie in parse_auth_cookies(&auth_cookie) {
        jar.add_cookie_str(&cookie, &url);
    }
    Ok(())
}

/// Load saved auth cookies into the shared cookie jar on app startup.
/// This ensures the session persists across app restarts without requiring
/// the cookie to be passed on every single request.
#[tauri::command]
pub async fn vrc_load_cookies_on_startup(
    state: tauri::State<'_, VrcState>,
    auth_cookie: String,
) -> Result<(), String> {
    if auth_cookie.is_empty() {
        return Ok(());
    }
    let jar = state.cookie_jar.read().await.clone();
    let url = "https://api.vrchat.cloud"
        .parse::<reqwest::Url>()
        .map_err(|e| e.to_string())?;
    let cookies = parse_auth_cookies(&auth_cookie);
    for cookie in &cookies {
        jar.add_cookie_str(cookie, &url);
    }
    eprintln!(
        "[VrcApi] Loaded {} cookies into jar on startup",
        cookies.len()
    );
    Ok(())
}

fn extract_auth_cookie(res: &reqwest::Response) -> Option<String> {
    let mut cookies = Vec::new();
    for val in res.headers().get_all(header::SET_COOKIE).iter() {
        if let Ok(s) = val.to_str() {
            let end = s.find(';').unwrap_or(s.len());
            cookies.push(s[0..end].to_string());
        }
    }

    if cookies.is_empty() {
        None
    } else {
        Some(serde_json::to_string(&cookies).unwrap_or_default())
    }
}

fn clean_cookie_segment(segment: &str) -> Option<String> {
    let mut part = segment.trim();
    let lower = part.to_ascii_lowercase();

    if lower.starts_with("set-cookie:") {
        part = part["set-cookie:".len()..].trim();
    } else if lower.starts_with("cookie:") {
        part = part["cookie:".len()..].trim();
    }

    let (name, value) = part.split_once('=')?;
    let name = name.trim();
    let value = value.trim();
    if name.is_empty() || value.is_empty() {
        return None;
    }

    match name.to_ascii_lowercase().as_str() {
        "domain" | "expires" | "httponly" | "max-age" | "path" | "samesite" | "secure" => None,
        _ => Some(format!("{}={}", name, value)),
    }
}

fn parse_auth_cookies(raw_cookie: &str) -> Vec<String> {
    let raw = raw_cookie.trim();
    if raw.is_empty() {
        return Vec::new();
    }

    if raw.starts_with('[') {
        if let Ok(cookies) = serde_json::from_str::<Vec<String>>(raw) {
            return cookies
                .iter()
                .flat_map(|cookie| parse_auth_cookies(cookie))
                .collect();
        }
    }

    let cookies: Vec<String> = raw.split(';').filter_map(clean_cookie_segment).collect();
    if !cookies.is_empty() {
        return cookies;
    }

    if raw.contains('=') {
        Vec::new()
    } else {
        vec![format!("auth={}", raw)]
    }
}

fn is_transient_http_status(status: u16) -> bool {
    matches!(status, 408 | 425 | 429 | 500 | 502 | 503 | 504 | 520..=524)
}

fn clean_network_error(error: &str) -> String {
    if error.contains("error sending request") || error.contains("network") {
        "Network changed or temporarily unavailable".to_string()
    } else {
        error.to_string()
    }
}

fn parse_http_url(raw: &str, allow_external_host: bool) -> Result<reqwest::Url, String> {
    let url = reqwest::Url::parse(raw).map_err(|error| format!("Invalid request URL: {error}"))?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(format!("Unsupported request URL scheme: {}", url.scheme()));
    }
    match url.host_str() {
        Some("api.vrchat.cloud") => Ok(url),
        Some(host) if allow_external_host && !host.is_empty() => Ok(url),
        _ => Err(format!(
            "Unsupported request URL host: {}",
            url.host_str().unwrap_or("unknown")
        )),
    }
}

fn parse_http_method(raw: &str) -> Result<reqwest::Method, String> {
    reqwest::Method::from_bytes(raw.trim().to_uppercase().as_bytes())
        .map_err(|error| format!("Invalid HTTP method: {error}"))
}

fn is_vrchat_image_host(host: &str) -> bool {
    host == "api.vrchat.cloud" || host == "vrchat.com" || host.ends_with(".vrchat.cloud")
}

fn parse_image_url(raw: &str) -> Result<reqwest::Url, String> {
    let url = reqwest::Url::parse(raw).map_err(|error| format!("Invalid image URL: {error}"))?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(format!("Unsupported image URL scheme: {}", url.scheme()));
    }
    match url.host_str() {
        Some(host) if is_vrchat_image_host(host) => Ok(url),
        _ => Err(format!(
            "Image URL host not allowed: {}",
            url.host_str().unwrap_or("unknown")
        )),
    }
}

fn request_error_message(error: reqwest::Error) -> String {
    if error.is_timeout() {
        format!("REQUEST_TIMEOUT: {error}")
    } else if error.is_connect() {
        format!("REQUEST_CONNECT: {error}")
    } else if error.is_body() || error.is_decode() {
        format!("RESPONSE_BODY: {error}")
    } else {
        format!("REQUEST_NETWORK: {error}")
    }
}

fn vrchat_error_message(data: &str) -> Option<String> {
    let json = serde_json::from_str::<serde_json::Value>(data).ok()?;
    json.pointer("/error/message")
        .and_then(|v| v.as_str())
        .or_else(|| json.get("message").and_then(|v| v.as_str()))
        .map(|s| s.to_string())
}

fn is_vrchat_permission_401(url: &str, data: &str) -> bool {
    let msg = vrchat_error_message(data)
        .unwrap_or_default()
        .to_lowercase();
    if url.contains("/avatars") && url.contains("userId=") && msg.contains("own avatars") {
        return true;
    }
    msg.contains("you can only")
        || msg.contains("permission")
        || msg.contains("forbidden")
        || msg.contains("private")
}

#[cfg(test)]
mod cookie_tests {
    use super::{parse_auth_cookies, parse_http_method, parse_http_url};

    #[test]
    fn parses_json_cookie_array() {
        let cookies = parse_auth_cookies(r#"["auth=a1","twoFactorAuth=t1"]"#);
        assert_eq!(cookies, vec!["auth=a1", "twoFactorAuth=t1"]);
    }

    #[test]
    fn parses_cookie_header_and_drops_attributes() {
        let cookies = parse_auth_cookies(
            "auth=a1; Path=/; HttpOnly; twoFactorAuth=t1; SameSite=None; Secure",
        );
        assert_eq!(cookies, vec!["auth=a1", "twoFactorAuth=t1"]);
    }

    #[test]
    fn wraps_bare_auth_token() {
        let cookies = parse_auth_cookies("abc123");
        assert_eq!(cookies, vec!["auth=abc123"]);
    }

    #[test]
    fn validates_http_urls_and_methods() {
        assert!(parse_http_url("https://api.vrchat.cloud/api/1/config", false).is_ok());
        assert!(parse_http_url("https://vrcdog.pcb.im/api/client/register", true).is_ok());
        assert!(parse_http_url("https://example.com/api", false).is_err());
        assert!(parse_http_url("file:///tmp/secret", true).is_err());
        assert_eq!(parse_http_method("post").unwrap(), reqwest::Method::POST);
        assert!(parse_http_method("not a method").is_err());
    }
}

#[tauri::command]
pub async fn vrc_get_image_bytes(
    state: tauri::State<'_, VrcState>,
    url: String,
    auth_cookie: Option<String>,
) -> Result<String, String> {
    if url.is_empty() {
        return Err("Empty URL".to_string());
    }
    let request_url = parse_image_url(&url)?;
    let client = state.client.read().await.clone();
    let direct_cookies = auth_cookie
        .as_deref()
        .map(parse_auth_cookies)
        .unwrap_or_default();

    let mut last_error = String::new();
    let mut response = None;
    for attempt in 0..3 {
        let mut req = client.get(request_url.clone());
        if !direct_cookies.is_empty() && request_url.host_str() == Some("api.vrchat.cloud") {
            let cookie_str = direct_cookies.join("; ");
            if let Ok(hv) = reqwest::header::HeaderValue::from_str(&cookie_str) {
                req = req.header(reqwest::header::COOKIE, hv);
            }
        }

        match req.send().await {
            Ok(res) => {
                let status = res.status().as_u16();
                if is_transient_http_status(status) && attempt < 2 {
                    last_error = format!("HTTP {}", status);
                    tokio::time::sleep(Duration::from_millis(180 * (attempt + 1) as u64)).await;
                    continue;
                }
                response = Some(res);
                break;
            }
            Err(e) => {
                last_error = clean_network_error(&e.to_string());
                if attempt < 2 {
                    tokio::time::sleep(Duration::from_millis(180 * (attempt + 1) as u64)).await;
                    continue;
                }
            }
        }
    }

    let res = response.ok_or_else(|| {
        if last_error.is_empty() {
            "Image request failed".to_string()
        } else {
            format!("Image request failed: {}", last_error)
        }
    })?;

    if !res.status().is_success() {
        return Err(format!("Unable to load image: {}", res.status()));
    }

    let content_type = res
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("image/jpeg")
        .to_string();

    let bytes = res
        .bytes()
        .await
        .map_err(|e| clean_network_error(&e.to_string()))?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);

    let mime = if content_type.is_empty() {
        "image/jpeg"
    } else {
        &content_type
    };
    Ok(format!("data:{};base64,{}", mime, b64))
}

#[derive(Deserialize)]
pub struct FormDataPart {
    pub name: String,
    pub value: Option<String>,
    pub file_name: Option<String>,
    pub file_content_base64: Option<String>,
    pub file_mime: Option<String>,
}

#[derive(Deserialize)]
pub struct RequestOptions {
    pub url: String,
    pub method: String,
    pub headers: Option<std::collections::HashMap<String, String>>,
    pub body: Option<String>,
    pub body_is_base64: Option<bool>,
    pub form_data: Option<Vec<FormDataPart>>,
    pub auth_cookie: Option<String>,
    /// Per-request timeout supplied by the UI.  Keep this bounded so a caller
    /// cannot accidentally keep a shared API connection alive indefinitely.
    pub timeout_ms: Option<u64>,
    #[serde(default)]
    pub allow_external_host: bool,
}

#[derive(Serialize)]
pub struct ResponsePayload {
    pub status: u16,
    pub data: Option<String>,
    pub auth_cookie: Option<String>,
    pub headers: std::collections::BTreeMap<String, String>,
}

#[tauri::command]
pub async fn vrc_execute(
    state: State<'_, VrcState>,
    options: RequestOptions,
) -> Result<ResponsePayload, String> {
    // Use the shared client to preserve session cookies across requests
    let client = state.client.read().await.clone();

    let method = parse_http_method(&options.method)?;
    let request_url = parse_http_url(&options.url, options.allow_external_host)?;

    let mut req = client.request(method, request_url.clone());

    if let Some(timeout_ms) = options.timeout_ms {
        req = req.timeout(Duration::from_millis(timeout_ms.clamp(1_000, 120_000)));
    }

    // Sync auth cookies into the shared jar so reqwest sends them automatically.
    // No need to attach a manual Cookie header - the jar handles it.
    if let Some(ref cv) = options.auth_cookie {
        let jar = state.cookie_jar.read().await.clone();
        if request_url.host_str() == Some("api.vrchat.cloud") {
            let url = "https://api.vrchat.cloud"
                .parse::<reqwest::Url>()
                .map_err(|error| error.to_string())?;
            let direct_cookies = parse_auth_cookies(cv);
            for cookie in &direct_cookies {
                jar.add_cookie_str(cookie, &url);
            }
        }
    }

    if let Some(headers) = options.headers {
        for (k, v) in headers {
            let h_name = reqwest::header::HeaderName::from_bytes(k.as_bytes())
                .map_err(|error| format!("Invalid request header name: {error}"))?;
            let h_value = reqwest::header::HeaderValue::from_str(&v)
                .map_err(|error| format!("Invalid request header value for {k}: {error}"))?;
            req = req.header(h_name, h_value);
        }
    }

    if let Some(form_data_parts) = options.form_data {
        let mut form = reqwest::multipart::Form::new();
        for part in form_data_parts {
            if let Some(val) = part.value {
                form = form.text(part.name, val);
            } else if let Some(b64) = part.file_content_base64 {
                if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(&b64) {
                    let mut part_req = reqwest::multipart::Part::bytes(bytes);
                    if let Some(fname) = part.file_name {
                        part_req = part_req.file_name(fname);
                    }
                    if let Some(mime) = part.file_mime {
                        part_req = part_req
                            .mime_str(&mime)
                            .map_err(|error| format!("Invalid multipart MIME type: {error}"))?;
                    }
                    form = form.part(part.name, part_req);
                } else {
                    return Err(format!(
                        "Invalid base64 content for multipart field {}",
                        part.name
                    ));
                }
            }
        }
        req = req.multipart(form);
    } else if let Some(body) = options.body {
        if options.body_is_base64.unwrap_or(false) {
            match base64::engine::general_purpose::STANDARD.decode(&body) {
                Ok(bytes) => req = req.body(bytes),
                Err(e) => return Err(format!("Base64 Decode Error: {}", e)),
            }
        } else {
            req = req.body(body);
        }
    }

    let res = req.send().await.map_err(request_error_message)?;
    let status = res.status().as_u16();
    let auth_cookie = extract_auth_cookie(&res);
    // Retry-After is required for cooperative VRChat rate-limit handling.
    // Expose a small, non-sensitive response-header subset to the frontend.
    let mut headers = std::collections::BTreeMap::new();
    if let Some(retry_after) = res.headers().get(header::RETRY_AFTER) {
        if let Ok(value) = retry_after.to_str() {
            headers.insert("retry-after".to_string(), value.to_string());
        }
    }
    let data = res.text().await.map_err(request_error_message)?;

    // Sync response cookies back into the jar so subsequent requests
    // within the same session use the refreshed tokens.
    if let Some(ref cookie_str) = auth_cookie {
        let jar = state.cookie_jar.read().await.clone();
        if let Ok(url) = "https://api.vrchat.cloud".parse::<reqwest::Url>() {
            for cookie in parse_auth_cookies(cookie_str) {
                jar.add_cookie_str(&cookie, &url);
            }
        }
    }

    if status == 401
        && request_url.host_str() == Some("api.vrchat.cloud")
        && !is_vrchat_permission_401(&options.url, &data)
    {
        eprintln!(
            "[VrcApi] Auth rejected for {} - {}",
            options.url,
            vrchat_error_message(&data).unwrap_or_else(|| "HTTP 401".to_string())
        );
    }

    Ok(ResponsePayload {
        status,
        data: Some(data),
        auth_cookie,
        headers,
    })
}
