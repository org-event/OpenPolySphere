import { showToast } from '../core/toast.js';

const WHISPER_HINTS = {
  auto: 'Auto picks the smallest downloaded model (tiny → base → small).',
  tiny: 'Tiny — ~75 MB. Fastest STT (~0.8–1.2 s). Good for live calls.',
  base: 'Base — ~145 MB. Balance of speed and accuracy.',
  small: 'Small — ~460 MB. Best accuracy, slowest (~2–3 s STT).',
};

export function isLocalStt() {
  const cb = document.getElementById('cfg-use-local-stt');
  return cb ? cb.checked : true;
}

export function sttBackendValue() {
  return isLocalStt() ? 'local' : 'deepgram';
}

export function whisperModelValue() {
  return document.getElementById('cfg-whisper-model')?.value || 'auto';
}

export function updateWhisperHint() {
  const hint = document.getElementById('stt-model-hint');
  const sel = whisperModelValue();
  if (hint) hint.textContent = WHISPER_HINTS[sel] || WHISPER_HINTS.auto;
}

export function updateSttEngineUI() {
  const useLocal = isLocalStt();
  const localPanel = document.getElementById('stt-local-panel');
  const cloudPanel = document.getElementById('stt-cloud-panel');
  const badge = document.getElementById('stt-engine-badge');
  if (localPanel) localPanel.style.display = useLocal ? '' : 'none';
  if (cloudPanel) cloudPanel.style.display = useLocal ? 'none' : '';
  if (badge) {
    badge.className = 'translation-engine-badge ' + (useLocal ? 'local' : 'cloud');
    if (useLocal) {
      const sel = whisperModelValue();
      badge.textContent =
        sel === 'auto' ? 'Active: Local Whisper (auto)' : 'Active: Local Whisper ' + sel;
    } else {
      badge.textContent = 'Active: Deepgram (cloud)';
    }
  }
  updateWhisperHint();
}

export async function refreshSttStatus() {
  const el = document.getElementById('stt-local-status');
  if (!el) return;
  try {
    const r = await fetch('/api/stt-status');
    const data = await r.json();
    if (data.backend === 'deepgram') {
      el.innerHTML = '<span style="color:var(--yellow)">Switch off “Local model” to use Deepgram</span>';
      return;
    }
    const installed = (data.installed || []).join(', ') || 'none';
    const active = data.model || '—';
    const selected = data.selected || 'auto';
    if (data.ready) {
      el.innerHTML =
        '<span style="color:var(--green)">Active: ' +
        active +
        '</span><br><span style="color:var(--text3)">Selected: ' +
        selected +
        ' · Installed: ' +
        installed +
        '</span>';
    } else {
      el.innerHTML =
        '<span style="color:var(--yellow)">Need whisper-' +
        (selected === 'auto' ? 'tiny' : selected) +
        '. Click Download selected.</span><br>' +
        '<span style="color:var(--text3)">Installed: ' +
        installed +
        '</span>';
    }
  } catch (_) {
    el.textContent = 'Could not check STT model';
  }
}

export async function downloadWhisperModel() {
  const btn = document.getElementById('btn-download-stt');
  if (!btn || btn.classList.contains('loading')) return;
  const variant = whisperModelValue();
  btn.classList.add('loading');
  btn.textContent = 'Downloading...';
  try {
    const r = await fetch('/api/download-whisper-model', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ variant }),
    });
    const data = await r.json();
    if (!r.ok) throw new Error(data.error || 'download failed');
    showToast('Whisper ' + (data.status?.model || variant) + ' installed — Save & Restart');
    await refreshSttStatus();
  } catch (e) {
    showToast('Download failed: ' + e.message);
  }
  btn.classList.remove('loading');
  btn.textContent = 'Download selected';
}

export function initSttListeners() {
  document.getElementById('cfg-use-local-stt')?.addEventListener('change', () => {
    updateSttEngineUI();
  });
  document.getElementById('cfg-whisper-model')?.addEventListener('change', () => {
    updateSttEngineUI();
    refreshSttStatus();
  });
}
