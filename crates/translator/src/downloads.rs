//! Hugging Face model downloads (Whisper STT, Opus-MT translation).

use std::path::Path;

use anyhow::{Context, Result};
use audio_core::stt::local::{invalidate_engine_cache, resolve_whisper_model_dir};
use futures::StreamExt;
use log::info;
use tokio::io::AsyncWriteExt;

use crate::paths::models_dir;
use crate::settings::{local_translation_status, stt_status, Settings, USER_AGENT};

const HF: &str = "https://huggingface.co";

pub async fn download_whisper_model() -> Result<()> {
    let variant = Settings::load()
        .map(|s| s.whisper_model())
        .unwrap_or_else(|_| "auto".into());
    download_whisper_for(&variant).await
}

/// Download a whisper variant (`auto` → tiny for first-time setup).
pub async fn download_whisper_for(variant: &str) -> Result<()> {
    let variant = variant.to_lowercase();
    let target = if variant == "auto" {
        "base-q8_0"
    } else {
        variant.as_str()
    };
    download_whisper_variant(target).await
}

pub async fn download_whisper_variant(variant: &str) -> Result<()> {
    let variant = variant.to_lowercase();
    let device = Settings::load()
        .map(|s| s.stt_device())
        .unwrap_or_else(|_| audio_core::stt::local::default_stt_device_name());
    if device == "cpu" || device == "ct2" {
        download_whisper_ct2(&variant).await
    } else {
        download_whisper_ggml(&variant).await
    }
}

const GGML_REPO: &str = "ggerganov/whisper.cpp";

async fn download_whisper_ggml(variant: &str) -> Result<()> {
    let v = audio_core::stt::local::WhisperVariant::parse(variant);
    let ggml_name = v.ggml_filename();
    let label = format!("whisper-{}", v.core);
    let dir = models_dir().join("stt").join(&label);
    let dest = dir.join(&ggml_name);
    if dest.is_file() {
        info!("{label} GGML ({ggml_name}) already present");
        invalidate_engine_cache();
        return Ok(());
    }
    tokio::fs::create_dir_all(&dir).await?;
    let client = client()?;
    info!(
        "Downloading {GGML_REPO}/{ggml_name} (~{} MB, Metal/GPU)...",
        ggml_size_hint(variant)
    );
    download_file(&client, GGML_REPO, &ggml_name, &dest)
        .await
        .with_context(|| format!("download {ggml_name}"))?;
    if !dest.is_file() {
        anyhow::bail!("{label} GGML download incomplete");
    }
    invalidate_engine_cache();
    info!("Installed {label} (GGML {ggml_name}) at {}", dest.display());
    Ok(())
}

async fn download_whisper_ct2(variant: &str) -> Result<()> {
    let core = audio_core::stt::local::WhisperVariant::parse(variant).core;
    let (repo, openai_repo, label) = match core {
        "tiny" => (
            "Systran/faster-whisper-tiny",
            "openai/whisper-tiny",
            "whisper-tiny",
        ),
        "base" => (
            "Systran/faster-whisper-base",
            "openai/whisper-base",
            "whisper-base",
        ),
        _ => (
            "Systran/faster-whisper-small",
            "openai/whisper-small",
            "whisper-small",
        ),
    };

    let dir = models_dir().join("stt").join(label);
    if whisper_ct2_ready(&dir) {
        info!("{label} CT2 already present");
        invalidate_engine_cache();
        return Ok(());
    }
    tokio::fs::create_dir_all(&dir).await?;

    let client = client()?;
    let files = [
        "model.bin",
        "tokenizer.json",
        "config.json",
        "vocabulary.txt",
    ];
    info!("Downloading {repo} (~{} MB, CPU)...", size_hint(core));
    for f in files {
        download_file(&client, repo, f, &dir.join(f))
            .await
            .with_context(|| format!("download whisper {f}"))?;
    }
    download_file(
        &client,
        openai_repo,
        "preprocessor_config.json",
        &dir.join("preprocessor_config.json"),
    )
    .await
    .context("download preprocessor_config.json")?;

    if !whisper_ct2_ready(&dir) {
        anyhow::bail!("{label} download incomplete");
    }
    invalidate_engine_cache();
    info!("Installed {label} (CT2) at {}", dir.display());
    Ok(())
}

fn size_hint(variant: &str) -> &'static str {
    match audio_core::stt::local::WhisperVariant::parse(variant).core {
        "tiny" => "75",
        "base" => "145",
        _ => "460",
    }
}

