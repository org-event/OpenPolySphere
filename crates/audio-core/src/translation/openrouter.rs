//! Cloud translation via OpenAI-compatible API (OpenRouter).

use std::time::Duration;

use anyhow::{bail, Result};
use log::{info, warn};
use serde::Deserialize;

use super::TranslationDirection;

const DEFAULT_API_URL: &str = "https://openrouter.ai/api/v1/chat/completions";
const DEFAULT_MODEL: &str = "liquid/lfm-2.5-1.2b-instruct:free";
const DEFAULT_POLISH_MODEL: &str = "liquid/lfm-2.5-1.2b-instruct:free";
const DEFAULT_FALLBACK_MODELS: &[&str] = &[
    "liquid/lfm-2.5-1.2b-instruct:free",
    "meta-llama/llama-3.2-3b-instruct:free",
    "google/gemma-3-12b-it:free",
];
const DEFAULT_POLISH_FALLBACK_MODELS: &[&str] = DEFAULT_FALLBACK_MODELS;
/// Live-call polish must not block the pipeline on rate-limit retries.
const POLISH_REQUEST_TIMEOUT: Duration = Duration::from_secs(3);

fn is_model_fallback_error(status: u16) -> bool {
    matches!(status, 402 | 404 | 429 | 502 | 503)
}

impl TranslateFailure {
    fn is_quota_exhausted(&self) -> bool {
        self.message.contains("free-models-per-day")
            || self.message.contains("Add 5 credits to unlock")
    }
}

pub struct OpenRouterClient {
    api_key: String,
    api_url: String,
    model: String,
}

#[derive(Debug)]
struct TranslateFailure {
    status: u16,
    message: String,
    retry_after: Option<u64>,
    provider: Option<String>,
    model: String,
}

impl std::fmt::Display for TranslateFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "model {} HTTP {}: {}",
            self.model, self.status, self.message
        )?;
        if let Some(provider) = &self.provider {
            write!(f, " (provider: {provider})")?;
        }
        if let Some(retry) = self.retry_after {
            write!(f, " — retry after {retry}s")?;
        }
        Ok(())
    }
}

impl OpenRouterClient {
    pub fn new() -> Result<Self> {
        Self::from_env(false)
    }

    /// Load client when an API key exists (for optional polish pass).
    pub fn try_new() -> Result<Self> {
        Self::from_env(true)
    }

    fn from_env(optional: bool) -> Result<Self> {
        let api_key = std::env::var("OPENROUTER_API_KEY")
            .or_else(|_| std::env::var("GROQ_API_KEY"))
            .or_else(|_| std::env::var("TRANSLATION_API_KEY"))
            .unwrap_or_default();
        if api_key.is_empty() {
            if optional {
                bail!("no OpenRouter API key");
            } else {
                bail!("OPENROUTER_API_KEY is not set (TRANSLATION_BACKEND=openrouter)");
            }
        }
        let api_url = std::env::var("TRANSLATION_API_URL")
            .unwrap_or_else(|_| DEFAULT_API_URL.into());
        let model = std::env::var("TRANSLATION_MODEL")
            .unwrap_or_else(|_| DEFAULT_MODEL.into());
        if !optional {
            info!("OpenRouter translation ready ({model})");
        }
        Ok(Self {
            api_key,
            api_url,
            model,
        })
    }

    /// Fix a draft MT output using the original STT transcript as context.
    pub fn polish(
        &self,
        source: &str,
        draft: &str,
        direction: &TranslationDirection,
    ) -> Result<String> {
        let from = &direction.from_name;
        let to = &direction.to_name;
        let system = format!(
            "You correct live phone-call translations. Speech-to-text and machine translation \
             both make mistakes.\n\
             Input: original {from} utterance + draft {to} translation.\n\
             Output ONLY the corrected {to} sentence to be spoken aloud.\n\
             Rules:\n\
             - Fix obvious STT/MT errors using context (tech terms, grammar, poetry rhythm).\n\
             - Preserve meaning and tone; do not add or remove ideas.\n\
             - Never answer questions — translate/correct them.\n\
             - No explanations, quotes, or prefixes."
        );
        let user = format!(
            "Original ({from}):\n{source}\n\nDraft ({to}):\n{draft}\n\nCorrected ({to}):"
        );
        let body = serde_json::json!({
            "model": self.model,
            "messages": [
                {"role": "system", "content": system},
                {"role": "user", "content": user}
            ],
            "max_tokens": 256,
            "temperature": 0.1
        });
        let model = std::env::var("TRANSLATION_POLISH_MODEL")
            .or_else(|_| std::env::var("TRANSLATION_MODEL"))
            .unwrap_or_else(|_| DEFAULT_POLISH_MODEL.into());
        let mut body = body.clone();
        if let Some(obj) = body.as_object_mut() {
            obj.insert("model".into(), serde_json::Value::String(model));
        }
        self.request_polish(&body)
    }

