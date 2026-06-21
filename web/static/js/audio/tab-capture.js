import { state } from '../core/state.js';
import { showToast } from '../core/toast.js';
import { processLine } from '../chat/messages.js';

let tabCaptureActive = false;
let tabStream = null;
let tabRecorder = null;
let tabDgSocket = null;

const DG_LANG_MAP = { pt: 'pt-BR', no: 'nb' };

function dgLang(code) {
  return DG_LANG_MAP[code] || code;
}

export async function toggleTabCapture() {
  if (tabCaptureActive) {
    stopTabCapture();
    return;
  }
  const key = state.currentSettings.deepgram_api_key;
  if (!key) {
    showToast('Set Deepgram API key in Settings first');
    return;
  }

  try {
    tabStream = await navigator.mediaDevices.getDisplayMedia({ audio: true, video: true });
    tabStream.getVideoTracks().forEach((t) => t.stop());
    if (tabStream.getAudioTracks().length === 0) {
      showToast('No audio in selected source — pick a browser tab');
      tabStream = null;
      return;
    }
  } catch (_) {
    showToast('Tab capture cancelled');
    return;
  }

  const lang = dgLang(state.currentSettings.their_language || 'en');
  const dgModel = state.currentSettings.deepgram_model || 'nova-3';
  const url =
    'wss://api.deepgram.com/v1/listen?model=' +
    encodeURIComponent(dgModel) +
    '&language=' +
    lang +
    '&interim_results=true&endpointing=' +
    (state.currentSettings.endpointing_ms || 300);
  tabDgSocket = new WebSocket(url, ['token', key]);

  tabDgSocket.onopen = () => {
    tabRecorder = new MediaRecorder(tabStream, { mimeType: 'audio/webm;codecs=opus' });
    tabRecorder.ondataavailable = (e) => {
      if (e.data.size > 0 && tabDgSocket && tabDgSocket.readyState === WebSocket.OPEN) {
        tabDgSocket.send(e.data);
      }
    };
    tabRecorder.start(250);
    showToast('Tab Capture ON');
  };

  tabDgSocket.onmessage = async (e) => {
    try {
      const msg = JSON.parse(e.data);
      if (msg.type !== 'Results') return;
      const alt = msg.channel?.alternatives?.[0];
      if (!alt || !alt.transcript) return;
      const text = alt.transcript.trim();
      if (!text) return;

      if (msg.is_final) {
        const t0 = performance.now();
        processLine('\uD83C\uDFA4 [incoming] ' + text);
        try {
          const resp = await fetch('/api/translate', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
              text,
              from: state.currentSettings.their_language || 'en',
              to: state.currentSettings.my_language || 'ru',
            }),
          });
          const result = await resp.json();
          const trlMs = Math.round(performance.now() - t0);
          if (result.error) console.warn('[TAB] translate error:', result.error);
          processLine('\uD83C\uDF10 [incoming] ' + result.translation);
          processLine('\u23F1  stt=0ms trl=' + trlMs + 'ms tts=0ms');
        } catch (err) {
          console.error('[TAB] translate fetch failed:', err);
        }
      }
    } catch (err) {
      console.warn('Tab STT parse error:', err);
    }
  };

  tabDgSocket.onerror = () => showToast('Deepgram connection error');
  tabDgSocket.onclose = () => {
    if (tabCaptureActive) stopTabCapture();
  };

  tabStream.getAudioTracks()[0].onended = () => stopTabCapture();

  tabCaptureActive = true;
  document.getElementById('btn-tab-capture')?.classList.add('on');
}

function stopTabCapture() {
  if (tabRecorder && tabRecorder.state !== 'inactive') tabRecorder.stop();
  if (tabDgSocket && tabDgSocket.readyState === WebSocket.OPEN) {
    tabDgSocket.send(new Uint8Array(0));
    tabDgSocket.close();
  }
  if (tabStream) tabStream.getTracks().forEach((t) => t.stop());
  tabRecorder = null;
  tabDgSocket = null;
  tabStream = null;
  tabCaptureActive = false;
  document.getElementById('btn-tab-capture')?.classList.remove('on');
  showToast('Tab Capture OFF');
}
