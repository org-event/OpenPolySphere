//! LLM polish pass: fix STT + Opus-MT errors using OpenRouter (optional API key).

use anyhow::Result;
use log::{debug, info, warn};

use super::openrouter::OpenRouterClient;
use super::TranslationDirection;

pub struct TranslationPolisher {
    client: OpenRouterClient,
}

impl TranslationPolisher {
    pub fn try_new() -> Option<Self> {
        OpenRouterClient::try_new().ok().map(|client| Self { client })
    }

    /// Rewrite draft translation using original utterance as context.
    pub fn polish(
        &self,
        source: &str,
        draft: &str,
        direction: &TranslationDirection,
    ) -> Result<String> {
        if draft.trim().is_empty() {
            return Ok(String::new());
        }
        let out = self.client.polish(source, draft, direction)?;
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

/// Polish if enabled and API key is configured; otherwise return draft.
pub fn maybe_polish(
    enabled: bool,
    polisher: &Option<TranslationPolisher>,
    source: &str,
    draft: &str,
    direction: &TranslationDirection,
) -> String {
    if !enabled {
        return draft.to_string();
    }
    let Some(p) = polisher else {
        return draft.to_string();
    };
    match p.polish(source, draft, direction) {
        Ok(fixed) => fixed,
        Err(e) => {
            warn!("Translation polish failed, using Opus-MT draft: {e:#}");
            draft.to_string()
        }
    }
}
