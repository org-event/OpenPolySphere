//! Bridge audio-core events → SSE broadcast + DB + side caches.

use std::sync::{Arc, Mutex};

use audio_core::protocol::Event;
use crossbeam_channel::Receiver;
use log::{error, info, warn};
use serde_json::{json, Value};
use tokio::sync::broadcast;

use crate::db::Db;

pub struct SideState {
    pub tts_queue: Mutex<Vec<Value>>,
    pub audio_levels: Mutex<Value>,
    pub devices: Mutex<Option<(Vec<String>, Vec<String>)>>,
}

impl Default for SideState {
    fn default() -> Self {
        Self {
            tts_queue: Mutex::new(Vec::new()),
            audio_levels: Mutex::new(json!({})),
            devices: Mutex::new(None),
        }
    }
}

pub fn spawn_event_loop(
    event_rx: Receiver<Event>,
    sse_tx: broadcast::Sender<String>,
    db: Arc<Db>,
    side: Arc<SideState>,
) {
    std::thread::Builder::new()
        .name("engine-events".into())
        .spawn(move || {
            while let Ok(event) = event_rx.recv() {
                dispatch_event(&event, &sse_tx, &db, &side);
            }
            info!("Engine event loop exited");
        })
        .expect("spawn engine-events thread");
}

fn dispatch_event(event: &Event, sse_tx: &broadcast::Sender<String>, db: &Db, side: &SideState) {
    match event {
        Event::Transcript {
            direction,
            text,
            ..
        } => {
            let line = format!("🎤 [{direction}] {text}");
            info!("{line}");
            let _ = sse_tx.send(line);
            db.record_transcript(direction, text);
        }
        Event::Translation { direction, text } => {
            let line = format!("🌐 [{direction}] {text}");
            info!("{line}");
            let _ = sse_tx.send(line);
            db.record_translation(direction, text);
        }
        Event::Metrics {
            stt_ms,
            translate_ms,
            tts_ms,
        } => {
            let line = format!("⏱  stt={stt_ms}ms trl={translate_ms}ms tts={tts_ms}ms\n");
            info!("{}", line.trim());
            let _ = sse_tx.send(line);
        }
        Event::Error { message } => {
            error!("Engine error: {message}");
            let _ = sse_tx.send(format!("⚠️ {message}"));
        }
        Event::Log { level, message } => match level.as_str() {
            "error" => error!("Engine: {message}"),
            "warn" => warn!("Engine: {message}"),
            _ => info!("Engine: {message}"),
        },
        Event::DeviceList { input, output } => {
            *side.devices.lock().unwrap() = Some((input.clone(), output.clone()));
        }
        Event::AudioLevels {
            mic,
            call_in,
            mic_active,
            call_in_active,
            mic_error,
            call_in_error,
        } => {
            *side.audio_levels.lock().unwrap() = json!({
                "mic": mic,
                "call_in": call_in,
                "mic_active": mic_active,
                "call_in_active": call_in_active,
                "mic_error": mic_error,
                "call_in_error": call_in_error,
            });
        }
        Event::TtsAudio {
            sample_rate,
            audio_b64,
            ..
        } => {
            let mut q = side.tts_queue.lock().unwrap();
            q.push(json!({ "sr": sample_rate, "b64": audio_b64 }));
            if q.len() > 5 {
                let drain = q.len() - 5;
                q.drain(0..drain);
            }
        }
        Event::Started { pipelines } => {
            info!("Engine started pipelines: {pipelines:?}");
        }
        Event::Stopped => info!("Engine stopped all pipelines"),
        Event::Pong | Event::TtsPreviewDone => {}
    }
}
