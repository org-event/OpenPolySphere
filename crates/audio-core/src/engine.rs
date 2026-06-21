//! Main engine coordinator.
//!
//! Two pipelines:
//!   OUTGOING: Mic -> STT(ru) -> Translate(ru->en) -> TTS(en) -> Speakers
//!   INCOMING: Meet input -> STT(en) -> Translate(en->ru) -> TTS(ru) -> Speakers

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use crossbeam_channel::{bounded, Sender};
use log::{error, info, warn};

use crate::audio;
use crate::audio::capture::{AudioCapture, AudioChunk};
use crate::audio::level::{self, LevelMonitor};
use crate::audio::pending::PendingMicAudio;
use crate::audio::playback::AudioPlayback;
use crate::protocol::Event;
use crate::stt::{local, SttEngine};
use crate::translation::{TranslationDirection, TranslationEngine};
use crate::tts::TtsEngine;

// ---------------------------------------------------------------------------
// EngineConfig
// ---------------------------------------------------------------------------

pub struct EngineConfig {
    pub deepgram_api_key: String,
    pub tts_en_model: String,
    pub tts_en_config: String,
    pub tts_ru_model: String,
    pub tts_ru_config: String,
    pub mic_device: String,
    pub speaker_device: String,
    pub meet_input_device: String,
    pub meet_output_device: String,
    pub sample_rate: u32,
    pub endpointing_ms: u32,
    pub my_language: String,
    pub their_language: String,
}

impl EngineConfig {
    pub fn from_env() -> Self {
        let base = std::env::var("TRANSLATOR_MODELS_DIR").unwrap_or_else(|_| "./models".into());
        Self::from_parts(base, Self::read_env_fields())
    }

