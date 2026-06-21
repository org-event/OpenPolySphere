pub mod audio;
pub mod engine;
pub mod protocol;
pub mod stt;
pub mod translation;
pub mod tts;

pub fn init_ort() {
    let ort_dylib = std::env::var("ORT_DYLIB_PATH").unwrap_or_else(|_| {
        if cfg!(target_arch = "x86_64") {
            "/usr/local/lib/libonnxruntime.dylib".into()
        } else {
            "/opt/homebrew/lib/libonnxruntime.dylib".into()
        }
    });
    ort::init_from(&ort_dylib)
        .unwrap_or_else(|e| panic!("Failed to load ONNX Runtime from {ort_dylib}: {e}"))
        .commit();
    log::info!("ONNX Runtime loaded from {}", ort_dylib);
}
