//! Settings management — port of web/settings.py.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::paths::{ensure_parent, settings_path};

pub const DEEPGRAM_API_URL: &str = "https://api.deepgram.com/v1/projects";
pub const TRANSLATION_API_URL: &str = "https://openrouter.ai/api/v1/chat/completions";
pub const OPENROUTER_MODELS_URL: &str = "https://openrouter.ai/api/v1/models";
pub const PIPER_VOICES_URL: &str = "https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0";
pub const USER_AGENT: &str = "translator/1.0";
pub const CALL_IDLE_TIMEOUT_SECS: u64 = 300;

pub fn default_voices() -> HashMap<&'static str, &'static str> {
    HashMap::from([
        ("en", "en_US-ryan-medium"),
        ("ru", "ru_RU-denis-medium"),
        ("de", "de_DE-thorsten-medium"),
        ("fr", "fr_FR-siwis-medium"),
        ("es", "es_ES-sharvard-medium"),
        ("it", "it_IT-riccardo-x_low"),
        ("pt", "pt_BR-faber-medium"),
        ("pl", "pl_PL-darkman-medium"),
        ("uk", "uk_UA-ukrainian_tts-medium"),
        ("zh", "zh_CN-huayan-medium"),
        ("ja", "ko_KR-kss-low"),
        ("ko", "ko_KR-kss-low"),
    ])
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(flatten)]
    pub fields: Map<String, Value>,
}

impl Default for Settings {
    fn default() -> Self {
        let mut fields = Map::new();
        fields.insert("deepgram_api_key".into(), Value::String(String::new()));
        fields.insert("openrouter_api_key".into(), Value::String(String::new()));
        fields.insert("translation_backend".into(), Value::String("local".into()));
        fields.insert("translation_polish".into(), Value::Bool(true));
        fields.insert(
            "translation_polish_backend".into(),
            Value::String("local".into()),
        );
        fields.insert("stt_backend".into(), Value::String("local".into()));
        fields.insert("deepgram_model".into(), Value::String("nova-3".into()));
        fields.insert("whisper_model".into(), Value::String("auto".into()));
        fields.insert("stt_device".into(), Value::String(String::new()));
        fields.insert("translation_model".into(), Value::String(String::new()));
        fields.insert("tts_outgoing_voice".into(), Value::String(String::new()));
        fields.insert("tts_incoming_voice".into(), Value::String(String::new()));
        fields.insert("mic_device".into(), Value::String("default".into()));
        fields.insert("speaker_device".into(), Value::String("default".into()));
        fields.insert(
            "meet_input_device".into(),
            Value::String("BlackHole 16ch".into()),
        );
        fields.insert(
            "meet_output_device".into(),
            Value::String("BlackHole 2ch".into()),
        );
        fields.insert("endpointing_ms".into(), Value::from(500));
        fields.insert("my_language".into(), Value::String("ru".into()));
        fields.insert("their_language".into(), Value::String("en".into()));
        Self { fields }
    }
}

impl Settings {
    pub fn load() -> Result<Self> {
        let path = settings_path();
        let mut settings = if path.exists() {
            let raw = fs::read_to_string(&path).context("read settings.json")?;
            let saved: Map<String, Value> = serde_json::from_str(&raw)?;
            let mut defaults = Settings::default().fields;
            for (k, v) in saved {
                defaults.insert(k, v);
            }
            Settings { fields: defaults }
        } else {
            Settings::default()
        };
        settings.migrate_groq_key();
        settings.apply_env_keys();
        Ok(settings)
    }

    pub fn save(&self) -> Result<()> {
        let path = settings_path();
        ensure_parent(&path)?;
        let mut to_save = self.fields.clone();
        to_save.remove("groq_api_key");
        if env_deepgram_key().is_some()
            && self.str_field("deepgram_api_key") == env_deepgram_key().unwrap_or_default()
        {
            to_save.insert("deepgram_api_key".into(), Value::String(String::new()));
        }
        if env_openrouter_key().is_some()
            && self.str_field("openrouter_api_key") == env_openrouter_key().unwrap_or_default()
        {
            to_save.insert("openrouter_api_key".into(), Value::String(String::new()));
        }
        fs::write(&path, serde_json::to_string_pretty(&to_save)?)?;
        Ok(())
    }

