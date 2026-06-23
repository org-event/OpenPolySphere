//! macOS helper binaries and bundle names (shown in System Settings permission UI).

pub const SPEECH_AUTH_APP: &str = "BanyanSpeech.app";
pub const SPEECH_EXECUTABLE: &str = "BanyanSpeech";
pub const TRANSLATE_BINARY: &str = "banyan-translate";
pub const SPEECH_CLI_BINARY: &str = "banyan-speech";

pub const ENV_SPEECH_HELPER: &str = "BANYAN_SPEECH_HELPER";
pub const ENV_SPEECH_AUTH_APP: &str = "BANYAN_SPEECH_AUTH_APP";
pub const ENV_SPEECH_CONTEXT: &str = "BANYAN_SPEECH_CONTEXT";
pub const ENV_TRANSLATE_HELPER: &str = "BANYAN_TRANSLATE_HELPER";

pub fn optional_env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|s| !s.is_empty())
}