    /// Build config from explicit settings (used by the `translator` binary).
    pub fn from_settings(
        models_base: &str,
        deepgram_api_key: &str,
        my_language: &str,
        their_language: &str,
        mic_device: &str,
        speaker_device: &str,
        meet_input: &str,
        meet_out: &str,
        endpointing_ms: u32,
        out_voice: &str,
        in_voice: &str,
    ) -> Self {
        let base = models_base.to_string();
        Self {
            deepgram_api_key: deepgram_api_key.to_string(),
            tts_en_model: format!("{base}/piper-{their_language}/{out_voice}.onnx"),
            tts_en_config: format!("{base}/piper-{their_language}/{out_voice}.onnx.json"),
            tts_ru_model: format!("{base}/piper-{my_language}/{in_voice}.onnx"),
            tts_ru_config: format!("{base}/piper-{my_language}/{in_voice}.onnx.json"),
            mic_device: mic_device.to_string(),
            speaker_device: speaker_device.to_string(),
            meet_input_device: meet_input.to_string(),
            meet_output_device: meet_out.to_string(),
            sample_rate: std::env::var("TRANSLATOR_SAMPLE_RATE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(48000),
            endpointing_ms,
            my_language: my_language.to_string(),
            their_language: their_language.to_string(),
        }
    }

    fn from_parts(base: String, fields: EnvFields) -> Self {
        Self {
            deepgram_api_key: fields.deepgram_api_key,
            tts_en_model: fields
                .tts_en_model
                .unwrap_or_else(|| format!("{base}/piper-en/en_US-ryan-medium.onnx")),
            tts_en_config: fields
                .tts_en_config
                .unwrap_or_else(|| format!("{base}/piper-en/en_US-ryan-medium.onnx.json")),
            tts_ru_model: fields
                .tts_ru_model
                .unwrap_or_else(|| format!("{base}/piper-ru/ru_RU-denis-medium.onnx")),
            tts_ru_config: fields
                .tts_ru_config
                .unwrap_or_else(|| format!("{base}/piper-ru/ru_RU-denis-medium.onnx.json")),
            mic_device: fields.mic_device.unwrap_or_else(|| "default".into()),
            speaker_device: fields.speaker_device.unwrap_or_else(|| "default".into()),
            meet_input_device: fields
                .meet_input
                .unwrap_or_else(|| "BlackHole 16ch".into()),
            meet_output_device: fields
                .meet_out
                .unwrap_or_else(|| "BlackHole 2ch".into()),
            sample_rate: fields.sample_rate.unwrap_or(48000),
            endpointing_ms: fields.endpointing_ms.unwrap_or(500),
            my_language: fields.my_language.unwrap_or_else(|| "ru".into()),
            their_language: fields.their_language.unwrap_or_else(|| "en".into()),
        }
    }

    fn read_env_fields() -> EnvFields {
        EnvFields {
            deepgram_api_key: std::env::var("DEEPGRAM_API_KEY").unwrap_or_default(),
            tts_en_model: std::env::var("TRANSLATOR_TTS_EN_MODEL").ok(),
            tts_en_config: std::env::var("TRANSLATOR_TTS_EN_CONFIG").ok(),
            tts_ru_model: std::env::var("TRANSLATOR_TTS_RU_MODEL").ok(),
            tts_ru_config: std::env::var("TRANSLATOR_TTS_RU_CONFIG").ok(),
            mic_device: std::env::var("TRANSLATOR_MIC_DEVICE").ok(),
            speaker_device: std::env::var("TRANSLATOR_SPEAKER_DEVICE").ok(),
            meet_input: std::env::var("TRANSLATOR_MEET_INPUT").ok(),
            meet_out: std::env::var("TRANSLATOR_MEET_OUTPUT").ok(),
            sample_rate: std::env::var("TRANSLATOR_SAMPLE_RATE")
                .ok()
                .and_then(|s| s.parse().ok()),
            endpointing_ms: std::env::var("TRANSLATOR_ENDPOINTING_MS")
                .ok()
                .and_then(|s| s.parse().ok()),
            my_language: std::env::var("TRANSLATOR_MY_LANG").ok(),
            their_language: std::env::var("TRANSLATOR_THEIR_LANG").ok(),
        }
    }
}

struct EnvFields {
    deepgram_api_key: String,
    tts_en_model: Option<String>,
    tts_en_config: Option<String>,
    tts_ru_model: Option<String>,
    tts_ru_config: Option<String>,
    mic_device: Option<String>,
    speaker_device: Option<String>,
    meet_input: Option<String>,
    meet_out: Option<String>,
    sample_rate: Option<u32>,
    endpointing_ms: Option<u32>,
    my_language: Option<String>,
    their_language: Option<String>,
}

// ---------------------------------------------------------------------------
// Engine
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq)]
enum EngineState {
    Idle,
    Running,
    ShuttingDown,
}

pub struct Engine {
    state: EngineState,
    config: EngineConfig,
    event_tx: Sender<Event>,
    pipeline_handles: Vec<thread::JoinHandle<()>>,
    stop_flag: Option<Arc<AtomicBool>>,
    mute_outgoing: Arc<AtomicBool>,
    mute_incoming: Arc<AtomicBool>,
    mic_level: Arc<AtomicU32>,
    call_in_level: Arc<AtomicU32>,
    level_monitor_mic: Option<LevelMonitor>,
    level_monitor_call_in: Option<LevelMonitor>,
    mic_monitor_error: Option<String>,
    call_in_monitor_error: Option<String>,
    level_ticker_stop: Option<Arc<AtomicBool>>,
    level_ticker_handle: Option<thread::JoinHandle<()>>,
}

impl Engine {
    pub fn new(config: EngineConfig, event_tx: Sender<Event>) -> Self {
        Self {
            state: EngineState::Idle,
            config,
            event_tx,
            pipeline_handles: Vec::new(),
            stop_flag: None,
            mute_outgoing: Arc::new(AtomicBool::new(false)),
            mute_incoming: Arc::new(AtomicBool::new(false)),
            mic_level: level::new_level_atomic(),
            call_in_level: level::new_level_atomic(),
            level_monitor_mic: None,
            level_monitor_call_in: None,
            mic_monitor_error: None,
            call_in_monitor_error: None,
            level_ticker_stop: None,
            level_ticker_handle: None,
        }
    }

    pub fn is_shutting_down(&self) -> bool {
        self.state == EngineState::ShuttingDown
    }

