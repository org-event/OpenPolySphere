import { state } from './core/state.js';
import { sleep } from './core/utils.js';
import { initTheme } from './core/theme.js';
import { startTimerInterval } from './core/timer.js';
import { connectSSE } from './chat/sse.js';
import { applyMuteButtons, syncEngineStatus } from './engine/control.js';
import { initMonitor } from './audio/monitor.js';
import { loadSettings } from './settings/form.js';
import { loadVoices, initVoiceListeners } from './settings/voices.js';
import { loadDevices, initDeviceListeners } from './settings/devices.js';
import { initKeyMasking } from './settings/keys.js';
import { initSttListeners } from './settings/stt.js';
import { initTranslationListeners } from './settings/translation.js';
import { initSettingsPanel } from './settings/panel.js';

export async function waitForEngine() {
  const overlay = document.getElementById('overlay');
  const text = document.getElementById('overlay-text');
  const spinner = document.getElementById('spinner');
  while (true) {
    try {
      const r = await fetch('/health');
      if (r.ok) {
        text.className = 'ready';
        text.textContent = 'Connected — press Start to translate';
        spinner.style.display = 'none';
        state.sessionStart = Date.now();
        await syncEngineStatus();
        await sleep(600);
        overlay.className = 'hidden';
        return;
      }
    } catch (_) {}
    await sleep(500);
  }
}

export async function boot() {
  initTheme();
  initKeyMasking();
  initSttListeners();
  initTranslationListeners();
  initVoiceListeners();
  initDeviceListeners();
  startTimerInterval();
  initMonitor();

  await Promise.all([loadSettings(), loadVoices(), loadDevices()]);
  if (typeof applyTooltips === 'function') applyTooltips();
  applyMuteButtons();
  initSettingsPanel();
}

export function startApp() {
  boot();
  waitForEngine();
  connectSSE();
}
