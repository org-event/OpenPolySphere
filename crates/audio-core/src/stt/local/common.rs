//! Shared VAD, endpointing, and hallucination filtering for local Whisper backends.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Instant;

use anyhow::Result;
use crossbeam_channel::{bounded, Receiver, Sender};
use log::{info, warn};

use crate::stt::SttResult;

pub const WHISPER_SAMPLE_RATE: u32 = 16_000;
pub const MIN_UTTERANCE_SAMPLES: usize = WHISPER_SAMPLE_RATE as usize / 4;
/// ~3 s max — call phrases; shorter buffers decode better on Metal tiny.
pub const MAX_UTTERANCE_SAMPLES: usize = WHISPER_SAMPLE_RATE as usize * 3;
pub const RESULT_QUEUE: usize = 16;
pub const RMS_THRESHOLD: f32 = 0.012;
pub const MIN_PEAK_RMS: f32 = 0.018;
pub const MIN_UTTERANCE_RMS: f32 = 0.008;
pub const AGC_TARGET_PEAK: f32 = 0.35;
pub const AGC_MAX_GAIN: f32 = 8.0;
pub const AGC_MIN_PEAK: f32 = 0.004;
/// Above this sample peak we treat audio as clipped/loud and pull down before Whisper.
pub const AGC_LIMIT_PEAK: f32 = 0.55;
pub const NO_SPEECH_PROB_THRESHOLD: f32 = 0.55;
const TRIM_SILENCE_RMS: f32 = 0.006;

type WhisperJob = (Vec<f32>, f32);
type WhisperJobSlot = Arc<Mutex<Option<WhisperJob>>>;

pub struct TranscribeOutcome {
    pub text: String,
    pub no_speech_prob: f32,
}

pub trait WhisperBackend: Send + Sync {
    fn transcribe(&self, samples: &[f32], language: &str) -> Result<TranscribeOutcome>;
}

pub struct LocalWhisperSession {
    endpointing_ms: u32,
    buffer: Vec<f32>,
    in_speech: bool,
    peak_amplitude: f32,
    last_voice: Instant,
    job_slot: WhisperJobSlot,
    job_notify: Sender<()>,
    result_rx: Receiver<SttResult>,
    _worker: JoinHandle<()>,
}

impl LocalWhisperSession {
    pub fn new(
        engine: std::sync::Arc<dyn WhisperBackend>,
        language: String,
        endpointing_ms: u32,
    ) -> Self {
        let (job_notify, job_rx) = bounded::<()>(64);
        let (result_tx, result_rx) = bounded::<SttResult>(RESULT_QUEUE);
        let job_slot: WhisperJobSlot = Arc::new(Mutex::new(None));
        let worker_slot = Arc::clone(&job_slot);

        let worker = thread::Builder::new()
            .name("whisper-stt".into())
            .spawn(move || {
                info!("Whisper STT worker started (language={language})");
                while job_rx.recv().is_ok() {
                    let job = worker_slot.lock().expect("job slot lock").take();
                    let Some((mut samples, peak)) = job else {
                        continue;
                    };
                    if peak < MIN_PEAK_RMS {
                        info!(
                            "Whisper skipped quiet segment (peak={:.4} < {:.4})",
                            peak, MIN_PEAK_RMS
                        );
                        continue;
                    }
                    trim_trailing_silence(&mut samples);
                    trim_leading_silence(&mut samples);
                    if samples.len() < MIN_UTTERANCE_SAMPLES {
                        continue;
                    }
                    let audio_ms =
                        samples.len() as u64 * 1000 / WHISPER_SAMPLE_RATE as u64;

                    let start = Instant::now();
                    match engine.transcribe(&samples, &language) {
                        Ok(outcome) => {
                            let infer_ms = start.elapsed().as_millis() as u64;
                            let text = outcome.text.trim();
                            if text.is_empty() {
                                info!(
                                    "Whisper skip: empty, no_speech={:.2}, {audio_ms}ms audio, infer={infer_ms}ms",
                                    outcome.no_speech_prob
                                );
                            } else if outcome.no_speech_prob >= NO_SPEECH_PROB_THRESHOLD {
                                info!(
                                    "Whisper skip: no_speech={:.2}, {audio_ms}ms audio, infer={infer_ms}ms, text='{text}'",
                                    outcome.no_speech_prob
                                );
                            } else if !passes_language_check(text, &language) {
                                info!(
                                    "Whisper skip: wrong-lang ({language}), {audio_ms}ms audio, infer={infer_ms}ms, text='{text}'"
                                );
                            } else if is_whisper_hallucination(text, &language) {
                                info!(
                                    "Whisper skip: hallucination, {audio_ms}ms audio, infer={infer_ms}ms, text='{text}'"
                                );
                            } else {
                                info!(
                                    "Whisper transcript: '{text}' (audio={audio_ms}ms, infer={infer_ms}ms)"
                                );
                                let _ = result_tx.send(SttResult {
                                    text: text.to_string(),
                                    stt_latency_ms: infer_ms,
                                });
                            }
                        }
                        Err(e) => warn!("Whisper transcription failed: {:#}", e),
                    }
                }
            })
            .expect("Failed to spawn whisper worker");

        Self {
            endpointing_ms,
            buffer: Vec::new(),
            in_speech: false,
            peak_amplitude: 0.0,
            last_voice: Instant::now(),
            job_slot,
            job_notify,
            result_rx,
            _worker: worker,
        }
    }

