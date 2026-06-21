import { showToast } from '../core/toast.js';

const WHISPER_HINTS = {
  auto: 'Auto: Metal prefers base-q8_0 → base → tiny. CPU picks smallest CT2 installed.',
  tiny: 'Tiny — ~75 MB. Fastest STT. Good for live calls.',
  base: 'Base — ~145 MB. Balance of speed and accuracy.',
  'base-q8_0': 'Base Q8_0 — ~148 MB. Higher-precision GGML quant for Metal GPU (recommended).',
  small: 'Small — ~460 MB. Best accuracy, slowest on CPU.',
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

export function sttDeviceValue() {
  return document.getElementById('cfg-stt-device')?.value || 'cpu';
}

export function updateWhisperHint() {
  const hint = document.getElementById('stt-model-hint');
  const sel = whisperModelValue();
  const dev = sttDeviceValue();
  const devNote =
    dev === 'cpu'
      ? 'CPU mode: CTranslate2 (no GPU).'
      : 'Metal mode: whisper.cpp on GPU (AMD Radeon / Intel).';
  if (hint) {
    hint.textContent = (WHISPER_HINTS[sel] || WHISPER_HINTS.auto) + ' ' + devNote;
  }
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
      const dev = sttDeviceValue() === 'cpu' ? 'CPU' : 'Metal GPU';
      badge.textContent =
        sel === 'auto'
          ? 'Active: Whisper ' + dev + ' (auto)'
          : 'Active: Whisper ' + sel.replace('-q8_0', ' q8') + ' · ' + dev;
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
    const device = data.device || sttDeviceValue();
    const devLabel = device === 'cpu' ? 'CPU' : 'Metal GPU';
    if (data.ready) {
      el.innerHTML =
        '<span style="color:var(--green)">Active: ' +
        active +
        ' · ' +
        devLabel +
        '</span><br><span style="color:var(--text3)">Selected: ' +
        selected +
        ' · Installed: ' +
        installed +
        '</span>';
    } else {
      const need = selected === 'auto' ? 'tiny' : selected;
      const fmt = device === 'cpu' ? 'CT2' : 'GGML';
      el.innerHTML =
        '<span style="color:var(--yellow)">Need whisper-' +
        need +
        ' (' +
        fmt +
        '). Click Download selected.</span><br>' +
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
  document.getElementById('cfg-stt-device')?.addEventListener('change', () => {
    updateSttEngineUI();
    refreshSttStatus();
  });
}
