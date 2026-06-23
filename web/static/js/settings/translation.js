import { state } from '../core/state.js';
import { showToast } from '../core/toast.js';
import { t } from '../core/i18n.js';
import { setStatus } from '../core/safe-dom.js';

export function translationBackendValue() {
  return document.getElementById('cfg-translation-backend')?.value || 'local';
}

function applyAppleOptionVisibility(apple) {
  const opt = document.getElementById('cfg-translation-backend-apple');
  const sel = document.getElementById('cfg-translation-backend');
  if (!opt || !sel) return;
  const show = apple?.available === true;
  opt.hidden = !show;
  if (!show && sel.value === 'apple') {
    sel.value = 'local';
    updateTranslationEngineUI();
  }
}

export function updateTranslationEngineUI() {
  const backend = translationBackendValue();
  const localPanel = document.getElementById('translation-local-panel');
  const applePanel = document.getElementById('translation-apple-panel');
  const cloudPanel = document.getElementById('translation-cloud-panel');
  const polishWrap = document.getElementById('cfg-translation-polish-wrap');
  const badge = document.getElementById('translation-engine-badge');

  if (localPanel) localPanel.style.display = backend === 'local' ? '' : 'none';
  if (applePanel) applePanel.style.display = backend === 'apple' ? '' : 'none';
  if (cloudPanel) cloudPanel.style.display = backend === 'openrouter' ? '' : 'none';
  if (polishWrap) polishWrap.style.display = backend === 'local' ? '' : 'none';

  if (badge) {
    const isCloud = backend === 'openrouter';
    badge.className = 'translation-engine-badge ' + (isCloud ? 'cloud' : 'local');
    if (backend === 'local') {
      const polishOn = document.getElementById('cfg-translation-polish')?.checked !== false;
      badge.textContent = polishOn
        ? t('settings.badgeActivePolish')
        : t('settings.badgeActiveLocal');
    } else if (backend === 'apple') {
      badge.textContent = t('settings.badgeActiveAppleTr');
    } else {
      const modelSel = document.getElementById('cfg-translation-model');
      const model =
        modelSel?.selectedOptions?.[0]?.textContent?.trim() ||
        state.currentSettings?.translation_model ||
        'OpenRouter';
      badge.textContent = t('settings.badgeActiveOpenrouter', {
        model: model.replace(/^\d+\.\s*/, '').slice(0, 48),
      });
    }
  }
}

function renderAppleTranslationStatus(el, apple) {
  if (!el) return;
  if (!apple?.helper) {
    setStatus(el, 'var(--yellow)', t('translation.appleHelperMissing'));
    return;
  }
  if (!apple.available) {
    setStatus(el, 'var(--yellow)', t('translation.applePairUnsupported'));
    return;
  }
  if (apple.ready) {
    setStatus(el, 'var(--green)', t('translation.appleReady'));
    return;
  }
  if (apple.status === 'supported') {
    setStatus(el, 'var(--yellow)', t('translation.appleSupported'));
    return;
  }
  setStatus(el, 'var(--yellow)', t('translation.appleChecking'));
}

export async function refreshTranslationStatus() {
  const el = document.getElementById('translation-local-status');
  const appleEl = document.getElementById('translation-apple-status');
  try {
    const r = await fetch('/api/translation-status');
    const data = await r.json();
    applyAppleOptionVisibility(data.apple);

    const lines = Object.entries(data.pairs || {}).map(([name, ok]) => (ok ? '✓ ' : '✗ ') + name);
    if (el) {
      if (data.backend === 'openrouter') {
        setStatus(el, 'var(--yellow)', t('translation.openrouterSelected'));
      } else if (data.backend === 'apple') {
        setStatus(el, 'var(--yellow)', t('translation.appleSelected'));
      } else if (data.ready) {
        let polishNote = '';
        if (data.polish_enabled && !data.polish_active && data.polish_disabled_reason) {
          polishNote = t('translation.polishReason', { reason: data.polish_disabled_reason });
        } else if (data.polish_enabled && data.polish_active) {
          polishNote = t('translation.polishActive', {
            model: data.polish_model || 'Qwen2.5-0.5B',
          });
        }
        setStatus(
          el,
          'var(--green)',
          t('translation.ready', { pairs: lines.join(', '), polish: polishNote })
        );
      } else {
        setStatus(
          el,
          'var(--yellow)',
          t('translation.modelsMissing', { pairs: lines.join(', ') })
        );
      }
    }
    if (appleEl) {
      renderAppleTranslationStatus(appleEl, data.apple);
    }
  } catch {
    if (el) el.textContent = t('translation.checkFailed');
    if (appleEl) appleEl.textContent = t('translation.appleCheckFailed');
  }
}

export async function downloadTranslationModels() {
  const btn = document.getElementById('btn-download-translate');
  if (!btn || btn.classList.contains('loading')) return;
  btn.classList.add('loading');
  btn.textContent = t('settings.downloading');
  try {
    const r = await fetch('/api/download-translation-models', { method: 'POST' });
    const data = await r.json();
    if (!r.ok) throw new Error(data.error || 'download failed');
    showToast(t('settings.modelsInstalled'));
    await refreshTranslationStatus();
  } catch (e) {
    showToast(t('toast.downloadFailed', { error: e.message }));
  }
  btn.classList.remove('loading');
  btn.textContent = t('settings.downloadOpusMt');
}

