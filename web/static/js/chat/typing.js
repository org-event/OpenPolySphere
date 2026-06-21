import { dom } from '../core/dom.js';
import { state } from '../core/state.js';
import { scrollBottom } from '../core/utils.js';

let typingTimeout = null;

export function showTyping() {
  dom.typingEl.classList.add('visible');
  scrollBottom();
  clearTimeout(typingTimeout);
  typingTimeout = setTimeout(hideTyping, 15000);
}

export function hideTyping() {
  dom.typingEl.classList.remove('visible');
  clearTimeout(typingTimeout);
  typingTimeout = null;
}

export function typewrite(el, text) {
  let i = 0;
  el.textContent = '';
  function tick() {
    if (i < text.length) {
      el.textContent += text[i++];
      scrollBottom();
      setTimeout(tick, 18);
    }
  }
  tick();
}

export function maybeAddTimeSep() {
  const now = Date.now();
  if (state.lastMsgTime && now - state.lastMsgTime > 60000) {
    const gap = Math.round((now - state.lastMsgTime) / 1000);
    const sep = document.createElement('div');
    sep.className = 'time-sep';
    sep.textContent = gap < 120 ? gap + 's pause' : Math.round(gap / 60) + ' min pause';
    dom.chat.insertBefore(sep, dom.typingEl);
  }
  state.lastMsgTime = now;
}
