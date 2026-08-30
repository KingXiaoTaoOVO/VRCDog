use chrono::Utc;
use hmac::{Hmac, Mac};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslateRequest {
    pub text: String,
    pub source_lang: String,
    pub target_lang: String,
    pub service: String,
    pub api_key: String,
    pub model: String,
    pub prompt: String,
    /// Custom API endpoint URL (for "custom" service type)
    #[serde(default)]
    pub custom_api_url: String,
    #[serde(default)]
    pub glossary: Vec<GlossaryTerm>,
    #[serde(default)]
    pub context: Vec<String>,
    #[serde(default = "default_retry_count")]
    pub retry_count: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GlossaryTerm {
    pub source: String,
    pub target: String,
    #[serde(default = "default_glossary_language")]
    pub source_lang: String,
    #[serde(default = "default_glossary_language")]
    pub target_lang: String,
    #[serde(default)]
    pub case_sensitive: bool,
}

fn default_retry_count() -> u8 { 2 }
fn default_glossary_language() -> String { "any".into() }

pub fn glossary_term_matches(term: &GlossaryTerm, source_lang: &str, target_lang: &str) -> bool {
    same_language(&term.source_lang, source_lang) && same_language(&term.target_lang, target_lang)
}

fn same_language(left: &str, right: &str) -> bool {
    if left.eq_ignore_ascii_case("any") || right.eq_ignore_ascii_case("any") { return true; }
    let left = left.replace('_', "-").to_ascii_lowercase();
    let right = right.replace('_', "-").to_ascii_lowercase();
    left == right || left.split('-').next() == right.split('-').next()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslateResult {
    pub original: String,
    pub translated: String,
    pub service: String,
}

struct GlossaryBinding {
    token: String,
    target: String,
}

static TRANSLATION_CACHE: OnceLock<Mutex<HashMap<String, (Instant, String)>>> = OnceLock::new();

fn translation_cache() -> &'static Mutex<HashMap<String, (Instant, String)>> {
    TRANSLATION_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn cache_key(req: &TranslateRequest) -> String {
    let glossary = req.glossary.iter().map(|term| format!("{}={}:{}:{}:{}", term.source, term.target, term.source_lang, term.target_lang, term.case_sensitive)).collect::<Vec<_>>().join("|");
    let context = req.context.iter().take(6).map(|item| item.trim()).collect::<Vec<_>>().join("|");
    format!("{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}", req.service, req.model, req.source_lang, req.target_lang, req.text.trim(), req.prompt, req.custom_api_url, glossary, context)
}

fn matching_glossary(req: &TranslateRequest) -> Vec<&GlossaryTerm> {
    let mut terms = req.glossary.iter().filter(|term| {
        !term.source.trim().is_empty() && !term.target.trim().is_empty()
            && glossary_term_matches(term, &req.source_lang, &req.target_lang)
    }).collect::<Vec<_>>();
    terms.sort_by_key(|term| std::cmp::Reverse(term.source.chars().count()));
    terms
}

fn protect_glossary(req: &TranslateRequest) -> (String, Vec<GlossaryBinding>) {
    let mut text = req.text.clone();
    let mut bindings = Vec::new();
    for (index, term) in matching_glossary(req).into_iter().enumerate() {
        let source = term.source.trim();
        let token = format!("VRCG{:06}X", index);
        let mut spans = Vec::new();
        if term.case_sensitive {
            let mut cursor = 0;
            while let Some(relative) = text[cursor..].find(source) {
                let start = cursor + relative;
                spans.push((start, start + source.len()));
                cursor = start + source.len();
            }
        } else {
            let source_chars = source.chars().collect::<Vec<_>>();
            let text_chars = text.char_indices().collect::<Vec<_>>();
            for start_index in 0..text_chars.len() {
                let end_index = start_index + source_chars.len();
                if end_index > text_chars.len() { break; }
                let matches = text_chars[start_index..end_index].iter().map(|(_, ch)| *ch)
                    .zip(source_chars.iter().copied()).all(|(left, right)| left.eq_ignore_ascii_case(&right));
                if matches {
                    let start = text_chars[start_index].0;
                    let end = if end_index < text_chars.len() { text_chars[end_index].0 } else { text.len() };
                    spans.push((start, end));
                }
            }
        }
        let mut replaced = text.clone();
        for (start, end) in spans.into_iter().rev() {
            replaced.replace_range(start..end, &token);
        }
        if replaced != text {
            text = replaced;
            bindings.push(GlossaryBinding { token, target: term.target.trim().into() });
        }
    }
    (text, bindings)
}

fn restore_glossary(mut text: String, bindings: &[GlossaryBinding]) -> String {
    for binding in bindings {
        text = text.replace(&binding.token, &binding.target);
        text = text.replace(&binding.token.to_ascii_lowercase(), &binding.target);
    }
    text
}

fn is_retryable(error: &str) -> bool {
    let lower = error.to_ascii_lowercase();
    ["http error", "timeout", "timed out", "connection", "429", "502", "503", "504"].iter().any(|marker| lower.contains(marker))
}

fn context_instruction(req: &TranslateRequest) -> String {
    let context = req.context.iter().map(|item| item.trim()).filter(|item| !item.is_empty()).take(6).collect::<Vec<_>>();
    if context.is_empty() { return String::new(); }
    format!("\nRecent conversation context (use only to resolve ambiguous wording):\n{}\n", context.join("\n"))
}

/// Dispatch translation with glossary protection, a short-lived cache, and bounded retries.
pub async fn translate(req: &TranslateRequest) -> Result<TranslateResult, String> {
    if req.text.trim().is_empty() {
        return Err("Translation text is empty".into());
    }
    if req.source_lang != "auto" && same_language(&req.source_lang, &req.target_lang) {
        return Ok(TranslateResult { original: req.text.clone(), translated: req.text.clone(), service: req.service.clone() });
    }
    let key = cache_key(req);
    if let Ok(mut cache) = translation_cache().lock() {
        cache.retain(|_, (created, _)| created.elapsed() < Duration::from_secs(300));
        if let Some((_, translated)) = cache.get(&key) {
            return Ok(TranslateResult { original: req.text.clone(), translated: translated.clone(), service: req.service.clone() });
        }
    }
    let (protected_text, bindings) = protect_glossary(req);
    let mut protected_req = req.clone();
    protected_req.text = protected_text;
    let attempts = req.retry_count.min(3).saturating_add(1);
    let mut last_error = String::new();
    for attempt in 0..attempts {
        match translate_provider(&protected_req).await {
            Ok(result) => {
                let translated = restore_glossary(result.translated, &bindings);
                if let Ok(mut cache) = translation_cache().lock() {
                    if cache.len() >= 256 {
                        if let Some(oldest) = cache.iter().min_by_key(|(_, (created, _))| *created).map(|(key, _)| key.clone()) {
                            cache.remove(&oldest);
                        }
                    }
                    cache.insert(key, (Instant::now(), translated.clone()));
                }
                return Ok(TranslateResult { original: req.text.clone(), translated, service: req.service.clone() });
            }
            Err(error) => {
                last_error = error;
                if attempt + 1 < attempts && is_retryable(&last_error) {
                    tokio::time::sleep(Duration::from_millis(250 * (1u64 << attempt))).await;
                } else {
                    break;
                }
            }
        }
    }
    Err(last_error)
}

async fn translate_provider(req: &TranslateRequest) -> Result<TranslateResult, String> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .connect_timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;
    let translated = match req.service.as_str() {
        "deepseek" => {
            translate_openai_compat(&client, req, "https://api.deepseek.com/v1/chat/completions")
                .await?
        }
        "openai" => {
            translate_openai_compat(&client, req, "https://api.openai.com/v1/chat/completions")
                .await?
        }
        "siliconflow" => {
            translate_openai_compat(
                &client,
                req,
                "https://api.siliconflow.cn/v1/chat/completions",
            )
            .await?
        }
        "moonshot" => {
            translate_openai_compat(&client, req, "https://api.moonshot.cn/v1/chat/completions")
                .await?
        }
        "zhipu" => {
            translate_openai_compat(
                &client,
                req,
                "https://open.bigmodel.cn/api/paas/v4/chat/completions",
            )
            .await?
        }
        "groq" => {
            translate_openai_compat(
                &client,
                req,
                "https://api.groq.com/openai/v1/chat/completions",
            )
            .await?
        }
        "openrouter" => {
            translate_openai_compat(
                &client,
                req,
                "https://openrouter.ai/api/v1/chat/completions",
            )
            .await?
        }
        "plamo" => {
            translate_openai_compat(
                &client,
                req,
                "https://api.platform.preferredai.jp/v1/chat/completions",
            )
            .await?
        }
        "ollama" => {
            translate_openai_compat(&client, req, "http://127.0.0.1:11434/v1/chat/completions")
                .await?
        }
        "lmstudio" | "lm_studio" => {
            translate_openai_compat(&client, req, "http://127.0.0.1:1234/v1/chat/completions")
                .await?
        }
        "custom_llm" => {
            if req.custom_api_url.is_empty() {
                return Err("自定义 LLM API URL 未配置".into());
            }
            translate_openai_compat(&client, req, &req.custom_api_url).await?
        }
        "gemini" => translate_gemini(&client, req).await?,
        "google_cloud" => translate_google_cloud(&client, req).await?,
        "papago" => translate_papago(&client, req).await?,
        "tencent" => translate_tencent(&client, req).await?,
        "baidu" => translate_baidu(&client, req).await?,
        "microsoft" | "bing" => translate_microsoft(&client, req).await?,
        "deepl" => translate_deepl(&client, req).await?,
        "deepl_free" => translate_deepl_free(&client, req).await?,
        "google_free" | _ => translate_google_free(&client, req).await?,
    };
    Ok(TranslateResult {
        original: req.text.clone(),
        translated,
        service: req.service.clone(),
    })
}

/// OpenAI-compatible API (DeepSeek, OpenAI, SiliconFlow, Moonshot, ZhiPu, etc.)
async fn translate_openai_compat(
    client: &Client,
    req: &TranslateRequest,
    url: &str,
) -> Result<String, String> {
    let model = if req.model.is_empty() {
        match req.service.as_str() {
            "deepseek" => "deepseek-chat",
            "openai" => "gpt-4o-mini",
            "siliconflow" => "Qwen/Qwen2.5-7B-Instruct",
            "moonshot" => "moonshot-v1-8k",
            "zhipu" => "glm-4-flash",
            "groq" => "llama-3.1-8b-instant",
            "openrouter" => "openai/gpt-4o-mini",
            "plamo" => "plamo-2-translate",
            "ollama" => "qwen2.5",
            "lmstudio" | "lm_studio" => "local-model",
            _ => "deepseek-chat",
        }
    } else {
        &req.model
    };

    let system_prompt = if req.prompt.is_empty() {
        format!(
            "You are a professional translator. Translate the following text from {} to {}. \
             Return ONLY the translated text, nothing else. Preserve names, URLs, emojis, \
             line breaks, and placeholder tokens exactly.{}",
            lang_name(&req.source_lang),
            lang_name(&req.target_lang),
            context_instruction(req)
        )
    } else {
        format!(
            "{}\nSource language: {}\nTarget language: {} Preserve names, URLs, emojis, \
             line breaks, and placeholder tokens exactly.{}",
            req.prompt,
            lang_name(&req.source_lang),
            lang_name(&req.target_lang),
            context_instruction(req)
        )
    };

    let body = serde_json::json!({
        "model": model,
        "messages": [
            {"role": "system", "content": system_prompt},
            {"role": "user", "content": &req.text}
        ],
        "temperature": 0.3,
        "max_tokens": 2048
    });

    let mut request = client.post(url).header("Content-Type", "application/json");
    if !req.api_key.trim().is_empty() {
        request = request.header("Authorization", format!("Bearer {}", req.api_key));
    }

    let resp = request
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("HTTP error: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("API error {}: {}", status, text));
    }

    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("JSON parse error: {}", e))?;
    let content = json["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .trim()
        .to_string();

    if content.is_empty() {
        Err("Empty translation response".into())
    } else {
        Ok(content)
    }
}

/// Tencent Cloud Translation API (腾讯翻译)
async fn translate_tencent(client: &Client, req: &TranslateRequest) -> Result<String, String> {
    // API key format: "SecretId:SecretKey"
    let parts: Vec<&str> = req.api_key.splitn(2, ':').collect();
    if parts.len() != 2 {
        return Err("Tencent API key format: SecretId:SecretKey".into());
    }
    let secret_id = parts[0];
    let secret_key = parts[1];

    let host = "tmt.tencentcloudapi.com";
    let action = "TextTranslate";
    let version = "2018-03-21";
    let timestamp = Utc::now().timestamp();

    let source = tencent_lang_code(&req.source_lang);
    let target = tencent_lang_code(&req.target_lang);

    let payload = serde_json::json!({
        "SourceText": &req.text,
        "Source": source,
        "Target": target,
        "ProjectId": 0
    })
    .to_string();

    // TC3-HMAC-SHA256 signing
    let date = Utc::now().format("%Y-%m-%d").to_string();
    let credential_scope = format!("{}/tmt/tc3_request", date);

    let canonical_request = format!(
        "POST\n/\n\ncontent-type:application/json\nhost:{}\n\ncontent-type;host\n{}",
        host,
        hex::encode(sha256_hash(payload.as_bytes()))
    );

    let string_to_sign = format!(
        "TC3-HMAC-SHA256\n{}\n{}\n{}",
        timestamp,
        credential_scope,
        hex::encode(sha256_hash(canonical_request.as_bytes()))
    );

    let secret_date = hmac_sha256(format!("TC3{}", secret_key).as_bytes(), date.as_bytes());
    let secret_service = hmac_sha256(&secret_date, b"tmt");
    let secret_signing = hmac_sha256(&secret_service, b"tc3_request");
    let signature = hex::encode(hmac_sha256(&secret_signing, string_to_sign.as_bytes()));

    let authorization = format!(
        "TC3-HMAC-SHA256 Credential={}/{}, SignedHeaders=content-type;host, Signature={}",
        secret_id, credential_scope, signature
    );

    let resp = client
        .post(format!("https://{}", host))
        .header("Authorization", &authorization)
        .header("Content-Type", "application/json")
        .header("Host", host)
        .header("X-TC-Action", action)
        .header("X-TC-Timestamp", timestamp.to_string())
        .header("X-TC-Version", version)
        .body(payload)
        .send()
        .await
        .map_err(|e| format!("HTTP error: {}", e))?;

    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("JSON error: {}", e))?;

    if let Some(err) = json["Response"]["Error"]["Message"].as_str() {
        return Err(format!("Tencent API error: {}", err));
    }

    json["Response"]["TargetText"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "No translation in response".into())
}

