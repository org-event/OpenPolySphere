//! OpenRouter HTTP helpers.

use anyhow::Result;
use serde_json::{json, Value};

use crate::settings::{Settings, OPENROUTER_MODELS_URL, TRANSLATION_API_URL, USER_AGENT};

pub async fn fetch_models(free_only: bool, sort: &str) -> Result<Vec<Value>> {
    let client = reqwest::Client::new();
    let mut url = format!("{OPENROUTER_MODELS_URL}?sort={sort}&output_modalities=text");
    if free_only {
        url.push_str("&max_price=0");
    }
    let resp = client
        .get(&url)
        .header("User-Agent", USER_AGENT)
        .send()
        .await?
        .json::<Value>()
        .await?;
    let mut models = Vec::new();
    if let Some(data) = resp.get("data").and_then(|d| d.as_array()) {
        for m in data {
            let pricing = m.get("pricing").cloned().unwrap_or(json!({}));
            let prompt_price = pricing.get("prompt").and_then(|p| p.as_str()).unwrap_or("");
            let id = m.get("id").and_then(|i| i.as_str()).unwrap_or("");
            let is_free = prompt_price == "0" || prompt_price == "0.0" || id.ends_with(":free");
            if free_only && !is_free {
                continue;
            }
            let ctx = m
                .get("context_length")
                .and_then(|c| c.as_u64())
                .unwrap_or(0);
            let ctx_label = if ctx >= 1000 {
                format!("{}K", ctx / 1000)
            } else {
                ctx.to_string()
            };
            models.push(json!({
                "id": id,
                "name": m.get("name").and_then(|n| n.as_str()).unwrap_or(id),
                "context_length": ctx,
                "context_label": ctx_label,
                "free": is_free,
                "description": m.get("description").and_then(|d| d.as_str()).unwrap_or("").chars().take(160).collect::<String>(),
            }));
        }
    }
    Ok(models)
}

pub async fn test_model(api_key: &str, model: &str) -> Value {
    let client = reqwest::Client::new();
    let body = json!({
        "model": model,
        "messages": [{"role": "user", "content": "hi"}],
        "max_tokens": 5,
    });
    match client
        .post(TRANSLATION_API_URL)
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Content-Type", "application/json")
        .header("User-Agent", USER_AGENT)
        .header("HTTP-Referer", "http://127.0.0.1:5050")
        .header("X-Title", "call-translator")
        .json(&body)
        .timeout(std::time::Duration::from_secs(12))
        .send()
        .await
    {
        Ok(resp) => {
            let status = resp.status();
            if status.is_success() {
                if let Ok(data) = resp.json::<Value>().await {
                    let sample = data["choices"][0]["message"]["content"]
                        .as_str()
                        .unwrap_or("")
                        .chars()
                        .take(80)
                        .collect::<String>();
                    return json!({ "ok": true, "model": model, "sample": sample });
                }
            } else {
                let text = resp.text().await.unwrap_or_default();
                return json!({
                    "ok": false,
                    "model": model,
                    "key_valid": status.as_u16() != 401,
                    "rate_limited": status.as_u16() == 429,
                    "message": text.chars().take(300).collect::<String>(),
                });
            }
            json!({ "ok": false, "model": model, "message": "unexpected response" })
        }
        Err(e) => json!({ "ok": false, "model": model, "message": e.to_string() }),
    }
}

