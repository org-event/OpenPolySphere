import { LANG_NAMES } from './state.js';

/** ISO 639-1 → flag emoji (best-effort; some languages share a country flag). */
const LANG_FLAGS = {
  ar: '🇸🇦',
  ca: '🇪🇸',
  cs: '🇨🇿',
  da: '🇩🇰',
  de: '🇩🇪',
  el: '🇬🇷',
  en: '🇬🇧',
  es: '🇪🇸',
  fa: '🇮🇷',
  fi: '🇫🇮',
  fr: '🇫🇷',
  hi: '🇮🇳',
  hu: '🇭🇺',
  id: '🇮🇩',
  it: '🇮🇹',
  ja: '🇯🇵',
  ko: '🇰🇷',
  lv: '🇱🇻',
  nl: '🇳🇱',
  no: '🇳🇴',
  pl: '🇵🇱',
  pt: '🇵🇹',
  ro: '🇷🇴',
  ru: '🇷🇺',
  sv: '🇸🇪',
  tr: '🇹🇷',
  uk: '🇺🇦',
  vi: '🇻🇳',
  zh: '🇨🇳',
};

/** Call / TTS languages — order matches legacy settings UI. */
export const CALL_LANGUAGE_CODES = [
  'ru',
  'en',
  'ar',
  'ca',
  'cs',
  'da',
  'de',
  'el',
  'es',
  'fa',
  'fi',
  'fr',
  'hi',
  'hu',
  'id',
  'it',
  'lv',
  'nl',
  'no',
  'pl',
  'pt',
  'ro',
  'sv',
  'tr',
  'uk',
  'vi',
  'zh',
];

export function callLangLabel(code) {
  const flag = LANG_FLAGS[code] || '🌐';
  const name = LANG_NAMES[code] || code;
  return `${flag} ${name}`;
}

export function populateCallLangSelect(selectId, selected) {
  const sel = document.getElementById(selectId);
  if (!sel) return;
  const current = selected ?? sel.value;
  sel.innerHTML = '';
  for (const code of CALL_LANGUAGE_CODES) {
    const opt = document.createElement('option');
    opt.value = code;
    opt.textContent = callLangLabel(code);
    sel.appendChild(opt);
  }
  if (current) sel.value = current;
}