/// Baidu Translation API (百度翻译)
/// API key format: "AppID:SecretKey"
async fn translate_baidu(client: &Client, req: &TranslateRequest) -> Result<String, String> {
    let parts: Vec<&str> = req.api_key.splitn(2, ':').collect();
    if parts.len() != 2 {
        return Err("Baidu API key format: AppID:SecretKey".into());
    }
    let app_id = parts[0];
    let secret_key = parts[1];

    let salt = Utc::now().timestamp().to_string();
    let sign_str = format!("{}{}{}{}", app_id, &req.text, &salt, secret_key);
    let sign = format!("{:x}", md5::compute(sign_str.as_bytes()));

    let from = baidu_lang_code(&req.source_lang);
    let to = baidu_lang_code(&req.target_lang);

    let params = [
        ("q", req.text.as_str()),
        ("from", from),
        ("to", to),
        ("appid", app_id),
        ("salt", &salt),
        ("sign", &sign),
    ];

    let resp = client
        .post("https://fanyi-api.baidu.com/api/trans/vip/translate")
        .form(&params)
        .send()
        .await
        .map_err(|e| format!("HTTP error: {}", e))?;

    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("JSON error: {}", e))?;

    if let Some(code) = json["error_code"].as_str() {
        let msg = json["error_msg"].as_str().unwrap_or("unknown");
        return Err(format!("Baidu API error {}: {}", code, msg));
    }

    if let Some(results) = json["trans_result"].as_array() {
        let mut translated = String::new();
        for r in results {
            if let Some(dst) = r["dst"].as_str() {
                if !translated.is_empty() {
                    translated.push('\n');
                }
                translated.push_str(dst);
            }
        }
        if !translated.is_empty() {
            return Ok(translated);
        }
    }

    Err("Baidu: No translation in response".into())
}