    pub fn merge(&mut self, patch: Map<String, Value>) {
        for (k, v) in patch {
            self.fields.insert(k, v);
        }
        self.apply_env_keys();
    }

    fn migrate_groq_key(&mut self) {
        if let Some(legacy) = self.fields.remove("groq_api_key") {
            if self.str_field("openrouter_api_key").is_empty() {
                if let Value::String(s) = legacy {
                    self.fields
                        .insert("openrouter_api_key".into(), Value::String(s));
                }
            }
        }
    }

    fn apply_env_keys(&mut self) {
        if let Some(k) = env_deepgram_key() {
            self.fields
                .insert("deepgram_api_key".into(), Value::String(k));
        } else if is_placeholder(&self.str_field("deepgram_api_key")) {
            self.fields
                .insert("deepgram_api_key".into(), Value::String(String::new()));
        }
        if let Some(k) = env_openrouter_key() {
            self.fields
                .insert("openrouter_api_key".into(), Value::String(k));
        } else if is_placeholder(&self.str_field("openrouter_api_key")) {
            self.fields
                .insert("openrouter_api_key".into(), Value::String(String::new()));
        }
    }

    pub fn str_field(&self, key: &str) -> String {
        self.fields
            .get(key)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    }

    pub fn u32_field(&self, key: &str, default: u32) -> u32 {
        self.fields
            .get(key)
            .and_then(|v| v.as_u64())
            .map(|n| n as u32)
            .unwrap_or(default)
    }

    pub fn stt_backend(&self) -> String {
        let v = self.str_field("stt_backend");
        if v.is_empty() {
            "local".into()
        } else {
            v.to_lowercase()
        }
    }

    pub fn deepgram_model(&self) -> String {
        let v = self.str_field("deepgram_model");
        if v.is_empty() {
            "nova-3".into()
        } else {
            v.to_lowercase()
        }
    }

    pub fn translation_backend(&self) -> String {
        let v = self.str_field("translation_backend");
        if v.is_empty() {
            "local".into()
        } else {
            v.to_lowercase()
        }
    }

    pub fn whisper_model(&self) -> String {
        let v = self.str_field("whisper_model");
        if v.is_empty() {
            "auto".into()
        } else {
            v.to_lowercase()
        }
    }

    pub fn stt_device(&self) -> String {
        let v = self.str_field("stt_device");
        if v.is_empty() {
            audio_core::stt::local::default_stt_device_name()
        } else {
            v.to_lowercase()
        }
    }

    pub fn translation_polish(&self) -> bool {
        self.fields
            .get("translation_polish")
            .and_then(|v| v.as_bool())
            .unwrap_or(true)
    }

    pub fn translation_polish_backend(&self) -> String {
        let v = self.str_field("translation_polish_backend");
        if v.is_empty() {
            "local".into()
        } else {
            v.to_lowercase()
        }
    }

    pub fn translation_model(&self) -> String {
        let saved = self.str_field("translation_model");
        if !saved.is_empty() {
            return saved;
        }
        "liquid/lfm-2.5-1.2b-instruct:free".into()
    }

    pub fn openrouter_key(&self) -> String {
        let k = self.str_field("openrouter_api_key");
        if !k.is_empty() {
            return k;
        }
        env_openrouter_key().unwrap_or_default()
    }

    pub fn default_voice(&self, lang: &str) -> String {
        let voices = default_voices();
        voices
            .get(lang)
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("{lang}_default"))
    }

    pub fn outgoing_voice(&self) -> String {
        let v = self.str_field("tts_outgoing_voice");
        if v.is_empty() {
            self.default_voice(&self.str_field("their_language"))
        } else {
            v
        }
    }

    pub fn incoming_voice(&self) -> String {
        let v = self.str_field("tts_incoming_voice");
        if v.is_empty() {
            self.default_voice(&self.str_field("my_language"))
        } else {
            v
        }
    }
}

pub fn env_deepgram_key() -> Option<String> {
    std::env::var("DEEPGRAM_API_KEY")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !is_placeholder(s))
}