pub async fn chat_completion(
    api_key: &str,
    model: &str,
    messages: Vec<Value>,
    temperature: f32,
    max_tokens: Option<u32>,
) -> Result<String> {
    let client = reqwest::Client::new();
    let mut body = json!({
        "model": model,
        "messages": messages,
        "temperature": temperature,
    });
    if let Some(n) = max_tokens {
        body["max_tokens"] = json!(n);
    }
    let resp = client
        .post(TRANSLATION_API_URL)
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Content-Type", "application/json")
        .header("User-Agent", USER_AGENT)
        .header("HTTP-Referer", "http://127.0.0.1:5050")
        .header("X-Title", "call-translator")
        .json(&body)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await?
        .error_for_status()?
        .json::<Value>()
        .await?;
    Ok(resp["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .trim()
        .to_string())
}

pub fn build_translation_messages(text: &str, from_lang: &str, to_lang: &str) -> Vec<Value> {
    let lang_names = [
        ("ru", "Russian"),
        ("en", "English"),
        ("de", "German"),
        ("fr", "French"),
        ("es", "Spanish"),
    ];
    let from_name = lang_names
        .iter()
        .find(|(c, _)| *c == from_lang)
        .map(|(_, n)| *n)
        .unwrap_or(from_lang);
    let to_name = lang_names
        .iter()
        .find(|(c, _)| *c == to_lang)
        .map(|(_, n)| *n)
        .unwrap_or(to_lang);
    let system = format!(
        "You are a machine translation API, not a chatbot. Two humans are on a phone call; you never speak, only translate their words.\n\
         Rules:\n- Input: a sentence in {from_name}. Output: the same sentence in {to_name}. Nothing else.\n\
         - NEVER answer questions — translate them.\n- No greetings or extra words."
    );
    let prefix = format!("Translate {from_name} to {to_name}:\n");
    let examples: &[(&str, &str)] = if from_lang == "ru" {
        &[
            ("кто ты", "Who are you?"),
            ("привет", "Hello."),
            ("мне нужна помощь", "I need help."),
        ]
    } else {
        &[
            ("who are you", "Кто ты?"),
            ("hello", "Привет."),
            ("I need help", "Мне нужна помощь."),
        ]
    };
    let mut messages = vec![json!({ "role": "system", "content": system })];
    for (src, dst) in examples {
        messages.push(json!({ "role": "user", "content": format!("{prefix}{src}") }));
        messages.push(json!({ "role": "assistant", "content": dst }));
    }
    messages.push(json!({ "role": "user", "content": format!("{prefix}{text}") }));
    messages
}

pub async fn translate_text(text: &str, from_lang: &str, to_lang: &str) -> Result<String> {
    let settings = Settings::load()?;
    if settings.translation_backend() == "openrouter"
        || settings.translation_backend() == "cloud"
        || settings.translation_backend() == "llm"
    {
        let key = settings.openrouter_key();
        if key.is_empty() {
            anyhow::bail!("OPENROUTER_API_KEY not set");
        }
        let messages = build_translation_messages(text, from_lang, to_lang);
        chat_completion(&key, &settings.translation_model(), messages, 0.0, Some(80)).await
    } else {
        let direction = audio_core::translation::TranslationDirection::new(from_lang, to_lang);
        let engine = audio_core::translation::TranslationEngine::new()?;
        engine.translate(text, &direction)
    }
}

pub async fn summarize_call(call_id: i64, db: &crate::Db) -> Result<String> {
    let rows = db.utterances_for_summary(call_id)?;
    if rows.is_empty() {
        anyhow::bail!("no utterances");
    }
    let mut lines = Vec::new();
    for (speaker, original, translated) in rows {
        let label = if speaker == "me" { "Me" } else { "Them" };
        lines.push(format!("{label}: {original}"));
        lines.push(format!("{label} (translated): {translated}"));
    }
    let settings = Settings::load()?;
    let key = settings.openrouter_key();
    if key.is_empty() {
        anyhow::bail!("openrouter_api_key not set");
    }
    let prompt = format!(
        "Summarize this call transcript in 3-5 bullet points. Include key topics, decisions, and action items. Write in the language of the 'Me' speaker.\n\n{}",
        lines.join("\n")
    );
    chat_completion(
        &key,
        &settings.translation_model(),
        vec![json!({ "role": "user", "content": prompt })],
        0.3,
        None,
    )
    .await
}
