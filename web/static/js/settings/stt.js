import { showToast } from '../core/toast.js';
import { t } from '../core/i18n.js';
import { clearChildren, setStatus, setStatusLines, statusSpan } from '../core/safe-dom.js';

const WHISPER_HINT_KEYS = {
  auto: 'hints.whisperAuto',
  tiny: 'hints.whisperTiny',
  base: 'hints.whisperBase',
  'base-q8_0': 'hints.whisperBaseQ8',
  small: 'hints.whisperSmall',
};

const DEEPGRAM_HINT_KEYS = {
  'nova-3': 'hints.dgNova3',
  'nova-2': 'hints.dgNova2',
  nova: 'hints.dgNova',
  enhanced: 'hints.dgEnhanced',
  base: 'hints.dgBase',
  'whisper-large': 'hints.dgWhisperLarge',
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

export function deepgramModelValue() {
  return document.getElementById('cfg-deepgram-model')?.value || 'nova-3';
}

export function updateDeepgramHint() {
  const hint = document.getElementById('stt-deepgram-hint');
  const sel = deepgramModelValue();
  if (hint) {
    const key = DEEPGRAM_HINT_KEYS[sel] || DEEPGRAM_HINT_KEYS['nova-3'];
    hint.textContent = t(key);
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
  const devNote = dev === 'cpu' ? t('hints.sttCpuMode') : t('hints.sttMetalMode');
  if (hint) {
    const key = WHISPER_HINT_KEYS[sel] || WHISPER_HINT_KEYS.auto;
    hint.textContent = t(key) + ' ' + devNote;
  }
}

function renderAppleSttStatus(el, apple) {
  if (!el) return;
  if (!apple?.helper) {
    setStatus(el, 'var(--yellow)', t('stt.appleHelperMissing'));
    return;
  }
  if (!apple.available) {
    setStatus(el, 'var(--yellow)', t('stt.appleUnavailable'));
    return;
  }
  if (apple.ready) {
    const onDevice = apple.on_device ? t('stt.appleOnDevice') : '';
    setStatus(el, 'var(--green)', t('stt.appleReady', { onDevice }));
    return;
  }
  if (apple.authorization === 'denied' || apple.authorization === 'restricted') {
    setStatus(el, 'var(--yellow)', t('stt.appleDenied'));
    return;
  }
  setStatus(el, 'var(--yellow)', t('stt.appleGrant'));
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
      const dev =
        sttDeviceValue() === 'cpu' ? t('settings.deviceCpu') : t('settings.deviceMetal');
      const modelLabel = sel === 'auto' ? '' : sel.replace('-q8_0', ' q8');
      badge.textContent =
        sel === 'auto'
          ? t('settings.badgeActiveWhisperAuto', { device: dev })
          : t('settings.badgeActiveWhisper', { model: modelLabel, device: dev });
    } else if (backend === 'apple') {
      badge.textContent = t('settings.badgeActiveAppleStt');
    } else {
      const dgModel = deepgramModelValue();
      badge.textContent = t('settings.badgeActiveDeepgram', { model: dgModel });
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
      setStatus(localEl, 'var(--text3)', t('stt.whisperDisabledDeepgram'));
      if (appleEl) appleEl.textContent = '';
      const model = data.deepgram_model || deepgramModelValue();
      if (cloudEl) {
        setStatus(
          cloudEl,
          data.ready ? 'var(--green)' : 'var(--yellow)',
          data.ready
            ? t('stt.deepgramActive', { model })
            : t('stt.deepgramNeedKey')
        );
      }
      return;
    }

    if (data.backend === 'apple') {
      if (cloudEl) cloudEl.textContent = '';
      setStatus(localEl, 'var(--text3)', t('stt.whisperDisabledApple'));
      renderAppleSttStatus(appleEl, data.apple);
      return;
    }

    if (cloudEl) cloudEl.textContent = '';
    if (appleEl) appleEl.textContent = '';
    const el = localEl;
    if (!el) return;
    const installed = (data.installed || []).join(', ') || t('common.none');
    const active = data.model || '—';
    const selected = data.selected || 'auto';
    const device = data.device || sttDeviceValue();
    const devLabel =
      device === 'cpu' ? t('settings.deviceCpu') : t('settings.deviceMetal');
    if (data.ready) {
      setStatusLines(el, [
        {
          color: 'var(--green)',
          text: t('stt.localActive', { model: active, device: devLabel }),
        },
        {
          color: 'var(--text3)',
          text: t('stt.localMeta', { selected, installed }),
        },
      ]);
    } else {
      const need = selected === 'auto' ? 'tiny' : selected;
      const fmt = device === 'cpu' ? 'CT2' : 'GGML';
      clearChildren(el);
      el.appendChild(
        statusSpan('var(--yellow)', t('stt.localNeedModel', { need, format: fmt }))
      );
      el.appendChild(document.createElement('br'));
      el.appendChild(statusSpan('var(--text3)', t('stt.localInstalled', { installed })));
    }
  } catch {
    if (localEl) localEl.textContent = t('stt.checkFailed');
  }
}

export async function downloadWhisperModel() {
  const btn = document.getElementById('btn-download-stt');
  if (!btn || btn.classList.contains('loading')) return;
  const variant = whisperModelValue();
  btn.classList.add('loading');
  btn.textContent = t('settings.downloading');
  try {
    const r = await fetch('/api/download-whisper-model', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ variant }),
    });
    const data = await r.json();
    if (!r.ok) throw new Error(data.error || 'download failed');
    showToast(t('toast.whisperInstalled', { model: data.status?.model || variant }));
    await refreshSttStatus();
  } catch (e) {
    showToast(t('toast.downloadFailed', { error: e.message }));
  }
  btn.classList.remove('loading');
  btn.textContent = t('settings.downloadSelected');
}

export async function requestAppleSpeechAuth() {
  const btn = document.getElementById('btn-banyan-speech-auth');
  if (!btn || btn.classList.contains('loading')) return;
  btn.classList.add('loading');
  btn.textContent = t('settings.requesting');
  try {
    const r = await fetch('/api/banyan-speech-authorize', { method: 'POST' });
    const data = await r.json();
    if (data.error) throw new Error(data.error);
    showToast(data.message || 'Authorization: ' + (data.authorization || 'unknown'));
    await refreshSttStatus();
  } catch (e) {
    showToast(t('toast.authFailed', { error: e.message }));
  }
  btn.classList.remove('loading');
  btn.textContent = t('settings.allowBanyanSpeech');
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
