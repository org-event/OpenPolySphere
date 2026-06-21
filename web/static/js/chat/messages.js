import { dom } from '../core/dom.js';
import { state } from '../core/state.js';
import { showToast } from '../core/toast.js';
import { latencyClass, scrollBottom } from '../core/utils.js';
import { updateStats } from './stats.js';
import { hideTyping, maybeAddTimeSep, showTyping, typewrite } from './typing.js';
import { appendLog, isLogLine } from '../logs/panel.js';

export function copyBubble(text) {
  navigator.clipboard.writeText(text).then(() => showToast('Copied!'));
}

export function flushPending() {
  if (!state.pending.direction || (!state.pending.translation && !state.pending.transcript)) return;
  hideTyping();
  maybeAddTimeSep();

  if (state.pending.direction !== state.lastRenderedDirection) {
    const label = document.createElement('div');
    label.className = 'direction-label ' + state.pending.direction;
    const myL = (state.currentSettings.my_language || 'RU').toUpperCase();
    const theirL = (state.currentSettings.their_language || 'EN').toUpperCase();
    label.textContent =
      state.pending.direction === 'outgoing'
        ? 'You (' + myL + ' \u2192 ' + theirL + ')'
        : 'Them (' + theirL + ' \u2192 ' + myL + ')';
    dom.chat.insertBefore(label, dom.typingEl);
    state.lastRenderedDirection = state.pending.direction;
  }

  const msg = document.createElement('div');
  msg.className = 'msg ' + state.pending.direction;
  const star = document.createElement('span');
  star.className = 'star';
  star.textContent = '\u2606';
  msg.appendChild(star);
  const bubble = document.createElement('div');
  bubble.className = 'bubble';
  msg.appendChild(bubble);
  const translationText = state.pending.translation || '';
  const transcriptText = state.pending.transcript;
  bubble.onclick = () => copyBubble(translationText || transcriptText);
  if (transcriptText && translationText && transcriptText !== translationText) {
    const orig = document.createElement('div');
    orig.className = 'original';
    orig.textContent = transcriptText;
    msg.appendChild(orig);
  }
  dom.chat.insertBefore(msg, dom.typingEl);
  state.lastMsgEl = msg;

  const msgData = {
    el: msg,
    direction: state.pending.direction,
    transcript: transcriptText,
    translation: translationText || null,
    bookmarked: false,
  };
  state.allMessages.push(msgData);
  star.onclick = (e) => {
    e.stopPropagation();
    msgData.bookmarked = !msgData.bookmarked;
    star.textContent = msgData.bookmarked ? '\u2605' : '\u2606';
    star.classList.toggle('on', msgData.bookmarked);
    msg.classList.toggle('bookmarked', msgData.bookmarked);
  };
  if (translationText) {
    typewrite(bubble, translationText);
  } else if (transcriptText) {
    bubble.classList.add('transcript-only');
    bubble.textContent = transcriptText;
  }
  state.stats.count++;
  updateStats();
  scrollBottom();
  state.pending = { direction: null, transcript: null, translation: null };
}

export function updateMessageTranslation(direction, translation) {
  hideTyping();
  const last = state.allMessages[state.allMessages.length - 1];
  if (!last || last.direction !== direction) return;
  last.translation = translation;
  const bubble = last.el.querySelector('.bubble');
  if (!bubble) return;
  if (last.transcript && last.transcript !== translation) {
    if (!last.el.querySelector('.original')) {
      const orig = document.createElement('div');
      orig.className = 'original';
      orig.textContent = last.transcript;
      last.el.appendChild(orig);
    }
  }
  bubble.classList.remove('transcript-only');
  bubble.textContent = '';
  bubble.onclick = () => copyBubble(translation);
  typewrite(bubble, translation);
  scrollBottom();
}

