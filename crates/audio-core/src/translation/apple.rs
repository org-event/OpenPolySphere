//! macOS Apple Translation framework via `apple-translate` helper binary.

#[cfg(target_os = "macos")]
mod imp {
    use anyhow::{bail, Context, Result};
    use log::debug;
    use serde::Deserialize;
    use std::path::PathBuf;
    use std::process::{Command, Stdio};

    use crate::translation::TranslationDirection;

    #[derive(Debug, Clone, Deserialize)]
    struct HelperResponse {
        #[serde(default)]
        available: bool,
        #[serde(default)]
        ready: bool,
        #[serde(default)]
        status: String,
        #[serde(default)]
        translation: String,
        #[serde(default)]
        error: String,
    }

    fn helper_path() -> Option<PathBuf> {
        if let Ok(path) = std::env::var("APPLE_TRANSLATE_HELPER") {
            let p = PathBuf::from(path);
            if p.is_file() {
                return Some(p);
            }
        }
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                let p = dir.join("apple-translate");
                if p.is_file() {
                    return Some(p);
                }
            }
        }
        [
            PathBuf::from("bin/apple-translate"),
            PathBuf::from("tools/apple-translate/.build/release/apple-translate"),
        ]
        .into_iter()
        .find(|candidate| candidate.is_file())
    }

    fn run_helper(args: &[&str]) -> Result<HelperResponse> {
        let bin = helper_path().context(
            "apple-translate helper not found (rebuild on macOS 15+ or set APPLE_TRANSLATE_HELPER)",
        )?;
        debug!("apple-translate: {} {}", bin.display(), args.join(" "));

        let output = Command::new(&bin)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .with_context(|| format!("failed to run {}", bin.display()))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let line = stdout.lines().next().unwrap_or("").trim();
        if line.is_empty() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!(
                "apple-translate returned no output (status={}): {}",
                output.status,
                stderr.trim()
            );
        }

        let resp: HelperResponse =
            serde_json::from_str(line).context("parse apple-translate JSON response")?;
        if !output.status.success() && resp.error.is_empty() {
            bail!("apple-translate failed (status={})", output.status);
        }
        Ok(resp)
    }

    #[derive(Debug, Clone)]
    pub struct AppleAvailability {
        pub available: bool,
        pub ready: bool,
        pub status: String,
    }

    pub fn pair_availability(from: &str, to: &str) -> Option<AppleAvailability> {
        let resp = run_helper(&["check", from, to]).ok()?;
        Some(AppleAvailability {
            available: resp.available,
            ready: resp.ready,
            status: if resp.status.is_empty() {
                "unknown".into()
            } else {
                resp.status
            },
        })
    }

    pub fn helper_installed() -> bool {
        helper_path().is_some()
    }

    pub struct AppleTranslateEngine;

    impl AppleTranslateEngine {
        pub fn new() -> Result<Self> {
            if helper_path().is_none() {
                bail!("Apple Translation helper binary is not available on this system");
            }
            Ok(Self)
        }

        pub fn translate(&self, text: &str, direction: &TranslationDirection) -> Result<String> {
            let resp = run_helper(&["translate", &direction.from_code, &direction.to_code, text])?;
            if !resp.error.is_empty() {
                bail!("{}", resp.error);
            }
            Ok(resp.translation)
        }
    }

    pub fn availability_for_settings(my_lang: &str, their_lang: &str) -> serde_json::Value {
        if !helper_installed() {
            return serde_json::json!({
                "helper": false,
                "available": false,
                "ready": false,
                "status": "missing",
            });
        }

        let forward = pair_availability(my_lang, their_lang);
        let reverse = pair_availability(their_lang, my_lang);
        let available = forward.as_ref().map(|a| a.available).unwrap_or(false)
            && reverse.as_ref().map(|a| a.available).unwrap_or(false);
        let ready = forward.as_ref().map(|a| a.ready).unwrap_or(false)
            && reverse.as_ref().map(|a| a.ready).unwrap_or(false);
        let status = if !available {
            "unsupported"
        } else if ready {
            "installed"
        } else {
            forward
                .as_ref()
                .map(|a| a.status.as_str())
                .unwrap_or("supported")
        };

        fn pair_json(pair: &Option<AppleAvailability>) -> serde_json::Value {
            match pair {
                Some(p) => serde_json::json!({
                    "available": p.available,
                    "ready": p.ready,
                    "status": p.status,
                }),
                None => serde_json::json!({
                    "available": false,
                    "ready": false,
                    "status": "error",
                }),
            }
        }

        serde_json::json!({
            "helper": true,
            "available": available,
            "ready": ready,
            "status": status,
            "pairs": {
                format!("{my_lang}-{their_lang}"): pair_json(&forward),
                format!("{their_lang}-{my_lang}"): pair_json(&reverse),
            },
        })
    }
}

#[cfg(not(target_os = "macos"))]
mod imp {
    use anyhow::{bail, Result};

    use crate::translation::TranslationDirection;

    pub fn availability_for_settings(_my_lang: &str, _their_lang: &str) -> serde_json::Value {
        serde_json::json!({
            "helper": false,
            "available": false,
            "ready": false,
            "status": "unsupported",
        })
    }

    pub struct AppleTranslateEngine;

    impl AppleTranslateEngine {
        pub fn new() -> Result<Self> {
            bail!("Apple Translation is only available on macOS")
        }

        pub fn translate(&self, _text: &str, _direction: &TranslationDirection) -> Result<String> {
            bail!("Apple Translation is only available on macOS")
        }
    }
}

pub use imp::*;

impl crate::translation::backend::TranslateBackend for AppleTranslateEngine {
    fn translate(
        &self,
        text: &str,
        direction: &crate::translation::TranslationDirection,
    ) -> anyhow::Result<String> {
        AppleTranslateEngine::translate(self, text, direction)
    }
}