pub fn env_openrouter_key() -> Option<String> {
    ["OPENROUTER_API_KEY", "GROQ_API_KEY", "TRANSLATION_API_KEY"]
        .iter()
        .find_map(|var| {
            std::env::var(var)
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !is_placeholder(s))
        })
}

fn is_placeholder(key: &str) -> bool {
    matches!(
        key.trim(),
        "" | "your_deepgram_api_key_here"
            | "your_groq_api_key_here"
            | "your_openrouter_api_key_here"
    )
}

/// Push settings into process env for audio-core backends that read env vars.
pub fn apply_env(settings: &Settings, models_base: &Path) {
    std::env::set_var("TRANSLATOR_MODELS_DIR", models_base);
    std::env::set_var("DEEPGRAM_API_KEY", settings.str_field("deepgram_api_key"));
    std::env::set_var("STT_BACKEND", settings.stt_backend());
    std::env::set_var("TRANSLATOR_DEEPGRAM_MODEL", settings.deepgram_model());
    std::env::set_var("TRANSLATOR_WHISPER_MODEL", settings.whisper_model());
    std::env::set_var("TRANSLATOR_STT_DEVICE", settings.stt_device());
    std::env::set_var("TRANSLATION_BACKEND", settings.translation_backend());
    std::env::set_var(
        "TRANSLATION_POLISH",
        if settings.translation_polish() {
            "1"
        } else {
            "0"
        },
    );
    std::env::set_var(
        "TRANSLATION_POLISH_BACKEND",
        settings.translation_polish_backend(),
    );
    std::env::set_var("OPENROUTER_API_KEY", settings.openrouter_key());
    std::env::set_var("TRANSLATION_MODEL", settings.translation_model());
    std::env::set_var(
        "TRANSLATION_API_URL",
        std::env::var("TRANSLATION_API_URL").unwrap_or_else(|_| TRANSLATION_API_URL.into()),
    );
    std::env::set_var("TRANSLATOR_MIC_DEVICE", settings.str_field("mic_device"));
    std::env::set_var(
        "TRANSLATOR_SPEAKER_DEVICE",
        settings.str_field("speaker_device"),
    );
    std::env::set_var(
        "TRANSLATOR_MEET_INPUT",
        settings.str_field("meet_input_device"),
    );
    std::env::set_var(
        "TRANSLATOR_MEET_OUTPUT",
        settings.str_field("meet_output_device"),
    );
    std::env::set_var(
        "TRANSLATOR_ENDPOINTING_MS",
        settings.u32_field("endpointing_ms", 500).to_string(),
    );
    std::env::set_var("TRANSLATOR_MY_LANG", settings.str_field("my_language"));
    std::env::set_var(
        "TRANSLATOR_THEIR_LANG",
        settings.str_field("their_language"),
    );

    let my = settings.str_field("my_language");
    let their = settings.str_field("their_language");
    let out_v = settings.outgoing_voice();
    let in_v = settings.incoming_voice();
    let base = models_base.to_string_lossy();
    std::env::set_var(
        "TRANSLATOR_TTS_EN_MODEL",
        format!("{base}/piper-{their}/{out_v}.onnx"),
    );
    std::env::set_var(
        "TRANSLATOR_TTS_EN_CONFIG",
        format!("{base}/piper-{their}/{out_v}.onnx.json"),
    );
    std::env::set_var(
        "TRANSLATOR_TTS_RU_MODEL",
        format!("{base}/piper-{my}/{in_v}.onnx"),
    );
    std::env::set_var(
        "TRANSLATOR_TTS_RU_CONFIG",
        format!("{base}/piper-{my}/{in_v}.onnx.json"),
    );
}

