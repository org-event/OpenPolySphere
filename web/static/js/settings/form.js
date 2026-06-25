import { state } from '../core/state.js';
import { initI18n, switchLocale, t } from '../core/i18n.js';
import { UI_LOCALES } from '../core/ui-locales.js';
import {
  sttBackendValue,
  updateSttEngineUI,
  refreshSttStatus,
  whisperModelValue,
  deepgramModelValue,
} from './stt.js';
import { loadVoices } from './voices.js';
import {
  translationBackendValue,
  updateTranslationEngineUI,
  refreshTranslationStatus,
  loadTranslationModels,
  updateTranslationModelMeta,
} from './translation.js';
import { loadDevices, applyPlatformAudioHints } from './devices.js';
import { populateCallLangSelect } from '../core/call-languages.js';
import { applyEngineButton } from '../engine/control.js';

function refreshSettingsButtonLabels() {
  for (const id of ['test-deepgram', 'test-openrouter', 'btn-test-translate', 'btn-test-translate-apple']) {
    const btn = document.getElementById(id);
    if (
      btn &&
      !btn.classList.contains('testing') &&
      !btn.classList.contains('ok') &&
      !btn.classList.contains('fail')
    ) {
      btn.textContent = t('settings.test');
    }
  }
  const dlTr = document.getElementById('btn-download-translate');
  if (dlTr && !dlTr.classList.contains('loading')) {
    dlTr.textContent = t('settings.downloadOpusMt');
  }
  const dlPl = document.getElementById('btn-download-polish');
  if (dlPl && !dlPl.classList.contains('loading')) {
    dlPl.textContent = t('settings.downloadPolish');
  }
}

export function populateForm(s) {
  const dg = document.getElementById('cfg-deepgram');
  const or = document.getElementById('cfg-openrouter');
  if (dg._setRealValue) dg._setRealValue(s.deepgram_api_key || '');
  else dg.value = s.deepgram_api_key || '';
  if (or._setRealValue) or._setRealValue(s.openrouter_api_key || '');
  else or.value = s.openrouter_api_key || '';
  if (!s.deepgram_api_key && s._deepgram_from_env) dg.placeholder = 'Set via .env file';
  if (!s.openrouter_api_key && s._openrouter_from_env) or.placeholder = 'Set via OPENROUTER_API_KEY in .env';

  const backendSel = document.getElementById('cfg-translation-backend');
  if (backendSel) {
    const backend = s.translation_backend || 'local';
    backendSel.value =
      backend === 'openrouter' || backend === 'cloud' || backend === 'llm'
        ? 'openrouter'
        : backend === 'apple' || backend === 'system' || backend === 'macos'
          ? 'apple'
          : 'local';
  }
  const sttBackendSel = document.getElementById('cfg-stt-backend');
  if (sttBackendSel) {
    const sttBackend = s.stt_backend || 'local';
    sttBackendSel.value =
      sttBackend === 'deepgram' || sttBackend === 'cloud'
        ? 'deepgram'
        : sttBackend === 'apple' || sttBackend === 'system' || sttBackend === 'macos'
          ? 'apple'
          : 'local';
  }
  const whisperSel = document.getElementById('cfg-whisper-model');
  if (whisperSel) {
    whisperSel.value = s.whisper_model || 'auto';
  }
  const sttDevSel = document.getElementById('cfg-stt-device');
  if (sttDevSel) {
    sttDevSel.value = s.stt_device || (s._default_stt_device === 'metal' ? 'metal' : 'cpu');
    if (!s.stt_device && !s._default_stt_device) {
      sttDevSel.value = 'metal';
    }
  }
  const dgModelSel = document.getElementById('cfg-deepgram-model');
  if (dgModelSel) {
    dgModelSel.value = s.deepgram_model || 'nova-3';
  }
  const polishCb = document.getElementById('cfg-translation-polish');
  if (polishCb) {
    polishCb.checked = s.translation_polish !== false;
  }
  updateTranslationEngineUI();
  updateSttEngineUI();

  const modelSel = document.getElementById('cfg-translation-model');
  if (modelSel && s.translation_model) {
    modelSel.dataset.current = s.translation_model;
  }
  updateTranslationModelMeta();

  populateCallLangSelect('cfg-my-lang', s.my_language || 'ru');
  populateCallLangSelect('cfg-their-lang', s.their_language || 'en');
  document.getElementById('cfg-endpointing').value = s.endpointing_ms || 500;
  document.getElementById('endpointing-val').textContent = (s.endpointing_ms || 500) + 'ms';
  populateUiLocaleSelect(s.ui_locale || '', s._system_locale);
  applyPlatformAudioHints(s);
}

