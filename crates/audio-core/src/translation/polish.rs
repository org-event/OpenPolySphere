//! Optional polish pass after Opus-MT: fix STT/MT errors using context.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use anyhow::Result;
use log::{debug, info, warn};

use super::openrouter::OpenRouterClient;
use super::polish_model::LocalPolishEngine;
use super::TranslationDirection;

static POLISH_SESSION_DISABLED: AtomicBool = AtomicBool::new(false);
static POLISH_WARNED: AtomicBool = AtomicBool::new(false);

enum Polisher {
    Local(Arc<LocalPolishEngine>),
    Cloud(OpenRouterClient),
}

pub struct TranslationPolisher {
    inner: Polisher,
}

impl TranslationPolisher {
    pub fn try_new() -> Option<Self> {
        let backend = std::env::var("TRANSLATION_POLISH_BACKEND")
            .unwrap_or_else(|_| "local".into())
            .to_lowercase();

        match backend.as_str() {
            "openrouter" | "cloud" => OpenRouterClient::try_new().ok().map(|client| Self {
                inner: Polisher::Cloud(client),
            }),
            _ => super::polish_model::try_shared().map(|engine| Self {
                inner: Polisher::Local(engine),
            }),
        }
    }

    pub fn polish(
        &self,
        source: &str,
        draft: &str,
        direction: &TranslationDirection,
    ) -> Result<String> {
        if draft.trim().is_empty() {
            return Ok(String::new());
        }

        let out = match &self.inner {
            Polisher::Local(engine) => engine.polish(source, draft, direction)?,
            Polisher::Cloud(client) => client.polish(source, draft, direction)?,
        };

        if out.trim().is_empty() {
            return Ok(draft.to_string());
        }
        if out.trim() == draft.trim() {
            debug!("Polish unchanged output");
        } else {
            info!(
                "Polish: \"{}\" → \"{}\"",
                draft.trim().chars().take(60).collect::<String>(),
                out.trim().chars().take(60).collect::<String>()
            );
        }
        Ok(out)
    }
}

pub fn reset_session() {
    POLISH_SESSION_DISABLED.store(false, Ordering::Relaxed);
    POLISH_WARNED.store(false, Ordering::Relaxed);
}

pub fn is_session_disabled() -> bool {
    POLISH_SESSION_DISABLED.load(Ordering::Relaxed)
}

fn disable_session(reason: &str) {
    POLISH_SESSION_DISABLED.store(true, Ordering::Relaxed);
    if !POLISH_WARNED.swap(true, Ordering::Relaxed) {
        warn!("Translation polish disabled for this session: {reason}");
    }
}

fn is_quota_error(msg: &str) -> bool {
    msg.contains("free-models-per-day") || msg.contains("Add 5 credits to unlock")
}

pub fn polish_backend_label() -> &'static str {
    let backend = std::env::var("TRANSLATION_POLISH_BACKEND")
        .unwrap_or_else(|_| "local".into())
        .to_lowercase();
    match backend.as_str() {
        "openrouter" | "cloud" => "OpenRouter",
        _ => "local Qwen2.5-0.5B (CTranslate2)",
    }
}

/// Polish if enabled and model is available; otherwise return draft.
pub fn maybe_polish(
    enabled: bool,
    polisher: &Option<TranslationPolisher>,
    source: &str,
    draft: &str,
    direction: &TranslationDirection,
) -> String {
    if !enabled || is_session_disabled() {
        return draft.to_string();
    }
    let Some(p) = polisher else {
        return draft.to_string();
    };
    match p.polish(source, draft, direction) {
        Ok(fixed) => fixed,
        Err(e) => {
            let msg = format!("{e:#}");
            if is_quota_error(&msg) {
                disable_session("OpenRouter free daily quota exhausted — using Opus-MT only");
            } else if !is_session_disabled() {
                warn!("Translation polish failed, using Opus-MT draft: {e:#}");
            }
            draft.to_string()
        }
    }
}
