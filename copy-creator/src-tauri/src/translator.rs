use serde::{Deserialize, Serialize};
use tauri::Manager;

#[derive(Debug, Serialize, Deserialize)]
pub struct TranslateResponse {
    pub source_text: String,
    pub target_text: String,
    pub engine: String,
}

#[tauri::command]
pub async fn translate(
    app: tauri::AppHandle,
    text: String,
    target_lang: String,
) -> Result<TranslateResponse, String> {
    let source_lang = "auto".to_string();

    let state = app.state::<crate::db::DbState>();
    let engine = {
        let conn = state.conn.lock().map_err(|e| e.to_string())?;
        conn.query_row(
            "SELECT value FROM settings WHERE key = 'default_translate_engine'",
            [],
            |row| row.get::<_, String>(0),
        )
        .unwrap_or_else(|_| "google".to_string())
    };

    // Check cache
    {
        let conn = state.conn.lock().map_err(|e| e.to_string())?;
        let cached: Option<String> = conn
            .query_row(
                "SELECT target_text FROM translation_history WHERE source_text = ?1 AND target_lang = ?2 AND engine = ?3 ORDER BY created_at DESC LIMIT 1",
                rusqlite::params![text, target_lang, engine],
                |row| row.get(0),
            )
            .ok();
        if let Some(cached_text) = cached {
            return Ok(TranslateResponse {
                source_text: text,
                target_text: cached_text,
                engine,
            });
        }
    }

    let result = if engine == "ai" {
        translate_ai(&app, &text, &source_lang, &target_lang).await?
    } else if engine == "google" {
        translate_google(&app, &text, &source_lang, &target_lang).await?
    } else {
        translate_baidu(&app, &text, &source_lang, &target_lang).await?
    };

    // Save to history/cache
    {
        let conn = state.conn.lock().map_err(|e| e.to_string())?;
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO translation_history (id, source_text, target_text, source_lang, target_lang, engine, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![id, text, result.target_text, source_lang, target_lang, engine, &now],
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(result)
}

async fn translate_ai(
    app: &tauri::AppHandle,
    text: &str,
    source_lang: &str,
    target_lang: &str,
) -> Result<TranslateResponse, String> {
    let state = app.state::<crate::db::DbState>();
    let (api_url, api_key, model) = {
        let conn = state.conn.lock().map_err(|e| e.to_string())?;
        let url: String = conn.query_row(
            "SELECT value FROM settings WHERE key = 'ai_api_url'", [], |r| r.get(0),
        ).unwrap_or_default();
        let key: String = conn.query_row(
            "SELECT value FROM settings WHERE key = 'ai_api_key'", [], |r| r.get(0),
        ).unwrap_or_default();
        let m: String = conn.query_row(
            "SELECT value FROM settings WHERE key = 'ai_model'", [], |r| r.get(0),
        ).unwrap_or_else(|_| "gpt-3.5-turbo".to_string());
        (url, key, m)
    };

    if api_url.is_empty() || api_key.is_empty() {
        return Err("AI 翻译未配置，请在设置中填写 API 地址和 Key".to_string());
    }

    let prompt = format!(
        "Translate the following text from {source} to {target}. Only output the translated text, nothing else.\n\nText: {text}",
        source = if source_lang == "auto" { "auto-detected language" } else { source_lang },
        target = target_lang,
        text = text
    );

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;

    let resp = client
        .post(&api_url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "model": model,
            "messages": [
                {"role": "system", "content": "You are a professional translator. Only output the translated text."},
                {"role": "user", "content": prompt}
            ],
            "temperature": 0.3
        }))
        .send().await.map_err(|e| format!("AI 翻译请求失败: {}", e))?;

    let status = resp.status();
    let body_text = resp.text().await.map_err(|e| format!("读取响应失败: {}", e))?;

    if !status.is_success() {
        return Err(format!("AI 翻译 HTTP {}: {}", status.as_u16(), body_text.chars().take(200).collect::<String>()));
    }

    let json: serde_json::Value = serde_json::from_str(&body_text)
        .map_err(|e| format!("解析响应失败: {}。原始响应: {}", e, body_text.chars().take(300).collect::<String>()))?;

    let translated = json["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| format!("AI 响应格式异常，未找到 choices[0].message.content。响应: {}", body_text.chars().take(300).collect::<String>()))?
        .trim()
        .to_string();

    Ok(TranslateResponse {
        source_text: text.to_string(),
        target_text: translated,
        engine: "ai".to_string(),
    })
}

async fn translate_baidu(
    app: &tauri::AppHandle,
    text: &str,
    source_lang: &str,
    target_lang: &str,
) -> Result<TranslateResponse, String> {
    let state = app.state::<crate::db::DbState>();
    let (appid, secret) = {
        let conn = state.conn.lock().map_err(|e| e.to_string())?;
        let id: String = conn.query_row(
            "SELECT value FROM settings WHERE key = 'baidu_appid'", [], |r| r.get(0),
        ).unwrap_or_default();
        let sk: String = conn.query_row(
            "SELECT value FROM settings WHERE key = 'baidu_secret'", [], |r| r.get(0),
        ).unwrap_or_default();
        (id, sk)
    };

    if appid.is_empty() || secret.is_empty() {
        return Err("百度翻译未配置，请在设置中填写百度翻译 AppID 和密钥".to_string());
    }

    let from = if source_lang == "auto" { "auto" } else { source_lang };
    let salt = rand::random::<u32>().to_string();
    let sign = format!("{:x}", md5::compute(format!("{}{}{}{}", appid, text, salt, secret)));

    let client = reqwest::Client::new();
    let resp = client
        .post("https://fanyi-api.baidu.com/api/trans/vip/translate")
        .form(&[
            ("q", text),
            ("from", from),
            ("to", target_lang),
            ("appid", &appid),
            ("salt", &salt),
            ("sign", &sign),
        ])
        .send().await.map_err(|e| format!("百度翻译请求失败: {}", e))?;

    let json: serde_json::Value = resp.json().await.map_err(|e| format!("解析百度响应失败: {}", e))?;

    if let Some(err_msg) = json.get("error_msg").and_then(|v| v.as_str()) {
        let err_code = json.get("error_code").and_then(|v| v.as_str()).unwrap_or("");
        return Err(format!("百度翻译错误 [{}]: {}", err_code, err_msg));
    }

    let translated = json["trans_result"][0]["dst"]
        .as_str()
        .unwrap_or("翻译失败")
        .to_string();

    Ok(TranslateResponse {
        source_text: text.to_string(),
        target_text: translated,
        engine: "builtin".to_string(),
    })
}

async fn translate_google(
    app: &tauri::AppHandle,
    text: &str,
    _source_lang: &str,
    target_lang: &str,
) -> Result<TranslateResponse, String> {
    let state = app.state::<crate::db::DbState>();
    let api_key: String = {
        let conn = state.conn.lock().map_err(|e| e.to_string())?;
        conn.query_row(
            "SELECT value FROM settings WHERE key = 'google_api_key'", [], |r| r.get(0),
        ).unwrap_or_default()
    };

    let client = reqwest::Client::new();

    // Free Google Translate API (unofficial endpoint, no key required)
    if api_key.is_empty() {
        let url = format!(
            "https://translate.googleapis.com/translate_a/single?client=gtx&sl=auto&tl={}&dt=t&q={}",
            target_lang,
            urlencoding(&text)
        );

        let resp = client
            .get(&url)
            .send().await.map_err(|e| format!("Google 翻译请求失败: {}", e))?;

        let body = resp.text().await.map_err(|e| format!("读取 Google 响应失败: {}", e))?;

        // Parse the unofficial Google Translate JSON format
        // Response: [[["translated text", "source", ...]], ...]
        let json: serde_json::Value = serde_json::from_str(&body)
            .map_err(|e| format!("解析 Google 响应失败: {}", e))?;

        let translated = json[0][0][0]
            .as_str()
            .unwrap_or("翻译失败")
            .to_string();

        return Ok(TranslateResponse {
            source_text: text.to_string(),
            target_text: translated,
            engine: "google".to_string(),
        });
    }

    // Official Google Cloud Translation API (with API key)
    let url = format!(
        "https://translation.googleapis.com/language/translate/v2?key={}",
        api_key
    );

    let resp = client
        .post(&url)
        .json(&serde_json::json!({
            "q": text,
            "target": target_lang,
            "format": "text"
        }))
        .send().await.map_err(|e| format!("Google 翻译请求失败: {}", e))?;

    let json: serde_json::Value = resp.json().await
        .map_err(|e| format!("解析 Google 响应失败: {}", e))?;

    if let Some(error) = json.get("error") {
        return Err(format!("Google 翻译错误: {}", error["message"].as_str().unwrap_or("未知错误")));
    }

    let translated = json["data"]["translations"][0]["translatedText"]
        .as_str()
        .unwrap_or("翻译失败")
        .to_string();

    Ok(TranslateResponse {
        source_text: text.to_string(),
        target_text: translated,
        engine: "google".to_string(),
    })
}

fn urlencoding(s: &str) -> String {
    let mut result = String::with_capacity(s.len() * 3);
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(byte as char);
            }
            b' ' => result.push('+'),
            _ => {
                result.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    result
}
