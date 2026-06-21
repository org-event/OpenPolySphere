import { dom } from '../core/dom.js';
import { state } from '../core/state.js';
import { avg, latencyClass, scrollBottom } from '../core/utils.js';

export function updateStats() {
  document.getElementById('avg-stt').textContent = avg(state.stats.stt);
  document.getElementById('avg-trl').textContent = avg(state.stats.trl);
  document.getElementById('avg-tts').textContent = avg(state.stats.tts);
  document.getElementById('avg-lat').textContent = avg(state.stats.lat);
  document.getElementById('total').textContent = state.stats.count;
}