    pub fn is_running(&self) -> bool {
        self.state == EngineState::Running
    }

    pub fn update_config(&mut self, config: EngineConfig) {
        self.config = config;
    }

    pub fn handle_command(&mut self, cmd: crate::protocol::Command) -> Vec<Event> {
        use crate::protocol::Command;

        match cmd {
            Command::Ping => vec![Event::Pong],

            Command::Start { pipelines } => {
                if self.state == EngineState::Running {
                    return vec![Event::Started {
                        pipelines: pipelines.clone(),
                    }];
                }

                if let Err(e) = audio::list_devices() {
                    info!("Could not enumerate audio devices: {:#}", e);
                }

                let stt_local = std::env::var("STT_BACKEND")
                    .unwrap_or_else(|_| "local".into())
                    .to_lowercase();
                if self.config.deepgram_api_key.is_empty() && stt_local == "deepgram" {
                    return vec![Event::Error {
                        message: "STT_BACKEND=deepgram but DEEPGRAM_API_KEY is not set".into(),
                    }];
                }

                self.stop_level_monitor_devices();

                match self.start_pipelines(&pipelines) {
                    Ok(()) => {
                        self.state = EngineState::Running;
                        self.ensure_level_ticker();
                        vec![
                            Event::Log {
                                level: "info".into(),
                                message: format!("Starting pipelines: {:?}", pipelines),
                            },
                            Event::Started {
                                pipelines: pipelines.clone(),
                            },
                        ]
                    }
                    Err(e) => {
                        error!("Failed to start pipelines: {:#}", e);
                        vec![Event::Error {
                            message: format!("Failed to start pipelines: {:#}", e),
                        }]
                    }
                }
            }

            Command::Stop => {
                self.stop_pipelines();
                self.state = EngineState::Idle;
                if self.level_monitor_mic.is_none() && self.level_monitor_call_in.is_none() {
                    self.stop_level_ticker();
                }
                vec![
                    Event::Log {
                        level: "info".into(),
                        message: "Pipelines stopped".into(),
                    },
                    Event::Stopped,
                ]
            }

            Command::SetConfig { key, value } => {
                self.apply_config(&key, &value);
                vec![Event::Log {
                    level: "info".into(),
                    message: format!("Config set: {} = {}", key, value),
                }]
            }

            Command::ListDevices => match audio::list_devices() {
                Ok((input, output)) => vec![Event::DeviceList { input, output }],
                Err(e) => vec![Event::Error {
                    message: format!("Failed to list devices: {:#}", e),
                }],
            },

            Command::MonitorLevels { mic, call_in } => {
                self.start_level_monitoring(&mic, &call_in);
                vec![self.audio_levels_event()]
            }

            Command::StopLevelMonitors => {
                self.stop_level_monitoring();
                vec![self.audio_levels_event()]
            }

            Command::TtsPreview { lang, voice } => {
                let models_base = std::env::var("TRANSLATOR_MODELS_DIR")
                    .unwrap_or_else(|_| "./models".into());
                let model_path = format!("{}/piper-{}/{}.onnx", models_base, lang, voice);
                let config_path = format!("{}/piper-{}/{}.onnx.json", models_base, lang, voice);
                let text = match lang.as_str() {
                    "ru" => "Привет, это тест голоса для перевода.",
                    "de" => "Hallo, dies ist ein Stimmtest.",
                    "fr" => "Bonjour, ceci est un test de voix.",
                    "es" => "Hola, esta es una prueba de voz.",
                    "it" => "Ciao, questo è un test vocale.",
                    "pt" => "Olá, este é um teste de voz.",
                    "zh" => "你好，这是语音测试。",
                    "ar" => "مرحبا، هذا اختبار صوتي.",
                    "hi" => "नमस्ते, यह एक आवाज़ परीक्षण है।",
                    "tr" => "Merhaba, bu bir ses testidir.",
                    "nl" => "Hallo, dit is een stemtest.",
                    "pl" => "Cześć, to jest test głosu.",
                    "uk" => "Привіт, це тест голосу.",
                    _ => "Hello, this is a voice preview test.",
                };

                match TtsEngine::new(&config_path, &model_path, self.config.sample_rate) {
                    Ok(mut tts) => {
                        match tts.synthesize(text) {
                            Ok(samples) => {
                                // Play through default speakers
                                let speaker = self.config.speaker_device.clone();
                                let sr = self.config.sample_rate;
                                let (tx, rx) = crossbeam_channel::bounded(4);
                                match AudioPlayback::new(&speaker, sr, rx) {
                                    Ok(playback) => {
                                        let _ = tx.send(samples);
                                        drop(tx);
                                        // Wait for playback to finish
                                        std::thread::sleep(std::time::Duration::from_secs(3));
                                        drop(playback);
                                        vec![Event::TtsPreviewDone]
                                    }
                                    Err(e) => vec![Event::Error {
                                        message: format!("Preview playback failed: {:#}", e),
                                    }],
                                }
                            }
                            Err(e) => vec![Event::Error {
                                message: format!("Preview synthesis failed: {:#}", e),
                            }],
                        }
                    }
                    Err(e) => vec![Event::Error {
                        message: format!("Preview TTS load failed: {:#}", e),
                    }],
                }
            }

            Command::Shutdown => {
                let mut events = Vec::new();
                if self.state == EngineState::Running {
                    self.stop_pipelines();
                    events.push(Event::Stopped);
                }
                self.state = EngineState::ShuttingDown;
                events
            }
        }
    }