export async function downloadPolishModel() {
  const btn = document.getElementById('btn-download-polish');
  if (!btn || btn.classList.contains('loading')) return;
  btn.classList.add('loading');
  btn.textContent = t('settings.downloading');
  try {
    const r = await fetch('/api/download-polish-model', { method: 'POST' });
    const data = await r.json();
    if (!r.ok) throw new Error(data.error || 'download failed');
    showToast(t('settings.polishInstalled'));
    await refreshTranslationStatus();
  } catch (e) {
    showToast(t('toast.downloadFailed', { error: e.message }));
  }
  btn.classList.remove('loading');
  btn.textContent = t('settings.downloadPolish');
}

export async function testLocalTranslation() {
  const btn =
    document.getElementById('btn-test-translate') ||
    document.getElementById('btn-test-translate-apple');
  if (!btn) return;
  btn.textContent = t('settings.keyTest.testing');
  btn.className = 'sp-test-btn testing';
  try {
    const r = await fetch('/api/translate', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ text: 'привет', from: 'ru', to: 'en' }),
    });
    const data = await r.json();
    if (data.error) {
      btn.textContent = t('settings.testFail');
      btn.className = 'sp-test-btn fail';
      showToast(data.error);
    } else {
      btn.textContent = '✓ ' + (data.translation || '').slice(0, 20);
      btn.className = 'sp-test-btn ok';
      showToast(
        t('settings.translationPreview', {
          text: data.translation || t('settings.translationEmpty'),
        })
      );
    }
  } catch {
    btn.textContent = t('settings.keyTest.error');
    btn.className = 'sp-test-btn fail';
  }
  setTimeout(() => {
    btn.textContent = t('settings.test');
    btn.className = 'sp-test-btn';
  }, 3000);
}

export function updateTranslationModelMeta() {
  const meta = document.getElementById('translation-model-meta');
  const sel = document.getElementById('cfg-translation-model');
  if (!meta || !sel) return;
  const id = sel.value;
  const m = state.translationModelsCache.find((x) => x.id === id);
  if (m) {
    meta.textContent =
      (m.free ? t('settings.metaFree') : '') +
      t('settings.metaContext', { label: m.context_label }) +
      (m.description ? ' · ' + m.description : '');
  } else if (id) {
    meta.textContent = id;
  } else {
    meta.textContent = t('settings.selectModelHint');
  }
}

export async function loadTranslationModels() {
  const sel = document.getElementById('cfg-translation-model');
  const btn = document.getElementById('btn-refresh-models');
  const freeOnly = document.getElementById('cfg-models-free')?.checked ?? true;
  const sort = document.getElementById('cfg-models-sort')?.value || 'latency-low-to-high';
  if (!sel) return;

  const current = sel.dataset.current || state.currentSettings.translation_model || sel.value || '';
  if (btn) btn.classList.add('loading');

  try {
    const r = await fetch(
      '/api/translation-models?free=' + (freeOnly ? '1' : '0') + '&sort=' + encodeURIComponent(sort)
    );
    const data = await r.json();
    state.translationModelsCache = data.models || [];
    const ids = new Set(state.translationModelsCache.map((m) => m.id));

    sel.innerHTML = '';
    if (state.translationModelsCache.length === 0) {
      const opt = document.createElement('option');
      opt.value = current;
      opt.textContent = current || t('settings.noModels');
      sel.appendChild(opt);
    } else {
      state.translationModelsCache.forEach((m, i) => {
        const opt = document.createElement('option');
        opt.value = m.id;
        const rank = i + 1;
        opt.textContent =
          rank +
          '. ' +
          m.name +
          (m.free ? ' · ' + t('settings.modelFreeTag') : '') +
          ' · ' +
          m.context_label;
        sel.appendChild(opt);
      });
    }

    const pick =
      current && (ids.has(current) || current)
        ? current
        : data.current || state.translationModelsCache[0]?.id || '';
    if (pick && !ids.has(pick)) {
      const opt = document.createElement('option');
      opt.value = pick;
      opt.textContent = t('settings.deviceSaved', { name: pick });
      sel.insertBefore(opt, sel.firstChild);
    }
    if (pick) sel.value = pick;
    sel.dataset.current = sel.value;
    updateTranslationModelMeta();
  } catch (e) {
    console.error('Failed to load translation models', e);
    if (current) {
      sel.innerHTML = '';
      const opt = document.createElement('option');
      opt.value = current;
      opt.textContent = t('settings.deviceOffline', { name: current });
      sel.appendChild(opt);
      sel.value = current;
    }
  }
  if (btn) btn.classList.remove('loading');
  updateTranslationEngineUI();
}

export function initTranslationListeners() {
  document.getElementById('cfg-translation-backend')?.addEventListener('change', () => {
    updateTranslationEngineUI();
    if (translationBackendValue() === 'openrouter') {
      loadTranslationModels().catch(() => {});
    }
  });

  document.getElementById('cfg-translation-polish')?.addEventListener('change', () => {
    updateTranslationEngineUI();
  });

  document.getElementById('cfg-translation-model')?.addEventListener('change', () => {
    updateTranslationModelMeta();
    updateTranslationEngineUI();
  });
}