/// Microsoft Translator API (微软翻译)
/// API key = subscription key
async fn translate_microsoft(client: &Client, req: &TranslateRequest) -> Result<String, String> {
    let from_param = if req.source_lang == "auto" {
        String::new()
    } else {
        format!("&from={}", ms_lang_code(&req.source_lang))
    };
    let to = ms_lang_code(&req.target_lang);

    let url = format!(
        "https://api.cognitive.microsofttranslator.com/translate?api-version=3.0{}&to={}",
        from_param, to
    );

    let body = serde_json::json!([{ "Text": &req.text }]);

    let resp = client
        .post(&url)
        .header("Ocp-Apim-Subscription-Key", &req.api_key)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("HTTP error: {}", e))?;

    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("Microsoft API error: {}", text));
    }

    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("JSON error: {}", e))?;
    json[0]["translations"][0]["text"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "Microsoft: No translation in response".into())
}

/// DeepL Pro API
/// API key = auth key
async fn translate_deepl(client: &Client, req: &TranslateRequest) -> Result<String, String> {
    translate_deepl_impl(client, req, "https://api.deepl.com/v2/translate").await
}

/// DeepL Free API
async fn translate_deepl_free(client: &Client, req: &TranslateRequest) -> Result<String, String> {
    translate_deepl_impl(client, req, "https://api-free.deepl.com/v2/translate").await
}

