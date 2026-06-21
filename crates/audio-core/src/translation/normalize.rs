//! Fix common local-Whisper mis-hearings before machine translation.

use super::TranslationDirection;

/// Light-touch cleanup on STT text (no LLM). Applied before Opus-MT.
pub fn normalize_stt_text(text: &str, direction: &TranslationDirection) -> String {
    let mut s = text.to_string();
    if direction.from_code == "ru" {
        apply_replacements(&mut s, RU_FIXES);
    } else if direction.from_code == "en" {
        apply_replacements(&mut s, EN_FIXES);
    }
    s
}

fn apply_replacements(s: &mut String, pairs: &[(&str, &str)]) {
    for (from, to) in pairs {
        if s.contains(from) {
            *s = s.replace(from, to);
        }
        let upper = capitalize_first(from);
        if s.contains(&upper) {
            *s = s.replace(&upper, &capitalize_first(to));
        }
    }
}

fn capitalize_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

const RU_FIXES: &[(&str, &str)] = &[
    ("инженерыговорят", "инженеры говорят"),
    ("чтовы", "что вы"),
    ("иясност", "ясност"),
    ("отценива", "оценива"),
    ("обратно связь", "обратная связь"),
    ("фрумулир", "формулир"),
    ("обстра", "абстра"),
    ("хитектур", "архитектур"),
    ("нoriальн", "реальн"),
    ("ревиор", "ревью"),
    ("ревior", "ревью"),
    ("код ревиор", "код-ревью"),
    ("к отревиую", "code review"),
    ("Revue", "ревью"),
    ("revue", "ревью"),
    ("reverior", "ревью"),
    ("Reverior", "ревью"),
    ("Policinous", "pull request"),
    ("policinous", "pull request"),
    ("полриквестной", "pull request"),
    ("полриквестный", "pull request"),
    ("полриквест", "pull request"),
    ("команди", "команде"),
    ("второ ", "автор "),
    ("кот без", "код без"),
    ("обсуждать кот", "обсуждать код"),
    ("полриквеста", "pull request"),
];

const EN_FIXES: &[(&str, &str)] = &[
    ("code reverior", "code review"),
    ("code revior", "code review"),
    ("pull reques", "pull request"),
    ("revue ", "review "),
];
