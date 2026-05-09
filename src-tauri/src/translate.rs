use serde::{Deserialize, Serialize};
use reqwest::Client;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use chrono::Utc;

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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslateResult {
    pub original: String,
    pub translated: String,
    pub service: String,
}

/// Dispatch translation to the configured service
pub async fn translate(req: &TranslateRequest) -> Result<TranslateResult, String> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .connect_timeout(std::time::Duration::from_secs(5))
        .danger_accept_invalid_certs(true) // Allow self-signed proxy certs
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;
    let translated = match req.service.as_str() {
        "deepseek" => translate_openai_compat(&client, req, "https://api.deepseek.com/v1/chat/completions").await?,
        "openai" => translate_openai_compat(&client, req, "https://api.openai.com/v1/chat/completions").await?,
        "siliconflow" => translate_openai_compat(&client, req, "https://api.siliconflow.cn/v1/chat/completions").await?,
        "moonshot" => translate_openai_compat(&client, req, "https://api.moonshot.cn/v1/chat/completions").await?,
        "zhipu" => translate_openai_compat(&client, req, "https://open.bigmodel.cn/api/paas/v4/chat/completions").await?,
        "custom_llm" => {
            if req.custom_api_url.is_empty() {
                return Err("自定义 LLM API URL 未配置".into());
            }
            translate_openai_compat(&client, req, &req.custom_api_url).await?
        },
        "tencent" => translate_tencent(&client, req).await?,
        "baidu" => translate_baidu(&client, req).await?,
        "microsoft" => translate_microsoft(&client, req).await?,
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
async fn translate_openai_compat(client: &Client, req: &TranslateRequest, url: &str) -> Result<String, String> {
    let model = if req.model.is_empty() {
        match req.service.as_str() {
            "deepseek" => "deepseek-chat",
            "openai" => "gpt-4o-mini",
            "siliconflow" => "Qwen/Qwen2.5-7B-Instruct",
            "moonshot" => "moonshot-v1-8k",
            "zhipu" => "glm-4-flash",
            _ => "deepseek-chat",
        }
    } else {
        &req.model
    };

    let system_prompt = if req.prompt.is_empty() {
        format!(
            "You are a professional translator. Translate the following text from {} to {}. \
             Return ONLY the translated text, nothing else.",
            lang_name(&req.source_lang), lang_name(&req.target_lang)
        )
    } else {
        format!("{}\nSource language: {}\nTarget language: {}", req.prompt, lang_name(&req.source_lang), lang_name(&req.target_lang))
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

    let resp = client.post(url)
        .header("Authorization", format!("Bearer {}", req.api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send().await
        .map_err(|e| format!("HTTP error: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("API error {}: {}", status, text));
    }

    let json: serde_json::Value = resp.json().await.map_err(|e| format!("JSON parse error: {}", e))?;
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
    }).to_string();

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

    let resp = client.post(format!("https://{}", host))
        .header("Authorization", &authorization)
        .header("Content-Type", "application/json")
        .header("Host", host)
        .header("X-TC-Action", action)
        .header("X-TC-Timestamp", timestamp.to_string())
        .header("X-TC-Version", version)
        .body(payload)
        .send().await
        .map_err(|e| format!("HTTP error: {}", e))?;

    let json: serde_json::Value = resp.json().await.map_err(|e| format!("JSON error: {}", e))?;

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
    let sign = format!("{:x}", md5_hash(sign_str.as_bytes()));

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

    let resp = client.post("https://fanyi-api.baidu.com/api/trans/vip/translate")
        .form(&params)
        .send().await
        .map_err(|e| format!("HTTP error: {}", e))?;

    let json: serde_json::Value = resp.json().await.map_err(|e| format!("JSON error: {}", e))?;

    if let Some(code) = json["error_code"].as_str() {
        let msg = json["error_msg"].as_str().unwrap_or("unknown");
        return Err(format!("Baidu API error {}: {}", code, msg));
    }

    if let Some(results) = json["trans_result"].as_array() {
        let mut translated = String::new();
        for r in results {
            if let Some(dst) = r["dst"].as_str() {
                if !translated.is_empty() { translated.push('\n'); }
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
    let from_param = if req.source_lang == "auto" { String::new() }
                     else { format!("&from={}", ms_lang_code(&req.source_lang)) };
    let to = ms_lang_code(&req.target_lang);

    let url = format!(
        "https://api.cognitive.microsofttranslator.com/translate?api-version=3.0{}&to={}",
        from_param, to
    );

    let body = serde_json::json!([{ "Text": &req.text }]);

    let resp = client.post(&url)
        .header("Ocp-Apim-Subscription-Key", &req.api_key)
        .header("Content-Type", "application/json")
        .json(&body)
        .send().await
        .map_err(|e| format!("HTTP error: {}", e))?;

    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("Microsoft API error: {}", text));
    }

    let json: serde_json::Value = resp.json().await.map_err(|e| format!("JSON error: {}", e))?;
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

async fn translate_deepl_impl(client: &Client, req: &TranslateRequest, url: &str) -> Result<String, String> {
    let target = deepl_lang_code(&req.target_lang);

    let mut params = vec![
        ("text", req.text.as_str()),
        ("target_lang", target),
    ];
    let source_code;
    if req.source_lang != "auto" {
        source_code = deepl_lang_code(&req.source_lang).to_string();
        params.push(("source_lang", &source_code));
    }

    let resp = client.post(url)
        .header("Authorization", format!("DeepL-Auth-Key {}", req.api_key))
        .form(&params)
        .send().await
        .map_err(|e| format!("HTTP error: {}", e))?;

    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("DeepL API error: {}", text));
    }

    let json: serde_json::Value = resp.json().await.map_err(|e| format!("JSON error: {}", e))?;
    json["translations"][0]["text"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "DeepL: No translation in response".into())
}

/// Google Translate Free API (gtx)
async fn translate_google_free(client: &Client, req: &TranslateRequest) -> Result<String, String> {
    let sl = if req.source_lang == "auto" { "auto" } else { &req.source_lang.split('-').next().unwrap_or("auto") };
    let tl = &req.target_lang;
    
    let url = format!(
        "https://translate.googleapis.com/translate_a/single?client=gtx&sl={}&tl={}&dt=t&q={}",
        sl, tl, urlencoding::encode(&req.text)
    );
    
    let resp = client.get(&url).send().await.map_err(|e| format!("HTTP error: {}", e))?;
    let json: serde_json::Value = resp.json().await.map_err(|e| format!("JSON error: {}", e))?;
    
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

/// Simple MD5 hash for Baidu API signing (uses built-in computation)
fn md5_hash(data: &[u8]) -> u128 {
    // Minimal MD5 via manual computation — avoiding extra dependency
    // We use the sha2 crate's Digest trait with a simple custom approach
    // Actually, for Baidu we just need hex md5. Let's compute via manual method.
    let len = data.len();
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 { msg.push(0); }
    let bit_len = (len as u64).wrapping_mul(8);
    msg.extend_from_slice(&bit_len.to_le_bytes());

    let (mut a, mut b, mut c, mut d): (u32, u32, u32, u32) = (0x67452301, 0xefcdab89, 0x98badcfe, 0x10325476);
    let s: [u32; 64] = [7,12,17,22,7,12,17,22,7,12,17,22,7,12,17,22,
                         5,9,14,20,5,9,14,20,5,9,14,20,5,9,14,20,
                         4,11,16,23,4,11,16,23,4,11,16,23,4,11,16,23,
                         6,10,15,21,6,10,15,21,6,10,15,21,6,10,15,21];
    let k: [u32; 64] = [
        0xd76aa478,0xe8c7b756,0x242070db,0xc1bdceee,0xf57c0faf,0x4787c62a,0xa8304613,0xfd469501,
        0x698098d8,0x8b44f7af,0xffff5bb1,0x895cd7be,0x6b901122,0xfd987193,0xa679438e,0x49b40821,
        0xf61e2562,0xc040b340,0x265e5a51,0xe9b6c7aa,0xd62f105d,0x02441453,0xd8a1e681,0xe7d3fbc8,
        0x21e1cde6,0xc33707d6,0xf4d50d87,0x455a14ed,0xa9e3e905,0xfcefa3f8,0x676f02d9,0x8d2a4c8a,
        0xfffa3942,0x8771f681,0x6d9d6122,0xfde5380c,0xa4beea44,0x4bdecfa9,0xf6bb4b60,0xbebfbc70,
        0x289b7ec6,0xeaa127fa,0xd4ef3085,0x04881d05,0xd9d4d039,0xe6db99e5,0x1fa27cf8,0xc4ac5665,
        0xf4292244,0x432aff97,0xab9423a7,0xfc93a039,0x655b59c3,0x8f0ccc92,0xffeff47d,0x85845dd1,
        0x6fa87e4f,0xfe2ce6e0,0xa3014314,0x4e0811a1,0xf7537e82,0xbd3af235,0x2ad7d2bb,0xeb86d391,
    ];
    for chunk in msg.chunks(64) {
        let mut m = [0u32; 16];
        for (i, w) in chunk.chunks(4).enumerate() {
            m[i] = u32::from_le_bytes([w[0], w[1], w[2], w[3]]);
        }
        let (mut aa, mut bb, mut cc, mut dd) = (a, b, c, d);
        for i in 0..64usize {
            let (f, g) = match i {
                0..=15  => ((bb & cc) | (!bb & dd), i),
                16..=31 => ((dd & bb) | (!dd & cc), (5*i+1) % 16),
                32..=47 => (bb ^ cc ^ dd, (3*i+5) % 16),
                _       => (cc ^ (bb | !dd), (7*i) % 16),
            };
            let temp = dd;
            dd = cc;
            cc = bb;
            bb = bb.wrapping_add(
                (aa.wrapping_add(f).wrapping_add(k[i]).wrapping_add(m[g]))
                .rotate_left(s[i])
            );
            aa = temp;
        }
        a = a.wrapping_add(aa);
        b = b.wrapping_add(bb);
        c = c.wrapping_add(cc);
        d = d.wrapping_add(dd);
    }
    let bytes = [a.to_le_bytes(), b.to_le_bytes(), c.to_le_bytes(), d.to_le_bytes()].concat();
    u128::from_le_bytes(bytes.try_into().unwrap())
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
        "fr" => "fr",  "de" => "de",  "es" => "es",  "ru" => "ru",
        "pt" | "pt-BR" => "pt",
        "it" => "it",  "nl" => "nl",  "pl" => "pl",  "ar" => "ar",
        "th" => "th",  "vi" => "vi",  "id" => "id",  "ms" => "ms",
        "tr" => "tr",  "hi" => "hi",
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
        "fr" => "fra",  "de" => "de",  "es" => "spa",  "ru" => "ru",
        "pt" | "pt-BR" => "pt",
        "it" => "it",   "nl" => "nl",  "pl" => "pl",   "ar" => "ara",
        "th" => "th",   "vi" => "vie", "id" => "id",   "ms" => "may",
        "tr" => "tr",   "hi" => "hi",  "el" => "el",   "hu" => "hu",
        "auto" => "auto",
        _ => code,
    }
}

fn ms_lang_code(code: &str) -> &str {
    match code {
        "zh" | "zh-CN" | "zh-Hans" => "zh-Hans",
        "zh-TW" | "zh-Hant" => "zh-Hant",
        "en" | "en-US" => "en",
        "ja" => "ja",  "ko" => "ko",  "fr" => "fr",  "de" => "de",
        "es" => "es",  "ru" => "ru",  "pt" | "pt-BR" => "pt-br",
        "it" => "it",  "nl" => "nl",  "pl" => "pl",  "ar" => "ar",
        "th" => "th",  "vi" => "vi",  "id" => "id",  "ms" => "ms",
        "tr" => "tr",  "uk" => "uk",  "cs" => "cs",  "sv" => "sv",
        "da" => "da",  "fi" => "fi",  "no" | "nb" => "nb",
        "hu" => "hu",  "el" => "el",  "ro" => "ro",  "bg" => "bg",
        "hi" => "hi",  "bn" => "bn",
        "auto" => "auto",
        _ => code,
    }
}

fn deepl_lang_code(code: &str) -> &str {
    match code {
        "zh" | "zh-CN" | "zh-Hans" => "ZH",
        "en" | "en-US" => "EN",
        "ja" => "JA",  "ko" => "KO",  "fr" => "FR",  "de" => "DE",
        "es" => "ES",  "ru" => "RU",  "pt" | "pt-BR" => "PT-BR",
        "it" => "IT",  "nl" => "NL",  "pl" => "PL",
        "sv" => "SV",  "da" => "DA",  "fi" => "FI",  "nb" | "no" => "NB",
        "hu" => "HU",  "el" => "EL",  "ro" => "RO",  "bg" => "BG",
        "cs" => "CS",  "tr" => "TR",  "uk" => "UK",  "id" => "ID",
        _ => code,
    }
}

// ==================== Tauri Commands ====================

#[tauri::command]
pub async fn ovr_translate(req: TranslateRequest) -> crate::AppResult<TranslateResult> {
    translate(&req).await.map_err(|e| e.into())
}