async fn translate_deepl_impl(
    client: &Client,
    req: &TranslateRequest,
    url: &str,
) -> Result<String, String> {
    let target = deepl_lang_code(&req.target_lang);

    let mut params = vec![("text", req.text.as_str()), ("target_lang", target)];
    let source_code;
    if req.source_lang != "auto" {
        source_code = deepl_lang_code(&req.source_lang).to_string();
        params.push(("source_lang", &source_code));
    }

    let resp = client
        .post(url)
        .header("Authorization", format!("DeepL-Auth-Key {}", req.api_key))
        .form(&params)
        .send()
        .await
        .map_err(|e| format!("HTTP error: {}", e))?;

    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("DeepL API error: {}", text));
    }

    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("JSON error: {}", e))?;
    json["translations"][0]["text"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "DeepL: No translation in response".into())
}

/// Google Gemini API
async fn translate_gemini(client: &Client, req: &TranslateRequest) -> Result<String, String> {
    if req.api_key.trim().is_empty() {
        return Err("Gemini API key is required".into());
    }
    let model = if req.model.trim().is_empty() {
        "gemini-1.5-flash"
    } else {
        req.model.as_str()
    };
    let prompt = if req.prompt.trim().is_empty() {
        format!(
            "Translate from {} to {}. Return only the translated text.\n\n{}",
            lang_name(&req.source_lang),
            lang_name(&req.target_lang),
            req.text
        )
    } else {
        format!(
            "{}\nSource language: {}\nTarget language: {}\n\n{}",
            req.prompt,
            lang_name(&req.source_lang),
            lang_name(&req.target_lang),
            req.text
        )
    };
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        model,
        urlencoding::encode(&req.api_key)
    );
    let body = serde_json::json!({
        "contents": [{ "parts": [{ "text": prompt }] }],
        "generationConfig": { "temperature": 0.2 }
    });
    let resp = client
        .post(url)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("HTTP error: {}", e))?;
    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("Gemini API error: {}", text));
    }
    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("JSON error: {}", e))?;
    json["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "Gemini: No translation in response".into())
}

