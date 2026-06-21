//! Local Whisper STT via CTranslate2 (CPU).

use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use ct2rs::{Config, Device, Whisper, WhisperOptions};
use log::info;

use super::common::{
    is_ct2_ready, models_base_dir, resolve_whisper_pref, TranscribeOutcome, WhisperBackend,
    MIN_PEAK_RMS, MIN_UTTERANCE_RMS,
};

pub struct Ct2WhisperEngine {
    whisper: Whisper,
}

impl Ct2WhisperEngine {
    pub fn new() -> Result<Self> {
        let model_dir = resolve_ct2_model_dir();
        if !is_ct2_ready(&model_dir) {
            bail!(
                "Whisper CT2 model not found in {}. Run setup or download (CPU mode).",
                model_dir.display()
            );
        }

        let model_label = model_dir
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "whisper".into());

        info!("Loading Whisper STT (CTranslate2 CPU) from {}", model_dir.display());
        let config = Config {
            device: Device::CPU,
            num_threads_per_replica: 0,
            ..Config::default()
        };
        let whisper = Whisper::new(&model_dir, config)
            .context("failed to load Whisper model (CTranslate2)")?;
        info!(
            "Whisper STT ready ({}, CPU, {}Hz, multilingual={})",
            model_label,
            whisper.sampling_rate(),
            whisper.is_multilingual()
        );
        Ok(Self { whisper })
    }
}

impl WhisperBackend for Ct2WhisperEngine {
    fn transcribe(&self, samples: &[f32], language: &str) -> Result<TranscribeOutcome> {
        let avg_rms = super::common::rms_energy(samples);
        if avg_rms < MIN_UTTERANCE_RMS {
            return Ok(TranscribeOutcome {
                text: String::new(),
                no_speech_prob: 1.0,
            });
        }

        let lang = super::common::whisper_language(language);
        let mut options = WhisperOptions::default();
        options.beam_size = 1;
        options.return_no_speech_prob = true;

        let results = self.whisper.generate(samples, Some(lang), false, &options)?;
        let text = results
            .into_iter()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" ");

        let no_speech_prob = if avg_rms < MIN_PEAK_RMS {
            0.9
        } else {
            (MIN_PEAK_RMS / avg_rms).clamp(0.0, 0.45)
        };

        Ok(TranscribeOutcome { text, no_speech_prob })
    }
}

pub fn resolve_ct2_model_dir() -> PathBuf {
    let base = models_base_dir();
    let pref = std::env::var("TRANSLATOR_WHISPER_MODEL").unwrap_or_else(|_| "auto".into());
    resolve_whisper_pref(&base, &pref.to_lowercase(), "cpu")
}