    fn request_polish(&self, body: &serde_json::Value) -> Result<String> {
        let primary = body
            .get("model")
            .and_then(|m| m.as_str())
            .unwrap_or(&self.model)
            .to_string();
        let mut models = vec![primary];
        if let Ok(fallbacks) = std::env::var("TRANSLATION_POLISH_FALLBACK_MODELS")
            .or_else(|_| std::env::var("TRANSLATION_FALLBACK_MODELS"))
        {
            for m in fallbacks.split(',') {
                let m = m.trim();
                if !m.is_empty() && !models.iter().any(|x| x == m) {
                    models.push(m.to_string());
                }
            }
        } else {
            for fb in DEFAULT_POLISH_FALLBACK_MODELS {
                if *fb != models[0].as_str() && !models.iter().any(|x| x == *fb) {
                    models.push((*fb).to_string());
                }
            }
        }
        let mut last_err: Option<TranslateFailure> = None;
        let primary_model = models[0].clone();
        for model in models {
            match self.request_once(body, &model, Some(POLISH_REQUEST_TIMEOUT)) {
                Ok(text) if !text.trim().is_empty() => {
                    if model != primary_model {
                        info!("Polish succeeded via fallback model {model}");
                    }
                    return Ok(text);
                }
                Ok(_) => {}
                Err(e) if e.is_quota_exhausted() => return Err(anyhow::anyhow!("{e}")),
                Err(e) if is_model_fallback_error(e.status) => {
                    warn!("Polish model {} unavailable ({})", model, e);
                    last_err = Some(e);
                }
                Err(e) => return Err(anyhow::anyhow!("{e}")),
            }
        }
        Err(anyhow::anyhow!(
            "{}",
            last_err.unwrap_or(TranslateFailure {
                status: 503,
                message: "All polish models failed".into(),
                retry_after: None,
                provider: None,
                model: primary_model,
            })
        ))
    }

    pub fn translate(&self, text: &str, direction: &TranslationDirection) -> Result<String> {
        if text.trim().is_empty() {
            return Ok(String::new());
        }

        let body = self.build_request_body(text, direction);
        let models = self.models_to_try();
        let mut last_err: Option<TranslateFailure> = None;

        for model in models {
            for attempt in 0..3 {
                match self.request_once(&body, &model, None) {
                    Ok(translated) => {
                        if model != self.model {
                            info!("Translation succeeded via fallback model {model}");
                        }
                        return Ok(translated);
                    }
                    Err(err) if err.status == 429 && attempt < 2 => {
                        let delay = err.retry_after.unwrap_or(2 + attempt as u64).min(30);
                        warn!(
                            "Model {} rate-limited (attempt {}), retry in {}s",
                            model,
                            attempt + 1,
                            delay
                        );
                        std::thread::sleep(Duration::from_secs(delay));
                        last_err = Some(err);
                    }
                    Err(err) if err.status == 429 => {
                        warn!("Model {} rate-limited, trying next model", model);
                        last_err = Some(err);
                        break;
                    }
                    Err(err) if is_model_fallback_error(err.status) => {
                        warn!("Model {} unavailable ({}), trying next model", model, err);
                        last_err = Some(err);
                        break;
                    }
                    Err(err) => return Err(anyhow::anyhow!("{err}")),
                }
            }
        }

        Err(anyhow::anyhow!(
            "{}",
            last_err.unwrap_or(TranslateFailure {
                status: 429,
                message: "All translation models rate-limited".into(),
                retry_after: None,
                provider: None,
                model: self.model.clone(),
            })
        ))
    }

    fn models_to_try(&self) -> Vec<String> {
        let mut models = vec![self.model.clone()];
        if let Ok(fallbacks) = std::env::var("TRANSLATION_FALLBACK_MODELS") {
            for m in fallbacks.split(',') {
                let m = m.trim();
                if !m.is_empty() && !models.iter().any(|x| x == m) {
                    models.push(m.to_string());
                }
            }
        } else {
            for fb in DEFAULT_FALLBACK_MODELS {
                if *fb != self.model.as_str() && !models.iter().any(|x| x == *fb) {
                    models.push((*fb).to_string());
                }
            }
        }
        models
    }