/// Naver Papago API. API key format: "ClientId:ClientSecret".
async fn translate_papago(client: &Client, req: &TranslateRequest) -> Result<String, String> {
    let parts: Vec<&str> = req.api_key.splitn(2, ':').collect();
    if parts.len() != 2 {
        return Err("Papago API key format: ClientId:ClientSecret".into());
    }
    let source = papago_lang_code(&req.source_lang);
    if source == "auto" {
        return Err("Papago does not support auto source language".into());
    }
    let target = papago_lang_code(&req.target_lang);
    let params = [
        ("source", source),
        ("target", target),
        ("text", req.text.as_str()),
    ];
    let resp = client
        .post("https://papago.apigw.ntruss.com/nmt/v1/translation")
        .header("X-NCP-APIGW-API-KEY-ID", parts[0])
        .header("X-NCP-APIGW-API-KEY", parts[1])
        .form(&params)
        .send()
        .await
        .map_err(|e| format!("HTTP error: {}", e))?;
    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("Papago API error: {}", text));
    }
    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("JSON error: {}", e))?;
    json["message"]["result"]["translatedText"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "Papago: No translation in response".into())
}

/// Google Translate Free API (gtx)
async fn translate_google_free(client: &Client, req: &TranslateRequest) -> Result<String, String> {
    let sl = if req.source_lang == "auto" {
        "auto"
    } else {
        &req.source_lang.split('-').next().unwrap_or("auto")
    };
    let tl = &req.target_lang;

    let url = format!(
        "https://translate.googleapis.com/translate_a/single?client=gtx&sl={}&tl={}&dt=t&q={}",
        sl,
        tl,
        urlencoding::encode(&req.text)
    );

    let mut last_error = String::new();
    let mut resp = None;
    for attempt in 0..3 {
        match client.get(&url).send().await {
            Ok(res) => {
                if res.status().is_server_error() && attempt < 2 {
                    last_error = format!("HTTP {}", res.status());
                    tokio::time::sleep(std::time::Duration::from_millis(
                        180 * (attempt + 1) as u64,
                    ))
                    .await;
                    continue;
                }
                resp = Some(res);
                break;
            }
            Err(e) => {
                last_error = e.to_string();
                if attempt < 2 {
                    tokio::time::sleep(std::time::Duration::from_millis(
                        180 * (attempt + 1) as u64,
                    ))
                    .await;
                    continue;
                }
            }
        }
    }
    let resp = resp.ok_or_else(|| format!("HTTP error: {}", last_error))?;
    if !resp.status().is_success() {
        return Err(format!("Google Translate HTTP error: {}", resp.status()));
    }
    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("JSON error: {}", e))?;

    if let Some(arr) = json[0].as_array() {
        let mut result = String::new();
        for item in arr {
            if let Some(text) = item[0].as_str() {
                result.push_str(text);
            }
        }
        if !result.is_empty() {
            return Ok(result);
        }
    }

    Err("Failed to parse Google Translate response".into())
}

