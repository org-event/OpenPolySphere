pub mod audio;
pub mod engine;
pub mod platform;
pub use audio_protocol as protocol;
pub mod stt;
pub mod translation;
pub mod tts;

pub fn init_ort() {
    let ort_dylib = std::env::var("ORT_DYLIB_PATH")
        .unwrap_or_else(|_| platform::default_ort_dylib().to_string());
    ort::init_from(&ort_dylib)
        .unwrap_or_else(|e| panic!("Failed to load ONNX Runtime from {ort_dylib}: {e}"))
        .commit();
    log::info!("ONNX Runtime loaded from {}", ort_dylib);
}