export function processLine(line) {
  if (isLogLine(line)) {
    hideTyping();
    appendLog(line);
    showToast(line.replace(/^[^\]]*\]\s*/, '').replace(/^⚠️?\s*/, ''));
    return;
  }
  let m = line.match(/\uD83C\uDFA4 \[(outgoing|incoming)\] (.+)/);
  if (m) {
    flushPending();
    state.pending.direction = m[1];
    state.pending.transcript = m[2];
    state.pending.translation = '';
    flushPending();
    showTyping();
    return;
  }
  m = line.match(/\uD83C\uDF10 \[(outgoing|incoming)\] (.+)/);
  if (m) {
    updateMessageTranslation(m[1], m[2]);
    return;
  }
  m = line.match(/\u23F1\s+stt=(\d+)ms\s+trl=(\d+)ms\s+tts=(\d+)ms/);
  if (m) {
    hideTyping();
    const stt = parseInt(m[1]);
    const trl = parseInt(m[2]);
    const tts = parseInt(m[3]);
    const total = stt + trl + tts;
    state.stats.stt.push(stt);
    state.stats.trl.push(trl);
    state.stats.tts.push(tts);
    state.stats.lat.push(total);
    updateStats();
    if (state.lastMsgEl) {
      const meta = document.createElement('div');
      meta.className = 'meta';
      meta.innerHTML =
        '<span class="' +
        latencyClass(stt) +
        '">stt ' +
        stt +
        'ms</span>' +
        '<span class="' +
        latencyClass(trl) +
        '">trl ' +
        trl +
        'ms</span>' +
        '<span class="' +
        latencyClass(tts) +
        '">tts ' +
        tts +
        'ms</span>' +
        '<span class="' +
        latencyClass(total) +
        '">= ' +
        total +
        'ms</span>';
      state.lastMsgEl.appendChild(meta);
      scrollBottom();
    }
  }
}

export function clearAll() {
  dom.chat.innerHTML = '';
  dom.chat.appendChild(dom.typingEl);
  hideTyping();
  state.stats = { stt: [], trl: [], tts: [], lat: [], count: 0 };
  state.lastRenderedDirection = null;
  state.lastMsgEl = null;
  state.lastMsgTime = 0;
  state.pending = { direction: null, transcript: null, translation: null };
  state.allMessages = [];
  state.bookmarkFilterOn = false;
  document.getElementById('btn-bookmarks')?.classList.remove('on');
  updateStats();
}

export function toggleCompact() {
  state.compactMode = !state.compactMode;
  dom.chat.classList.toggle('compact', state.compactMode);
  document.getElementById('btn-compact')?.classList.toggle('on', state.compactMode);
}

export function toggleBookmarkFilter() {
  state.bookmarkFilterOn = !state.bookmarkFilterOn;
  document.getElementById('btn-bookmarks')?.classList.toggle('on', state.bookmarkFilterOn);
  state.allMessages.forEach((m) => {
    m.el.style.display = state.bookmarkFilterOn && !m.bookmarked ? 'none' : '';
  });
  dom.chat.querySelectorAll('.direction-label, .time-sep').forEach((el) => {
    el.style.display = state.bookmarkFilterOn ? 'none' : '';
  });
  scrollBottom();
}

export function exportChat() {
  const lines = [];
  state.allMessages.forEach((m) => {
    const dir = m.direction === 'outgoing' ? 'YOU' : 'THEM';
    const bk = m.bookmarked ? ' *' : '';
    if (m.transcript) lines.push('[' + dir + '] ' + m.transcript);
    lines.push('[' + dir + '] >> ' + m.translation + bk);
    lines.push('');
  });
  const blob = new Blob([lines.join('\n')], { type: 'text/plain' });
  const a = document.createElement('a');
  a.href = URL.createObjectURL(blob);
  a.download = 'transcript-' + new Date().toISOString().slice(0, 16).replace(':', '-') + '.txt';
  a.click();
  showToast('Exported!');
}
