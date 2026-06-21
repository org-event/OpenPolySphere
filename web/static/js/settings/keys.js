import { state } from '../core/state.js';
import { showToast } from '../core/toast.js';

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
    btn.textContent = 'Empty';
    btn.className = 'sp-test-btn fail';
    return;
  }

  btn.textContent = '...';
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
      btn.textContent = '\u2713 Valid';
      btn.className = 'sp-test-btn ok';
    } else if (data.rate_limited) {
      btn.textContent = '429 Limit';
      btn.className = 'sp-test-btn fail';
      const hint = data.retry_after ? 'Retry in ~' + data.retry_after + 's. ' : '';
      showToast(hint + (data.error || 'Model rate-limited upstream'));
      const meta = document.getElementById('translation-model-meta');
      if (meta) {
        meta.innerHTML =
          '<span style="color:var(--yellow)">' +
          (data.provider ? data.provider + ': ' : '') +
          (data.error || 'Rate limited') +
          (data.retry_after ? ' — wait ' + data.retry_after + 's or switch model' : '') +
          '</span>';
      }
    } else {
      btn.textContent = '\u2717 Invalid';
      btn.className = 'sp-test-btn fail';
      if (data.error) showToast(data.error);
    }
  } catch (_) {
    btn.textContent = 'Error';
    btn.className = 'sp-test-btn fail';
  }

  setTimeout(() => {
    btn.textContent = 'Test';
    btn.className = 'sp-test-btn';
  }, 4000);
}
