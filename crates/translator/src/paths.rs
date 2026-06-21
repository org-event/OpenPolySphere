//! Project paths relative to repo / install root.

use std::path::{Path, PathBuf};

pub fn base_dir() -> PathBuf {
    std::env::var("CALL_TRANSLATOR_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
}

pub fn settings_path() -> PathBuf {
    base_dir().join("settings.json")
}

pub fn models_dir() -> PathBuf {
    std::env::var("TRANSLATOR_MODELS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| base_dir().join("models"))
}

pub fn db_path() -> PathBuf {
    base_dir().join("calls.db")
}

pub fn web_static_dir() -> PathBuf {
    base_dir().join("web/static")
}

pub fn ensure_parent(path: &Path) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(())
}
