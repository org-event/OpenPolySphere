export function getTheme() {
  return localStorage.getItem('translator-theme') || 'dark';
}

export function applyTheme(t) {
  document.documentElement.setAttribute('data-theme', t);
  const btn = document.getElementById('theme-btn');
  if (btn) btn.textContent = t === 'dark' ? '\u2600' : '\u263E';
}

export function toggleTheme() {
  const t = getTheme() === 'dark' ? 'light' : 'dark';
  localStorage.setItem('translator-theme', t);
  applyTheme(t);
}

export function initTheme() {
  applyTheme(getTheme());
}