    fn build_request_body(
        &self,
        text: &str,
        direction: &TranslationDirection,
    ) -> serde_json::Value {
        let from = &direction.from_name;
        let to = &direction.to_name;
        let system_prompt = format!(
            "You are a machine translation API, not a chatbot. \
             Two humans are on a phone call; you never speak, only translate their words.\n\
             Rules:\n\
             - Input: a sentence in {from}. Output: the same sentence in {to}. Nothing else.\n\
             - NEVER answer questions — translate them.\n\
             - NEVER say you are a translator or offer help.\n\
             - No greetings, explanations, or extra words."
        );
        let prefix = format!("Translate {from} to {to}:\n");
        let examples: &[(&str, &str)] = if direction.from_code == "ru" {
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

        let mut messages = vec![serde_json::json!({"role": "system", "content": system_prompt})];
        for (src, dst) in examples {
            messages.push(serde_json::json!({"role": "user", "content": format!("{prefix}{src}")}));
            messages.push(serde_json::json!({"role": "assistant", "content": dst}));
        }
        messages.push(serde_json::json!({"role": "user", "content": format!("{prefix}{text}")}));

        serde_json::json!({
            "model": self.model,
            "messages": messages,
            "max_tokens": 80,
            "temperature": 0.0
        })
    }

    fn request_once(
        &self,
        body: &serde_json::Value,
        model: &str,
        timeout: Option<Duration>,
    ) -> Result<String, TranslateFailure> {
        let mut req_body = body.clone();
        if let Some(obj) = req_body.as_object_mut() {
            obj.insert("model".into(), serde_json::Value::String(model.to_string()));
        }

        let mut req = ureq::post(&self.api_url)
            .set("Authorization", &format!("Bearer {}", self.api_key))
            .set("Content-Type", "application/json")
            .set("HTTP-Referer", "http://127.0.0.1:5050")
            .set("X-Title", "call-translator");
        if let Some(t) = timeout {
            req = req.timeout(t);
        }
        let response = match req.send_json(req_body) {
            Ok(resp) => resp,
            Err(ureq::Error::Status(code, resp)) => {
                let raw = resp.into_string().unwrap_or_default();
                return Err(parse_error_response(code, model, &raw));
            }
            Err(e) => {
                return Err(TranslateFailure {
                    status: 0,
                    message: format!("{e:#}"),
                    retry_after: None,
                    provider: None,
                    model: model.to_string(),
                });
            }
        };

        let parsed: LlmResponse = response.into_json().map_err(|e| TranslateFailure {
            status: 500,
            message: format!("Failed to parse response: {e:#}"),
            retry_after: None,
            provider: None,
            model: model.to_string(),
        })?;

        Ok(parsed
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content.trim().to_string())
            .unwrap_or_default())
    }
}

fn parse_error_response(status: u16, model: &str, body: &str) -> TranslateFailure {
    let mut message = if body.is_empty() {
        format!("HTTP {status}")
    } else {
        body.to_string()
    };
    let mut retry_after = None;
    let mut provider = None;

    if let Ok(v) = serde_json::from_str::<serde_json::Value>(body) {
        let err = v.get("error").unwrap_or(&v);
        if let Some(msg) = err.get("message").and_then(|m| m.as_str()) {
            message = msg.to_string();
        }
        let meta = err.get("metadata").or_else(|| v.get("metadata"));
        if let Some(meta) = meta {
            provider = meta
                .get("provider_name")
                .and_then(|p| p.as_str())
                .map(str::to_string);
            retry_after = meta
                .get("retry_after_seconds")
                .and_then(|r| r.as_u64())
                .or_else(|| {
                    meta.get("retry_after_seconds_raw")
                        .and_then(|r| r.as_f64())
                        .map(|f| f.ceil() as u64)
                });
            if let Some(raw) = meta.get("raw").and_then(|r| r.as_str()) {
                message = raw.to_string();
            }
        }
    }

    TranslateFailure {
        status,
        message,
        retry_after,
        provider,
        model: model.to_string(),
    }
}

#[derive(Deserialize)]
struct LlmResponse {
    choices: Vec<LlmChoice>,
}

#[derive(Deserialize)]
struct LlmChoice {
    message: LlmMessage,
}

#[derive(Deserialize)]
struct LlmMessage {
    content: String,
}
