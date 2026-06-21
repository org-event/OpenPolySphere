//! Ring-style buffer for mic audio captured while TTS echo gate is active.

const DEFAULT_MAX_SECONDS: f32 = 12.0;
const WHISPER_RATE: u32 = 16_000;

/// Holds 16 kHz mono samples recorded during echo suppression so speech is not lost.
pub struct PendingMicAudio {
    samples: Vec<f32>,
    max_samples: usize,
}

impl PendingMicAudio {
    pub fn new() -> Self {
        Self::with_max_seconds(DEFAULT_MAX_SECONDS)
    }

    pub fn with_max_seconds(secs: f32) -> Self {
        Self {
            samples: Vec::new(),
            max_samples: (WHISPER_RATE as f32 * secs) as usize,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    pub fn push(&mut self, chunk: &[f32]) {
        if chunk.is_empty() {
            return;
        }
        self.samples.extend_from_slice(chunk);
        if self.samples.len() > self.max_samples {
            let drop = self.samples.len() - self.max_samples;
            self.samples.drain(..drop);
        }
    }

    /// Take all buffered samples (caller feeds them to STT).
    pub fn take(&mut self) -> Vec<f32> {
        std::mem::take(&mut self.samples)
    }
}