/// Google Cloud Translation Basic REST API.
/// API key = Google Cloud API key with Cloud Translation enabled.
async fn translate_google_cloud(client: &Client, req: &TranslateRequest) -> Result<String, String> {
    if req.api_key.trim().is_empty() {
        return Err("Google Cloud API key is required".into());
    }
    let mut body = serde_json::json!({
        "q": [req.text.as_str()],
        "target": req.target_lang,
        "format": "text"
    });
    if req.source_lang != "auto" {
        body["source"] = serde_json::Value::String(req.source_lang.clone());
    }
    let resp = client
        .post("https://translation.googleapis.com/language/translate/v2")
        .query(&[("key", req.api_key.as_str())])
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("HTTP error: {}", e))?;
    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("Google Cloud API error: {}", text));
    }
    let json: serde_json::Value = resp.json().await.map_err(|e| format!("JSON error: {}", e))?;
    json["data"]["translations"][0]["translatedText"]
        .as_str()
        .map(|text| text.to_string())
        .ok_or_else(|| "Google Cloud: No translation in response".into())
}

// ==================== Crypto Helpers ====================

fn sha256_hash(data: &[u8]) -> Vec<u8> {
    use sha2::Digest;
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().to_vec()
}

fn hmac_sha256(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC key");
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

// ==================== Language Code Mappings (30+ languages) ====================

fn lang_name(code: &str) -> &str {
    match code {
        "zh" | "zh-CN" | "zh-Hans" | "zh-Hans-CN" => "Chinese (Simplified)",
        "zh-TW" | "zh-Hant" => "Chinese (Traditional)",
        "en" | "en-US" => "English",
        "ja" => "Japanese",
        "ko" => "Korean",
        "fr" => "French",
        "de" => "German",
        "es" => "Spanish",
        "ru" => "Russian",
        "pt" | "pt-BR" => "Portuguese",
        "it" => "Italian",
        "nl" => "Dutch",
        "pl" => "Polish",
        "ar" => "Arabic",
        "th" => "Thai",
        "vi" => "Vietnamese",
        "id" => "Indonesian",
        "ms" => "Malay",
        "tr" => "Turkish",
        "uk" => "Ukrainian",
        "cs" => "Czech",
        "sv" => "Swedish",
        "da" => "Danish",
        "fi" => "Finnish",
        "no" | "nb" => "Norwegian",
        "hu" => "Hungarian",
        "el" => "Greek",
        "ro" => "Romanian",
        "bg" => "Bulgarian",
        "hi" => "Hindi",
        "bn" => "Bengali",
        "auto" => "Auto-detect",
        _ => code,
    }
}

fn tencent_lang_code(code: &str) -> &str {
    match code {
        "zh" | "zh-CN" | "zh-Hans" => "zh",
        "zh-TW" | "zh-Hant" => "zh-TW",
        "en" | "en-US" => "en",
        "ja" => "jp",
        "ko" => "kr",
        "fr" => "fr",
        "de" => "de",
        "es" => "es",
        "ru" => "ru",
        "pt" | "pt-BR" => "pt",
        "it" => "it",
        "nl" => "nl",
        "pl" => "pl",
        "ar" => "ar",
        "th" => "th",
        "vi" => "vi",
        "id" => "id",
        "ms" => "ms",
        "tr" => "tr",
        "hi" => "hi",
        "auto" => "auto",
        _ => code,
    }
}

fn baidu_lang_code(code: &str) -> &str {
    match code {
        "zh" | "zh-CN" | "zh-Hans" => "zh",
        "zh-TW" | "zh-Hant" => "cht",
        "en" | "en-US" => "en",
        "ja" => "jp",
        "ko" => "kor",
        "fr" => "fra",
        "de" => "de",
        "es" => "spa",
        "ru" => "ru",
        "pt" | "pt-BR" => "pt",
        "it" => "it",
        "nl" => "nl",
        "pl" => "pl",
        "ar" => "ara",
        "th" => "th",
        "vi" => "vie",
        "id" => "id",
        "ms" => "may",
        "tr" => "tr",
        "hi" => "hi",
        "el" => "el",
        "hu" => "hu",
        "auto" => "auto",
        _ => code,
    }
}

fn ms_lang_code(code: &str) -> &str {
    match code {
        "zh" | "zh-CN" | "zh-Hans" => "zh-Hans",
        "zh-TW" | "zh-Hant" => "zh-Hant",
        "en" | "en-US" => "en",
        "ja" => "ja",
        "ko" => "ko",
        "fr" => "fr",
        "de" => "de",
        "es" => "es",
        "ru" => "ru",
        "pt" | "pt-BR" => "pt-br",
        "it" => "it",
        "nl" => "nl",
        "pl" => "pl",
        "ar" => "ar",
        "th" => "th",
        "vi" => "vi",
        "id" => "id",
        "ms" => "ms",
        "tr" => "tr",
        "uk" => "uk",
        "cs" => "cs",
        "sv" => "sv",
        "da" => "da",
        "fi" => "fi",
        "no" | "nb" => "nb",
        "hu" => "hu",
        "el" => "el",
        "ro" => "ro",
        "bg" => "bg",
        "hi" => "hi",
        "bn" => "bn",
        "auto" => "auto",
        _ => code,
    }
}

fn deepl_lang_code(code: &str) -> &str {
    match code {
        "zh" | "zh-CN" | "zh-Hans" => "ZH",
        "en" | "en-US" => "EN",
        "ja" => "JA",
        "ko" => "KO",
        "fr" => "FR",
        "de" => "DE",
        "es" => "ES",
        "ru" => "RU",
        "pt" | "pt-BR" => "PT-BR",
        "it" => "IT",
        "nl" => "NL",
        "pl" => "PL",
        "sv" => "SV",
        "da" => "DA",
        "fi" => "FI",
        "nb" | "no" => "NB",
        "hu" => "HU",
        "el" => "EL",
        "ro" => "RO",
        "bg" => "BG",
        "cs" => "CS",
        "tr" => "TR",
        "uk" => "UK",
        "id" => "ID",
        _ => code,
    }
}

fn papago_lang_code(code: &str) -> &str {
    match code {
        "zh" | "zh-CN" | "zh-Hans" => "zh-CN",
        "zh-TW" | "zh-Hant" => "zh-TW",
        "en" | "en-US" => "en",
        "ja" => "ja",
        "ko" => "ko",
        "fr" => "fr",
        "de" => "de",
        "es" => "es",
        "ru" => "ru",
        "pt" | "pt-BR" => "pt",
        "it" => "it",
        "vi" => "vi",
        "id" => "id",
        "th" => "th",
        "auto" => "auto",
        _ => code,
    }
}

// ==================== Tauri Commands ====================

#[tauri::command]
pub async fn ovr_translate(req: TranslateRequest) -> crate::AppResult<TranslateResult> {
    translate(&req).await.map_err(|e| e.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(text: &str) -> TranslateRequest {
        TranslateRequest {
            text: text.into(), source_lang: "en".into(), target_lang: "zh-CN".into(),
            service: "google_free".into(), api_key: String::new(), model: String::new(),
            prompt: String::new(), custom_api_url: String::new(),
            glossary: vec![GlossaryTerm {
                source: "VRChat".into(), target: "VRChat（虚拟聊天）".into(),
                source_lang: "any".into(), target_lang: "any".into(), case_sensitive: false,
            }],
            context: vec![], retry_count: 2,
        }
    }

    #[test]
    fn glossary_matching_accepts_regional_language_codes() {
        let term = GlossaryTerm { source: "世界".into(), target: "world".into(), source_lang: "zh-CN".into(), target_lang: "en".into(), case_sensitive: false };
        assert!(glossary_term_matches(&term, "zh-CN", "en-US"));
        assert!(!glossary_term_matches(&term, "ja", "en"));
    }

    #[test]
    fn glossary_protection_is_unicode_safe_and_restores_terms() {
        let req = request("欢迎来到 VRChat！");
        let (protected, bindings) = protect_glossary(&req);
        assert!(!protected.contains("VRChat"));
        assert_eq!(restore_glossary(protected, &bindings), "欢迎来到 VRChat（虚拟聊天）！");
    }

    #[test]
    fn same_language_short_circuits_without_provider() {
        let mut req = request("保持原文");
        req.source_lang = "zh-CN".into(); req.target_lang = "zh".into();
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
        let result = runtime.block_on(translate(&req)).expect("same-language result");
        assert_eq!(result.translated, "保持原文");
    }
}
