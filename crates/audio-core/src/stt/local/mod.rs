//! Local speech-to-text: whisper.cpp + Metal (macOS) or CTranslate2 CPU.

mod common;
mod ct2;

#[cfg(target_os = "macos")]
mod metal;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::Result;
use log::info;

use crate::platform::Capabilities;

pub use common::is_whisper_hallucination;
pub use common::TranscribeOutcome;
pub use common::WhisperBackend;
pub use common::WhisperVariant;
pub use common::WHISPER_SAMPLE_RATE;

use common::{
    default_stt_device, is_ct2_ready, is_variant_ready_for_device, list_ggml_installed,
    models_base_dir, stt_device, variant_dir,
};

pub struct LocalWhisperEngine {
    backend: Arc<dyn WhisperBackend>,
}

impl LocalWhisperEngine {
    fn new() -> Result<Self> {
        let device = stt_device();
        let caps = Capabilities::current();
        let backend: Arc<dyn WhisperBackend> = match device.as_str() {
            "metal" | "gpu" if caps.whisper_metal => match metal::MetalWhisperEngine::new() {
                Ok(engine) => {
                    info!("STT compute: Metal GPU (whisper.cpp)");
                    Arc::new(engine)
                }
                Err(e) => {
                    info!("Metal Whisper unavailable ({e:#}), falling back to CPU CT2");
                    Arc::new(ct2::Ct2WhisperEngine::new()?)
                }
            },
            "cpu" | "ct2" => {
                info!("STT compute: CPU (CTranslate2)");
                Arc::new(ct2::Ct2WhisperEngine::new()?)
            }
            other if caps.whisper_metal => {
                info!("Unknown STT device '{other}', trying Metal");
                match metal::MetalWhisperEngine::new() {
                    Ok(engine) => Arc::new(engine),
                    Err(_) => Arc::new(ct2::Ct2WhisperEngine::new()?),
                }
            }
            other => {
                info!("Unknown STT device '{other}', using CPU");
                Arc::new(ct2::Ct2WhisperEngine::new()?)
            }
        };

        Ok(Self { backend })
    }

    fn backend(&self) -> Arc<dyn WhisperBackend> {
        self.backend.clone()
    }
}

pub struct LocalWhisperSession {
    inner: common::LocalWhisperSession,
}

impl LocalWhisperSession {
    pub fn new(engine: Arc<LocalWhisperEngine>, language: String, endpointing_ms: u32) -> Self {
        Self::from_backend(engine.backend(), language, endpointing_ms)
    }

    pub fn from_backend(
        backend: Arc<dyn WhisperBackend>,
        language: String,
        endpointing_ms: u32,
    ) -> Self {
        Self {
            inner: common::LocalWhisperSession::new(backend, language, endpointing_ms),
        }
    }

    pub fn reset_buffer(&mut self) {
        self.inner.reset_buffer();
    }

    pub fn send_audio(&mut self, samples: &[f32]) -> Result<()> {
        self.inner.send_audio(samples)
    }

    pub fn poll_transcript(&mut self) -> Result<Option<crate::stt::SttResult>> {
        self.inner.poll_transcript()
    }

    pub fn close(&mut self) {
        self.inner.close();
    }
}

pub fn resolve_whisper_model_dir() -> PathBuf {
    let device = stt_device();
    let base = models_base_dir();
    let pref = std::env::var("TRANSLATOR_WHISPER_MODEL").unwrap_or_else(|_| "auto".into());
    common::resolve_whisper_pref(&base, &pref.to_lowercase(), &device)
}

pub fn resolve_whisper_pref(base: &Path, pref: &str) -> PathBuf {
    common::resolve_whisper_pref(base, pref, &stt_device())
}

pub fn list_installed_whisper_variants() -> Vec<String> {
    list_installed_whisper_variants_for(&stt_device())
}

pub fn list_installed_whisper_variants_for(device: &str) -> Vec<String> {
    let base = models_base_dir();
    if device == "cpu" || device == "ct2" {
        ["tiny", "base", "small"]
            .iter()
            .filter(|name| is_ct2_ready(&variant_dir(&base, name)))
            .map(|name| format!("whisper-{name}"))
            .collect()
    } else {
        list_ggml_installed(&base)
    }
}

/// All variants with either GGML (Metal) or CT2 (CPU) weights on disk.
pub fn list_all_installed_whisper_variants() -> Vec<String> {
    let base = models_base_dir();
    let mut out = list_ggml_installed(&base);
    for name in ["tiny", "base", "small"] {
        let label = format!("whisper-{name}");
        if is_ct2_ready(&variant_dir(&base, name)) && !out.iter().any(|x| x == &label) {
            out.push(label);
        }
    }
    out
}

pub fn is_variant_ready(pref: &str) -> bool {
    is_variant_ready_for(pref, &stt_device())
}

pub fn is_variant_ready_for(pref: &str, device: &str) -> bool {
    if pref == "auto" {
        return !list_installed_whisper_variants_for(device).is_empty();
    }
    is_variant_ready_for_device(&models_base_dir(), pref, device)
}

pub fn whisper_model_status() -> (String, bool) {
    let device = stt_device();
    let pref = std::env::var("TRANSLATOR_WHISPER_MODEL").unwrap_or_else(|_| "auto".into());
    let base = models_base_dir();
    let ready = if pref == "auto" {
        !list_installed_whisper_variants().is_empty()
    } else {
        is_variant_ready_for_device(&base, &pref, &device)
    };
    let label = if device == "metal" || device == "gpu" {
        format!("whisper-{} (Metal)", pref)
    } else {
        format!("whisper-{} (CPU)", pref)
    };
    (label, ready)
}

pub fn stt_device_name() -> String {
    stt_device()
}

pub fn default_stt_device_name() -> String {
    default_stt_device()
}

static LOCAL_ENGINE: Mutex<Option<Arc<LocalWhisperEngine>>> = Mutex::new(None);

pub fn shared_engine() -> Result<Arc<LocalWhisperEngine>> {
    let mut guard = LOCAL_ENGINE.lock().unwrap();
    if let Some(engine) = guard.as_ref() {
        return Ok(engine.clone());
    }
    let engine = Arc::new(LocalWhisperEngine::new()?);
    *guard = Some(engine.clone());
    Ok(engine)
}

pub fn invalidate_engine_cache() {
    *LOCAL_ENGINE.lock().unwrap() = None;
}

pub fn is_ct2_dir_ready(path: &Path) -> bool {
    is_ct2_ready(path)
}

pub fn whisper_variant_dir(variant: &str) -> PathBuf {
    variant_dir(&models_base_dir(), WhisperVariant::parse(variant).core)
}

pub fn ggml_download_target(pref: &str) -> (String, PathBuf) {
    let v = WhisperVariant::parse(pref);
    let label = format!("whisper-{}", v.core);
    let path = common::ggml_model_path(&models_base_dir(), pref);
    (label, path)
}