    fn apply_config(&mut self, key: &str, value: &serde_json::Value) {
        match key {
            "endpointing_ms" => {
                if let Some(v) = value.as_u64() {
                    self.config.endpointing_ms = v as u32;
                    info!("Updated endpointing_ms to {}", v);
                }
            }
            "mute_outgoing" => {
                let muted = value.as_bool().unwrap_or(false);
                self.mute_outgoing.store(muted, Ordering::SeqCst);
                info!("Outgoing mute: {}", muted);
            }
            "mute_incoming" => {
                let muted = value.as_bool().unwrap_or(false);
                self.mute_incoming.store(muted, Ordering::SeqCst);
                info!("Incoming mute: {}", muted);
            }
            _ => warn!("Unknown config key: {}", key),
        }
    }

    fn start_pipelines(&mut self, pipelines: &[String]) -> Result<()> {
        let stop_flag = Arc::new(AtomicBool::new(false));
        self.stop_flag = Some(stop_flag.clone());
        let tts_echo_suppress = Arc::new(AtomicBool::new(false));

        info!("Loading STT engine...");
        let stt_engine = Arc::new(
            SttEngine::new(
                self.config.deepgram_api_key.clone(),
                self.config.endpointing_ms,
            )
            .context("Failed to initialize STT engine")?,
        );

        info!("Loading translation engine...");
        let translator = Arc::new(
            TranslationEngine::new().context("Failed to initialize translation engine")?,
        );

        info!("Loading TTS models...");
        let mut tts_out = Some(
            TtsEngine::new(
                &self.config.tts_en_config,
                &self.config.tts_en_model,
                self.config.sample_rate,
            )
            .context("Failed to load TTS engine (outgoing/en)")?,
        );
        let mut tts_in = Some(
            TtsEngine::new(
                &self.config.tts_ru_config,
                &self.config.tts_ru_model,
                self.config.sample_rate,
            )
            .context("Failed to load TTS engine (incoming/ru)")?,
        );

        info!("All models loaded, spawning pipelines...");

        for pipeline_name in pipelines {
            match pipeline_name.as_str() {
                "outgoing" => {
                    let tts = tts_out.take().expect("outgoing TTS already taken");
                    let handle = spawn_pipeline(
                        "outgoing",
                        self.config.mic_device.clone(),
                        self.config.meet_output_device.clone(),
                        self.config.sample_rate,
                        stt_engine.clone(),
                        translator.clone(),
                        TranslationDirection::new(&self.config.my_language, &self.config.their_language),
                        &self.config.my_language,
                        tts,
                        self.event_tx.clone(),
                        stop_flag.clone(),
                        self.mute_outgoing.clone(),
                        self.mic_level.clone(),
                        tts_echo_suppress.clone(),
                    )?;
                    self.pipeline_handles.push(handle);
                }
                "incoming" => {
                    let tts = tts_in.take().expect("incoming TTS already taken");
                    let handle = spawn_pipeline(
                        "incoming",
                        self.config.meet_input_device.clone(),
                        self.config.speaker_device.clone(),
                        self.config.sample_rate,
                        stt_engine.clone(),
                        translator.clone(),
                        TranslationDirection::new(&self.config.their_language, &self.config.my_language),
                        &self.config.their_language,
                        tts,
                        self.event_tx.clone(),
                        stop_flag.clone(),
                        self.mute_incoming.clone(),
                        self.call_in_level.clone(),
                        tts_echo_suppress.clone(),
                    )?;
                    self.pipeline_handles.push(handle);
                }
                other => warn!("Unknown pipeline: {}", other),
            }
        }

        Ok(())
    }

