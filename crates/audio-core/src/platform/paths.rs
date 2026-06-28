//! Default paths and install hints per OS.

use std::path::Path;
use std::process::Command;

/// Default ONNX Runtime dynamic library name/path for this target.
pub fn default_ort_dylib() -> &'static str {
    #[cfg(target_os = "macos")]
    {
        if cfg!(target_arch = "x86_64") {
            "/usr/local/lib/libonnxruntime.dylib"
        } else {
            "/opt/homebrew/lib/libonnxruntime.dylib"
        }
    }
    #[cfg(target_os = "windows")]
    {
        "onnxruntime.dll"
    }
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    {
        "libonnxruntime.so"
    }
}

/// Resolve ONNX Runtime dylib: env → bundled .app Frameworks → dev default.
pub fn resolve_ort_dylib() -> String {
    if let Ok(path) = std::env::var("ORT_DYLIB_PATH") {
        if !path.is_empty() {
            return path;
        }
    }
    if let Some(path) = bundled_ort_dylib() {
        return path.to_string_lossy().into_owned();
    }
    default_ort_dylib().to_string()
}

/// ONNX Runtime shipped inside OpenPolySphere.app (Frameworks/) or next to the binary.
pub fn bundled_ort_dylib() -> Option<std::path::PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let parent = exe.parent()?;

    #[cfg(target_os = "macos")]
    {
        if parent.file_name().and_then(|n| n.to_str()) == Some("Resources") {
            let frameworks = parent
                .parent()?
                .join("Frameworks")
                .join("libonnxruntime.dylib");
            if frameworks.is_file() {
                return Some(frameworks);
            }
        }
        if parent.file_name().and_then(|n| n.to_str()) == Some("MacOS") {
            let frameworks = parent
                .parent()?
                .join("Frameworks")
                .join("libonnxruntime.dylib");
            if frameworks.is_file() {
                return Some(frameworks);
            }
        }
    }

    let sibling = parent.join(
        #[cfg(target_os = "macos")]
        "libonnxruntime.dylib",
        #[cfg(target_os = "windows")]
        "onnxruntime.dll",
        #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
        "libonnxruntime.so",
    );
    if sibling.is_file() {
        return Some(sibling);
    }
    None
}

/// User-facing hint when ONNX Runtime is missing at setup time.
pub fn ort_missing_hint(path: &str) -> String {
    #[cfg(target_os = "windows")]
    {
        format!(
            "[!] ONNX Runtime not at {path} — place onnxruntime.dll next to translator.exe or set ORT_DYLIB_PATH"
        )
    }
    #[cfg(target_os = "macos")]
    {
        format!("[!] ONNX Runtime not at {path} — brew install onnxruntime")
    }
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    {
        format!(
            "[!] ONNX Runtime not at {path} — install libonnxruntime (e.g. apt install libonnxruntime-dev)"
        )
    }
}

/// Resolve espeak-ng binary (checks PATH and common install locations).
pub fn find_espeak_ng() -> anyhow::Result<String> {
    #[cfg(target_os = "macos")]
    let candidates = [
        "espeak-ng",
        "/usr/bin/espeak-ng",
        "/opt/homebrew/bin/espeak-ng",
        "/usr/local/bin/espeak-ng",
    ];
    #[cfg(target_os = "windows")]
    let candidates = [
        "espeak-ng",
        "/usr/bin/espeak-ng",
        r"C:\Program Files\eSpeak NG\espeak-ng.exe",
    ];
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    let candidates = ["espeak-ng", "/usr/bin/espeak-ng"];

    for candidate in candidates {
        if Command::new(candidate)
            .arg("--version")
            .output()
            .ok()
            .is_some_and(|o| o.status.success())
        {
            return Ok(candidate.to_string());
        }
    }

    #[cfg(target_os = "windows")]
    anyhow::bail!("espeak-ng not found. Install with: choco install espeak-ng");
    #[cfg(target_os = "macos")]
    anyhow::bail!("espeak-ng not found. Install with: brew install espeak-ng");
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    anyhow::bail!(
        "espeak-ng not found. Install with your package manager (e.g. apt install espeak-ng)"
    );
}

/// True when `path` points at an existing ONNX Runtime library file.
pub fn ort_dylib_exists(path: &str) -> bool {
    Path::new(path).is_file()
}
