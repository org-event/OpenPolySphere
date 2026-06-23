//! Cross-platform boundaries: capabilities and default paths.
//!
//! `#[cfg(target_os = …)]` for product behavior should live here or in
//! `stt/apple`, `translation/apple`, and `stt/local/metal` — not in engine
//! orchestrators.

mod capabilities;
mod paths;

pub use capabilities::Capabilities;
pub use paths::{default_ort_dylib, find_espeak_ng, ort_dylib_exists, ort_missing_hint};
