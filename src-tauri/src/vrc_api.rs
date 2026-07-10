use base64::Engine;
use reqwest::{header, Client};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tauri::State;
use tokio::sync::RwLock;

const VRC_USER_AGENT: &str = "VRCDog/5.0.0";

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
        Self {
            client: RwLock::new(build_vrc_client(None, None, jar.clone())),
            proxy_url: RwLock::new(None),
            cookie_jar: RwLock::new(jar),
        }
    }
}

fn build_vrc_client(
    proxy_url: Option<String>,
    auth_cookie: Option<String>,
    jar: Arc<reqwest::cookie::Jar>,
) -> Client {
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
            if let Ok(proxy) = reqwest::Proxy::all(&url) {
                builder = builder.proxy(proxy);
            }
        }
    }

    builder.build().unwrap_or_else(|_| Client::new())
}

#[tauri::command]
pub async fn vrc_set_proxy(
    state: tauri::State<'_, VrcState>,
    proxy_url: Option<String>,
    auth_cookie: Option<String>,
) -> Result<(), String> {
    let mut proxy_lock = state.proxy_url.write().await;
    *proxy_lock = proxy_url.clone();

    let jar = Arc::new(reqwest::cookie::Jar::default());
    let mut jar_lock = state.cookie_jar.write().await;
    *jar_lock = jar.clone();

    let mut client = state.client.write().await;
    *client = build_vrc_client(proxy_url, auth_cookie, jar);
    Ok(())
}

#[tauri::command]
pub async fn vrc_clear_cookies(state: tauri::State<'_, VrcState>) -> Result<(), String> {
    let proxy_url = state.proxy_url.read().await.clone();

    let jar = Arc::new(reqwest::cookie::Jar::default());
    let mut jar_lock = state.cookie_jar.write().await;
    *jar_lock = jar.clone();

    let mut client = state.client.write().await;
    *client = build_vrc_client(proxy_url, None, jar);
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

#[cfg(test)]
mod cookie_tests {
    use super::parse_auth_cookies;

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
    let client = state.client.read().await.clone();

    let mut req = client.get(&url);

    // Inject cookie

    if let Some(ref cv) = auth_cookie {
        let direct_cookies = parse_auth_cookies(cv);

        if !direct_cookies.is_empty() {
            let cookie_str = direct_cookies.join("; ");
            if let Ok(hv) = reqwest::header::HeaderValue::from_str(&cookie_str) {
                req = req.header(reqwest::header::COOKIE, hv);
            }
        }
    }

    let res = req.send().await.map_err(|e| e.to_string())?;

    if !res.status().is_success() {
        return Err(format!("无法加载图片: {}", res.status()));
    }

    let content_type = res
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("image/jpeg")
        .to_string();

    let bytes = res.bytes().await.map_err(|e| e.to_string())?;
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
}

#[derive(Serialize)]
pub struct ResponsePayload {
    pub status: u16,
    pub data: Option<String>,
    pub auth_cookie: Option<String>,
}

#[tauri::command]
pub async fn vrc_execute(
    state: State<'_, VrcState>,
    options: RequestOptions,
) -> Result<ResponsePayload, String> {
    // Use the shared client to preserve session cookies across requests
    let client = state.client.read().await.clone();

    let method = match options.method.to_uppercase().as_str() {
        "GET" => reqwest::Method::GET,
        "POST" => reqwest::Method::POST,
        "PUT" => reqwest::Method::PUT,
        "DELETE" => reqwest::Method::DELETE,
        _ => reqwest::Method::GET,
    };

    let mut req = client.request(method.clone(), &options.url);

    // Align with VrcDog auth handling: keep cookies in the jar and also attach
    // a Cookie header for the current VRChat API request.
    let mut direct_cookies: Vec<String> = Vec::new();
    if let Some(ref cv) = options.auth_cookie {
        let jar = state.cookie_jar.read().await.clone();
        if let Ok(url) = "https://api.vrchat.cloud".parse::<reqwest::Url>() {
            direct_cookies = parse_auth_cookies(cv);
            for cookie in &direct_cookies {
                jar.add_cookie_str(cookie, &url);
            }
        }
    }

    // Attach direct cookies to VRChat API requests as a second auth path.
    if !direct_cookies.is_empty() && options.url.contains("api.vrchat.cloud") {
        let cookie_str = direct_cookies.join("; ");
        if let Ok(hv) = reqwest::header::HeaderValue::from_str(&cookie_str) {
            req = req.header(reqwest::header::COOKIE, hv);
        }
    }

    if let Some(headers) = options.headers {
        for (k, v) in headers {
            if let (Ok(h_name), Ok(h_value)) = (
                reqwest::header::HeaderName::from_bytes(k.as_bytes()),
                reqwest::header::HeaderValue::from_str(&v),
            ) {
                req = req.header(h_name, h_value);
            }
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
                        part_req = part_req.mime_str(&mime).unwrap();
                    }
                    form = form.part(part.name, part_req);
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

    let res = req.send().await.map_err(|e| e.to_string())?;
    let status = res.status().as_u16();
    let auth_cookie = extract_auth_cookie(&res);

    // Debug logging for auth issues.
    if status == 401 && options.url.contains("api.vrchat.cloud") {
        eprintln!(
            "[VrcApi] 401 for {} - auth_cookie provided: {}, direct_cookies count: {}",
            options.url,
            options.auth_cookie.is_some(),
            direct_cookies.len()
        );
    }

    let data = res.text().await.unwrap_or_default();

    Ok(ResponsePayload {
        status,
        data: Some(data),
        auth_cookie,
    })
}
