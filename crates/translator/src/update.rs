//! GitHub release check and platform self-update (macOS .app).

use std::path::PathBuf;
use std::process::Command;

use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::paths::{is_packaged_macos_app, user_data_dir};
use crate::settings::Settings;

const REPO: &str = "org-event/OpenPolySphere";
const USER_AGENT: &str = concat!("openpolysphere/", env!("OPENPOLYSPHERE_VERSION"));

pub fn app_version() -> &'static str {
    env!("OPENPOLYSPHERE_VERSION")
}

pub fn app_info() -> Value {
    json!({
        "version": app_version(),
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "packaged": is_packaged_macos_app(),
    })
}

pub fn check_updates_enabled(settings: &Settings) -> bool {
    settings
        .fields
        .get("check_updates")
        .and_then(|v| v.as_bool())
        .unwrap_or(true)
}

fn parse_version_parts(v: &str) -> Vec<u32> {
    v.trim()
        .trim_start_matches('v')
        .split('.')
        .filter_map(|p| p.parse().ok())
        .collect()
}

pub fn is_newer_version(latest: &str, current: &str) -> bool {
    let a = parse_version_parts(latest);
    let b = parse_version_parts(current);
    for i in 0..a.len().max(b.len()) {
        let av = a.get(i).copied().unwrap_or(0);
        let bv = b.get(i).copied().unwrap_or(0);
        if av > bv {
            return true;
        }
        if av < bv {
            return false;
        }
    }
    false
}

#[derive(Debug, Deserialize)]
struct GhRelease {
    tag_name: String,
    html_url: String,
    body: Option<String>,
    assets: Vec<GhAsset>,
}

#[derive(Debug, Deserialize)]
struct GhAsset {
    name: String,
    browser_download_url: String,
}

pub async fn check_for_update(settings: &Settings) -> Result<Value> {
    let current = app_version();
    if !check_updates_enabled(settings) {
        return Ok(json!({
            "status": "disabled",
            "current": current,
            "update_available": false,
        }));
    }

    let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
    let client = reqwest::Client::builder().user_agent(USER_AGENT).build()?;
    let release: GhRelease = client
        .get(&url)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let latest = release.tag_name.trim_start_matches('v').to_string();
    let update_available = is_newer_version(&latest, current);
    let download_url = if update_available {
        pick_asset_url(&release.assets).unwrap_or_default()
    } else {
        String::new()
    };

    Ok(json!({
        "status": "ok",
        "current": current,
        "latest": latest,
        "tag": release.tag_name,
        "update_available": update_available,
        "download_url": download_url,
        "release_url": release.html_url,
        "notes": release.body.unwrap_or_default(),
    }))
}

fn pick_asset_url(assets: &[GhAsset]) -> Option<String> {
    let arch = match std::env::consts::ARCH {
        "aarch64" => "arm64",
        "x86_64" => "x64",
        other => other,
    };
    let patterns: Vec<String> = match std::env::consts::OS {
        "macos" => vec![format!("-macos-{arch}.zip"), format!("-macos-{arch}.dmg")],
        "windows" => vec!["-windows-x64.zip".into(), "-windows-x64-setup.exe".into()],
        "linux" => vec!["-linux-x64.zip".into(), "_amd64.deb".into()],
        _ => vec![],
    };
    for pat in &patterns {
        if let Some(a) = assets.iter().find(|a| a.name.contains(pat)) {
            return Some(a.browser_download_url.clone());
        }
    }
    assets.first().map(|a| a.browser_download_url.clone())
}

pub async fn apply_update(download_url: &str) -> Result<Value> {
    if download_url.is_empty() {
        anyhow::bail!("empty download URL");
    }
    match std::env::consts::OS {
        "macos" => apply_update_macos(download_url).await,
        "windows" => Ok(json!({
            "status": "manual",
            "message": "Download the installer from GitHub Releases and run it.",
            "url": download_url,
        })),
        "linux" => Ok(json!({
            "status": "manual",
            "message": "Download the .deb or .zip from GitHub Releases.",
            "url": download_url,
        })),
        _ => Ok(json!({
            "status": "unsupported",
            "message": "Self-update is not supported on this platform.",
        })),
    }
}

async fn apply_update_macos(download_url: &str) -> Result<Value> {
    let app_path = detect_macos_app_path().context("Could not locate OpenPolySphere.app")?;
    let work = user_data_dir().join("update");
    std::fs::create_dir_all(&work)?;
    let archive = work.join("update.zip");
    let client = reqwest::Client::builder().user_agent(USER_AGENT).build()?;
    let bytes = client
        .get(download_url)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;
    std::fs::write(&archive, &bytes)?;

    let script = work.join("apply-update.sh");
    std::fs::write(
        &script,
        format!(
            r#"#!/bin/bash
set -euo pipefail
sleep 2
pkill -f "{app_path}/Contents/MacOS/OpenPolySphere" 2>/dev/null || true
pkill -f "{app_path}/Contents/MacOS/translator" 2>/dev/null || true
rm -rf "{work}/unpack"
mkdir -p "{work}/unpack"
ditto -x -k "{archive}" "{work}/unpack" 2>/dev/null || unzip -oq "{archive}" -d "{work}/unpack"
NEW_APP=$(find "{work}/unpack" -name 'OpenPolySphere.app' -maxdepth 3 | head -1)
if [[ -z "$NEW_APP" ]]; then echo "OpenPolySphere.app not found in archive"; exit 1; fi
rsync -a "$NEW_APP/" "{app_path}/"
open "{app_path}"
"#,
            app_path = app_path.display(),
            work = work.display(),
            archive = archive.display(),
        ),
    )?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755))?;
    }
    Command::new("/bin/bash")
        .arg(&script)
        .spawn()
        .context("failed to spawn updater script")?;

    Ok(json!({
        "status": "ok",
        "message": "Update downloaded. The app will restart in a few seconds.",
        "relaunch": true,
    }))
}

fn detect_macos_app_path() -> Option<PathBuf> {
    if let Ok(exe) = std::env::current_exe() {
        let mut path = exe;
        while path.pop() {
            if path.extension().and_then(|e| e.to_str()) == Some("app") {
                return Some(path);
            }
        }
    }
    let default = PathBuf::from("/Applications/OpenPolySphere.app");
    if default.is_dir() {
        return Some(default);
    }
    None
}
