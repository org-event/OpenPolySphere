import { state } from '../core/state.js';
import { showToast } from '../core/toast.js';
import { t } from '../core/i18n.js';
import { setStatus } from '../core/safe-dom.js';

export function initKeyMasking() {
  document.querySelectorAll('.sp-key').forEach((input) => {
    let realValue = input.value;
    const mask = (v) => (v.length > 4 ? '••••••••' + v.slice(-4) : v);

    input.addEventListener('focus', () => {
      input.value = realValue;
    });
    input.addEventListener('blur', () => {
      realValue = input.value;
      input.value = mask(realValue);
    });
    input.addEventListener('input', () => {
      realValue = input.value;
    });

    input._getRealValue = () => realValue;
    input._setRealValue = (v) => {
      realValue = v;
      input.value = mask(v);
    };
  });
}

export async function testKey(provider) {
  const inputId = provider === 'deepgram' ? 'cfg-deepgram' : 'cfg-openrouter';
  const btnId = provider === 'deepgram' ? 'test-deepgram' : 'test-openrouter';
  const el = document.getElementById(inputId);
  const key = (el._getRealValue ? el._getRealValue() : el.value).trim();
  const btn = document.getElementById(btnId);

  if (!key) {
    btn.textContent = t('settings.keyTest.empty');
    btn.className = 'sp-test-btn fail';
    return;
  }

  btn.textContent = t('settings.keyTest.testing');
  btn.className = 'sp-test-btn testing';

  try {
    const body = { provider, key };
    if (provider === 'openrouter') {
      body.model =
        document.getElementById('cfg-translation-model')?.value || state.currentSettings.translation_model;
    }
    const r = await fetch('/api/test-key', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    });
    const data = await r.json();
    if (data.valid) {
      btn.textContent = t('settings.keyTest.valid');
      btn.className = 'sp-test-btn ok';
    } else if (data.rate_limited) {
      btn.textContent = t('settings.keyTest.rateLimited');
      btn.className = 'sp-test-btn fail';
      const hint = data.retry_after
        ? t('settings.rateLimitRetry', { seconds: data.retry_after })
        : '';
      showToast(hint + (data.error || t('settings.rateLimitedUpstream')));
      const meta = document.getElementById('translation-model-meta');
      if (meta) {
        const msg = t('settings.rateLimitMeta', {
          prefix: data.provider ? data.provider + ': ' : '',
          error: data.error || t('settings.rateLimitedUpstream'),
          wait: data.retry_after
            ? t('settings.rateLimitWait', { seconds: data.retry_after })
            : '',
        });
        setStatus(meta, 'var(--yellow)', msg);
      }
    } else {
      btn.textContent = t('settings.keyTest.invalid');
      btn.className = 'sp-test-btn fail';
      if (data.error) showToast(data.error);
    }
  } catch {
    btn.textContent = t('settings.keyTest.error');
    btn.className = 'sp-test-btn fail';
  }

  setTimeout(() => {
    btn.textContent = t('settings.test');
    btn.className = 'sp-test-btn';
  }, 4000);
}
