//! Local translation polish via CTranslate2 Generator (Qwen2.5-0.5B-Instruct).
//!
//! Same runtime stack as Whisper STT and Opus-MT translation.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::{bail, Context, Result};
use ct2rs::{Config, Device, GenerationOptions, Generator};
use log::info;

use super::TranslationDirection;

const MODEL_LABEL: &str = "qwen2.5-0.5b-instruct";
const IM_END: &str = concat!("<|", "im_end", "|>");

pub struct LocalPolishEngine {
    generator: Generator<ct2rs::tokenizers::auto::Tokenizer>,
}

impl LocalPolishEngine {
    pub fn new() -> Result<Self> {
        let dir = polish_model_dir();
        if !is_ready(&dir) {
            bail!(
                "Polish model not found in {}. Run setup or download from Settings.",
                dir.display()
            );
        }

        info!("Loading local polish model from {}", dir.display());
        let config = Config {
            device: Device::CPU,
            num_threads_per_replica: 0,
            ..Config::default()
        };
        let generator =
            Generator::new(&dir, &config).context("failed to load polish CTranslate2 Generator")?;
        info!("Local polish ready ({MODEL_LABEL}, CTranslate2)");
        Ok(Self { generator })
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

        let prompt = build_prompt(source, draft, direction);
        let options = GenerationOptions {
            beam_size: 1,
            max_length: 160,
            sampling_temperature: 0.1,
            ..Default::default()
        };

        let results = self
            .generator
            .generate_batch(&[prompt], &options, None)
            .context("polish generation failed")?;

        let text = results
            .into_iter()
            .next()
            .and_then(|(seqs, _)| seqs.into_iter().next())
            .unwrap_or_default();

        Ok(clean_output(&text))
    }
}

fn build_prompt(source: &str, draft: &str, direction: &TranslationDirection) -> String {
    let from = &direction.from_name;
    let to = &direction.to_name;
    let system = format!(
        "You correct live phone-call translations. Speech-to-text and machine translation \
         both make mistakes. Output ONLY the corrected {to} sentence to be spoken aloud. \
         Fix obvious errors using context. Preserve meaning. No explanations or quotes."
    );
    let user =
        format!("Original ({from}):\n{source}\n\nDraft ({to}):\n{draft}\n\nCorrected ({to}):");
    format!(
        "<|im_start|>system\n{system}\n{IM_END}\n<|im_start|>user\n{user}\n{IM_END}\n<|im_start|>assistant\n"
    )
}

fn clean_output(s: &str) -> String {
    s.trim()
        .trim_end_matches(IM_END)
        .lines()
        .next()
        .unwrap_or(s)
        .trim()
        .to_string()
}

pub fn models_base_dir() -> PathBuf {
    PathBuf::from(std::env::var("TRANSLATOR_MODELS_DIR").unwrap_or_else(|_| "./models".into()))
}

pub fn polish_model_dir() -> PathBuf {
    models_base_dir().join("polish").join(MODEL_LABEL)
}

pub fn is_ready(dir: &Path) -> bool {
    dir.join("model.bin").is_file() && dir.join("tokenizer.json").is_file()
}

pub fn model_status() -> (String, bool) {
    let dir = polish_model_dir();
    (MODEL_LABEL.into(), is_ready(&dir))
}

static ENGINE: Mutex<Option<Arc<LocalPolishEngine>>> = Mutex::new(None);

pub fn shared_engine() -> Result<Arc<LocalPolishEngine>> {
    let mut guard = ENGINE.lock().unwrap();
    if let Some(engine) = guard.as_ref() {
        return Ok(engine.clone());
    }
    let engine = Arc::new(LocalPolishEngine::new()?);
    *guard = Some(engine.clone());
    Ok(engine)
}

pub fn try_shared() -> Option<Arc<LocalPolishEngine>> {
    shared_engine().ok()
}

pub fn invalidate_engine_cache() {
    *ENGINE.lock().unwrap() = None;
}
