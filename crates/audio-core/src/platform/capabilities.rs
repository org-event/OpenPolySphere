//! Platform capabilities — single place for `cfg(target_os = …)` checks.

/// What this build of Banyan can use on the host OS.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Capabilities {
    pub apple_stt: bool,
    pub apple_translation: bool,
    pub whisper_metal: bool,
}

impl Capabilities {
    /// Compile-time capabilities for the target triple being built.
    pub const fn current() -> Self {
        Self {
            apple_stt: cfg!(target_os = "macos"),
            apple_translation: cfg!(target_os = "macos"),
            whisper_metal: cfg!(target_os = "macos"),
        }
    }
}

impl Capabilities {
    pub fn require_apple_stt(&self) -> anyhow::Result<()> {
        if self.apple_stt {
            Ok(())
        } else {
            anyhow::bail!("Apple Speech STT is only available on macOS")
        }
    }

    pub fn require_apple_translation(&self) -> anyhow::Result<()> {
        if self.apple_translation {
            Ok(())
        } else {
            anyhow::bail!("Apple Translation is only available on macOS")
        }
    }
}
