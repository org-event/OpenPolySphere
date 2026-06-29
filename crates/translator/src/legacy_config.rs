//! One-time migration of deprecated config names (remove after v0.5).

use std::fs;
use std::path::Path;

/// Deprecated `.env` key for OpenRouter (renamed to `OPENROUTER_API_KEY`).
const LEGACY_ENV_OPENROUTER: &str = "GROQ_API_KEY";

/// Deprecated `settings.json` field (renamed to `openrouter_api_key`).
pub const LEGACY_SETTINGS_OPENROUTER: &str = "groq_api_key";

pub fn migrate_dotenv_file(path: &Path) {
    let Ok(raw) = fs::read_to_string(path) else {
        return;
    };
    if !raw.contains(LEGACY_ENV_OPENROUTER) {
        return;
    }
    let migrated: String = raw
        .lines()
        .map(|line| {
            if line
                .trim_start()
                .starts_with(&format!("{LEGACY_ENV_OPENROUTER}="))
            {
                line.replacen(LEGACY_ENV_OPENROUTER, "OPENROUTER_API_KEY", 1)
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    let _ = fs::write(path, migrated);
}
