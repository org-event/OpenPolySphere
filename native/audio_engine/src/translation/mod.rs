//! Translation via Groq API (llama-3.1-8b-instant).
//!
//! Replaces CTranslate2/Opus-MT with a Groq chat completion call.
//! Synchronous HTTP via ureq — fits the existing sync pipeline architecture.

use anyhow::{bail, Context, Result};
use log::{debug, info};
use serde::Deserialize;

/// Translation direction with source/target language names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranslationDirection {
    pub from_code: String,
    pub from_name: String,
    pub to_code: String,
    pub to_name: String,
}

impl TranslationDirection {
    pub fn new(from: &str, to: &str) -> Self {
        Self {
            from_code: from.to_string(),
            from_name: lang_name(from).to_string(),
            to_code: to.to_string(),
            to_name: lang_name(to).to_string(),
        }
    }
}

impl std::fmt::Display for TranslationDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}→{}", self.from_code, self.to_code)
    }
}

fn lang_name(code: &str) -> &str {
    match code {
        "ar" => "Arabic",
        "ca" => "Catalan",
        "cs" => "Czech",
        "cy" => "Welsh",
        "da" => "Danish",
        "de" => "German",
        "el" => "Greek",
        "en" => "English",
        "es" => "Spanish",
        "fa" => "Persian",
        "fi" => "Finnish",
        "fr" => "French",
        "hi" => "Hindi",
        "hu" => "Hungarian",
        "is" => "Icelandic",
        "it" => "Italian",
        "ja" => "Japanese",
        "ka" => "Georgian",
        "kk" => "Kazakh",
        "ko" => "Korean",
        "lb" => "Luxembourgish",
        "lv" => "Latvian",
        "ml" => "Malayalam",
        "ne" => "Nepali",
        "nl" => "Dutch",
        "no" => "Norwegian",
        "pl" => "Polish",
        "pt" => "Portuguese",
        "ro" => "Romanian",
        "ru" => "Russian",
        "sk" => "Slovak",
        "sl" => "Slovenian",
        "sr" => "Serbian",
        "sv" => "Swedish",
        "sw" => "Swahili",
        "tr" => "Turkish",
        "uk" => "Ukrainian",
        "vi" => "Vietnamese",
        "zh" => "Chinese",
        _ => code,
    }
}

#[derive(Clone)]
pub struct TranslationEngine {
    api_key: String,
}

impl TranslationEngine {
    pub fn new(api_key: &str) -> Result<Self> {
        if api_key.is_empty() {
            bail!("GROQ_API_KEY is not set");
        }
        info!("Translation engine ready (Groq llama-3.3-70b-versatile)");
        Ok(Self { api_key: api_key.to_string() })
    }

    pub fn translate(&self, text: &str, direction: &TranslationDirection) -> Result<String> {
        if text.trim().is_empty() {
            return Ok(String::new());
        }

        let system_prompt = format!(
            "You are a live interpreter in a phone call. \
             You hear {from}, you say the same thing in {to}. \
             You translate word for word. \
             You have no opinions, no knowledge, no personality. \
             You are a transparent pipe between two languages.\n\
             Rules:\n\
             - Output ONLY the {to} translation, nothing else.\n\
             - Keep the same tone, register, and emotion.\n\
             - Translate profanity as equivalent profanity.\n\
             - Keep names and proper nouns as-is (transliterate if needed).\n\
             - For filler words (well, uh, like / ну, ага, типа) use natural equivalents.\n\
             - Never add explanations, notes, or commentary.",
            from = direction.from_name,
            to = direction.to_name,
        );

        let body = serde_json::json!({
            "model": "llama-3.3-70b-versatile",
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user",   "content": text}
            ],
            "max_tokens": 80,
            "temperature": 0.1
        });

        debug!("Groq translate [{}]: {:?}", direction, text);

        let response: GroqResponse = ureq::post("https://api.groq.com/openai/v1/chat/completions")
            .set("Authorization", &format!("Bearer {}", self.api_key))
            .set("Content-Type", "application/json")
            .send_json(body)
            .context("Groq API request failed")?
            .into_json()
            .context("Failed to parse Groq response")?;

        let translated = response
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content.trim().to_string())
            .unwrap_or_default();

        info!("Groq [{}]: {:?} → {:?}", direction, text, translated);
        Ok(translated)
    }
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct GroqResponse {
    choices: Vec<GroqChoice>,
}

#[derive(Deserialize)]
struct GroqChoice {
    message: GroqMessage,
}

#[derive(Deserialize)]
struct GroqMessage {
    content: String,
}
