import { state } from '../core/state.js';
import { showToast } from '../core/toast.js';

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
      if (polishOn) {
        badge.textContent = 'Active: Opus-MT + local polish ru↔en';
      } else {
        badge.textContent = 'Active: Local Opus-MT ru↔en';
      }
    } else if (backend === 'apple') {
      badge.textContent = 'Active: Apple Translation (system, on-device)';
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

function appleStatusHtml(apple) {
  if (!apple?.helper) {
    return '<span style="color:var(--yellow)">Apple Translation helper not built (macOS 14.4+ required)</span>';
  }
  if (!apple.available) {
    return '<span style="color:var(--yellow)">This language pair is not supported by Apple Translation on this Mac</span>';
  }
  if (apple.ready) {
    return '<span style="color:var(--green)">Ready — language packs installed (works offline)</span>';
  }
  if (apple.status === 'supported') {
    return '<span style="color:var(--yellow)">Supported but not downloaded — install language packs in System Settings → Language & Region or the Translate app</span>';
  }
  return '<span style="color:var(--yellow)">Checking language availability…</span>';
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
        el.innerHTML =
          '<span style="color:var(--yellow)">OpenRouter selected — configure API key below</span>';
      } else if (data.backend === 'apple') {
        el.innerHTML =
          '<span style="color:var(--yellow)">Apple Translation selected — see panel below</span>';
      } else if (data.ready) {
        let polishNote =
          data.polish_enabled && !data.polish_active && data.polish_disabled_reason
            ? ' · <span style="color:var(--yellow)">Polish: ' +
              data.polish_disabled_reason +
              '</span>'
            : data.polish_enabled && data.polish_active
              ? ' · polish: ' + (data.polish_model || 'Qwen2.5-0.5B')
              : '';
        el.innerHTML =
          '<span style="color:var(--green)">Ready — ' +
          lines.join(', ') +
          polishNote +
          '</span>';
      } else {
        el.innerHTML =
          '<span style="color:var(--yellow)">Models missing: ' +
          lines.join(', ') +
          '. Click Download models (~600 MB).</span>';
      }
    }
    if (appleEl) {
      appleEl.innerHTML = appleStatusHtml(data.apple);
    }
  } catch {
    if (el) el.textContent = 'Could not check translation models';
    if (appleEl) appleEl.textContent = 'Could not check Apple Translation';
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
  btn.textContent = 'Opus-MT';
}

export async function downloadPolishModel() {
  const btn = document.getElementById('btn-download-polish');
  if (!btn || btn.classList.contains('loading')) return;
  btn.classList.add('loading');
  btn.textContent = 'Downloading...';
  try {
    const r = await fetch('/api/download-polish-model', { method: 'POST' });
    const data = await r.json();
    if (!r.ok) throw new Error(data.error || 'download failed');
    showToast('Polish model installed — Save & restart engine');
    await refreshTranslationStatus();
  } catch (e) {
    showToast('Download failed: ' + e.message);
  }
  btn.classList.remove('loading');
  btn.textContent = 'Polish model';
}

export async function testLocalTranslation() {
  const btn =
    document.getElementById('btn-test-translate') ||
    document.getElementById('btn-test-translate-apple');
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
  } catch {
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
