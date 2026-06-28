//! Cross-platform boundaries: capabilities and default paths.
//!
//! `#[cfg(target_os = …)]` for product behavior should live here or in
//! `stt/apple`, `translation/apple`, and `stt/local/metal` — not in engine
//! orchestrators.

mod bundle;
mod capabilities;
mod defaults;
mod paths;

pub use bundle::{
    optional_env, ENV_SPEECH_AUTH_APP, ENV_SPEECH_CONTEXT, ENV_SPEECH_HELPER, ENV_TRANSLATE_HELPER,
    SPEECH_AUTH_APP, SPEECH_CLI_BINARY, SPEECH_EXECUTABLE, TRANSLATE_BINARY,
};
pub use capabilities::Capabilities;
pub use defaults::{default_meet_input_device, default_meet_output_device};
pub use paths::{
    bundled_ort_dylib, default_ort_dylib, find_espeak_ng, ort_dylib_exists, ort_missing_hint,
    resolve_ort_dylib,
};
