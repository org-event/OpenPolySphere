//! Packaged-install environment (mirrors paths in translator crate).

use std::path::PathBuf;

const APP_NAME: &str = "OpenPolySphere";

/// True when running from a shipped install (.app, FHS, Program Files), not `cargo run`.
pub fn is_packaged() -> bool {
    macos_resources_dir().is_some() || packaged_user_data_dir().is_some()
}

pub fn apply_packaged_env() {
    if !is_packaged() {
        apply_dev_env();
        return;
    }
    if let Some(home) = bundle_home() {
        std::env::set_var("CALL_TRANSLATOR_HOME", &home);
    }
    let data = user_data_dir();
    let _ = std::fs::create_dir_all(&data);
    std::env::set_var("TRANSLATOR_DATA_DIR", &data);

    if let Some(ort) = ort_dylib_path() {
        std::env::set_var("ORT_DYLIB_PATH", &ort);
    }
    if let Some(speech) = speech_helper_app() {
        std::env::set_var("POLYSPHERE_SPEECH_AUTH_APP", &speech);
    }
}

fn apply_dev_env() {
    if std::env::var("OPENPOLYSPHERE_TRANSLATOR").is_err() {
        if let Some(exe) = dev_translator_exe() {
            std::env::set_var("OPENPOLYSPHERE_TRANSLATOR", &exe);
        }
    }
    if std::env::var("CALL_TRANSLATOR_HOME").is_err() {
        if let Some(root) = workspace_root() {
            std::env::set_var("CALL_TRANSLATOR_HOME", &root);
        }
    }
    if std::env::var("TRANSLATOR_DATA_DIR").is_err() {
        if let Some(root) = workspace_root() {
            std::env::set_var("TRANSLATOR_DATA_DIR", &root);
        }
    }
}

fn workspace_root() -> Option<PathBuf> {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .ok()
}

fn dev_translator_exe() -> Option<PathBuf> {
    let root = workspace_root()?;
    for profile in ["release", "debug"] {
        let name = if cfg!(windows) {
            "translator.exe"
        } else {
            "translator"
        };
        let candidate = root.join("target").join(profile).join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn bundle_home() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("CALL_TRANSLATOR_HOME") {
        return Some(PathBuf::from(dir));
    }
    macos_resources_dir().or_else(|| {
        std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(|p| p.to_path_buf()))
    })
}

pub fn user_data_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("TRANSLATOR_DATA_DIR") {
        return PathBuf::from(dir);
    }
    if let Some(dir) = packaged_user_data_dir() {
        return dir;
    }
    #[cfg(target_os = "macos")]
    if macos_resources_dir().is_some() {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home)
                .join("Library")
                .join("Application Support")
                .join(APP_NAME);
        }
    }
    bundle_home().unwrap_or_else(|| PathBuf::from("."))
}

fn packaged_user_data_dir() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let path = exe.to_string_lossy();

    #[cfg(target_os = "linux")]
    {
        if path.contains("/usr/lib/openpolysphere/")
            || path.ends_with("/openpolysphere")
            || path.ends_with("/bin/openpolysphere")
        {
            return linux_xdg_data_dir();
        }
    }

    #[cfg(target_os = "windows")]
    {
        let lower = path.to_lowercase();
        if lower.contains("\\program files\\") || lower.contains("\\program files (x86)\\") {
            return std::env::var("LOCALAPPDATA")
                .ok()
                .map(|p| PathBuf::from(p).join(APP_NAME));
        }
    }

    let _ = path;
    None
}

#[cfg(target_os = "linux")]
fn linux_xdg_data_dir() -> Option<PathBuf> {
    if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
        if !xdg.is_empty() {
            return Some(PathBuf::from(xdg).join(APP_NAME));
        }
    }
    std::env::var("HOME")
        .ok()
        .map(|h| PathBuf::from(h).join(".local").join("share").join(APP_NAME))
}

fn macos_resources_dir() -> Option<PathBuf> {
    #[cfg(not(target_os = "macos"))]
    {
        None
    }
    #[cfg(target_os = "macos")]
    {
        let exe = std::env::current_exe().ok()?;
        let mut path = exe;
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
}

fn ort_dylib_path() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("ORT_DYLIB_PATH") {
        if !p.is_empty() {
            return Some(PathBuf::from(p));
        }
    }
    if let Some(res) = macos_resources_dir() {
        let p = res
            .parent()
            .map(|c| c.join("Frameworks").join("libonnxruntime.dylib"));
        if p.as_ref().is_some_and(|x| x.is_file()) {
            return p;
        }
    }
    let exe = std::env::current_exe().ok()?;
    let sibling = exe.parent()?.join(if cfg!(windows) {
        "onnxruntime.dll"
    } else if cfg!(target_os = "macos") {
        "libonnxruntime.dylib"
    } else {
        "libonnxruntime.so"
    });
    if sibling.is_file() {
        return Some(sibling);
    }
    #[cfg(target_os = "linux")]
    {
        let fhs = PathBuf::from("/usr/lib/openpolysphere/lib/libonnxruntime.so");
        if fhs.is_file() {
            return Some(fhs);
        }
    }
    None
}

fn speech_helper_app() -> Option<PathBuf> {
    if let Some(res) = macos_resources_dir() {
        let p = res.join("Helpers").join("PolySphereSpeech.app");
        if p.is_dir() {
            return Some(p);
        }
    }
    None
}

pub fn translator_exe() -> PathBuf {
    if let Ok(p) = std::env::var("OPENPOLYSPHERE_TRANSLATOR") {
        return PathBuf::from(p);
    }
    if let Some(res) = macos_resources_dir() {
        let p = res.join("translator");
        if p.is_file() {
            return p;
        }
    }
    #[cfg(target_os = "linux")]
    {
        let fhs = PathBuf::from("/usr/lib/openpolysphere/bin/translator");
        if fhs.is_file() {
            return fhs;
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let name = if cfg!(windows) {
                "translator.exe"
            } else {
                "translator"
            };
            let sibling = dir.join(name);
            if sibling.is_file() {
                return sibling;
            }
        }
    }
    PathBuf::from(if cfg!(windows) {
        "translator.exe"
    } else {
        "translator"
    })
}
