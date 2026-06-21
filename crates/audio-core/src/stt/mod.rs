//! Speech-to-text: local Whisper (CTranslate2) or Deepgram cloud.

mod deepgram;
pub mod local;

use anyhow::{bail, Context, Result};
use log::info;

pub use deepgram::{DeepgramSession, DeepgramStt};

/// Transcript with STT latency info.
pub struct SttResult {
    pub text: String,
    pub stt_latency_ms: u64,
}

enum SttBackend {
    Local,
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
                info!("STT backend: Deepgram (cloud)");
                SttBackend::Deepgram
            }
            _ => {
                info!("STT backend: local Whisper (CTranslate2)");
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
                let stt = DeepgramStt::new(
                    self.deepgram_api_key.clone(),
                    language.to_string(),
                    self.endpointing_ms,
                );
                Ok(SttSession::Deepgram(stt.create_session(sample_rate)?))
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
    Deepgram(DeepgramSession),
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
