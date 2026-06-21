import { state } from '../core/state.js';
import { sttBackendValue, updateSttEngineUI, refreshSttStatus, whisperModelValue } from './stt.js';
import {
  translationBackendValue,
  updateTranslationEngineUI,
  refreshTranslationStatus,
  loadTranslationModels,
  updateTranslationModelMeta,
} from './translation.js';

export function populateForm(s) {
  const dg = document.getElementById('cfg-deepgram');
  const or = document.getElementById('cfg-openrouter');
  if (dg._setRealValue) dg._setRealValue(s.deepgram_api_key || '');
  else dg.value = s.deepgram_api_key || '';
  if (or._setRealValue) or._setRealValue(s.openrouter_api_key || '');
  else or.value = s.openrouter_api_key || '';
  if (!s.deepgram_api_key && s._deepgram_from_env) dg.placeholder = 'Set via .env file';
  if (!s.openrouter_api_key && s._openrouter_from_env) or.placeholder = 'Set via OPENROUTER_API_KEY in .env';

  const useLocalCb = document.getElementById('cfg-use-local-model');
  if (useLocalCb) {
    useLocalCb.checked = (s.translation_backend || 'local') !== 'openrouter';
  }
  const useLocalSttCb = document.getElementById('cfg-use-local-stt');
  if (useLocalSttCb) {
    useLocalSttCb.checked = (s.stt_backend || 'local') !== 'deepgram';
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

  document.getElementById('cfg-my-lang').value = s.my_language || 'ru';
  document.getElementById('cfg-their-lang').value = s.their_language || 'en';
  document.getElementById('cfg-endpointing').value = s.endpointing_ms || 500;
  document.getElementById('endpointing-val').textContent = (s.endpointing_ms || 500) + 'ms';
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
      'TranslateTelega',
    meet_output_device:
      (document.getElementById('cfg-meet-out') || {}).value ||
      state.currentSettings.meet_output_device ||
      'TranslateTelega',
    endpointing_ms: parseInt(document.getElementById('cfg-endpointing').value),
  };
}

export async function loadSettings() {
  try {
    const r = await fetch('/api/settings');
    state.currentSettings = await r.json();
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
  state.currentSettings = settings;
}
