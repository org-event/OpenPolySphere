//! Translation backend trait — platform-neutral orchestration.

use anyhow::Result;

use super::TranslationDirection;

pub(crate) trait TranslateBackend: Send + Sync {
    fn translate(&self, text: &str, direction: &TranslationDirection) -> Result<String>;
}
