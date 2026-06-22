use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Stream, StreamConfig};
use crossbeam_channel::Receiver;
use log::{debug, error, info};
use ringbuf::{
    traits::{Consumer, Producer, Split},
    HeapRb,
};

/// Plays audio received from a channel to a named output device.
pub struct AudioPlayback {
    stream: Stream,
    _device_name: String,
    _config: StreamConfig,
    _feeder: std::thread::JoinHandle<()>,
}

impl AudioPlayback {
    /// Create playback to a specific device name.
    ///
    /// `device_name`: `"default"` for default output, or a specific name like `"BlackHole 2ch"`.
    /// `sample_rate`: desired sample rate (e.g. 48000).
    /// `receiver`: channel providing audio sample buffers to play.
    pub fn new(device_name: &str, sample_rate: u32, receiver: Receiver<Vec<f32>>) -> Result<Self> {
        let device = find_output_device(device_name)?;
        let actual_name = device.name().unwrap_or_else(|_| "unknown".into());

        let config = StreamConfig {
            channels: 1,
            sample_rate: cpal::SampleRate(sample_rate),
            buffer_size: cpal::BufferSize::Fixed(960), // 20ms at 48kHz
        };

        info!(
            "Opening playback device '{}' with config: rate={}, channels={}, buffer=960",
            actual_name, sample_rate, config.channels
        );

        let supported = device
            .supported_output_configs()
            .context("Failed to query supported output configs")?;

        debug!("Supported output configs for '{}':", actual_name);
        for cfg in supported {
            debug!("  {:?}", cfg);
        }

        // Ring buffer: 10 seconds — enough for long TTS phrases
        let ring_size = sample_rate as usize * 10;
        let ring = HeapRb::<f32>::new(ring_size);
        let (mut producer, mut consumer) = ring.split();

        // Feeder thread: reads from crossbeam channel, pushes into ring buffer
        let feeder_name = actual_name.clone();
        let feeder = std::thread::Builder::new()
            .name(format!("playback-feeder-{}", feeder_name))
            .spawn(move || {
                debug!("Playback feeder thread started for '{}'", feeder_name);
                loop {
                    match receiver.recv() {
                        Ok(samples) => {
                            for &sample in &samples {
                                // If ring buffer is full, overwrite oldest (skip failed pushes)
                                let _ = producer.try_push(sample);
                            }
                        }
                        Err(_) => {
                            debug!("Playback feeder: channel disconnected, stopping");
                            break;
                        }
                    }
                }
            })
            .context("Failed to spawn playback feeder thread")?;

        let stream = device
            .build_output_stream(
                &config,
                move |data: &mut [f32], _info: &cpal::OutputCallbackInfo| {
                    let filled = consumer.pop_slice(data);
                    // Fill remaining with silence if underrun
                    if filled < data.len() {
                        for sample in &mut data[filled..] {
                            *sample = 0.0;
                        }
                        if filled > 0 {
                            debug!("Playback underrun: got {} / {} samples", filled, data.len());
                        }
                    }
                },
                move |err| {
                    error!("Playback stream error: {}", err);
                },
                None, // no timeout
            )
            .context("Failed to build output stream")?;

        Ok(Self {
            stream,
            _device_name: actual_name,
            _config: config,
            _feeder: feeder,
        })
    }

    pub fn start(&self) -> Result<()> {
        self.stream
            .play()
            .context("Failed to start playback stream")?;
        info!("Playback started on '{}'", self._device_name);
        Ok(())
    }

    pub fn stop(&self) -> Result<()> {
        self.stream
            .pause()
            .context("Failed to pause playback stream")?;
        info!("Playback stopped on '{}'", self._device_name);
        Ok(())
    }
}

/// Find an output device by name. `"default"` returns the default output device.
fn find_output_device(name: &str) -> Result<Device> {
    let host = cpal::default_host();

    if name == "default" {
        return host
            .default_output_device()
            .context("No default output device available");
    }

    let devices = host
        .output_devices()
        .context("Failed to enumerate output devices")?;

    let mut available = Vec::new();
    for device in devices {
        let dev_name = device.name().unwrap_or_else(|_| "unknown".into());
        if dev_name == name {
            return Ok(device);
        }
        available.push(dev_name);
    }

    anyhow::bail!(
        "Output device '{}' not found. Available output devices: {:?}",
        name,
        available
    )
}
