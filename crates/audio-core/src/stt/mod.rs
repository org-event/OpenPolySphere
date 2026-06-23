//! Speech-to-text: local Whisper, Apple Speech (macOS), or Deepgram cloud.

pub mod apple;
mod deepgram;
pub mod local;

use anyhow::{bail, Result};
#[cfg(target_os = "macos")]
use anyhow::Context;
use log::info;

pub use deepgram::{deepgram_model_from_env, DeepgramSession, DeepgramStt};

/// Transcript with STT latency info.
pub struct SttResult {
    pub text: String,
    pub stt_latency_ms: u64,
}

enum SttBackend {
    Local,
    #[cfg(target_os = "macos")]
    Apple,
    Deepgram,
}

pub struct SttEngine {
    backend: SttBackend,
    deepgram_api_key: String,
    endpointing_ms: u32,
}

impl SttEngine {
    pub fn new(deepgram_api_key: String, endpointing_ms: u32) -> Result<Self> {
        let mode = std::env::var("STT_BACKEND")
            .unwrap_or_else(|_| "local".into())
            .to_lowercase();

        let backend = match mode.as_str() {
            "deepgram" | "cloud" => {
                if deepgram_api_key.trim().is_empty() {
                    bail!("STT_BACKEND=deepgram but DEEPGRAM_API_KEY is not set");
                }
                info!(
                    "STT backend: Deepgram (cloud, model={})",
                    deepgram_model_from_env()
                );
                SttBackend::Deepgram
            }
            "apple" | "system" | "macos" => {
                info!("STT backend: Apple Speech (system, on-device)");
                #[cfg(target_os = "macos")]
                {
                    apple::apple_speech_ensure_authorized()
                        .context("Apple Speech authorization")?;
                    apple::apple_speech_backend().context("Failed to load Apple Speech STT")?;
                    SttBackend::Apple
                }
                #[cfg(not(target_os = "macos"))]
                {
                    bail!("STT_BACKEND=apple but Apple Speech is only available on macOS");
                }
            }
            _ => {
                let device = local::stt_device_name();
                info!("STT backend: local Whisper ({device})");
                local::shared_engine().context("Failed to load local Whisper STT")?;
                SttBackend::Local
            }
        };

        Ok(Self {
            backend,
            deepgram_api_key,
            endpointing_ms,
        })
    }

    pub fn create_session(&self, sample_rate: u32, language: &str) -> Result<SttSession> {
        match self.backend {
            SttBackend::Deepgram => {
                let model = deepgram_model_from_env();
                let stt = DeepgramStt::new(
                    self.deepgram_api_key.clone(),
                    language.to_string(),
                    self.endpointing_ms,
                    model,
                );
                Ok(SttSession::Deepgram(Box::new(
                    stt.create_session(sample_rate)?,
                )))
            }
            #[cfg(target_os = "macos")]
            SttBackend::Apple => {
                #[cfg(target_os = "macos")]
                {
                    let backend = apple::apple_speech_backend()?;
                    Ok(SttSession::Local(local::LocalWhisperSession::from_backend(
                        backend,
                        language.to_string(),
                        self.endpointing_ms,
                    )))
                }
                #[cfg(not(target_os = "macos"))]
                {
                    bail!("Apple Speech STT is only available on macOS")
                }
            }
            SttBackend::Local => {
                let engine = local::shared_engine()?;
                Ok(SttSession::Local(local::LocalWhisperSession::new(
                    engine,
                    language.to_string(),
                    self.endpointing_ms,
                )))
            }
        }
    }
}

pub enum SttSession {
    Deepgram(Box<DeepgramSession>),
    Local(local::LocalWhisperSession),
}

impl SttSession {
    pub fn reset_buffer(&mut self) {
        match self {
            SttSession::Deepgram(_) => {}
            SttSession::Local(s) => s.reset_buffer(),
        }
    }

    pub fn send_audio(&mut self, samples: &[f32]) -> Result<()> {
        match self {
            SttSession::Deepgram(s) => s.send_audio(samples),
            SttSession::Local(s) => s.send_audio(samples),
        }
    }

    pub fn poll_transcript(&mut self) -> Result<Option<SttResult>> {
        match self {
            SttSession::Deepgram(s) => s.poll_transcript(),
            SttSession::Local(s) => s.poll_transcript(),
        }
    }

    pub fn close(&mut self) {
        match self {
            SttSession::Deepgram(s) => s.close(),
            SttSession::Local(s) => s.close(),
        }
    }
}
