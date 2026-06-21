//! First-run setup: system checks + model downloads.

use anyhow::Result;

use crate::downloads::{self, download_default_voices, download_polish_model, download_translation_models, download_whisper_model};
use crate::paths::{base_dir, ensure_parent};
use crate::settings::Settings;

pub async fn run() -> Result<()> {
    println!("=== Call Translator setup (Rust) ===\n");

    check_toolchain();
    check_ort_hint();
    ensure_env_file()?;

    println!("--- Default TTS voices (en, ru) ---");
    download_default_voices().await?;

    println!("--- Local translation (Opus-MT) ---");
    download_translation_models().await?;

    println!("--- Local STT (Whisper) ---");
    download_whisper_model().await?;

    println!("--- Local polish (Qwen2.5-0.5B, CTranslate2) ---");
    download_polish_model().await?;

    println!("\n--- Build ---");
    println!("Run: cargo build --release -p translator");

    println!("\n=== Setup complete ===");
    downloads::print_setup_status();
    println!("\nStart server:");
    println!("  cargo run --release -p translator");
    println!("  # or: ./target/release/translator");
    println!("Open http://127.0.0.1:5050");
    Ok(())
}

fn check_toolchain() {
    if std::process::Command::new("rustc")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        println!("[OK] Rust toolchain");
    } else {
        println!("[!] Rust not found — install: brew install rustup && rustup-init");
    }
}

fn check_ort_hint() {
    let dylib = std::env::var("ORT_DYLIB_PATH").unwrap_or_else(|_| {
        if cfg!(target_arch = "x86_64") {
            "/usr/local/lib/libonnxruntime.dylib".into()
        } else {
            "/opt/homebrew/lib/libonnxruntime.dylib".into()
        }
    });
    if std::path::Path::new(&dylib).is_file() {
        println!("[OK] ONNX Runtime ({dylib})");
    } else {
        println!("[!] ONNX Runtime not at {dylib} — brew install onnxruntime");
    }
}

fn ensure_env_file() -> Result<()> {
    let env = base_dir().join(".env");
    if env.is_file() {
        println!("[OK] .env");
        return Ok(());
    }
    let example = base_dir().join(".env.example");
    if example.is_file() {
        std::fs::copy(&example, &env)?;
        println!("[!] Created .env from .env.example — add API keys if using cloud STT/translation");
    } else {
        ensure_parent(&env)?;
        std::fs::write(
            &env,
            "DEEPGRAM_API_KEY=\nOPENROUTER_API_KEY=\nORT_DYLIB_PATH=\n",
        )?;
        println!("[!] Created minimal .env — add API keys only (settings live in settings.json)");
    }
    let _ = Settings::load();
    Ok(())
}
