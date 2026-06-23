//! Translation backends: local CTranslate2 (default), Apple (macOS), or OpenRouter cloud.

mod apple;
mod backend;
mod local;
mod model;
mod normalize;
mod openrouter;
mod polish;
mod polish_model;

pub use apple::availability_for_settings as apple_translation_availability;

pub use polish::{is_session_disabled, polish_backend_label, reset_session};
pub use polish_model::{
    invalidate_engine_cache as invalidate_polish_cache, model_status as polish_model_status,
};

use anyhow::Result;
use log::info;

use crate::platform::Capabilities;

pub use openrouter::OpenRouterClient;

use backend::TranslateBackend;

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

pub struct TranslationEngine {
    backend: Box<dyn TranslateBackend>,
    polisher: Option<polish::TranslationPolisher>,
    apply_polish: bool,
}

impl TranslationEngine {
    pub fn new() -> Result<Self> {
        let mode = std::env::var("TRANSLATION_BACKEND")
            .unwrap_or_else(|_| "local".into())
            .to_lowercase();
        let polish_enabled = std::env::var("TRANSLATION_POLISH")
            .map(|v| v != "0" && v.to_lowercase() != "false")
            .unwrap_or(true);
        let polish_for_backend = polish_enabled && mode != "apple" && mode != "system";
        polish::reset_session();
        let polisher = if polish_for_backend {
            match polish::TranslationPolisher::try_new() {
                Some(p) => {
                    info!(
                        "Translation polish: {} (fixes STT/MT errors)",
                        polish_backend_label()
                    );
                    Some(p)
                }
                None => {
                    if mode == "local" {
                        info!(
                            "Translation polish: off (download Qwen2.5-0.5B from Settings → Translation)"
                        );
                    }
                    None
                }
            }
        } else {
            None
        };

        let (backend, apply_polish): (Box<dyn TranslateBackend>, bool) = match mode.as_str() {
            "openrouter" | "cloud" | "llm" => {
                info!("Translation backend: OpenRouter (cloud)");
                (Box::new(OpenRouterClient::new()?), false)
            }
            "apple" | "system" | "macos" => {
                info!("Translation backend: Apple Translation (system, on-device)");
                Capabilities::current().require_apple_translation()?;
                (Box::new(apple::AppleTranslateEngine::new()?), false)
            }
            _ => {
                info!("Translation backend: local Opus-MT + optional polish");
                (Box::new(local::LocalEngine::new()?), polish_for_backend)
            }
        };

        Ok(Self {
            backend,
            polisher,
            apply_polish,
        })
    }

    pub fn translate(&self, text: &str, direction: &TranslationDirection) -> Result<String> {
        if text.trim().is_empty() {
            return Ok(String::new());
        }

        let source = normalize::normalize_stt_text(text, direction);
        let draft = self.backend.translate(&source, direction)?;

        if self.apply_polish {
            Ok(polish::maybe_polish(
                true,
                &self.polisher,
                &source,
                &draft,
                direction,
            ))
        } else {
            Ok(draft)
        }
    }
}
