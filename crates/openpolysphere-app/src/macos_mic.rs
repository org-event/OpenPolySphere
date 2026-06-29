//! Trigger macOS microphone permission from the main `.app` executable (TCC).

use std::time::Duration;

pub fn ensure_microphone_access() -> anyhow::Result<()> {
    use anyhow::Context;
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .context("No microphone available")?;
    let supported = device
        .default_input_config()
        .context("Failed to read microphone configuration")?;
    let sample_format = supported.sample_format();
    let config = supported.config();
    let stream = match sample_format {
        cpal::SampleFormat::F32 => device.build_input_stream(
            config,
            |_data: &[f32], _| {},
            |err| log::warn!("microphone probe: {err}"),
            None,
        ),
        cpal::SampleFormat::I16 => {
            let config = device.default_input_config()?.config();
            device.build_input_stream(
                config,
                |_data: &[i16], _| {},
                |err| log::warn!("microphone probe: {err}"),
                None,
            )
        }
        other => anyhow::bail!("Unsupported microphone sample format: {other:?}"),
    }
    .context(
        "Microphone access denied — enable OpenPolySphere in System Settings → Privacy & Security → Microphone",
    )?;
    stream.play().context("Failed to start microphone stream")?;
    std::thread::sleep(Duration::from_millis(200));
    drop(stream);
    log::info!("Microphone access OK (main app)");
    Ok(())
}
