fn main() {
    let version = std::env::var("OPENPOLYSPHERE_VERSION")
        .ok()
        .or_else(|| {
            std::env::var("GITHUB_REF_NAME")
                .ok()
                .map(|s| s.trim_start_matches('v').to_string())
        })
        .unwrap_or_else(|| "dev".to_string());
    println!("cargo:rustc-env=OPENPOLYSPHERE_VERSION={version}");
}