    fn stop_pipelines(&mut self) {
        if let Some(flag) = self.stop_flag.take() {
            flag.store(true, Ordering::SeqCst);
        }

        for handle in self.pipeline_handles.drain(..) {
            let name = handle.thread().name().unwrap_or("unnamed").to_string();
            info!("Waiting for pipeline thread '{}' to stop...", name);
            if let Err(e) = handle.join() {
                error!("Pipeline thread '{}' panicked: {:?}", name, e);
            }
        }
        self.mic_level.store(0, Ordering::Relaxed);
        self.call_in_level.store(0, Ordering::Relaxed);
        info!("All pipeline threads stopped");
    }

    fn audio_levels_event(&self) -> Event {
        Event::AudioLevels {
            mic: level::read_level(&self.mic_level),
            call_in: level::read_level(&self.call_in_level),
            mic_active: self.level_monitor_mic.is_some() || self.state == EngineState::Running,
            call_in_active: self.level_monitor_call_in.is_some() || self.state == EngineState::Running,
            mic_error: self.mic_monitor_error.clone(),
            call_in_error: self.call_in_monitor_error.clone(),
        }
    }

    fn start_level_monitoring(&mut self, mic: &str, call_in: &str) {
        if self.state == EngineState::Running {
            self.ensure_level_ticker();
            let _ = self.event_tx.send(self.audio_levels_event());
            return;
        }

        self.stop_level_monitor_devices();
        self.mic_monitor_error = None;
        self.call_in_monitor_error = None;
        self.mic_level.store(0, Ordering::Relaxed);
        self.call_in_level.store(0, Ordering::Relaxed);

        if !mic.is_empty() {
            match LevelMonitor::start(mic, self.mic_level.clone()) {
                Ok(monitor) => {
                    info!("Level monitor started for mic '{}'", mic);
                    self.level_monitor_mic = Some(monitor);
                }
                Err(e) => {
                    let msg = format!("{:#}", e);
                    error!("Mic level monitor failed: {}", msg);
                    self.mic_monitor_error = Some(msg);
                }
            }
        }

        if !call_in.is_empty() && call_in != mic {
            match LevelMonitor::start(call_in, self.call_in_level.clone()) {
                Ok(monitor) => {
                    info!("Level monitor started for call input '{}'", call_in);
                    self.level_monitor_call_in = Some(monitor);
                }
                Err(e) => {
                    let msg = format!("{:#}", e);
                    error!("Call input level monitor failed: {}", msg);
                    self.call_in_monitor_error = Some(msg);
                }
            }
        } else if !call_in.is_empty() && call_in == mic {
            self.level_monitor_call_in = None;
            self.call_in_monitor_error = Some(
                "Same device as microphone — use a separate Call Input device".into(),
            );
        }

        self.ensure_level_ticker();
        let _ = self.event_tx.send(self.audio_levels_event());
    }

    fn stop_level_monitor_devices(&mut self) {
        self.level_monitor_mic = None;
        self.level_monitor_call_in = None;
    }