    pub fn reset_buffer(&mut self) {
        self.buffer.clear();
        self.in_speech = false;
        self.peak_amplitude = 0.0;
    }

    pub fn send_audio(&mut self, samples: &[f32]) -> Result<()> {
        if samples.is_empty() {
            return Ok(());
        }

        let rms = rms_energy(samples);
        let peak = peak_amplitude(samples);
        let now = Instant::now();
        let voice = rms >= RMS_THRESHOLD || peak >= RMS_THRESHOLD;

        if voice {
            self.in_speech = true;
            self.last_voice = now;
            self.peak_amplitude = self.peak_amplitude.max(peak).max(rms);
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
            Ok(result) => Ok(Some(result)),
            Err(crossbeam_channel::TryRecvError::Empty) => Ok(None),
            Err(crossbeam_channel::TryRecvError::Disconnected) => Ok(None),
        }
    }

    pub fn close(&mut self) {
        self.flush_utterance();
        drop(self.job_notify.clone());
    }

    fn flush_utterance(&mut self) {
        if self.buffer.len() >= MIN_UTTERANCE_SAMPLES {
            if self.peak_amplitude < MIN_PEAK_RMS {
                info!(
                    "Whisper skip: too quiet (peak={:.4} < {:.4})",
                    self.peak_amplitude, MIN_PEAK_RMS
                );
                self.buffer.clear();
                self.peak_amplitude = 0.0;
                self.in_speech = false;
                return;
            }
            let peak_before = self.peak_amplitude;
            let mut samples = std::mem::take(&mut self.buffer);
            let peak = apply_utterance_agc(&mut samples, peak_before);
            self.peak_amplitude = 0.0;
            *self.job_slot.lock().expect("job slot lock") = Some((samples, peak));
            if self.job_notify.try_send(()).is_err() {
                warn!("Whisper STT notify queue full (worker busy, latest utterance kept)");
            }
        } else {
            self.buffer.clear();
            self.peak_amplitude = 0.0;
        }
        self.in_speech = false;
    }
}

