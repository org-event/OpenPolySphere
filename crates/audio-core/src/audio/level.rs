use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::thread;

use anyhow::Result;
use crossbeam_channel::Receiver;

use super::capture::{AudioCapture, AudioChunk};

/// Shared peak meter (0.0–1.0) updated from capture callbacks.
pub fn new_level_atomic() -> Arc<AtomicU32> {
    Arc::new(AtomicU32::new(0))
}

pub fn read_level(level: &AtomicU32) -> f32 {
    level.load(Ordering::Relaxed) as f32 / 1000.0
}

pub fn update_level(level: &AtomicU32, samples: &[f32]) {
    if samples.is_empty() {
        return;
    }

    let sum_sq: f32 = samples.iter().map(|s| s * s).sum();
    let rms = (sum_sq / samples.len() as f32).sqrt();
    // Boost quiet mics; clamp to 0..1
    let instant = (rms * 25.0).min(1.0);
    let prev = read_level(level);
    let smoothed = if instant > prev { instant } else { prev * 0.82 };
    level.store((smoothed * 1000.0) as u32, Ordering::Relaxed);
}

/// Opens an input device and updates `level` without starting a pipeline.
pub struct LevelMonitor {
    _capture: AudioCapture,
    _drain: thread::JoinHandle<()>,
}

impl LevelMonitor {
    pub fn start(device_name: &str, level: Arc<AtomicU32>) -> Result<Self> {
        level.store(0, Ordering::Relaxed);
        let (tx, rx) = crossbeam_channel::bounded::<AudioChunk>(4);
        let capture = AudioCapture::new(device_name, Some(tx), level.clone())?;
        capture.start()?;
        let drain = thread::spawn(move || drain_capture(rx));
        Ok(Self {
            _capture: capture,
            _drain: drain,
        })
    }
}

fn drain_capture(rx: Receiver<AudioChunk>) {
    while rx.recv().is_ok() {}
}
