import { state } from './core/state.js';
import { sleep } from './core/utils.js';
import { t } from './core/i18n.js';
import { initTheme } from './core/theme.js';
import { startTimerInterval } from './core/timer.js';
import { connectSSE } from './chat/sse.js';
import { applyMuteButtons, syncEngineStatus } from './engine/control.js';
import { initMonitor } from './audio/monitor.js';
import { loadSettings, initUiLocaleListener } from './settings/form.js';
import { loadVoices, initVoiceListeners } from './settings/voices.js';
import { loadDevices, initDeviceListeners } from './settings/devices.js';
import { initKeyMasking } from './settings/keys.js';
import { initSttListeners } from './settings/stt.js';
import { initTranslationListeners } from './settings/translation.js';
import { initSettingsPanel } from './settings/panel.js';
import { initBrandLogo, showOverlayLogo } from './ui/brand-logo.js';

export async function waitForEngine() {
  const overlay = document.getElementById('overlay');
  const text = document.getElementById('overlay-text');
  const spinner = document.getElementById('spinner');
  while (true) {
    try {
      const r = await fetch('/health');
      if (r.ok) {
        spinner.style.display = 'none';
        await showOverlayLogo();
        text.className = 'ready';
        text.textContent = t('app.connected');
        state.sessionStart = Date.now();
        await syncEngineStatus();
        await sleep(900);
        overlay.className = 'hidden';
        return;
      }
    } catch {}
    await sleep(500);
  }
}

export async function boot() {
  initBrandLogo();
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
  initUiLocaleListener();
}

export async function startApp() {
  connectSSE();
  await boot();
  waitForEngine();
}
