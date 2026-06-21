import { state } from './state.js';

export function updateTimer() {
  if (state.timerPaused) return;
  const elapsed = Date.now() - state.sessionStart - state.timerOffset;
  const s = Math.max(0, Math.floor(elapsed / 1000));
  const m = Math.floor(s / 60);
  const el = document.getElementById('timer');
  if (el) el.textContent = m + ':' + String(s % 60).padStart(2, '0');
}

export function startTimerInterval() {
  setInterval(updateTimer, 1000);
}