    fn stop_level_monitoring(&mut self) {
        self.stop_level_monitor_devices();
        self.stop_level_ticker();
        self.mic_monitor_error = None;
        self.call_in_monitor_error = None;
        self.mic_level.store(0, Ordering::Relaxed);
        self.call_in_level.store(0, Ordering::Relaxed);
    }

    fn ensure_level_ticker(&mut self) {
        if self.level_ticker_handle.is_some() {
            return;
        }

        let stop = Arc::new(AtomicBool::new(false));
        let stop_clone = stop.clone();
        let event_tx = self.event_tx.clone();
        let mic_level = self.mic_level.clone();
        let call_in_level = self.call_in_level.clone();

        let handle = thread::Builder::new()
            .name("level-ticker".into())
            .spawn(move || {
                while !stop_clone.load(Ordering::Relaxed) {
                    let _ = event_tx.send(Event::AudioLevels {
                        mic: level::read_level(&mic_level),
                        call_in: level::read_level(&call_in_level),
                        mic_active: true,
                        call_in_active: true,
                        mic_error: None,
                        call_in_error: None,
                    });
                    thread::sleep(Duration::from_millis(100));
                }
            })
            .expect("Failed to spawn level ticker");

        self.level_ticker_stop = Some(stop);
        self.level_ticker_handle = Some(handle);
    }

    fn stop_level_ticker(&mut self) {
        if let Some(stop) = self.level_ticker_stop.take() {
            stop.store(true, Ordering::Relaxed);
        }
        if let Some(handle) = self.level_ticker_handle.take() {
            let _ = handle.join();
        }
    }
}

// ---------------------------------------------------------------------------
// Pipeline spawning
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn spawn_pipeline(
    direction: &str,
    capture_device: String,
    playback_device: String,
    sample_rate: u32,
    stt_engine: Arc<SttEngine>,
    translator: Arc<TranslationEngine>,
    translate_direction: TranslationDirection,
    source_lang: &str,
    tts: TtsEngine,
    event_tx: Sender<Event>,
    stop_flag: Arc<AtomicBool>,
    mute_flag: Arc<AtomicBool>,
    capture_level: Arc<AtomicU32>,
    tts_echo_suppress: Arc<AtomicBool>,
) -> Result<thread::JoinHandle<()>> {
    let dir_name = direction.to_string();
    let src_lang = source_lang.to_string();
    let echo_suppress = tts_echo_suppress;

    let handle = thread::Builder::new()
        .name(format!("pipeline-{}", direction))
        .spawn(move || {
            if let Err(e) = run_pipeline(
                &dir_name,
                &capture_device,
                &playback_device,
                sample_rate,
                stt_engine,
                translator,
                translate_direction,
                &src_lang,
                tts,
                &event_tx,
                &stop_flag,
                &mute_flag,
                capture_level,
                &echo_suppress,
            ) {
                error!("{} pipeline failed: {:#}", dir_name, e);
                let _ = event_tx.send(Event::Error {
                    message: format!("{} pipeline failed: {:#}", dir_name, e),
                });
            }
            info!("{} pipeline thread exiting", dir_name);
        })
        .context("Failed to spawn pipeline thread")?;

    Ok(handle)
}

