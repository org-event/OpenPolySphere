//! Local speech-to-text via CTranslate2 Whisper (faster-whisper compatible models).

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Instant;

use anyhow::{bail, Context, Result};
use crossbeam_channel::{bounded, Receiver, Sender};
use ct2rs::{Config, Device, Whisper, WhisperOptions};
use log::{info, warn};

use super::SttResult;

const WHISPER_SAMPLE_RATE: u32 = 16_000;
const MIN_UTTERANCE_SAMPLES: usize = WHISPER_SAMPLE_RATE as usize / 4;
const MAX_UTTERANCE_SAMPLES: usize = WHISPER_SAMPLE_RATE as usize * 30;
const UTTERANCE_QUEUE: usize = 16;
const RESULT_QUEUE: usize = 16;
/// Chunk-level VAD: ignore quiet background noise between words.
const RMS_THRESHOLD: f32 = 0.012;
const MIN_PEAK_RMS: f32 = 0.010;
const MIN_UTTERANCE_RMS: f32 = 0.006;
const AGC_TARGET_PEAK: f32 = 0.35;
const AGC_MAX_GAIN: f32 = 10.0;
const AGC_MIN_PEAK: f32 = 0.005;
const NO_SPEECH_PROB_THRESHOLD: f32 = 0.55;

struct TranscribeOutcome {
    text: String,
    no_speech_prob: f32,
}

pub struct LocalWhisperEngine {
    whisper: Whisper,
    model_label: String,
}

impl LocalWhisperEngine {
    pub fn new() -> Result<Self> {
        let model_dir = resolve_whisper_model_dir();
        if !is_model_ready(&model_dir) {
            bail!(
                "Whisper model not found in {}. Run: cargo run --release -p translator -- setup",
                model_dir.display()
            );
        }

        let model_label = model_dir
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "whisper".into());

