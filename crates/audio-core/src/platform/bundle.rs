//! macOS helper binaries and bundle names (shown in System Settings permission UI).

pub const SPEECH_AUTH_APP: &str = "PolySphereSpeech.app";
pub const SPEECH_EXECUTABLE: &str = "PolySphereSpeech";
pub const TRANSLATE_BINARY: &str = "polysphere-translate";
pub const SPEECH_CLI_BINARY: &str = "polysphere-speech";

pub const ENV_SPEECH_HELPER: &str = "POLYSPHERE_SPEECH_HELPER";
pub const ENV_SPEECH_AUTH_APP: &str = "POLYSPHERE_SPEECH_AUTH_APP";
pub const ENV_SPEECH_CONTEXT: &str = "POLYSPHERE_SPEECH_CONTEXT";
pub const ENV_TRANSLATE_HELPER: &str = "POLYSPHERE_TRANSLATE_HELPER";

pub fn optional_env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|s| !s.is_empty())
}
