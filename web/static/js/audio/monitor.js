import { showToast } from '../core/toast.js';

let monitorEnabled = false;
let audioCtx = null;
let monitorQueue = [];
let monitorPlaying = false;

export function toggleMonitor() {
  monitorEnabled = !monitorEnabled;
  document.getElementById('btn-monitor')?.classList.toggle('on', monitorEnabled);
  if (monitorEnabled && !audioCtx) {
    audioCtx = new (window.AudioContext || window.webkitAudioContext)();
  }
  if (monitorEnabled && audioCtx && audioCtx.state === 'suspended') {
    audioCtx.resume();
  }
  showToast(monitorEnabled ? 'Monitor ON' : 'Monitor OFF');
}

async function pollAudio() {
  if (!monitorEnabled || !audioCtx) return;
  try {
    const r = await fetch('/api/poll-audio');
    const items = await r.json();
    for (const item of items) {
      monitorQueue.push(item);
    }
    if (items.length > 0 && !monitorPlaying) drainMonitorQueue();
  } catch (e) {
    console.error('[MONITOR] poll error:', e);
  }
}

async function drainMonitorQueue() {
  monitorPlaying = true;
  while (monitorQueue.length > 0) {
    const { sr, b64 } = monitorQueue.shift();
    try {
      const raw = atob(b64);
      const bytes = new Uint8Array(raw.length);
      for (let i = 0; i < raw.length; i++) bytes[i] = raw.charCodeAt(i);
      const pcm16 = new Int16Array(bytes.buffer);
      const floats = new Float32Array(pcm16.length);
      for (let i = 0; i < pcm16.length; i++) {
        floats[i] = pcm16[i] / 32768.0;
      }
      const buf = audioCtx.createBuffer(1, floats.length, sr);
      buf.getChannelData(0).set(floats);
      const src = audioCtx.createBufferSource();
      src.buffer = buf;
      const gain = audioCtx.createGain();
      gain.gain.value = 0.8;
      src.connect(gain).connect(audioCtx.destination);
      src.start();
      await new Promise((r) => {
        src.onended = r;
      });
    } catch (e) {
      console.warn('Monitor playback error:', e);
    }
  }
  monitorPlaying = false;
}

export function initMonitor() {
  setInterval(pollAudio, 500);
}
