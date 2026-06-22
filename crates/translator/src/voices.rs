//! Piper voice catalog and local scan.

use std::collections::HashMap;
use std::fs;

use anyhow::{Context, Result};
use serde_json::{json, Value};

use crate::paths::models_dir;
use crate::settings::{default_voices, PIPER_VOICES_URL, USER_AGENT};

pub fn scan_local_voices() -> HashMap<String, Vec<String>> {
    let mut voices = HashMap::new();
    let models = models_dir();
    let Ok(entries) = fs::read_dir(&models) else {
        return voices;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if let Some(lang) = name.strip_prefix("piper-") {
            let mut list = Vec::new();
            if let Ok(files) = fs::read_dir(&path) {
                for f in files.flatten() {
                    let p = f.path();
                    if p.extension().and_then(|e| e.to_str()) == Some("onnx") {
                        if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                            list.push(stem.to_string());
                        }
                    }
                }
            }
            list.sort();
            if !list.is_empty() {
                voices.insert(lang.to_string(), list);
            }
        }
    }
    voices
}

pub async fn fetch_catalog() -> HashMap<String, Vec<Value>> {
    let client = reqwest::Client::new();
    let url = format!("{PIPER_VOICES_URL}/voices.json");
    let resp = client
        .get(&url)
        .header("User-Agent", USER_AGENT)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await;
    let Ok(resp) = resp else {
        return HashMap::new();
    };
    let Ok(data) = resp.json::<Value>().await else {
        return HashMap::new();
    };
    let mut catalog: HashMap<String, Vec<Value>> = HashMap::new();
    if let Some(obj) = data.as_object() {
        for (key, info) in obj {
            let family = info["language"]["family"]
                .as_str()
                .unwrap_or("unknown")
                .to_string();
            let files_obj = info.get("files").and_then(|f| f.as_object());
            let mut total_size = 0u64;
            let mut file_list = Vec::new();
            if let Some(files) = files_obj {
                for (fpath, meta) in files {
                    let size = meta.get("size_bytes").and_then(|s| s.as_u64()).unwrap_or(0);
                    total_size += size;
                    file_list.push(json!({
                        "url": format!("{PIPER_VOICES_URL}/{fpath}"),
                        "path": fpath.rsplit('/').next().unwrap_or(fpath),
                        "size": size,
                    }));
                }
            }
            catalog.entry(family).or_default().push(json!({
                "name": key,
                "quality": info.get("quality").and_then(|q| q.as_str()).unwrap_or(""),
                "size": total_size,
                "files": file_list,
            }));
        }
    }
    catalog
}

pub async fn voices_api() -> Value {
    let local = scan_local_voices();
    let catalog = fetch_catalog().await;
    let mut langs: std::collections::BTreeSet<String> = local.keys().cloned().collect();
    langs.extend(catalog.keys().cloned());
    let mut result = serde_json::Map::new();
    for lang in langs {
        let local_set: std::collections::HashSet<_> = local
            .get(&lang)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .collect();
        let cat_voices = catalog.get(&lang).cloned().unwrap_or_default();
        let mut voice_list = Vec::new();
        for v in &cat_voices {
            let name = v["name"].as_str().unwrap_or("");
            voice_list.push(json!({
                "name": name,
                "downloaded": local_set.contains(name),
                "size_mb": (v["size"].as_u64().unwrap_or(0) as f64 / 1_048_576.0).round() / 1.0,
                "quality": v["quality"],
            }));
        }
        let catalog_names: std::collections::HashSet<String> = cat_voices
            .iter()
            .filter_map(|v| v["name"].as_str().map(str::to_string))
            .collect();
        for v in local_set.difference(&catalog_names) {
            voice_list.push(json!({
                "name": v,
                "downloaded": true,
                "size_mb": 0,
                "quality": "",
            }));
        }
        voice_list.sort_by(|a, b| {
            a["name"]
                .as_str()
                .unwrap_or("")
                .cmp(b["name"].as_str().unwrap_or(""))
        });
        result.insert(lang, Value::Array(voice_list));
    }
    Value::Object(result)
}

pub fn default_voice_name(lang: &str) -> String {
    default_voices()
        .get(lang)
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("{lang}_default"))
}

pub async fn download_voice_stream(lang: &str, voice_name: &str) -> Result<String> {
    let catalog = fetch_catalog().await;
    let voices = catalog.get(lang).cloned().unwrap_or_default();
    let voice_name = if voice_name.is_empty() {
        default_voice_name(lang)
    } else {
        voice_name.to_string()
    };
    let voice = voices
        .iter()
        .find(|v| v["name"].as_str() == Some(&voice_name));
    let Some(voice) = voice else {
        return Ok(format!(
            "data: {}\n\n",
            json!({ "error": "Voice not found in catalog" })
        ));
    };
    let target = models_dir().join(format!("piper-{lang}"));
    fs::create_dir_all(&target)?;
    let files = voice["files"].as_array().cloned().unwrap_or_default();
    let all_exist = files
        .iter()
        .all(|f| target.join(f["path"].as_str().unwrap_or("")).exists());
    if all_exist {
        return Ok(format!(
            "data: {}\n\n",
            json!({ "done": true, "voice": voice_name, "cached": true })
        ));
    }
    let total = voice["size"].as_u64().unwrap_or(1).max(1);
    let client = reqwest::Client::new();
    let mut out = String::new();
    let mut downloaded = 0u64;
    for fi in files {
        let dest = target.join(fi["path"].as_str().unwrap_or(""));
        if dest.exists() {
            downloaded += fi["size"].as_u64().unwrap_or(0);
            continue;
        }
        let url = fi["url"].as_str().context("file url")?;
        let mut resp = client
            .get(url)
            .header("User-Agent", USER_AGENT)
            .send()
            .await?;
        let mut file = fs::File::create(&dest)?;
        while let Some(chunk) = resp.chunk().await? {
            use std::io::Write;
            file.write_all(&chunk)?;
            downloaded += chunk.len() as u64;
            let pct = (downloaded * 100 / total) as u32;
            out.push_str(&format!(
                "data: {}\n\n",
                json!({
                    "progress": pct,
                    "mb_done": (downloaded as f64 / 1_048_576.0 * 10.0).round() / 10.0,
                    "mb_total": (total as f64 / 1_048_576.0 * 10.0).round() / 10.0,
                })
            ));
        }
    }
    out.push_str(&format!(
        "data: {}\n\n",
        json!({ "done": true, "voice": voice_name })
    ));
    Ok(out)
}