function populateUiLocaleSelect(selected, systemLocale) {
  const sel = document.getElementById('cfg-ui-locale');
  if (!sel) return;
  sel.replaceChildren();
  const sysOpt = document.createElement('option');
  sysOpt.value = '';
  sysOpt.id = 'cfg-ui-locale-system';
  sysOpt.textContent = t('settings.uiLocaleSystem', { locale: systemLocale || 'en' });
  sel.appendChild(sysOpt);
  for (const { code, flag, name } of UI_LOCALES) {
    const opt = document.createElement('option');
    opt.value = code;
    opt.textContent = `${flag} ${name}`;
    sel.appendChild(opt);
  }
  sel.value = selected;
}

function updateUiLocaleLabels(systemLocale) {
  const sysOpt = document.getElementById('cfg-ui-locale-system');
  if (sysOpt && systemLocale) {
    sysOpt.textContent = t('settings.uiLocaleSystem', { locale: systemLocale });
  }
}

export function readForm() {
  return {
    deepgram_api_key: (
      document.getElementById('cfg-deepgram')._getRealValue ||
      (() => document.getElementById('cfg-deepgram').value)
    )().trim(),
    openrouter_api_key: (
      document.getElementById('cfg-openrouter')._getRealValue ||
      (() => document.getElementById('cfg-openrouter').value)
    )().trim(),
    stt_backend: sttBackendValue(),
    deepgram_model: deepgramModelValue(),
    stt_device: document.getElementById('cfg-stt-device')?.value || '',
    whisper_model: whisperModelValue(),
    translation_backend: translationBackendValue(),
    translation_polish: document.getElementById('cfg-translation-polish')?.checked !== false,
    translation_model: document.getElementById('cfg-translation-model')?.value || '',
    my_language: document.getElementById('cfg-my-lang').value,
    their_language: document.getElementById('cfg-their-lang').value,
    tts_outgoing_voice: document.getElementById('cfg-voice-out').value,
    tts_incoming_voice: document.getElementById('cfg-voice-in').value,
    mic_device: document.getElementById('cfg-mic').value || 'default',
    speaker_device: document.getElementById('cfg-speaker').value || 'default',
    meet_input_device:
      (document.getElementById('cfg-meet-in') || {}).value ||
      state.currentSettings.meet_input_device ||
      'default',
    meet_output_device:
      (document.getElementById('cfg-meet-out') || {}).value ||
      state.currentSettings.meet_output_device ||
      'default',
    endpointing_ms: parseInt(document.getElementById('cfg-endpointing').value),
    ui_locale: document.getElementById('cfg-ui-locale')?.value || '',
  };
}

export async function loadSettings() {
  try {
    const r = await fetch('/api/settings');
    state.currentSettings = await r.json();
    await initI18n(state.currentSettings._effective_ui_locale || 'en');
    populateForm(state.currentSettings);
    await refreshTranslationStatus();
    await refreshSttStatus();
    updateTranslationEngineUI();
    updateSttEngineUI();
    if (state.currentSettings.translation_backend === 'openrouter') {
      await loadTranslationModels();
    }
  } catch (e) {
    console.error('Failed to load settings', e);
  }
}

export async function saveSettings() {
  const settings = readForm();
  await fetch('/api/settings', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(settings),
  });
  state.currentSettings = { ...state.currentSettings, ...settings };
}

export function initUiLocaleListener() {
  document.getElementById('cfg-ui-locale')?.addEventListener('change', async (e) => {
    const code = e.target.value;
    await switchLocale(code, state.currentSettings._system_locale);
    updateUiLocaleLabels(state.currentSettings._system_locale);
    refreshSettingsButtonLabels();
    applyPlatformAudioHints();
    applyEngineButton(state.engineRunning);
    await loadDevices();
    await refreshSttStatus();
    await refreshTranslationStatus();
    await loadVoices();
    updateSttEngineUI();
    updateTranslationEngineUI();
  });
}
