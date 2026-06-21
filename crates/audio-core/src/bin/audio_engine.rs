//! Legacy stdin/stdout protocol binary (debug only). Use `translator` for production.

use std::io::{BufReader, BufWriter, Write};
use std::thread;

use anyhow::Result;
use audio_core::engine::{Engine, EngineConfig};
use audio_core::protocol::{read_command, write_event, Event};
use crossbeam_channel::bounded;
use log::{debug, info};

fn main() -> Result<()> {
    env_logger::Builder::from_default_env()
        .target(env_logger::Target::Stderr)
        .init();

    audio_core::init_ort();

    let (event_tx, event_rx) = bounded::<Event>(256);

    let writer_handle = thread::Builder::new()
        .name("event-writer".into())
        .spawn(move || {
            let stdout = std::io::stdout().lock();
            let mut writer = BufWriter::new(stdout);
            while let Ok(event) = event_rx.recv() {
                if write_event(&mut writer, &event).is_err() {
                    break;
                }
                let _ = writer.flush();
            }
        })?;

    let config = EngineConfig::from_env();
    let mut engine = Engine::new(config, event_tx.clone());

    let stdin = std::io::stdin().lock();
    let mut reader = BufReader::new(stdin);

    loop {
        let cmd = match read_command(&mut reader)? {
            Some(cmd) => cmd,
            None => break,
        };
        debug!("Received command: {:?}", cmd);
        for event in engine.handle_command(cmd) {
            let _ = event_tx.send(event);
        }
        if engine.is_shutting_down() {
            break;
        }
    }

    drop(event_tx);
    let _ = writer_handle.join();
    info!("audio_engine stopped");
    Ok(())
}
