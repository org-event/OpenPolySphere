//! Speech-to-text via Deepgram Nova-3 streaming WebSocket API.

use std::io::ErrorKind;
use std::time::Instant;

use anyhow::{bail, Context, Result};
use log::{debug, info, warn};
use serde::Deserialize;
use tungstenite::client::IntoClientRequest;
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{connect, Message, WebSocket};

use super::SttResult;

pub struct DeepgramStt {
    api_key: String,
    language: String,
    endpointing_ms: u32,
}

impl DeepgramStt {
    pub fn new(api_key: String, language: String, endpointing_ms: u32) -> Self {
        let language = match language.as_str() {
            "pt" => "pt-BR",
            "no" => "nb",
            code => code,
        }
        .to_string();
        Self {
            api_key,
            language,
            endpointing_ms,
        }
    }

    pub fn create_session(&self, sample_rate: u32) -> Result<DeepgramSession> {
        let url = format!(
            "wss://api.deepgram.com/v1/listen\
             ?model=nova-3\
             &language={}\
             &encoding=linear16\
             &sample_rate={}\
             &channels=1\
             &interim_results=true\
             &endpointing={}",
            self.language, sample_rate, self.endpointing_ms
        );

        let mut request = url
            .into_client_request()
            .context("Failed to build Deepgram request")?;
        request.headers_mut().insert(
            "Authorization",
            format!("Token {}", self.api_key)
                .parse()
                .context("Invalid API key header value")?,
        );

        info!(
            "Connecting to Deepgram (lang={}, {}Hz, endpointing={}ms)...",
            self.language, sample_rate, self.endpointing_ms
        );

        let (mut ws, _) = connect(request).context("Failed to connect to Deepgram WebSocket")?;
        set_nonblocking(&mut ws)?;

        info!("Deepgram session connected");
        Ok(DeepgramSession {
            ws,
            audio_sent_secs: 0.0,
            last_send_time: Instant::now(),
            sample_rate,
        })
    }
}

pub struct DeepgramSession {
    ws: WebSocket<MaybeTlsStream<std::net::TcpStream>>,
    audio_sent_secs: f64,
    last_send_time: Instant,
    sample_rate: u32,
}

impl DeepgramSession {
    pub fn send_audio(&mut self, samples: &[f32]) -> Result<()> {
        let bytes: Vec<u8> = samples
            .iter()
            .flat_map(|&s| {
                let i = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
                i.to_le_bytes()
            })
            .collect();

        match self.ws.send(Message::Binary(bytes)) {
            Ok(()) => {
                self.audio_sent_secs += samples.len() as f64 / self.sample_rate as f64;
                self.last_send_time = Instant::now();
                Ok(())
            }
            Err(tungstenite::Error::Io(e)) if e.kind() == ErrorKind::WouldBlock => Ok(()),
            Err(e) => Err(anyhow::anyhow!("Failed to send audio to Deepgram: {}", e)),
        }
    }

    pub fn poll_transcript(&mut self) -> Result<Option<SttResult>> {
        loop {
            match self.ws.read() {
                Ok(Message::Text(text)) => {
                    debug!("Deepgram: {}", &text[..text.len().min(200)]);
                    match serde_json::from_str::<DgResponse>(&text) {
                        Ok(resp) if resp.is_final == Some(true) => {
                            let transcript = resp
                                .channel
                                .and_then(|c| c.alternatives.into_iter().next())
                                .map(|a| a.transcript)
                                .unwrap_or_default();

                            if !transcript.trim().is_empty() {
                                let utterance_end_secs = resp.start.unwrap_or(0.0)
                                    + resp.duration.unwrap_or(0.0);
                                let backlog_secs = self.audio_sent_secs - utterance_end_secs;
                                let since_last_send_ms =
                                    self.last_send_time.elapsed().as_millis() as u64;
                                let stt_latency_ms =
                                    (backlog_secs * 1000.0).max(0.0) as u64 + since_last_send_ms;

                                info!(
                                    "Deepgram is_final: '{}' (stt={}ms)",
                                    transcript, stt_latency_ms
                                );
                                return Ok(Some(SttResult {
                                    text: transcript,
                                    stt_latency_ms,
                                }));
                            }
                        }
                        Ok(_) => {}
                        Err(e) => debug!("Deepgram parse error: {}", e),
                    }
                }
                Ok(_) => {}
                Err(tungstenite::Error::Io(e)) if e.kind() == ErrorKind::WouldBlock => {
                    return Ok(None);
                }
                Err(e) => bail!("Deepgram WebSocket error: {}", e),
            }
        }
    }

    pub fn close(&mut self) {
        let _ = self.ws.send(Message::Binary(vec![]));
        let _ = self.ws.close(None);
    }
}

#[derive(Deserialize)]
struct DgResponse {
    is_final: Option<bool>,
    start: Option<f64>,
    duration: Option<f64>,
    channel: Option<DgChannel>,
}

#[derive(Deserialize)]
struct DgChannel {
    alternatives: Vec<DgAlternative>,
}

#[derive(Deserialize)]
struct DgAlternative {
    transcript: String,
}

fn set_nonblocking(ws: &mut WebSocket<MaybeTlsStream<std::net::TcpStream>>) -> Result<()> {
    match ws.get_mut() {
        MaybeTlsStream::Plain(s) => s.set_nonblocking(true).context("set_nonblocking (plain)")?,
        MaybeTlsStream::NativeTls(s) => s
            .get_ref()
            .set_nonblocking(true)
            .context("set_nonblocking (tls)")?,
        _ => warn!("Unknown stream type, non-blocking not set"),
    }
    Ok(())
}
