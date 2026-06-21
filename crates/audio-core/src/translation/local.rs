//! Local translation via CTranslate2 (Helsinki Opus-MT).

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use log::info;

use super::model::TranslationModel;
use super::TranslationDirection;

pub struct LocalEngine {
    models: ModelPair,
}

struct ModelPair {
    ru_en: Option<TranslationModel>,
    en_ru: Option<TranslationModel>,
    base_dir: PathBuf,
}

impl LocalEngine {
    pub fn new() -> Result<Self> {
        let base_dir = translate_models_dir();
        info!("Loading local translation models from {}", base_dir.display());

        let ru_en_path = base_dir.join("opus-mt-ru-en");
        let en_ru_path = base_dir.join("opus-mt-en-ru");

        let ru_en = load_if_present(&ru_en_path)?;
        let en_ru = load_if_present(&en_ru_path)?;

        if ru_en.is_none() && en_ru.is_none() {
            bail!(
                "No local translation models found in {}. \
                 Run: cargo run --release -p translator -- setup",
                base_dir.display()
            );
        }

        if let Some(_) = &ru_en {
            info!("Loaded opus-mt-ru-en");
        }
        if let Some(_) = &en_ru {
            info!("Loaded opus-mt-en-ru");
        }

        Ok(Self {
            models: ModelPair {
                ru_en,
                en_ru,
                base_dir,
            },
        })
    }

    pub fn translate(&self, text: &str, direction: &TranslationDirection) -> Result<String> {
        let model = self.models.get(direction)?;
        model.translate(text)
    }
}

impl ModelPair {
    fn get(&self, direction: &TranslationDirection) -> Result<&TranslationModel> {
        match (direction.from_code.as_str(), direction.to_code.as_str()) {
            ("ru", "en") => self
                .ru_en
                .as_ref()
                .with_context(|| format_missing("ru", "en", &self.base_dir)),
            ("en", "ru") => self
                .en_ru
                .as_ref()
                .with_context(|| format_missing("en", "ru", &self.base_dir)),
            (from, to) => bail!(
                "Local Opus-MT supports ru↔en only (got {from}→{to}). \
                 Set TRANSLATION_BACKEND=openrouter for other language pairs."
            ),
        }
    }
}

fn format_missing(from: &str, to: &str, base: &Path) -> String {
    format!(
        "Model opus-mt-{from}-{to} not found in {}. \
         Run: cargo run --release -p translator -- setup",
        base.display()
    )
}

fn load_if_present(path: &Path) -> Result<Option<TranslationModel>> {
    if path.join("model.bin").exists() {
        Ok(Some(TranslationModel::new(path)?))
    } else {
        Ok(None)
    }
}

pub fn translate_models_dir() -> PathBuf {
    let base = std::env::var("TRANSLATOR_MODELS_DIR").unwrap_or_else(|_| "./models".into());
    PathBuf::from(base).join("translate")
}