        info!("Loading local Whisper STT from {}", model_dir.display());
        let config = Config {
            device: Device::CPU,
            // 0 = auto (uses Accelerate on Apple Silicon)
            num_threads_per_replica: 0,
            ..Config::default()
        };
        let whisper = Whisper::new(&model_dir, config)
            .context("failed to load Whisper model (CTranslate2)")?;
        info!(
            "Whisper STT ready ({}, {}Hz, multilingual={})",
            model_label,
            whisper.sampling_rate(),
            whisper.is_multilingual()
        );
        Ok(Self { whisper, model_label })
    }

    pub fn model_label(&self) -> &str {
        &self.model_label
    }

    fn transcribe(&self, samples: &[f32], language: &str) -> Result<TranscribeOutcome> {
        let avg_rms = rms_energy(samples);
        if avg_rms < MIN_UTTERANCE_RMS {
            return Ok(TranscribeOutcome {
                text: String::new(),
                no_speech_prob: 1.0,
            });
        }

        let lang = whisper_language(language);
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

pub struct LocalWhisperSession {
    endpointing_ms: u32,
    buffer: Vec<f32>,
    in_speech: bool,
    peak_rms: f32,
    last_voice: Instant,
    utterance_tx: Sender<(Vec<f32>, f32)>,
    result_rx: Receiver<SttResult>,
    _worker: JoinHandle<()>,
}

impl LocalWhisperSession {
    pub fn new(engine: Arc<LocalWhisperEngine>, language: String, endpointing_ms: u32) -> Self {
        let (utterance_tx, utterance_rx) = bounded::<(Vec<f32>, f32)>(UTTERANCE_QUEUE);
        let (result_tx, result_rx) = bounded::<SttResult>(RESULT_QUEUE);

        let lang = language.clone();
        let worker = thread::Builder::new()
            .name("whisper-stt".into())
            .spawn(move || {
                while let Ok((samples, peak_rms)) = utterance_rx.recv() {
                    if peak_rms < MIN_PEAK_RMS {
                        info!(
                            "Whisper skipped quiet segment (peak_rms={:.4} < {:.4})",
                            peak_rms, MIN_PEAK_RMS
                        );
                        continue;
                    }

                    let start = Instant::now();
                    match engine.transcribe(&samples, &lang) {
                        Ok(outcome) if outcome.no_speech_prob >= NO_SPEECH_PROB_THRESHOLD => {
                            info!(
                                "Whisper no-speech (prob={:.2}): skipped",
                                outcome.no_speech_prob
                            );
                        }
                        Ok(outcome) if is_whisper_hallucination(&outcome.text) => {
                            info!("Whisper hallucination filtered: '{}'", outcome.text);
                        }
                        Ok(outcome) if !outcome.text.trim().is_empty() => {
                            let _ = result_tx.send(SttResult {
                                text: outcome.text,
                                stt_latency_ms: start.elapsed().as_millis() as u64,
                            });
                        }
                        Ok(_) => {}
                        Err(e) => warn!("Whisper transcription failed: {:#}", e),
                    }
                }
            })
            .expect("Failed to spawn whisper worker");

        Self {
            endpointing_ms,
            buffer: Vec::new(),
            in_speech: false,
            peak_rms: 0.0,
            last_voice: Instant::now(),
            utterance_tx,
            result_rx,
            _worker: worker,
        }
    }

    pub fn reset_buffer(&mut self) {
        self.buffer.clear();
        self.in_speech = false;
        self.peak_rms = 0.0;
    }

    pub fn send_audio(&mut self, samples: &[f32]) -> Result<()> {
        if samples.is_empty() {
            return Ok(());
        }

        let rms = rms_energy(samples);
        let now = Instant::now();

        if rms >= RMS_THRESHOLD {
            self.in_speech = true;
            self.last_voice = now;
            self.peak_rms = self.peak_rms.max(rms);
            self.buffer.extend_from_slice(samples);
            if self.buffer.len() > MAX_UTTERANCE_SAMPLES {
                self.flush_utterance();
            }
        } else if self.in_speech {
            self.buffer.extend_from_slice(samples);
            let silence_ms = now.duration_since(self.last_voice).as_millis() as u32;
            if silence_ms >= self.endpointing_ms {
                self.flush_utterance();
            }
        }

        Ok(())
    }

    pub fn poll_transcript(&mut self) -> Result<Option<SttResult>> {
        match self.result_rx.try_recv() {
            Ok(result) => {
                info!(
                    "Whisper transcript: '{}' (stt={}ms)",
                    result.text, result.stt_latency_ms
                );
                Ok(Some(result))
            }
            Err(crossbeam_channel::TryRecvError::Empty) => Ok(None),
            Err(crossbeam_channel::TryRecvError::Disconnected) => Ok(None),
        }
    }

    pub fn close(&mut self) {
        self.flush_utterance();
        drop(self.utterance_tx.clone());
    }

    fn flush_utterance(&mut self) {
        if self.buffer.len() >= MIN_UTTERANCE_SAMPLES {
            let mut samples = std::mem::take(&mut self.buffer);
            let peak_before = self.peak_rms;
            let peak = apply_utterance_agc(&mut samples, peak_before);
            self.peak_rms = 0.0;
            if self.utterance_tx.try_send((samples, peak)).is_err() {
                warn!("Whisper STT queue full, dropping utterance");
            }
        } else {
            self.buffer.clear();
            self.peak_rms = 0.0;
        }
        self.in_speech = false;
    }
}

fn apply_utterance_agc(samples: &mut [f32], peak_before: f32) -> f32 {
    let peak = samples
        .iter()
        .map(|s| s.abs())
        .fold(peak_before, f32::max);
    if peak < AGC_MIN_PEAK {
        return peak;
    }
    if peak >= AGC_TARGET_PEAK {
        return peak;
    }
    let gain = (AGC_TARGET_PEAK / peak).min(AGC_MAX_GAIN);
    if gain > 1.2 {
        info!(
            "Whisper AGC: peak {:.4} → {:.4} (×{:.1})",
            peak,
            (peak * gain).min(AGC_TARGET_PEAK),
            gain
        );
    }
    for s in samples.iter_mut() {
        *s = (*s * gain).clamp(-1.0, 1.0);
    }
    samples
        .iter()
        .map(|s| s.abs())
        .fold(0.0f32, f32::max)
}

pub fn is_whisper_hallucination(text: &str) -> bool {
    let t = text.trim();
    if t.len() < 2 {
        return true;
    }

    let lower = t.to_lowercase();
    let patterns = [
        "subtitle",
        "subtitles by",
        "субтитр",
        "редактор субтитров",
        "корректор",
        "закомолд",
        "zacomold",
        "enzak",
        "molden",
        "thanks for watching",
        "thank you for watching",
        "thank you for your attention",
        "for your attention",
        "please subscribe",
        "amara.org",
        "mbc",
        "www.",
        "http://",
        "https://",
    ];
    if patterns.iter().any(|p| lower.contains(p)) {
        return true;
    }

    let letters = t.chars().filter(|c| c.is_alphabetic()).count();
    letters < 2
}

fn rms_energy(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum: f32 = samples.iter().map(|s| s * s).sum();
    (sum / samples.len() as f32).sqrt()
}

fn whisper_language(code: &str) -> &str {
    match code {
        "pt" => "pt",
        "no" => "no",
        other => other,
    }
}

/// Resolve whisper model directory. Pref from `TRANSLATOR_WHISPER_MODEL` (set by server from settings.json).
pub fn resolve_whisper_model_dir() -> PathBuf {
    let base = models_base_dir();
    let pref = std::env::var("TRANSLATOR_WHISPER_MODEL").unwrap_or_else(|_| "auto".into());
    resolve_whisper_pref(&base, &pref.to_lowercase())
}

pub fn resolve_whisper_pref(base: &Path, pref: &str) -> PathBuf {
    if pref != "auto" {
        let dir = base.join("stt").join(format!("whisper-{pref}"));
        if is_model_ready(&dir) {
            return dir;
        }
        // Explicit choice missing — fall through to any installed model.
    }
    for name in ["tiny", "base", "small"] {
        let dir = base.join("stt").join(format!("whisper-{name}"));
        if is_model_ready(&dir) {
            return dir;
        }
    }
    base.join("stt").join("whisper-tiny")
}

pub fn list_installed_whisper_variants() -> Vec<String> {
    let base = models_base_dir().join("stt");
    ["tiny", "base", "small"]
        .iter()
        .filter(|name| is_model_ready(&base.join(format!("whisper-{name}"))))
        .map(|name| format!("whisper-{name}"))
        .collect()
}

pub fn is_variant_ready(pref: &str) -> bool {
    if pref == "auto" {
        return !list_installed_whisper_variants().is_empty();
    }
    is_model_ready(&models_base_dir().join("stt").join(format!("whisper-{pref}")))
}

pub fn whisper_model_status() -> (String, bool) {
    let dir = resolve_whisper_model_dir();
    let label = dir
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "whisper".into());
    (label, is_model_ready(&dir))
}

fn models_base_dir() -> PathBuf {
    PathBuf::from(std::env::var("TRANSLATOR_MODELS_DIR").unwrap_or_else(|_| "./models".into()))
}

fn is_model_ready(path: &Path) -> bool {
    path.join("model.bin").exists() && path.join("preprocessor_config.json").exists()
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

/// Drop cached engine after model download or env change.
pub fn invalidate_engine_cache() {
    *LOCAL_ENGINE.lock().unwrap() = None;
}
