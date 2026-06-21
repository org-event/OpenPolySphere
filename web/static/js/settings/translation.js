import { state } from '../core/state.js';
import { showToast } from '../core/toast.js';

export function isLocalTranslation() {
  const cb = document.getElementById('cfg-use-local-model');
  return cb ? cb.checked : true;
}

export function translationBackendValue() {
  return isLocalTranslation() ? 'local' : 'openrouter';
}

export function updateTranslationEngineUI() {
  const useLocal = isLocalTranslation();
  const localPanel = document.getElementById('translation-local-panel');
  const cloudPanel = document.getElementById('translation-cloud-panel');
  const badge = document.getElementById('translation-engine-badge');
  if (localPanel) localPanel.style.display = useLocal ? '' : 'none';
  if (cloudPanel) cloudPanel.style.display = useLocal ? 'none' : '';
  if (badge) {
    badge.className = 'translation-engine-badge ' + (useLocal ? 'local' : 'cloud');
    if (useLocal) {
      const polishOn = document.getElementById('cfg-translation-polish')?.checked !== false;
      const hasKey =
        !!(state.currentSettings?.openrouter_api_key || state.currentSettings?._openrouter_from_env);
      if (polishOn && hasKey) {
        badge.textContent = 'Active: Opus-MT + AI polish ru↔en';
      } else if (polishOn && !hasKey) {
        badge.textContent = 'Active: Opus-MT (polish needs OpenRouter key)';
      } else {
        badge.textContent = 'Active: Local Opus-MT ru↔en';
      }
    } else {
      const modelSel = document.getElementById('cfg-translation-model');
      const model =
        modelSel?.selectedOptions?.[0]?.textContent?.trim() ||
        state.currentSettings?.translation_model ||
        'OpenRouter';
      badge.textContent = 'Active: OpenRouter · ' + model.replace(/^\d+\.\s*/, '').slice(0, 48);
    }
  }
}

export async function refreshTranslationStatus() {
  const el = document.getElementById('translation-local-status');
  if (!el) return;
  try {
    const r = await fetch('/api/translation-status');
    const data = await r.json();
    const lines = Object.entries(data.pairs || {}).map(([name, ok]) => (ok ? '✓ ' : '✗ ') + name);
    if (data.backend === 'openrouter') {
      el.innerHTML =
        '<span style="color:var(--yellow)">Switch off “Local model” to use OpenRouter</span>';
    } else if (data.ready) {
      el.innerHTML = '<span style="color:var(--green)">Ready — ' + lines.join(', ') + '</span>';
    } else {
      el.innerHTML =
        '<span style="color:var(--yellow)">Models missing: ' +
        lines.join(', ') +
        '. Click Download models (~600 MB).</span>';
    }
  } catch (_) {
    el.textContent = 'Could not check translation models';
  }
}

export async function downloadTranslationModels() {
  const btn = document.getElementById('btn-download-translate');
  if (!btn || btn.classList.contains('loading')) return;
  btn.classList.add('loading');
  btn.textContent = 'Downloading...';
  try {
    const r = await fetch('/api/download-translation-models', { method: 'POST' });
    const data = await r.json();
    if (!r.ok) throw new Error(data.error || 'download failed');
    showToast('Translation models installed');
    await refreshTranslationStatus();
  } catch (e) {
    showToast('Download failed: ' + e.message);
  }
  btn.classList.remove('loading');
  btn.textContent = 'Download models';
}

export async function testLocalTranslation() {
  const btn = document.getElementById('btn-test-translate');
  if (!btn) return;
  btn.textContent = '...';
  btn.className = 'sp-test-btn testing';
  try {
    const r = await fetch('/api/translate', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ text: 'привет', from: 'ru', to: 'en' }),
    });
    const data = await r.json();
    if (data.error) {
      btn.textContent = 'Fail';
      btn.className = 'sp-test-btn fail';
      showToast(data.error);
    } else {
      btn.textContent = '✓ ' + (data.translation || '').slice(0, 20);
      btn.className = 'sp-test-btn ok';
      showToast('Translation: ' + (data.translation || '(empty)'));
    }
  } catch (_) {
    btn.textContent = 'Error';
    btn.className = 'sp-test-btn fail';
  }
  setTimeout(() => {
    btn.textContent = 'Test';
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
      (m.free ? 'Free · ' : '') +
      'Context ' +
      m.context_label +
      (m.description ? ' · ' + m.description : '');
  } else if (id) {
    meta.textContent = id;
  } else {
    meta.textContent = 'Select a model (free + low latency recommended for live calls)';
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
      opt.textContent = current || '(no models — check connection)';
      sel.appendChild(opt);
    } else {
      state.translationModelsCache.forEach((m, i) => {
        const opt = document.createElement('option');
        opt.value = m.id;
        const rank = i + 1;
        opt.textContent = rank + '. ' + m.name + (m.free ? ' · free' : '') + ' · ' + m.context_label;
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
      opt.textContent = pick + ' (saved)';
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
      opt.textContent = current + ' (offline)';
      sel.appendChild(opt);
      sel.value = current;
    }
  }
  if (btn) btn.classList.remove('loading');
  updateTranslationEngineUI();
}

export function initTranslationListeners() {
  document.getElementById('cfg-use-local-model')?.addEventListener('change', () => {
    updateTranslationEngineUI();
    if (!isLocalTranslation()) {
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
