import { state } from '../core/state.js';
import { startLevelMonitoring, stopLevelMonitoring } from '../audio/levels.js';
import { refreshSttStatus, updateSttEngineUI } from './stt.js';
import {
  refreshTranslationStatus,
  updateTranslationEngineUI,
  loadTranslationModels,
  translationBackendValue,
} from './translation.js';

function closeLogsPanel() {
  document.getElementById('lp-backdrop')?.classList.remove('open');
  document.getElementById('lp')?.classList.remove('open');
}

export function openSettings() {
  closeLogsPanel();
  document.getElementById('sp-backdrop')?.classList.add('open');
  document.getElementById('sp')?.classList.add('open');
  refreshTranslationStatus();
  refreshSttStatus();
  updateTranslationEngineUI();
  updateSttEngineUI();
  if (translationBackendValue() === 'openrouter') {
    loadTranslationModels().catch((e) => console.warn('loadTranslationModels failed', e));
  }
  startLevelMonitoring();
}

export function closeSettings() {
  document.getElementById('sp-backdrop')?.classList.remove('open');
  document.getElementById('sp')?.classList.remove('open');
  stopLevelMonitoring();
}

export function toggleSection(id) {
  document.getElementById(id)?.classList.toggle('collapsed');
}

export function initSettingsPanel() {
  const sttBackend = state.currentSettings.stt_backend || 'local';
  if (sttBackend === 'deepgram' && !state.currentSettings.deepgram_api_key) {
    openSettings();
  }
}
