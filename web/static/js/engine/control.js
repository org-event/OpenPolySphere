import { sendCmd } from '../core/api.js';
import { state } from '../core/state.js';
import { showToast } from '../core/toast.js';
import { sleep } from '../core/utils.js';
import { clearAll } from '../chat/messages.js';
import { connectSSE } from '../chat/sse.js';
import { hideTyping } from '../chat/typing.js';
import { startLevelMonitoring } from '../audio/levels.js';
import { setEnginePill } from './pill.js';

export function applyEngineButton(running) {
  state.engineRunning = running;
  const btn = document.getElementById('btn-engine');
  const icon = document.getElementById('engine-icon');
  const text = document.getElementById('engine-toggle-text');
  if (!btn || !icon || !text) return;
  if (running) {
    btn.className = 'btn btn-engine running';
    icon.innerHTML = '&#9724;';
    text.textContent = 'Stop';
    setEnginePill('running', 'Translating');
  } else {
    btn.className = 'btn btn-engine stopped';
    icon.innerHTML = '&#9654;';
    text.textContent = 'Start';
    setEnginePill('stopped', 'Stopped');
  }
}

export async function syncEngineStatus() {
  try {
    const data = await sendCmd('status');
    const status = (data.status || '').replace(/^ok:/, '');
    const running = status === 'running';
    applyEngineButton(running);
    return running;
  } catch (_) {
    applyEngineButton(false);
    return false;
  }
}

export async function toggleEngine() {
  if (state.engineBusy) return;
  state.engineBusy = true;

  const btn = document.getElementById('btn-engine');
  const icon = document.getElementById('engine-icon');
  const text = document.getElementById('engine-toggle-text');
  btn.disabled = true;

  try {
    if (state.engineRunning) {
      text.textContent = 'Stopping...';
      icon.innerHTML = '&#8987;';
      setEnginePill('restarting', 'Stopping...');
      const resp = await sendCmd('stop');
      if (!(resp.status || '').startsWith('ok')) {
        showToast('Stop failed: ' + (resp.status || 'unknown'));
        await syncEngineStatus();
        return;
      }
      state.timerPaused = true;
      state.timerPausedAt = Date.now();
      applyEngineButton(false);
      hideTyping();
      state.pending = { direction: null, transcript: null, translation: null };
      showToast('Engine stopped');
      fetch('/api/calls/end', { method: 'POST' }).catch(() => {});
      if (document.getElementById('sp')?.classList.contains('open')) {
        startLevelMonitoring();
      }
      return;
    }

    btn.className = 'btn btn-engine stopped';
    text.textContent = 'Starting...';
    icon.innerHTML = '&#8987;';
    setEnginePill('restarting', 'Starting...');

    await fetch('/api/calls/new-session', { method: 'POST' });
    clearAll();
    if (state.evtSource) state.evtSource.close();
    connectSSE();

    const resp = await sendCmd(state.muteState.incoming ? 'start outgoing' : 'start');
    if (!(resp.status || '').startsWith('ok')) {
      showToast('Start failed: ' + (resp.status || 'unknown'));
      applyEngineButton(false);
      return;
    }

    let running = false;
    for (let i = 0; i < 30; i++) {
      await sleep(500);
      running = await syncEngineStatus();
      if (running) break;
    }

    if (!running) {
      showToast('Engine did not start — check API keys and model');
      applyEngineButton(false);
      return;
    }

    state.sessionStart = Date.now();
    state.timerOffset = 0;
    state.timerPaused = false;
    showToast('Engine started');
    if (document.getElementById('sp')?.classList.contains('open')) {
      startLevelMonitoring();
    }
  } catch (e) {
    showToast('Error: ' + (e.message || 'command failed'));
    await syncEngineStatus();
  } finally {
    state.engineBusy = false;
    btn.disabled = false;
  }
}

export async function toggleMute(direction) {
  state.muteState[direction] = !state.muteState[direction];
  applyMuteButton(direction);
  const muted = state.muteState[direction];
  await sendCmd(muted ? 'mute_' + direction : 'unmute_' + direction);
}

export function applyMuteButton(direction) {
  const btn = document.getElementById(direction === 'outgoing' ? 'btn-mic-out' : 'btn-mic-in');
  if (!btn) return;
  btn.className = state.muteState[direction] ? 'btn muted' : 'btn active';
}

export function applyMuteButtons() {
  applyMuteButton('outgoing');
  applyMuteButton('incoming');
}