pub fn engine_config(settings: &Settings, models_base: &Path) -> audio_core::engine::EngineConfig {
    audio_core::engine::EngineConfig::from_settings(audio_core::engine::EngineSettingsParams {
        models_base: &models_base.to_string_lossy(),
        deepgram_api_key: &settings.str_field("deepgram_api_key"),
        my_language: &settings.str_field("my_language"),
        their_language: &settings.str_field("their_language"),
        mic_device: &settings.str_field("mic_device"),
        speaker_device: &settings.str_field("speaker_device"),
        meet_input: &settings.str_field("meet_input_device"),
        meet_out: &settings.str_field("meet_output_device"),
        endpointing_ms: settings.u32_field("endpointing_ms", 500),
        out_voice: &settings.outgoing_voice(),
        in_voice: &settings.incoming_voice(),
    })
}

pub fn local_translation_status() -> serde_json::Value {
    let models = crate::paths::models_dir();
    let translate = models.join("translate");
    let pairs = [("opus-mt-ru-en", "ru", "en"), ("opus-mt-en-ru", "en", "ru")];
    let mut map = serde_json::Map::new();
    let mut all = true;
    for (name, _, _) in pairs {
        let ok = translate.join(name).join("model.bin").is_file();
        map.insert(name.into(), serde_json::Value::Bool(ok));
        all &= ok;
    }
    let backend = Settings::load()
        .map(|s| s.translation_backend())
        .unwrap_or_else(|_| "local".into());
    let polish_enabled = Settings::load()
        .map(|s| s.translation_polish())
        .unwrap_or(true);
    let polish_backend = Settings::load()
        .map(|s| s.translation_polish_backend())
        .unwrap_or_else(|_| "local".into());
    let (polish_model, polish_ready) = audio_core::translation::polish_model_status();
    let polish_disabled = if !polish_enabled {
        ""
    } else if backend == "apple" {
        "Not used with Apple Translation"
    } else if audio_core::translation::is_session_disabled() {
        "Polish unavailable for this session"
    } else if polish_backend == "local" && !polish_ready {
        "Download polish model (~400 MB)"
    } else {
        ""
    };
    let apple = {
        let settings = Settings::load().unwrap_or_default();
        audio_core::translation::apple_translation_availability(
            &settings.str_field("my_language"),
            &settings.str_field("their_language"),
        )
    };
    let ready = match backend.as_str() {
        "openrouter" | "cloud" | "llm" => false,
        "apple" | "system" | "macos" => apple
            .get("ready")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        _ => all,
    };
    serde_json::json!({
        "backend": backend,
        "pairs": map,
        "ready": ready,
        "apple": apple,
        "polish_enabled": polish_enabled,
        "polish_backend": polish_backend,
        "polish_model": polish_model,
        "polish_ready": polish_ready,
        "polish_active": polish_enabled
            && polish_ready
            && !audio_core::translation::is_session_disabled(),
        "polish_disabled_reason": polish_disabled,
    })
}

pub fn stt_status() -> serde_json::Value {
    let settings = Settings::load().unwrap_or_default();
    let device = settings.stt_device();
    let selected = settings.whisper_model();
    let backend = settings.stt_backend();
    let deepgram_model = settings.deepgram_model();
    let my_lang = settings.str_field("my_language");
    let apple = audio_core::stt::apple::apple_speech_availability(&my_lang);
    let (model, _) = audio_core::stt::local::whisper_model_status();
    let installed = audio_core::stt::local::list_all_installed_whisper_variants();
    let selected_ready = audio_core::stt::local::is_variant_ready_for(&selected, &device);
    let device_ready =
        !audio_core::stt::local::list_installed_whisper_variants_for(&device).is_empty();
    let ready = match backend.as_str() {
        "deepgram" | "cloud" => {
            !settings.str_field("deepgram_api_key").trim().is_empty()
                || env_deepgram_key().is_some()
        }
        "apple" | "system" | "macos" => apple
            .get("ready")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        _ => device_ready && selected_ready,
    };
    serde_json::json!({
        "backend": backend,
        "device": device,
        "model": if backend == "deepgram" {
            format!("deepgram-{deepgram_model}")
        } else if backend == "apple" || backend == "system" || backend == "macos" {
            format!("apple-speech-{my_lang}")
        } else {
            model
        },
        "deepgram_model": deepgram_model,
        "selected": selected,
        "installed": installed,
        "ready": ready,
        "metal_available": audio_core::platform::Capabilities::current().whisper_metal,
        "apple": apple,
    })
}
