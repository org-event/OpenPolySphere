//! macOS Apple Speech framework via `LiveTranslator.app` / `LiveTranslateSpeech` helper.

use anyhow::{bail, Context, Result};

#[cfg(target_os = "macos")]
mod macos {
    use super::*;
    use std::path::PathBuf;
    use std::process::Command;
    use std::sync::OnceLock;

    use log::debug;
    use serde::Deserialize;

    use crate::stt::local::{TranscribeOutcome, WhisperBackend, WHISPER_SAMPLE_RATE};

    #[derive(Debug, Deserialize)]
    struct HelperResponse {
        #[serde(default)]
        available: bool,
        #[serde(default)]
        ready: bool,
        #[serde(default)]
        on_device: bool,
        #[serde(default)]
        status: String,
        #[serde(default)]
        authorization: String,
        #[serde(default)]
        transcript: String,
        #[serde(default)]
        no_speech_prob: f32,
        #[serde(default)]
        error: String,
    }

    fn helper_path() -> Option<PathBuf> {
        if let Ok(path) = std::env::var("APPLE_SPEECH_HELPER") {
            let p = PathBuf::from(path);
            if p.is_file() {
                return Some(p);
            }
        }
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                let bundled = dir
                    .join("LiveTranslator.app")
                    .join("Contents/MacOS/LiveTranslateSpeech");
                if bundled.is_file() {
                    return Some(bundled);
                }
            }
        }
        [PathBuf::from(
            "target/release/LiveTranslator.app/Contents/MacOS/LiveTranslateSpeech",
        )]
        .into_iter()
        .find(|candidate| candidate.is_file())
    }

    fn auth_app_path() -> Option<PathBuf> {
        if let Ok(path) = std::env::var("APPLE_SPEECH_AUTH_APP") {
            let p = PathBuf::from(path);
            if p.is_dir() {
                return Some(p);
            }
        }
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                let p = dir.join("LiveTranslator.app");
                if p.is_dir() {
                    return Some(p);
                }
            }
        }
        [
            PathBuf::from("target/release/LiveTranslator.app"),
            PathBuf::from("tools/apple-speech-auth/LiveTranslator.app"),
        ]
        .into_iter()
        .find(|candidate| candidate.is_dir())
    }

    pub fn request_authorization() -> Result<serde_json::Value> {
        let app = auth_app_path()
            .context("LiveTranslator.app not found (rebuild translator on macOS)")?;
        let out = std::env::temp_dir().join(format!(
            "call-translator-speech-auth-{}.json",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&out);

        log::info!("Opening speech recognition permission dialog (LiveTranslator.app)...");
        let status = Command::new("open")
            .arg("-W")
            .arg("-a")
            .arg(&app)
            .arg("--args")
            .arg(&out)
            .status()
            .with_context(|| format!("failed to open {}", app.display()))?;

        if out.is_file() {
            let raw = std::fs::read_to_string(&out)?;
            let _ = std::fs::remove_file(&out);
            let value: serde_json::Value = serde_json::from_str(&raw)?;
            return Ok(value);
        }

        if !status.success() {
            bail!("LiveTranslator.app exited without granting speech recognition");
        }

        let check = run_helper(
            &[
                "check",
                &std::env::var("TRANSLATOR_MY_LANG").unwrap_or_else(|_| "ru".into()),
            ],
            None,
            None,
        )
        .ok();
        Ok(serde_json::json!({
            "authorization": check.as_ref().map(|r| r.authorization.as_str()).unwrap_or("unknown"),
            "ready": check.as_ref().map(|r| r.ready).unwrap_or(false),
            "message": "Open System Settings → Privacy & Security → Speech Recognition and allow Live Translator.",
        }))
    }

    pub fn ensure_speech_authorized(lang: &str) -> Result<()> {
        let status = availability(lang);
        if status
            .get("ready")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            log::info!("Apple Speech already authorized for {lang}");
            return Ok(());
        }
        log::info!(
            "Speech recognition permission required for {lang} — waiting for system dialog..."
        );
        let result = request_authorization()?;
        if result
            .get("ready")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            log::info!("Apple Speech authorized");
            return Ok(());
        }
        let auth = result
            .get("authorization")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        bail!(
            "Speech recognition not authorized ({auth}). Enable Live Translator in System Settings → Privacy & Security → Speech Recognition."
        );
    }

    use std::sync::atomic::{AtomicU64, Ordering};

    static HELPER_SEQ: AtomicU64 = AtomicU64::new(0);

    fn run_helper(
        args: &[&str],
        pcm: Option<&[f32]>,
        context: Option<&str>,
    ) -> Result<HelperResponse> {
        let app = auth_app_path()
            .context("LiveTranslator.app not found (rebuild translator on macOS)")?;
        let seq = HELPER_SEQ.fetch_add(1, Ordering::Relaxed);
        let out = std::env::temp_dir().join(format!("lt-speech-{}-{seq}.json", std::process::id()));
        let _ = std::fs::remove_file(&out);

        let pcm_path = if let Some(samples) = pcm {
            let path =
                std::env::temp_dir().join(format!("lt-pcm-{}-{seq}.raw", std::process::id()));
            {
                use std::io::Write;
                let mut file = std::fs::File::create(&path)
                    .with_context(|| format!("create {}", path.display()))?;
                for &s in samples {
                    file.write_all(&s.to_le_bytes())?;
                }
            }
            Some(path)
        } else {
            None
        };

        let mut open_args: Vec<String> = args.iter().map(|s| (*s).to_string()).collect();
        if let Some(ref path) = pcm_path {
            open_args.push(path.to_string_lossy().into_owned());
        }
        if let Some(ctx) = context.filter(|s| !s.is_empty()) {
            open_args.push("--context".into());
            open_args.push(ctx.to_string());
        }
        open_args.push("--out".into());
        open_args.push(out.to_string_lossy().into_owned());

        debug!("LiveTranslator.app: {}", open_args.join(" "));

        let status = Command::new("open")
            .arg("-W")
            .arg("-a")
            .arg(&app)
            .arg("--args")
            .args(&open_args)
            .status()
            .with_context(|| format!("failed to open {}", app.display()))?;

        if let Some(path) = pcm_path.as_ref() {
            let _ = std::fs::remove_file(path);
        }

        if !out.is_file() {
            bail!("LiveTranslateSpeech returned no output (status={status})");
        }

        let raw = std::fs::read_to_string(&out)?;
        let _ = std::fs::remove_file(&out);
        let line = raw.lines().next().unwrap_or("").trim();
        if line.is_empty() {
            bail!("LiveTranslateSpeech returned empty JSON (status={status})");
        }

        let resp: HelperResponse =
            serde_json::from_str(line).context("parse LiveTranslateSpeech JSON response")?;
        if !status.success() && resp.error.is_empty() {
            bail!("LiveTranslateSpeech failed (status={status})");
        }
        Ok(resp)
    }

    pub fn availability(lang: &str) -> serde_json::Value {
        if helper_path().is_none() {
            return serde_json::json!({
                "helper": false,
                "available": false,
                "ready": false,
                "status": "missing",
            });
        }
        match run_helper(&["check", lang], None, None) {
            Ok(resp) => serde_json::json!({
                "helper": true,
                "available": resp.available,
                "ready": resp.ready,
                "on_device": resp.on_device,
                "status": if resp.status.is_empty() {
                    "unknown".to_string()
                } else {
                    resp.status
                },
                "authorization": resp.authorization,
            }),
            Err(e) => {
                log::debug!("LiveTranslateSpeech check failed: {e:#}");
                serde_json::json!({
                    "helper": true,
                    "available": true,
                    "ready": false,
                    "status": "needs_permission",
                    "authorization": "unknown",
                })
            }
        }
    }

    fn context_words(source_lang: &str) -> Option<String> {
        if let Ok(extra) = std::env::var("APPLE_SPEECH_CONTEXT") {
            let extra = extra.trim();
            if !extra.is_empty() {
                return Some(extra.to_string());
            }
        }

        let their = std::env::var("TRANSLATOR_THEIR_LANG").unwrap_or_else(|_| "en".into());
        let words: Vec<&str> = match (source_lang, their.as_str()) {
            ("ru", "en") | ("ru", _) => vec![
                "ok",
                "okay",
                "hello",
                "hi",
                "thanks",
                "thank you",
                "please",
                "sorry",
                "yes",
                "no",
                "email",
                "call",
                "meeting",
                "zoom",
                "google",
                "apple",
                "microsoft",
                "team",
                "slack",
                "website",
                "online",
                "update",
                "bug",
                "fix",
                "test",
                "deploy",
                "server",
                "client",
                "api",
                "app",
                "product",
                "manager",
                "developer",
                "design",
                "marketing",
                "sales",
                "price",
                "deal",
                "contract",
                "deadline",
                "project",
                "status",
                "report",
                "review",
            ],
            ("en", "ru") | ("en", _) => vec![
                "privet",
                "spasibo",
                "da",
                "net",
                "horosho",
                "ladno",
                "pozhaluysta",
                "izvinite",
            ],
            _ => return None,
        };
        Some(words.join(","))
    }

    pub struct AppleSpeechBackend;

    impl WhisperBackend for AppleSpeechBackend {
        fn transcribe(&self, samples: &[f32], language: &str) -> Result<TranscribeOutcome> {
            let sample_rate = WHISPER_SAMPLE_RATE;
            let context = context_words(language);
            let resp = run_helper(
                &["recognize", language, &sample_rate.to_string()],
                Some(samples),
                context.as_deref(),
            )?;
            if !resp.error.is_empty() {
                bail!("{}", resp.error);
            }
            Ok(TranscribeOutcome {
                text: resp.transcript,
                no_speech_prob: resp.no_speech_prob,
            })
        }
    }

    static ENGINE: OnceLock<std::sync::Arc<AppleSpeechBackend>> = OnceLock::new();

    pub fn shared_backend() -> Result<std::sync::Arc<AppleSpeechBackend>> {
        if helper_path().is_none() {
            bail!("Apple Speech helper binary is not available on this system");
        }
        Ok(ENGINE
            .get_or_init(|| std::sync::Arc::new(AppleSpeechBackend))
            .clone())
    }
}

