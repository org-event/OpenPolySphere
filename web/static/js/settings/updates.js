import { state } from '../core/state.js';
import { t } from '../core/i18n.js';
import { showToast } from '../core/toast.js';

export async function showAppVersion() {
  const el = document.getElementById('app-version');
  if (!el) return;
  try {
    const r = await fetch('/api/app-info');
    const d = await r.json();
    el.textContent = `v${d.version}`;
    el.title = `${d.os} / ${d.arch}`;
  } catch {
    el.textContent = '';
  }
}

function setUpdateHint(text, color) {
  const hint = document.getElementById('update-hint');
  if (!hint) return;
  hint.textContent = text || '';
  if (color) hint.style.color = color;
}

export async function checkForUpdates() {
  const btn = document.getElementById('btn-check-updates');
  if (state.currentSettings.check_updates === false) {
    setUpdateHint(t('settings.updatesDisabled'), 'var(--text3)');
    return;
  }
  if (btn) btn.disabled = true;
  setUpdateHint(t('settings.updatesChecking'), 'var(--text3)');
  try {
    const r = await fetch('/api/update/check', { method: 'POST' });
    const d = await r.json();
    if (d.status === 'disabled') {
      setUpdateHint(t('settings.updatesDisabled'), 'var(--text3)');
      return;
    }
    if (d.status === 'error') {
      setUpdateHint(d.message || t('settings.updatesError'), 'var(--red)');
      showToast(d.message || t('settings.updatesError'));
      return;
    }
    if (!d.update_available) {
      setUpdateHint(t('settings.updatesCurrent', { version: d.current }), 'var(--green)');
      showToast(t('settings.updatesCurrent', { version: d.current }));
      return;
    }
    setUpdateHint(
      t('settings.updatesFound', { version: d.latest }),
      'var(--blue)',
    );
    showToast(t('settings.updatesDownloading', { version: d.latest }));
    const apply = await fetch('/api/update/apply', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ download_url: d.download_url }),
    });
    const result = await apply.json();
    if (result.status === 'ok' && result.relaunch) {
      setUpdateHint(result.message, 'var(--green)');
      showToast(result.message);
    } else if (result.status === 'manual') {
      setUpdateHint(result.message, 'var(--text2)');
      if (result.url) window.open(result.url, '_blank');
    } else if (result.status === 'error') {
      setUpdateHint(result.message, 'var(--red)');
      showToast(result.message);
    }
  } catch {
    setUpdateHint(t('settings.updatesError'), 'var(--red)');
    showToast(t('settings.updatesError'));
  } finally {
    if (btn) btn.disabled = false;
  }
}
