//! In-process engine wrapper — Engine lives on a dedicated thread (cpal is !Send).

use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use anyhow::{Context, Result};
use audio_core::engine::{Engine, EngineConfig};
use audio_core::protocol::{Command, Event};
use crossbeam_channel::{bounded, Receiver, Sender};

use crate::db::Db;
use crate::events::{spawn_event_loop, SideState};
use crate::paths::models_dir;
use crate::settings::{apply_env, engine_config, Settings};

enum EngineOp {
    Command(Command, Sender<Vec<Event>>),
    /// Engine thread responds (always true if alive).
    Alive(Sender<()>),
    /// Whether STT/TTS pipelines are actively running.
    IsRunning(Sender<bool>),
    Reload(Settings, Sender<Result<()>>),
    Shutdown,
}

pub struct EngineService {
    op_tx: Sender<EngineOp>,
    _thread: Mutex<Option<JoinHandle<()>>>,
    side: Arc<SideState>,
}

impl EngineService {
    pub fn new(
        settings: &Settings,
        db: Arc<Db>,
        side: Arc<SideState>,
        sse_tx: tokio::sync::broadcast::Sender<String>,
    ) -> Result<Arc<Self>> {
        let models = models_dir();
        apply_env(settings, &models);
        let config = engine_config(settings, &models);
        let (event_tx, event_rx) = bounded(256);
        spawn_event_loop(event_rx, sse_tx, db, side.clone());
        let (op_tx, op_rx) = bounded(64);
        let side2 = side.clone();
        let handle = thread::Builder::new()
            .name("audio-engine".into())
            .spawn(move || engine_loop(op_rx, config, event_tx, side2))
            .context("spawn audio-engine thread")?;
        Ok(Arc::new(Self {
            op_tx,
            _thread: Mutex::new(Some(handle)),
            side,
        }))
    }

    fn send(&self, op: EngineOp) -> Result<()> {
        self.op_tx
            .send(op)
            .map_err(|e| anyhow::anyhow!("engine thread gone: {e}"))
    }

    pub fn command(&self, cmd: Command) -> Vec<Event> {
        let (tx, rx) = bounded(1);
        if self.send(EngineOp::Command(cmd, tx)).is_err() {
            return vec![];
        }
        rx.recv().unwrap_or_default()
    }

    pub fn alive(&self) -> bool {
        let (tx, rx) = bounded(1);
        if self.send(EngineOp::Alive(tx)).is_err() {
            return false;
        }
        rx.recv().is_ok()
    }

    pub fn pipelines_running(&self) -> bool {
        let (tx, rx) = bounded(1);
        if self.send(EngineOp::IsRunning(tx)).is_err() {
            return false;
        }
        rx.recv().unwrap_or(false)
    }

