use std::path::PathBuf;
use std::process::Command;

fn build_swift_tool(manifest: &PathBuf, out_dir: &PathBuf, tool_name: &str, sources_dir: &str) {
    let tool_dir = manifest.join(sources_dir);
    if !tool_dir.join("Package.swift").is_file() {
        println!("cargo:warning={tool_name} sources not found; skipping");
        return;
    }

    let scratch = out_dir.join(format!("swift-build-{tool_name}"));
    let dest = out_dir.join(tool_name);

    let swift_build = Command::new("swift")
        .args([
            "build",
            "-c",
            "release",
            "--product",
            tool_name,
            "--scratch-path",
            scratch.to_str().unwrap(),
        ])
        .current_dir(&tool_dir)
        .status();

    match swift_build {
        Ok(status) if status.success() => {}
        Ok(status) => {
            println!(
                "cargo:warning=swift build for {tool_name} failed (status={status})"
            );
            return;
        }
        Err(e) => {
            println!("cargo:warning=swift not available ({e}); skipping {tool_name}");
            return;
        }
    }

    let bin_path = Command::new("swift")
        .args([
            "build",
            "-c",
            "release",
            "--product",
            tool_name,
            "--scratch-path",
            scratch.to_str().unwrap(),
            "--show-bin-path",
        ])
        .current_dir(&tool_dir)
        .output();

    let built = match bin_path {
        Ok(output) if output.status.success() => {
            let dir = String::from_utf8_lossy(&output.stdout).trim().to_string();
            PathBuf::from(dir).join(tool_name)
        }
        _ => scratch.join("release").join(tool_name),
    };

    if !built.is_file() {
        println!("cargo:warning={tool_name} binary missing after swift build");
        return;
    }

    if std::fs::copy(&built, &dest).is_err() {
        println!("cargo:warning=failed to copy {tool_name} into OUT_DIR");
        return;
    }

    if let Some(profile_dir) = out_dir.parent().and_then(|p| p.parent()).and_then(|p| p.parent()) {
        let bin_dest = profile_dir.join(tool_name);
        let _ = std::fs::copy(&dest, &bin_dest);
    }

    println!(
        "cargo:rerun-if-changed={}",
        tool_dir.join("Package.swift").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        tool_dir.join("Info.plist").display()
    );
}

fn main() {
    if !cfg!(target_os = "macos") {
        return;
    }

    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());

    build_swift_tool(&manifest, &out_dir, "apple-translate", "../../tools/apple-translate");
    build_speech_app(&manifest, &out_dir);
}

fn build_speech_app(manifest: &PathBuf, out_dir: &PathBuf) {
    let tool_dir = manifest.join("../../tools/apple-speech-auth");
    if !tool_dir.join("Package.swift").is_file() {
        println!("cargo:warning=apple-speech-auth sources not found; skipping");
        return;
    }

    let scratch = out_dir.join("swift-build-LiveTranslateSpeech");
    let product = "LiveTranslateSpeech";

    let swift_build = Command::new("swift")
        .args([
            "build",
            "-c",
            "release",
            "--product",
            product,
            "--scratch-path",
            scratch.to_str().unwrap(),
        ])
        .current_dir(&tool_dir)
        .status();

    match swift_build {
        Ok(status) if status.success() => {}
        Ok(status) => {
            println!("cargo:warning=swift build for {product} failed (status={status})");
            return;
        }
        Err(e) => {
            println!("cargo:warning=swift not available ({e}); skipping {product}");
            return;
        }
    }

    let bin_path = Command::new("swift")
        .args([
            "build",
            "-c",
            "release",
            "--product",
            product,
            "--scratch-path",
            scratch.to_str().unwrap(),
            "--show-bin-path",
        ])
        .current_dir(&tool_dir)
        .output();

    let built = match bin_path {
        Ok(output) if output.status.success() => {
            let dir = String::from_utf8_lossy(&output.stdout).trim().to_string();
            PathBuf::from(dir).join(product)
        }
        _ => scratch.join("release").join(product),
    };

    if !built.is_file() {
        println!("cargo:warning={product} binary missing after swift build");
        return;
    }

    let staging = out_dir.join("LiveTranslator.app");
    let contents = staging.join("Contents");
    let macos = contents.join("MacOS");
    let _ = std::fs::remove_dir_all(&staging);
    if std::fs::create_dir_all(&macos).is_err() {
        println!("cargo:warning=failed to create LiveTranslator.app layout");
        return;
    }
    if std::fs::copy(tool_dir.join("Info.plist"), contents.join("Info.plist")).is_err() {
        println!("cargo:warning=failed to copy AppleSpeechAuth Info.plist");
        return;
    }
    if std::fs::copy(&built, macos.join(product)).is_err() {
        println!("cargo:warning=failed to copy LiveTranslateSpeech binary into .app");
        return;
    }

    if let Some(profile_dir) = out_dir.parent().and_then(|p| p.parent()).and_then(|p| p.parent()) {
        let dest = profile_dir.join("LiveTranslator.app");
        let _ = std::fs::remove_dir_all(&dest);
        let _ = copy_dir_all(&staging, &dest);
    }

    println!("cargo:rerun-if-changed={}", tool_dir.join("Package.swift").display());
    println!("cargo:rerun-if-changed={}", tool_dir.join("Info.plist").display());
    println!(
        "cargo:rerun-if-changed={}",
        tool_dir.join("Sources").display()
    );
}

fn copy_dir_all(src: &PathBuf, dst: &PathBuf) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&from, &to)?;
        } else {
            std::fs::copy(from, to)?;
        }
    }
    Ok(())
}