pub fn is_whisper_hallucination(text: &str, expected_lang: &str) -> bool {
    let t = text.trim();
    if t.len() < 2 {
        return true;
    }

    if t.chars()
        .all(|c| c.is_ascii_punctuation() || c.is_whitespace())
    {
        return true;
    }

    if is_wrong_language_garbage(t, expected_lang) {
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

fn passes_language_check(text: &str, expected_lang: &str) -> bool {
    let cyrillic = letter_count(text, is_cyrillic);
    let latin = letter_count(text, |c| c.is_ascii_alphabetic());
    let total = cyrillic + latin;
    if total == 0 {
        return false;
    }
    if apple_stt_enabled() {
        // Banyan Speech on a fixed locale — allow Russian/English code-switching.
        return total >= 2;
    }
    match expected_lang {
        // RU mic: need real Cyrillic, not English/Spanish/Swedish hallucinations.
        "ru" => cyrillic >= 2 && cyrillic * 100 / total >= 50,
        "en" => latin >= 2 && latin * 100 / total >= 50,
        _ => true,
    }
}

fn apple_stt_enabled() -> bool {
    matches!(
        std::env::var("STT_BACKEND")
            .unwrap_or_else(|_| "local".into())
            .to_lowercase()
            .as_str(),
        "apple" | "system" | "macos"
    )
}

fn letter_count(text: &str, pred: fn(char) -> bool) -> usize {
    text.chars()
        .filter(|c| c.is_alphabetic() && pred(*c))
        .count()
}

fn is_cyrillic(c: char) -> bool {
    ('\u{0400}'..='\u{04FF}').contains(&c)
}

fn is_wrong_language_garbage(text: &str, expected_lang: &str) -> bool {
    !passes_language_check(text, expected_lang)
}

/// Drop low-energy tail (endpointing padding) before Whisper encode.
fn trim_trailing_silence(samples: &mut Vec<f32>) {
    const FRAME: usize = WHISPER_SAMPLE_RATE as usize / 100; // 10 ms
    while samples.len() >= FRAME {
        let start = samples.len() - FRAME;
        let frame = &samples[start..];
        if rms_energy(frame) >= TRIM_SILENCE_RMS {
            break;
        }
        samples.truncate(start);
    }
}

fn trim_leading_silence(samples: &mut Vec<f32>) {
    const FRAME: usize = WHISPER_SAMPLE_RATE as usize / 100;
    while samples.len() >= FRAME {
        let frame = &samples[..FRAME];
        if rms_energy(frame) >= TRIM_SILENCE_RMS {
            break;
        }
        samples.drain(..FRAME);
    }
}

pub fn rms_energy(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum: f32 = samples.iter().map(|s| s * s).sum();
    (sum / samples.len() as f32).sqrt()
}

fn peak_amplitude(samples: &[f32]) -> f32 {
    samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max)
}

pub fn whisper_language(code: &str) -> &str {
    match code {
        "pt" => "pt",
        "no" => "no",
        other => other,
    }
}

pub fn models_base_dir() -> PathBuf {
    PathBuf::from(std::env::var("TRANSLATOR_MODELS_DIR").unwrap_or_else(|_| "./models".into()))
}

pub fn stt_device() -> String {
    std::env::var("TRANSLATOR_STT_DEVICE")
        .unwrap_or_else(|_| default_stt_device())
        .to_lowercase()
}

pub fn default_stt_device() -> String {
    // CT2/CPU is more reliable for RU on Intel Mac; Metal optional for speed tuning.
    "cpu".into()
}

pub fn variant_dir(base: &Path, core: &str) -> PathBuf {
    base.join("stt").join(format!("whisper-{core}"))
}

/// Whisper size + optional GGML quant (Metal only). CPU uses CT2 core name only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WhisperVariant {
    pub core: &'static str,
    pub quant: Option<&'static str>,
}

impl WhisperVariant {
    pub fn parse(pref: &str) -> Self {
        match pref {
            "base-q8_0" => Self {
                core: "base",
                quant: Some("q8_0"),
            },
            "tiny-q8_0" => Self {
                core: "tiny",
                quant: Some("q8_0"),
            },
            "tiny" => Self {
                core: "tiny",
                quant: None,
            },
            "base" => Self {
                core: "base",
                quant: None,
            },
            "small" => Self {
                core: "small",
                quant: None,
            },
            _ => Self {
                core: "tiny",
                quant: None,
            },
        }
    }