// ---------------------------------------------------------------------------
// Core pipeline logic
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn run_pipeline(
    direction: &str,
    capture_device: &str,
    playback_device: &str,
    sample_rate: u32,
    stt_engine: Arc<SttEngine>,
    translator: Arc<TranslationEngine>,
    translate_direction: TranslationDirection,
    source_lang: &str,
    mut tts: TtsEngine,
    event_tx: &Sender<Event>,
    stop_flag: &AtomicBool,
    mute_flag: &AtomicBool,
    capture_level: Arc<AtomicU32>,
    echo_suppress: &Arc<AtomicBool>,
) -> Result<()> {
    info!(
        "[{}] Starting pipeline: capture='{}', playback='{}'",
        direction, capture_device, playback_device
    );

    let (audio_tx, audio_rx) = bounded::<AudioChunk>(1024);
    let (playback_tx, playback_rx) = bounded::<Vec<f32>>(64);
    let (proc_tx, proc_rx) = bounded::<(String, u64)>(64);

    let capture = AudioCapture::new(capture_device, Some(audio_tx), capture_level)
        .with_context(|| format!("[{}] Failed to create AudioCapture", direction))?;
    let capture_rate = capture.sample_rate();

    let playback = AudioPlayback::new(playback_device, sample_rate, playback_rx)
        .with_context(|| format!("[{}] Failed to create AudioPlayback", direction))?;

    // Connect STT — stream at 16kHz
    let stt_sample_rate = 16_000_u32;
    let mut session = stt_engine
        .create_session(stt_sample_rate, source_lang)
        .with_context(|| format!("[{}] Failed to create STT session", direction))?;

    capture
        .start()
        .with_context(|| format!("[{}] Failed to start capture", direction))?;
    playback
        .start()
        .with_context(|| format!("[{}] Failed to start playback", direction))?;

    let drained = audio_rx.try_iter().count();
    if drained > 0 {
        info!("[{}] Drained {} stale audio chunks", direction, drained);
    }

    info!("[{}] Pipeline running", direction);

    // Global echo suppression: while any pipeline plays TTS, all STT inputs are muted.
    let proc_translator = translator.clone();
    let proc_playback_tx = playback_tx.clone();
    let proc_event_tx = event_tx.clone();
    let proc_echo_suppress = echo_suppress.clone();
    let proc_direction = direction.to_string();
    let proc_source_lang = source_lang.to_string();
    let proc_sample_rate = sample_rate;
    let _proc_handle = std::thread::spawn(move || {
        while let Ok((text, stt_ms)) = proc_rx.recv() {
            if local::is_whisper_hallucination(&text) {
                info!("[{}] Hallucination dropped before translate: '{}'", proc_direction, text);
                continue;
            }
            let _audio_len = process_utterance(
                &proc_direction,
                &text,
                stt_ms,
                proc_translator.as_ref(),
                &translate_direction,
                &proc_source_lang,
                &mut tts,
                proc_sample_rate,
                &proc_playback_tx,
                &proc_event_tx,
                &proc_echo_suppress,
            );
        }
    });

    info!("[{}] Capture rate: {}Hz, STT rate: {}Hz", direction, capture_rate, stt_sample_rate);

    let mut pending_mic = PendingMicAudio::new();

    loop {
        if stop_flag.load(Ordering::SeqCst) {
            info!("[{}] Stop flag set, exiting", direction);
            break;
        }

        match audio_rx.recv_timeout(std::time::Duration::from_millis(100)) {
            Ok(chunk) => {
                if mute_flag.load(Ordering::Relaxed) {
                    continue;
                }
                let samples_16k = resample(&chunk.samples, capture_rate, stt_sample_rate);
                if echo_suppress.load(Ordering::SeqCst) {
                    pending_mic.push(&samples_16k);
                    continue;
                }
                if !pending_mic.is_empty() {
                    let backlog = pending_mic.take();
                    if let Err(e) = session.send_audio(&backlog) {
                        warn!("[{}] STT backlog error: {:#}", direction, e);
                    }
                }
                if let Err(e) = session.send_audio(&samples_16k) {
                    warn!("[{}] STT send error: {:#}", direction, e);
                }
            }
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => {}
            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                info!("[{}] Audio capture channel disconnected", direction);
                break;
            }
        }

        match session.poll_transcript() {
            Ok(Some(result)) => {
                if let Err(e) = proc_tx.try_send((result.text, result.stt_latency_ms)) {
                    warn!("[{}] Processor channel full, dropping transcript: {}", direction, e);
                }
            }
            Ok(None) => {}
            Err(e) => {
                error!("[{}] STT error: {:#}", direction, e);
                let _ = event_tx.send(Event::Error {
                    message: format!("[{}] STT failed: {:#}", direction, e),
                });
                break;
            }
        }
    }

    session.close();
    let _ = capture.stop();
    let _ = playback.stop();
    drop(playback_tx);

    info!("[{}] Pipeline stopped cleanly", direction);
    Ok(())
}

