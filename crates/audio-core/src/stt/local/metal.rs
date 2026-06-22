//! Local Whisper STT via whisper.cpp + Metal (macOS GPU).

use std::path::PathBuf;
use std::sync::Mutex;

use anyhow::{bail, Context, Result};
use log::{info, warn};
use whisper_rs::{
    FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters, WhisperState,
};

use super::common::{
    is_ggml_ready, models_base_dir, resolve_whisper_pref, whisper_initial_prompt,
    TranscribeOutcome, WhisperBackend, MIN_UTTERANCE_RMS,
};

pub struct MetalWhisperEngine {
    state: Mutex<WhisperState>,
}

impl MetalWhisperEngine {
    pub fn new() -> Result<Self> {
        let model_path = resolve_ggml_model_path();
        if !is_ggml_ready(&model_path) {
            bail!(
                "Whisper GGML model not found at {}. Run setup or download (Metal mode).",
                model_path.display()
            );
        }

        let model_label = model_path
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "ggml".into());

        if model_label.contains("small") {
            warn!(
                "Whisper Metal: ggml-small is heavy on 4 GB GPU — tiny/base in Settings is faster"
            );
        }

        info!(
            "Loading Whisper STT (whisper.cpp + Metal) from {}",
            model_path.display()
        );

        let ctx = WhisperContext::new_with_params(
            model_path.to_string_lossy().as_ref(),
            WhisperContextParameters::default(),
        )
        .context("failed to load Whisper GGML model")?;

        let state = ctx
            .create_state()
            .context("failed to create Whisper Metal state")?;

        info!("Whisper STT ready ({model_label}, Metal GPU)");
        Ok(Self {
            state: Mutex::new(state),
        })
    }
}

impl WhisperBackend for MetalWhisperEngine {
    fn transcribe(&self, samples: &[f32], language: &str) -> Result<TranscribeOutcome> {
        let avg_rms = super::common::rms_energy(samples);
        if avg_rms < MIN_UTTERANCE_RMS {
            return Ok(TranscribeOutcome {
                text: String::new(),
                no_speech_prob: 1.0,
            });
        }

        let mut state = self
            .state
            .lock()
            .expect("Whisper Metal state lock poisoned");
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_n_threads(4);
        params.set_translate(false);
        params.set_detect_language(false);
        params.set_language(Some(super::common::whisper_language(language)));
        if let Some(prompt) = whisper_initial_prompt(language) {
            params.set_initial_prompt(prompt);
        }
        params.set_print_progress(false);
        params.set_print_special(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);

        state.full(params, samples)?;

        let n = state.full_n_segments();
        let mut text = String::new();
        let mut no_speech_prob = 0.0f32;
        for i in 0..n {
            if let Some(segment) = state.get_segment(i) {
                no_speech_prob = no_speech_prob.max(segment.no_speech_probability());
                let segment_text = segment.to_str_lossy()?.trim().to_string();
                if segment_text.is_empty() {
                    continue;
                }
                if !text.is_empty() {
                    text.push(' ');
                }
                text.push_str(&segment_text);
            }
        }

        if text.is_empty() && n > 0 {
            info!(
                "Whisper Metal: {n} segment(s) but empty text ({:.0}ms audio, rms={avg_rms:.4})",
                samples.len() as f64 * 1000.0 / 16_000.0
            );
        } else if n == 0 {
            info!(
                "Whisper Metal: 0 segments ({:.0}ms audio, rms={avg_rms:.4})",
                samples.len() as f64 * 1000.0 / 16_000.0
            );
        }

        Ok(TranscribeOutcome {
            text,
            no_speech_prob,
        })
    }
}

pub fn resolve_ggml_model_path() -> PathBuf {
    let base = models_base_dir();
    let pref = std::env::var("TRANSLATOR_WHISPER_MODEL").unwrap_or_else(|_| "auto".into());
    resolve_whisper_pref(&base, &pref.to_lowercase(), "metal")
}
