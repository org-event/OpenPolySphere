import { showToast } from '../core/toast.js';

const WHISPER_HINTS = {
  auto: 'Auto: Metal prefers base-q8_0 → base → tiny. CPU picks smallest CT2 installed.',
  tiny: 'Tiny — ~75 MB. Fastest STT. Good for live calls.',
  base: 'Base — ~145 MB. Balance of speed and accuracy.',
  'base-q8_0': 'Base Q8_0 — ~148 MB. Higher-precision GGML quant for Metal GPU (recommended).',
  small: 'Small — ~460 MB. Best accuracy, slowest on CPU.',
};

export function sttBackendValue() {
  return document.getElementById('cfg-stt-backend')?.value || 'local';
}

function applyAppleSttOptionVisibility(apple) {
  const opt = document.getElementById('cfg-stt-backend-apple');
  const sel = document.getElementById('cfg-stt-backend');
  if (!opt || !sel) return;
  const show =
    apple?.helper === true &&
    apple?.available === true &&
    apple?.status !== 'unsupported' &&
    apple?.status !== 'missing';
  opt.hidden = !show;
  if (!show && sel.value === 'apple') {
    sel.value = 'local';
    updateSttEngineUI();
  }
}

const DEEPGRAM_HINTS = {
  'nova-3': 'Best accuracy for live calls. Default.',
  'nova-2': 'Often lower latency than Nova-3; good for RU/EN.',
  nova: 'Older Nova — may be faster, less accurate.',
  enhanced: 'Legacy model — balanced speed/quality.',
  base: 'Fastest Deepgram model; weaker on accents and RU.',
  'whisper-large': 'OpenAI Whisper large hosted by Deepgram.',
};

export function deepgramModelValue() {
  return document.getElementById('cfg-deepgram-model')?.value || 'nova-3';
}

export function updateDeepgramHint() {
  const hint = document.getElementById('stt-deepgram-hint');
  const sel = deepgramModelValue();
  if (hint) {
    hint.textContent = DEEPGRAM_HINTS[sel] || DEEPGRAM_HINTS['nova-3'];
  }
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

function appleSttStatusHtml(apple) {
  if (!apple?.helper) {
    return '<span style="color:var(--yellow)">Apple Speech helper not built</span>';
  }
  if (!apple.available) {
    return '<span style="color:var(--yellow)">Speech recognition not available for your language on this Mac</span>';
  }
  if (apple.ready) {
    const onDevice = apple.on_device ? ' · on-device' : '';
    return '<span style="color:var(--green)">Ready — Apple Speech' + onDevice + '</span>';
  }
  if (apple.authorization === 'denied' || apple.authorization === 'restricted') {
    return '<span style="color:var(--yellow)">Allow Speech Recognition in System Settings → Privacy & Security</span>';
  }
  return '<span style="color:var(--yellow)">Apple Speech available — grant permission on first use</span>';
}

export function updateSttEngineUI() {
  const backend = sttBackendValue();
  const localPanel = document.getElementById('stt-local-panel');
  const applePanel = document.getElementById('stt-apple-panel');
  const cloudPanel = document.getElementById('stt-cloud-panel');
  const badge = document.getElementById('stt-engine-badge');

  if (localPanel) localPanel.style.display = backend === 'local' ? '' : 'none';
  if (applePanel) applePanel.style.display = backend === 'apple' ? '' : 'none';
  if (cloudPanel) cloudPanel.style.display = backend === 'deepgram' ? '' : 'none';

  if (badge) {
    const isCloud = backend === 'deepgram';
    badge.className = 'translation-engine-badge ' + (isCloud ? 'cloud' : 'local');
    if (backend === 'local') {
      const sel = whisperModelValue();
      const dev = sttDeviceValue() === 'cpu' ? 'CPU' : 'Metal GPU';
      badge.textContent =
        sel === 'auto'
          ? 'Active: Whisper ' + dev + ' (auto)'
          : 'Active: Whisper ' + sel.replace('-q8_0', ' q8') + ' · ' + dev;
    } else if (backend === 'apple') {
      badge.textContent = 'Active: Apple Speech (system, on-device)';
    } else {
      const dgModel = deepgramModelValue();
      badge.textContent = 'Active: Deepgram ' + dgModel;
    }
  }
  updateWhisperHint();
  updateDeepgramHint();
}

export async function refreshSttStatus() {
  const localEl = document.getElementById('stt-local-status');
  const appleEl = document.getElementById('stt-apple-status');
  const cloudEl = document.getElementById('stt-cloud-status');
  try {
    const r = await fetch('/api/stt-status');
    const data = await r.json();
    applyAppleSttOptionVisibility(data.apple);

    if (data.backend === 'deepgram') {
      if (localEl) {
        localEl.innerHTML =
          '<span style="color:var(--text3)">Whisper disabled — using Deepgram cloud</span>';
      }
      if (appleEl) appleEl.textContent = '';
      if (cloudEl) {
        const model = data.deepgram_model || deepgramModelValue();
        cloudEl.innerHTML = data.ready
          ? '<span style="color:var(--green)">Active: Deepgram ' + model + ' · streaming</span>'
          : '<span style="color:var(--yellow)">Set Deepgram API key below</span>';
      }
      return;
    }

    if (data.backend === 'apple') {
      if (cloudEl) cloudEl.textContent = '';
      if (localEl) {
        localEl.innerHTML =
          '<span style="color:var(--text3)">Whisper disabled — using Apple Speech</span>';
      }
      if (appleEl) appleEl.innerHTML = appleSttStatusHtml(data.apple);
      return;
    }

    if (cloudEl) cloudEl.textContent = '';
    if (appleEl) appleEl.textContent = '';
    const el = localEl;
    if (!el) return;
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
  } catch {
    if (localEl) localEl.textContent = 'Could not check STT model';
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

export async function requestAppleSpeechAuth() {
  const btn = document.getElementById('btn-apple-speech-auth');
  if (!btn || btn.classList.contains('loading')) return;
  btn.classList.add('loading');
  btn.textContent = 'Requesting...';
  try {
    const r = await fetch('/api/apple-speech-authorize', { method: 'POST' });
    const data = await r.json();
    if (data.error) throw new Error(data.error);
    showToast(data.message || 'Authorization: ' + (data.authorization || 'unknown'));
    await refreshSttStatus();
  } catch (e) {
    showToast('Failed: ' + e.message);
  }
  btn.classList.remove('loading');
  btn.textContent = 'Allow Speech Recognition';
}

export function initSttListeners() {
  document.getElementById('cfg-stt-backend')?.addEventListener('change', () => {
    updateSttEngineUI();
    refreshSttStatus();
  });
  document.getElementById('cfg-whisper-model')?.addEventListener('change', () => {
    updateSttEngineUI();
    refreshSttStatus();
  });
  document.getElementById('cfg-stt-device')?.addEventListener('change', () => {
    updateSttEngineUI();
    refreshSttStatus();
  });
  document.getElementById('cfg-deepgram-model')?.addEventListener('change', () => {
    updateSttEngineUI();
    refreshSttStatus();
  });
}
