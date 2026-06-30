fn main() {
    let version = match std::env::var("OPENPOLYSPHERE_VERSION")
        .or_else(|_| std::env::var("GITHUB_REF_NAME"))
    {
        Ok(v) => v.trim_start_matches('v').to_string(),
        Err(_) => "dev".into(),
    };
    println!("cargo:rustc-env=OPENPOLYSPHERE_VERSION={version}");
}