// ---------------------------------------------------------------------------
// Utterance processing: transcript -> translate -> TTS -> playback
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn process_utterance(
    direction: &str,
    text: &str,
    stt_ms: u64,
    translator: &TranslationEngine,
    translate_direction: &TranslationDirection,
    source_lang: &str,
    tts: &mut TtsEngine,
    sample_rate: u32,
    playback_tx: &Sender<Vec<f32>>,
    event_tx: &Sender<Event>,
    echo_suppress: &Arc<AtomicBool>,
) -> usize {
    info!("[{}] Transcript: '{}'", direction, text);

    let _ = event_tx.send(Event::Transcript {
        direction: direction.to_string(),
        text: text.to_string(),
        lang: source_lang.to_string(),
    });

    let translate_start = Instant::now();
    let translated = match translator.translate(text, translate_direction) {
        Ok(t) => t,
        Err(e) => {
            error!("[{}] Translation error: {:#}", direction, e);
            let _ = event_tx.send(Event::Error {
                message: format!("[{}] Translation failed: {:#}", direction, e),
            });
            return 0;
        }
    };
    let translate_ms = translate_start.elapsed().as_millis() as u64;

    info!("[{}] Translation: '{}'", direction, translated);
    let _ = event_tx.send(Event::Translation {
        direction: direction.to_string(),
        text: translated.clone(),
    });

    if translated.trim().is_empty() {
        return 0;
    }

    let tts_start = Instant::now();
    let audio = match tts.synthesize(&translated) {
        Ok(samples) => samples,
        Err(e) => {
            error!("[{}] TTS error: {:#}", direction, e);
            let _ = event_tx.send(Event::Error {
                message: format!("[{}] TTS failed: {:#}", direction, e),
            });
            return 0;
        }
    };
    let tts_ms = tts_start.elapsed().as_millis() as u64;

    let audio_len = audio.len();

    if !audio.is_empty() {
        echo_suppress.store(true, Ordering::SeqCst);

        // Downsample to 16kHz for browser monitor (good quality, ~40KB per phrase)
        let monitor_rate = 16000u32;
        let monitor_samples = resample(&audio, sample_rate, monitor_rate);
        let mut pcm_bytes = Vec::with_capacity(monitor_samples.len() * 2);
        for &s in &monitor_samples {
            let i = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
            pcm_bytes.extend_from_slice(&i.to_le_bytes());
        }
        use base64::Engine as _;
        let b64 = base64::engine::general_purpose::STANDARD.encode(&pcm_bytes);
        let _ = event_tx.send(Event::TtsAudio {
            direction: direction.to_string(),
            sample_rate: monitor_rate,
            audio_b64: b64,
        });

        if let Err(e) = playback_tx.try_send(audio) {
            warn!("[{}] Playback channel full or disconnected: {}", direction, e);
        }

        let playback_ms = (audio_len as u64 * 1000) / sample_rate.max(1) as u64;
        const ECHO_COOLDOWN_MS: u64 = 280;
        let suppress = echo_suppress.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(playback_ms + ECHO_COOLDOWN_MS));
            suppress.store(false, Ordering::SeqCst);
        });
    }

    let _ = event_tx.send(Event::Metrics {
        stt_ms,
        translate_ms,
        tts_ms,
    });

    audio_len
}

// ---------------------------------------------------------------------------
// Audio utility
// ---------------------------------------------------------------------------

/// Resample audio from `from_rate` to `to_rate` using linear interpolation.
/// Handles arbitrary rate ratios (e.g. 24000→16000, 48000→16000).
fn resample(samples: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if from_rate == to_rate || samples.is_empty() {
        return samples.to_vec();
    }
    let ratio = from_rate as f64 / to_rate as f64;
    let output_len = (samples.len() as f64 / ratio) as usize;
    (0..output_len)
        .map(|i| {
            let src = i as f64 * ratio;
            let idx = src as usize;
            let frac = src - idx as f64;
            if idx + 1 < samples.len() {
                samples[idx] * (1.0 - frac as f32) + samples[idx + 1] * frac as f32
            } else {
                samples[idx.min(samples.len() - 1)]
            }
        })
        .collect()
}
