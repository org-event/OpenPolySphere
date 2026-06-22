import { state, MAX_LOGS } from '../core/state.js';

export function isLogLine(line) {
  return (
    line.includes('\u26A0') ||
    line.startsWith('⚠') ||
    /Translation failed|status code \d{3}|rate-limited|Rate limited|provider:/i.test(line)
  );
}

function parseLogLine(line) {
  const tsMatch = line.match(/^\[([^\]]+)\]\s*(.+)$/);
  const ts = tsMatch ? tsMatch[1] : new Date().toISOString();
  const msg = tsMatch ? tsMatch[2] : line;
  const level = /failed|error|status code [45]\d{2}/i.test(msg) ? 'error' : 'warn';
  const text = msg.replace(/^⚠️?\s*/, '').trim();
  return { ts, level, text };
}

function formatLogTime(ts) {
  try {
    return new Date(ts).toLocaleTimeString([], {
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
    });
  } catch {
    return ts.length > 19 ? ts.slice(11, 19) : ts;
  }
}

export function appendLog(line) {
  const entry = parseLogLine(line);
  state.logEntries.push(entry);
  if (state.logEntries.length > MAX_LOGS) {
    state.logEntries.shift();
    document.getElementById('log-list')?.querySelector('.lp-entry')?.remove();
  }

  const empty = document.getElementById('log-empty');
  if (empty) empty.style.display = 'none';

  const el = document.createElement('div');
  el.className = 'lp-entry lp-' + entry.level;
  const time = document.createElement('span');
  time.className = 'lp-time';
  time.textContent = formatLogTime(entry.ts);
  const body = document.createElement('span');
  body.className = 'lp-msg';
  body.textContent = entry.text;
  el.appendChild(time);
  el.appendChild(body);
  document.getElementById('log-list')?.appendChild(el);

  const lp = document.getElementById('lp');
  if (!lp?.classList.contains('open')) {
    state.unreadLogs++;
    updateLogBadge();
  } else {
    const list = document.getElementById('log-list');
    list.scrollTop = list.scrollHeight;
  }
}

export function updateLogBadge() {
  const badge = document.getElementById('log-badge');
  const btn = document.getElementById('btn-logs');
  if (!badge) return;
  if (state.unreadLogs > 0) {
    badge.textContent = state.unreadLogs > 99 ? '99+' : String(state.unreadLogs);
    badge.classList.remove('hidden');
    btn?.classList.add('has-logs');
  } else {
    badge.classList.add('hidden');
    btn?.classList.remove('has-logs');
  }
}

function closeSettingsPanel() {
  document.getElementById('sp-backdrop')?.classList.remove('open');
  document.getElementById('sp')?.classList.remove('open');
}

export function openLogs() {
  closeSettingsPanel();
  document.getElementById('lp-backdrop')?.classList.add('open');
  document.getElementById('lp')?.classList.add('open');
  state.unreadLogs = 0;
  updateLogBadge();
  const list = document.getElementById('log-list');
  if (list) list.scrollTop = list.scrollHeight;
}

export function closeLogs() {
  document.getElementById('lp-backdrop')?.classList.remove('open');
  document.getElementById('lp')?.classList.remove('open');
}

export function clearLogs() {
  state.logEntries = [];
  document.querySelectorAll('#log-list .lp-entry').forEach((el) => el.remove());
  const empty = document.getElementById('log-empty');
  if (empty) empty.style.display = '';
  state.unreadLogs = 0;
  updateLogBadge();
}
