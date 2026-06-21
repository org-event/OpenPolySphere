import { dom } from './dom.js';

let toastTimeout = null;

export function showToast(text) {
  dom.toastEl.textContent = text;
  dom.toastEl.classList.add('show');
  clearTimeout(toastTimeout);
  toastTimeout = setTimeout(() => dom.toastEl.classList.remove('show'), 1500);
}