fn ggml_size_hint(variant: &str) -> &'static str {
    match variant {
        "base-q8_0" => "148",
        "tiny-q8_0" => "78",
        "tiny" => "75",
        "base" => "145",
        _ => "460",
    }
}

const POLISH_REPO: &str = "winstxnhdw/Qwen2.5-0.5B-Instruct-ct2-int8";
const POLISH_LABEL: &str = "qwen2.5-0.5b-instruct";

pub async fn download_polish_model() -> Result<()> {
    use audio_core::translation::invalidate_polish_cache;

    let dir = models_dir().join("polish").join(POLISH_LABEL);
    if polish_ready(&dir) {
        info!("{POLISH_LABEL} already present");
        invalidate_polish_cache();
        return Ok(());
    }
    if dir.exists() {
        tokio::fs::remove_dir_all(&dir).await.ok();
    }
    tokio::fs::create_dir_all(&dir).await?;

    let client = client()?;
    let files = [
        "model.bin",
        "config.json",
        "tokenizer.json",
        "vocabulary.json",
    ];
    info!("Downloading {POLISH_REPO} (~400 MB)...");
    for f in files {
        download_file(&client, POLISH_REPO, f, &dir.join(f))
            .await
            .with_context(|| format!("download polish/{f}"))?;
    }
    if !polish_ready(&dir) {
        anyhow::bail!("polish model download incomplete");
    }
    invalidate_polish_cache();
    info!("Installed {POLISH_LABEL} at {}", dir.display());
    Ok(())
}

fn polish_ready(dir: &Path) -> bool {
    dir.join("model.bin").is_file() && dir.join("tokenizer.json").is_file()
}

pub async fn download_translation_models() -> Result<()> {
    let base = models_dir().join("translate");
    tokio::fs::create_dir_all(&base).await?;
    let pairs = [
        ("ooeoeo/opus-mt-ru-en-ct2-float16", "opus-mt-ru-en"),
        ("ooeoeo/opus-mt-en-ru-ct2-float16", "opus-mt-en-ru"),
    ];
    let files = [
        "model.bin",
        "source.spm",
        "target.spm",
        "shared_vocabulary.json",
        "vocab.json",
        "config.json",
        "tokenizer_config.json",
    ];
    let client = client()?;
    for (repo, name) in pairs {
        let out = base.join(name);
        if translate_ready(&out) {
            info!("{name} already present");
            continue;
        }
        if out.exists() {
            tokio::fs::remove_dir_all(&out).await.ok();
        }
        tokio::fs::create_dir_all(&out).await?;
        info!("Downloading {repo} → {name}...");
        for f in files {
            download_file(&client, repo, f, &out.join(f))
                .await
                .with_context(|| format!("download {name}/{f}"))?;
        }
        if !translate_ready(&out) {
            anyhow::bail!("{name} download incomplete");
        }
    }
    Ok(())
}

fn whisper_ct2_ready(dir: &Path) -> bool {
    dir.join("model.bin").is_file() && dir.join("preprocessor_config.json").is_file()
}

fn translate_ready(dir: &Path) -> bool {
    dir.join("model.bin").is_file() && dir.join("shared_vocabulary.json").is_file()
}

fn client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(std::time::Duration::from_secs(600))
        .build()
        .context("http client")
}

async fn download_file(
    client: &reqwest::Client,
    repo: &str,
    file: &str,
    dest: &Path,
) -> Result<()> {
    if dest.is_file() {
        return Ok(());
    }
    if let Some(parent) = dest.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let url = format!("{HF}/{repo}/resolve/main/{file}");
    let resp = client.get(&url).send().await?.error_for_status()?;
    let mut file_out = tokio::fs::File::create(dest).await?;
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        file_out.write_all(&chunk?).await?;
    }
    file_out.flush().await?;
    Ok(())
}

pub async fn download_default_voices() -> Result<()> {
    use crate::settings::default_voices;
    for (lang, voice) in default_voices() {
        if lang != "en" && lang != "ru" {
            continue;
        }
        let _ = crate::voices::download_voice_stream(lang, voice).await?;
    }
    Ok(())
}

pub fn print_setup_status() {
    println!("STT: {}", stt_status());
    println!("Translation: {}", local_translation_status());
    println!("Active whisper: {}", resolve_whisper_model_dir().display());
}