    pub fn pref_key(&self) -> String {
        match self.quant {
            None => self.core.to_string(),
            Some(q) => format!("{}-{q}", self.core),
        }
    }

    pub fn ggml_filename(&self) -> String {
        match self.quant {
            None => format!("ggml-{}.bin", self.core),
            Some(q) => format!("ggml-{}-{q}.bin", self.core),
        }
    }

    pub fn ggml_path(&self, base: &Path) -> PathBuf {
        variant_dir(base, self.core).join(self.ggml_filename())
    }
}

pub fn ggml_model_path(base: &Path, pref: &str) -> PathBuf {
    WhisperVariant::parse(pref).ggml_path(base)
}

pub fn is_ggml_ready(path: &Path) -> bool {
    path.is_file()
}

pub fn is_ct2_ready(dir: &Path) -> bool {
    dir.join("model.bin").exists() && dir.join("preprocessor_config.json").exists()
}

pub fn is_variant_ready_for_device(base: &Path, pref: &str, device: &str) -> bool {
    let v = WhisperVariant::parse(pref);
    match device {
        "cpu" | "ct2" => is_ct2_ready(&variant_dir(base, v.core)),
        _ => is_ggml_ready(&v.ggml_path(base)),
    }
}

/// Metal: scan whisper-{tiny,base,small} for any installed GGML file.
pub fn list_ggml_installed(base: &Path) -> Vec<String> {
    let mut out = Vec::new();
    for core in ["tiny", "base", "small"] {
        let dir = variant_dir(base, core);
        if is_ggml_ready(&dir.join(format!("ggml-{core}.bin"))) {
            out.push(format!("whisper-{core}"));
        }
        if is_ggml_ready(&dir.join(format!("ggml-{core}-q8_0.bin"))) {
            out.push(format!("whisper-{core}-q8_0"));
        }
    }
    out
}

pub fn resolve_whisper_pref(base: &Path, pref: &str, device: &str) -> PathBuf {
    if pref != "auto" {
        let v = WhisperVariant::parse(pref);
        if is_variant_ready_for_device(base, pref, device) {
            return if device == "cpu" || device == "ct2" {
                variant_dir(base, v.core)
            } else {
                v.ggml_path(base)
            };
        }
    }
    let metal_order = ["base-q8_0", "base", "tiny-q8_0", "tiny", "small"];
    let cpu_order = ["tiny", "base", "small"];
    let order: &[&str] = if device == "cpu" || device == "ct2" {
        &cpu_order
    } else {
        &metal_order
    };
    for name in order {
        if is_variant_ready_for_device(base, name, device) {
            let v = WhisperVariant::parse(name);
            return if device == "cpu" || device == "ct2" {
                variant_dir(base, v.core)
            } else {
                v.ggml_path(base)
            };
        }
    }
    if device == "cpu" || device == "ct2" {
        variant_dir(base, "tiny")
    } else {
        WhisperVariant::parse("tiny").ggml_path(base)
    }
}

fn apply_utterance_agc(samples: &mut [f32], peak_hint: f32) -> f32 {
    let peak = samples.iter().map(|s| s.abs()).fold(peak_hint, f32::max);
    if peak < AGC_MIN_PEAK {
        return peak;
    }
    // Normalize quiet speech up and loud/clipped speech down — Whisper needs ~0.2–0.4 peak.
    let gain = (AGC_TARGET_PEAK / peak).min(AGC_MAX_GAIN);
    if gain > 1.05 || peak >= AGC_LIMIT_PEAK {
        info!(
            "Whisper norm: peak {:.4} → {:.4} (×{:.2})",
            peak,
            (peak * gain).min(AGC_TARGET_PEAK),
            gain
        );
    }
    for s in samples.iter_mut() {
        *s = (*s * gain).clamp(-1.0, 1.0);
    }
    samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max)
}
