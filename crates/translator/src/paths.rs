//! Project paths relative to repo / install root / .app bundle.

use std::path::{Path, PathBuf};

const APP_SUPPORT_NAME: &str = "OpenPolySphere";

/// Read-only bundle root (web assets, shipped binaries).
pub fn bundle_dir() -> PathBuf {
    if let Ok(home) = std::env::var("CALL_TRANSLATOR_HOME") {
        return PathBuf::from(home);
    }
    if let Some(resources) = macos_app_resources_dir() {
        return resources;
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// Writable user data (settings, models, db). In .app builds: Application Support.
pub fn user_data_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("TRANSLATOR_DATA_DIR") {
        return PathBuf::from(dir);
    }
    if macos_app_resources_dir().is_some() {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home)
                .join("Library")
                .join("Application Support")
                .join(APP_SUPPORT_NAME);
        }
    }
    bundle_dir()
}

/// Alias for bundle root (logs, static assets).
pub fn base_dir() -> PathBuf {
    bundle_dir()
}

pub fn settings_path() -> PathBuf {
    user_data_dir().join("settings.json")
}

pub fn models_dir() -> PathBuf {
    std::env::var("TRANSLATOR_MODELS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| user_data_dir().join("models"))
}

pub fn db_path() -> PathBuf {
    user_data_dir().join("calls.db")
}

pub fn web_static_dir() -> PathBuf {
    bundle_dir().join("web/static")
}

pub fn ensure_parent(path: &Path) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(())
}

pub fn is_packaged_macos_app() -> bool {
    macos_app_resources_dir().is_some()
}

fn macos_app_resources_dir() -> Option<PathBuf> {
    #[cfg(not(target_os = "macos"))]
    {
        None
    }
    #[cfg(target_os = "macos")]
    {
        let exe = std::env::current_exe().ok()?;
        macos_resources_from_exe(&exe)
    }
}

#[cfg(target_os = "macos")]
fn macos_resources_from_exe(exe: &Path) -> Option<PathBuf> {
    let mut path = exe.to_path_buf();
    while path.pop() {
        if path.extension().and_then(|e| e.to_str()) != Some("app") {
            continue;
        }
        let resources = path.join("Contents").join("Resources");
        if resources.is_dir() {
            return Some(resources);
        }
    }
    None
}