    pub fn status(&self) -> &'static str {
        if self.pipelines_running() {
            "running"
        } else {
            "idle"
        }
    }

    pub fn reload(&self, settings: &Settings) -> Result<()> {
        let (tx, rx) = bounded(1);
        self.send(EngineOp::Reload(settings.clone(), tx))?;
        rx.recv()
            .map_err(|e| anyhow::anyhow!("reload reply: {e}"))?
    }

    pub fn handle_text(&self, raw: &str, settings: &Settings) -> String {
        let cmd = raw.trim();
        if cmd.is_empty() {
            return "error:empty".into();
        }
        match cmd {
            "start" => self.start_pipelines(&["outgoing", "incoming"]),
            "start outgoing" => self.start_pipelines(&["outgoing"]),
            "start incoming" => self.start_pipelines(&["incoming"]),
            "stop" => {
                let events = self.command(Command::Stop);
                if events.iter().any(|e| matches!(e, Event::Stopped)) {
                    "ok".into()
                } else {
                    "ok:already_stopped".into()
                }
            }
            "status" => format!("ok:{}", self.status()),
            "mute_outgoing" => {
                self.command(Command::SetConfig {
                    key: "mute_outgoing".into(),
                    value: serde_json::json!(true),
                });
                "ok".into()
            }
            "unmute_outgoing" => {
                self.command(Command::SetConfig {
                    key: "mute_outgoing".into(),
                    value: serde_json::json!(false),
                });
                "ok".into()
            }
            "mute_incoming" => {
                self.command(Command::SetConfig {
                    key: "mute_incoming".into(),
                    value: serde_json::json!(true),
                });
                "ok".into()
            }
            "unmute_incoming" => {
                self.command(Command::SetConfig {
                    key: "mute_incoming".into(),
                    value: serde_json::json!(false),
                });
                "ok".into()
            }
            "stop_level_monitors" => {
                self.command(Command::StopLevelMonitors);
                "ok".into()
            }
            "poll_levels" => serde_json::to_string(&*self.side.audio_levels.lock().unwrap())
                .unwrap_or_else(|_| "{}".into()),
            "poll_audio" => {
                let mut q = self.side.tts_queue.lock().unwrap();
                let items: Vec<_> = q.drain(..).collect();
                serde_json::to_string(&items).unwrap_or_else(|_| "[]".into())
            }
            "list_devices" => {
                self.command(Command::ListDevices);
                "ok:listing".into()
            }
            "restart" => {
                if let Err(e) = self.reload(settings) {
                    return format!("error:{e}");
                }
                "ok:restarting".into()
            }
            other if other.starts_with("preview:") => {
                let parts: Vec<_> = other.split(':').collect();
                if parts.len() >= 3 {
                    self.command(Command::TtsPreview {
                        lang: parts[1].to_string(),
                        voice: parts[2..].join(":"),
                    });
                    "ok:previewing".into()
                } else {
                    "error:bad_preview_format".into()
                }
            }
            other if other.starts_with("monitor_levels ") => {
                let json_str = other.trim_start_matches("monitor_levels ");
                match serde_json::from_str::<serde_json::Value>(json_str) {
                    Ok(v) => {
                        let mic = v.get("mic").and_then(|x| x.as_str()).unwrap_or("");
                        let call_in = v.get("call_in").and_then(|x| x.as_str()).unwrap_or("");
                        self.command(Command::MonitorLevels {
                            mic: mic.to_string(),
                            call_in: call_in.to_string(),
                        });
                        "ok".into()
                    }
                    Err(_) => "error:bad_monitor_format".into(),
                }
            }
            _ => {
                log::warn!("Unknown command: {cmd}");
                "error:unknown_command".into()
            }
        }
    }

    fn start_pipelines(&self, pipelines: &[&str]) -> String {
        let list: Vec<String> = pipelines.iter().map(|s| s.to_string()).collect();
        let events = self.command(Command::Start {
            pipelines: list.clone(),
        });
        if events.iter().any(|e| matches!(e, Event::Error { .. })) {
            if let Some(Event::Error { message }) =
                events.iter().find(|e| matches!(e, Event::Error { .. }))
            {
                return format!("error:{message}");
            }
        }
        if events.iter().any(|e| matches!(e, Event::Started { .. })) {
            "ok".into()
        } else {
            "error:start_failed".into()
        }
    }
}

impl Drop for EngineService {
    fn drop(&mut self) {
        let _ = self.send(EngineOp::Shutdown);
        if let Some(h) = self._thread.lock().unwrap().take() {
            let _ = h.join();
        }
    }
}

fn engine_loop(
    op_rx: Receiver<EngineOp>,
    config: EngineConfig,
    event_tx: Sender<Event>,
    _side: Arc<SideState>,
) {
    let mut engine = Engine::new(config, event_tx.clone());
    while let Ok(op) = op_rx.recv() {
        match op {
            EngineOp::Command(cmd, reply) => {
                let events = engine.handle_command(cmd);
                let _ = reply.send(events);
            }
            EngineOp::Alive(reply) => {
                let _ = reply.send(());
            }
            EngineOp::IsRunning(reply) => {
                let _ = reply.send(engine.is_running());
            }
            EngineOp::Reload(settings, reply) => {
                let result = {
                    if engine.is_running() {
                        for _ in engine.handle_command(Command::Stop) {}
                    }
                    let models = models_dir();
                    apply_env(&settings, &models);
                    let config = engine_config(&settings, &models);
                    engine.update_config(config);
                    Ok(())
                };
                let _ = reply.send(result);
            }
            EngineOp::Shutdown => break,
        }
    }
}

pub fn recreate_engine(
    settings: &Settings,
    db: Arc<Db>,
    side: Arc<SideState>,
    sse_tx: tokio::sync::broadcast::Sender<String>,
) -> Result<Arc<EngineService>> {
    audio_core::stt::local::invalidate_engine_cache();
    audio_core::translation::invalidate_polish_cache();
    EngineService::new(settings, db, side, sse_tx).context("create engine")
}
