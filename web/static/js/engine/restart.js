import { sendCmd } from '../core/api.js';
import { t } from '../core/i18n.js';
import { showToast } from '../core/toast.js';
import { sleep } from '../core/utils.js';
import { applyEngineButton, syncEngineStatus } from './control.js';
import { setEnginePill } from './pill.js';
import { saveSettings } from '../settings/form.js';
import { state } from '../core/state.js';

export async function saveAndRestart() {
  const btn = document.getElementById('restart-btn');
  const txt = document.getElementById('restart-text');
  const bar = document.getElementById('restart-progress');

  btn.classList.add('restarting');
  btn.classList.remove('success', 'error');

  try {
    txt.textContent = 'Saving settings...';
    bar.style.width = '15%';
    setEnginePill('restarting', 'Saving...');
    await saveSettings();
    await sleep(300);

    txt.textContent = 'Restarting engine...';
    bar.style.width = '35%';
    setEnginePill('restarting', 'Restarting...');
    if (state.engineRunning) {
      await sendCmd('stop').catch(() => {});
      await fetch('/api/calls/end', { method: 'POST' }).catch(() => {});
      applyEngineButton(false);
    }
    await fetch('/api/engine/restart', { method: 'POST' });
    await sleep(500);

    txt.textContent = 'Loading models...';
    bar.style.width = '60%';
    setEnginePill('restarting', 'Loading...');

    let attempts = 0;
    while (attempts < 60) {
      await sleep(1000);
      attempts++;
      bar.style.width = Math.min(60 + attempts, 95) + '%';
      try {
        const r = await fetch('/health');
        if (r.ok) break;
      } catch {}
    }

    txt.textContent = 'Ready...';
    bar.style.width = '95%';
    await sleep(1500);
    await syncEngineStatus();

    bar.style.width = '100%';
    btn.classList.remove('restarting');
    btn.classList.add('success');
    txt.innerHTML = '&#10003; Ready!';
    showToast(t('toast.savedPressStart'));

    await sleep(2500);
    btn.classList.remove('success');
    txt.textContent = t('settings.saveRestart');
    bar.style.width = '0%';
  } catch (e) {
    btn.classList.remove('restarting');
    btn.classList.add('error');
    txt.textContent = 'Error: ' + (e.message || 'restart failed');
    setEnginePill('stopped', 'Error');

    await sleep(3000);
    btn.classList.remove('error');
    txt.textContent = 'Save & Restart Engine';
    bar.style.width = '0%';
  }
}
