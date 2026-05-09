use reqwest::{Client, header};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::State;
use base64::Engine;

const VRC_USER_AGENT: &str = "VRCX 2024.03.23";

pub struct VrcState {
    pub client: Arc<Mutex<Client>>,
    pub proxy_url: Arc<Mutex<Option<String>>>,
    pub cookie_jar: Arc<Mutex<Arc<reqwest::cookie::Jar>>>,
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
            client: Arc::new(Mutex::new(build_vrc_client(None, None, jar.clone()))),
            proxy_url: Arc::new(Mutex::new(None)),
            cookie_jar: Arc::new(Mutex::new(jar)),
        }
    }
}

fn build_vrc_client(proxy_url: Option<String>, auth_cookie: Option<String>, jar: Arc<reqwest::cookie::Jar>) -> Client {
    let mut headers = header::HeaderMap::new();
    headers.insert(header::USER_AGENT, header::HeaderValue::from_static(VRC_USER_AGENT));
    headers.insert(header::REFERER, header::HeaderValue::from_static("https://vrchat.com/"));

    if let Some(cv) = auth_cookie {
        if let Ok(url) = "https://api.vrchat.cloud".parse::<reqwest::Url>() {
            if cv.starts_with('[') {
                if let Ok(cookies) = serde_json::from_str::<Vec<String>>(&cv) {
                    for c in cookies {
                        jar.add_cookie_str(&c, &url);
                    }
                }
            } else {
                let c = if cv.starts_with("auth=") || cv.starts_with("twoFactorAuth=") { cv.clone() } else { format!("auth={}", cv) };
                jar.add_cookie_str(&c, &url);
            }
        }
    }

    let mut builder = Client::builder()
        .cookie_provider(jar)
        .default_headers(headers);

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
pub async fn vrc_set_proxy(state: tauri::State<'_, VrcState>, proxy_url: Option<String>, auth_cookie: Option<String>) -> Result<(), String> {
    let mut proxy_lock = state.proxy_url.lock().await;
    *proxy_lock = proxy_url.clone();
    
    let jar = Arc::new(reqwest::cookie::Jar::default());
    let mut jar_lock = state.cookie_jar.lock().await;
    *jar_lock = jar.clone();
    
    let mut client = state.client.lock().await;
    *client = build_vrc_client(proxy_url, auth_cookie, jar);
    Ok(())
}

#[tauri::command]
pub async fn vrc_clear_cookies(state: tauri::State<'_, VrcState>) -> Result<(), String> {
    let proxy_url = state.proxy_url.lock().await.clone();
    
    let jar = Arc::new(reqwest::cookie::Jar::default());
    let mut jar_lock = state.cookie_jar.lock().await;
    *jar_lock = jar.clone();
    
    let mut client = state.client.lock().await;
    *client = build_vrc_client(proxy_url, None, jar);
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

#[tauri::command]
pub async fn vrc_get_image_bytes(
    state: tauri::State<'_, VrcState>,
    url: String,
    auth_cookie: Option<String>,
) -> Result<String, String> {
    if url.is_empty() { return Err("Empty URL".to_string()); }
    let client = state.client.lock().await.clone();
    
    let mut req = client.get(&url);
    
    // Inject cookie
    
    if let Some(ref cv) = auth_cookie {
        let mut direct_cookies: Vec<String> = Vec::new();
        if cv.starts_with('[') {
            if let Ok(cookies) = serde_json::from_str::<Vec<String>>(cv) {
                direct_cookies = cookies;
            }
        } else {
            let c = if cv.starts_with("auth=") || cv.starts_with("twoFactorAuth=") { cv.clone() } else { format!("auth={}", cv) };
            direct_cookies.push(c);
        }
        
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
    
    let content_type = res.headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("image/jpeg")
        .to_string();

    let bytes = res.bytes().await.map_err(|e| e.to_string())?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
    
    let mime = if content_type.is_empty() { "image/jpeg" } else { &content_type };
    Ok(format!("data:{};base64,{}", mime, b64))
}

#[derive(Deserialize)]
pub struct RequestOptions {
    pub url: String,
    pub method: String,
    pub headers: Option<std::collections::HashMap<String, String>>,
    pub body: Option<String>,
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
    let client = state.client.lock().await.clone();

    let method = match options.method.to_uppercase().as_str() {
        "GET" => reqwest::Method::GET,
        "POST" => reqwest::Method::POST,
        "PUT" => reqwest::Method::PUT,
        "DELETE" => reqwest::Method::DELETE,
        _ => reqwest::Method::GET,
    };

    let mut req = client.request(method.clone(), &options.url);

    // [VRCX 对齐] 注入 auth cookie — 既更新 jar（长期持久化），也直接设置 Cookie header（确保本次请求一定携带）
    // VRCX 的 CookieContainer 在 HttpClient 层面管理 Cookie，不会出现遗漏
    // 我们使用双重保障：jar + header
    let mut direct_cookies: Vec<String> = Vec::new();
    if let Some(ref cv) = options.auth_cookie {
        let jar = state.cookie_jar.lock().await.clone();
        if let Ok(url) = "https://api.vrchat.cloud".parse::<reqwest::Url>() {
            if cv.starts_with('[') {
                if let Ok(cookies) = serde_json::from_str::<Vec<String>>(cv) {
                    for c in &cookies {
                        jar.add_cookie_str(c, &url);
                    }
                    direct_cookies = cookies;
                }
            } else {
                let c = if cv.starts_with("auth=") || cv.starts_with("twoFactorAuth=") { cv.clone() } else { format!("auth={}", cv) };
                jar.add_cookie_str(&c, &url);
                direct_cookies.push(c);
            }
        }
    }

    // 如果有直接的 cookie 需要注入，且目标是 VRChat API，设置 Cookie header 作为双重保障
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

    if let Some(body) = options.body {
        req = req.body(body);
    }

    let res = req.send().await.map_err(|e| e.to_string())?;
    let status = res.status().as_u16();
    let auth_cookie = extract_auth_cookie(&res);

    // Debug logging for auth issues
    if status == 401 && options.url.contains("api.vrchat.cloud") {
        eprintln!("[VrcApi] ⚠ 401 for {} — auth_cookie provided: {}, direct_cookies count: {}", 
            options.url, options.auth_cookie.is_some(), direct_cookies.len());
    }

    let data = res.text().await.unwrap_or_default();

    Ok(ResponsePayload {
        status,
        data: Some(data),
        auth_cookie,
    })
}
