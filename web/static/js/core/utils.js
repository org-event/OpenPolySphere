import { dom } from './dom.js';
import { LANG_NAMES } from './state.js';

export function sleep(ms) {
  return new Promise((r) => setTimeout(r, ms));
}

export function scrollBottom() {
  dom.chat.scrollTop = dom.chat.scrollHeight;
}

export function latencyClass(ms) {
  return ms < 400 ? 'fast' : ms < 800 ? 'medium' : 'slow';
}

export function avg(arr) {
  if (!arr.length) return '-';
  return Math.round(arr.reduce((a, b) => a + b, 0) / arr.length) + 'ms';
}

export function langName(code) {
  return LANG_NAMES[code] || code;
}