#[cfg(target_os = "macos")]
pub fn apple_speech_ensure_authorized() -> Result<()> {
    let lang = std::env::var("TRANSLATOR_MY_LANG").unwrap_or_else(|_| "ru".into());
    macos::ensure_speech_authorized(&lang)
}

#[cfg(not(target_os = "macos"))]
pub fn apple_speech_ensure_authorized() -> Result<()> {
    Ok(())
}

#[cfg(target_os = "macos")]
pub fn apple_speech_request_authorization() -> Result<serde_json::Value> {
    macos::request_authorization()
}

#[cfg(not(target_os = "macos"))]
pub fn apple_speech_request_authorization() -> Result<serde_json::Value> {
    bail!("Apple Speech is only available on macOS")
}

#[cfg(target_os = "macos")]
pub fn apple_speech_availability(lang: &str) -> serde_json::Value {
    macos::availability(lang)
}

#[cfg(target_os = "macos")]
pub fn apple_speech_backend() -> Result<std::sync::Arc<dyn crate::stt::local::WhisperBackend>> {
    let backend: std::sync::Arc<dyn crate::stt::local::WhisperBackend> = macos::shared_backend()?;
    Ok(backend)
}

#[cfg(not(target_os = "macos"))]
pub fn apple_speech_availability(_lang: &str) -> serde_json::Value {
    serde_json::json!({
        "helper": false,
        "available": false,
        "ready": false,
        "status": "unsupported",
    })
}

#[cfg(not(target_os = "macos"))]
pub fn apple_speech_backend() -> Result<std::sync::Arc<dyn crate::stt::local::WhisperBackend>> {
    bail!("Apple Speech is only available on macOS")
}
