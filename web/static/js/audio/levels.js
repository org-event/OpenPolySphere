import { sendCmd } from '../core/api.js';
import { state } from '../core/state.js';

function setMeter(meterId, hintId, level, error, active) {
  const meter = document.getElementById(meterId);
  const hint = document.getElementById(hintId);
  if (!meter) return;
  const fill = meter.querySelector('.level-meter-fill');
  const pct = Math.min(100, Math.max(0, (level || 0) * 100));
  fill.style.width = pct + '%';
  meter.classList.toggle('error', !!error);
  if (hint) {
    if (error) {
      hint.textContent = error;
      hint.style.color = 'var(--red)';
    } else if (level > 0.008) {
      hint.textContent = '';
      hint.style.color = '';
    } else if (active) {
      hint.textContent = 'Listening… speak louder';
      hint.style.color = 'var(--text3)';
    } else {
      hint.textContent = 'No signal';
      hint.style.color = '';
    }
  }
}

export async function startLevelMonitoring() {
  const micEl = document.getElementById('cfg-mic');
  const callInEl = document.getElementById('cfg-meet-in');
  const mic = micEl ? micEl.value : state.currentSettings.mic_device || '';
  const callIn = callInEl ? callInEl.value : state.currentSettings.meet_input_device || '';
  if (!mic && !callIn) return;

  try {
    await fetch('/api/monitor-levels', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ mic, call_in: callIn }),
    });
  } catch (e) {
    console.warn('monitor-levels failed', e);
  }

  state.levelMonitoring = true;
  if (!state.levelPollTimer) {
    state.levelPollTimer = setInterval(pollAudioLevels, 120);
  }
  pollAudioLevels();
}

export async function stopLevelMonitoring() {
  state.levelMonitoring = false;
  if (state.levelPollTimer) {
    clearInterval(state.levelPollTimer);
    state.levelPollTimer = null;
  }
  try {
    await sendCmd('stop_level_monitors');
  } catch (_) {}
  setMeter('meter-mic', 'meter-mic-hint', 0);
  setMeter('meter-call-in', 'meter-call-in-hint', 0);
}

async function pollAudioLevels() {
  if (!state.levelMonitoring) return;
  try {
    const r = await fetch('/api/audio-levels');
    const data = await r.json();
    setMeter('meter-mic', 'meter-mic-hint', data.mic, data.mic_error, data.mic_active);
    setMeter('meter-call-in', 'meter-call-in-hint', data.call_in, data.call_in_error, data.call_in_active);
  } catch (_) {}
}

export function bindLevelMeterDeviceChange() {
  ['cfg-mic', 'cfg-meet-in'].forEach((id) => {
    const el = document.getElementById(id);
    if (!el || el._levelBound) return;
    el._levelBound = true;
    el.addEventListener('change', () => {
      if (document.getElementById('sp')?.classList.contains('open')) {
        startLevelMonitoring();
      }
    });
  });
}
