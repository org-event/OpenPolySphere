import { dom } from '../core/dom.js';
import { state } from '../core/state.js';
import { processLine } from './messages.js';

export function connectSSE(replay) {
  if (state.evtSource) state.evtSource.close();
  const url = replay ? '/stream?replay=1' : '/stream';
  state.evtSource = new EventSource(url);
  state.evtSource.onmessage = (e) => processLine(e.data);
  state.evtSource.onerror = () => {
    dom.statusEl.textContent = 'Disconnected';
    dom.statusEl.className = 'disconnected';
  };
  state.evtSource.onopen = () => {
    dom.statusEl.textContent = 'Connected';
    dom.statusEl.className = '';
  };
}
