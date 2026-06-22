pub mod audio;
pub mod engine;
pub mod protocol;
pub mod stt;
pub mod translation;
pub mod tts;

pub fn init_ort() {
    let ort_dylib = std::env::var("ORT_DYLIB_PATH").unwrap_or_else(|_| default_ort_dylib());
    ort::init_from(&ort_dylib)
        .unwrap_or_else(|e| panic!("Failed to load ONNX Runtime from {ort_dylib}: {e}"))
        .commit();
    log::info!("ONNX Runtime loaded from {}", ort_dylib);
}

fn default_ort_dylib() -> String {
    #[cfg(target_os = "macos")]
    {
        if cfg!(target_arch = "x86_64") {
            "/usr/local/lib/libonnxruntime.dylib".into()
        } else {
            "/opt/homebrew/lib/libonnxruntime.dylib".into()
        }
    }
    #[cfg(target_os = "windows")]
    {
        "onnxruntime.dll".into()
    }
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    {
        "libonnxruntime.so".into()
    }
}
