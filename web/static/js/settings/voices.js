import { state, LANGS_NO_TTS } from '../core/state.js';
import { showToast } from '../core/toast.js';
import { sleep, langName } from '../core/utils.js';
import { saveAndRestart } from '../engine/restart.js';

export async function loadVoices() {
  try {
    const r = await fetch('/api/voices');
    state.allVoices = await r.json();
    updateVoiceDropdowns();
  } catch (e) {
    console.error('Failed to load voices', e);
  }
}

export function updateVoiceDropdowns() {
  const theirLang = document.getElementById('cfg-their-lang').value;
  const myLang = document.getElementById('cfg-my-lang').value;

  fillVoiceSelect('cfg-voice-out', theirLang, state.currentSettings.tts_outgoing_voice);
  fillVoiceSelect('cfg-voice-in', myLang, state.currentSettings.tts_incoming_voice);

  document.getElementById('voice-label-in').textContent = langName(myLang) + ' Voice (I hear)';
  document.getElementById('voice-label-out').textContent = langName(theirLang) + ' Voice (they hear)';

  updateDlButton('in');
  updateDlButton('out');

  const hintOut = document.getElementById('voice-hint-out');
  const hintIn = document.getElementById('voice-hint-in');
  const voicesOut = state.allVoices[theirLang] || [];
  const voicesIn = state.allVoices[myLang] || [];
  const hasDownloadedOut = voicesOut.some((v) => v.downloaded);
  const hasDownloadedIn = voicesIn.some((v) => v.downloaded);

  if (!hasDownloadedOut) showDownloadPrompt(theirLang, 'voice-hint-out');
  else {
    hintOut.textContent = '';
    hintOut.style.color = '';
  }
  if (!hasDownloadedIn) showDownloadPrompt(myLang, 'voice-hint-in');
  else {
    hintIn.textContent = '';
    hintIn.style.color = '';
  }
}

function fillVoiceSelect(selId, lang, currentVal) {
  const sel = document.getElementById(selId);
  sel.innerHTML = '';
  const voices = state.allVoices[lang] || [];
  if (voices.length === 0) {
    const opt = document.createElement('option');
    opt.value = '';
    opt.textContent = 'No voices for ' + langName(lang);
    sel.appendChild(opt);
    return;
  }
  const downloaded = voices.filter((v) => v.downloaded);
  const available = voices.filter((v) => !v.downloaded);

  if (downloaded.length > 0) {
    const grp = document.createElement('optgroup');
    grp.label = 'Downloaded';
    downloaded.forEach((v) => {
      const opt = document.createElement('option');
      opt.value = v.name;
      opt.textContent = v.name.replace(/-/g, ' ');
      grp.appendChild(opt);
    });
    sel.appendChild(grp);
  }
  if (available.length > 0) {
    const grp = document.createElement('optgroup');
    grp.label = 'Available (' + available.length + ')';
    available.forEach((v) => {
      const opt = document.createElement('option');
      opt.value = v.name;
      opt.textContent = v.name.replace(/-/g, ' ') + ' \u2014 ' + v.size_mb + ' MB';
      grp.appendChild(opt);
    });
    sel.appendChild(grp);
  }
  if (currentVal && voices.some((v) => v.name === currentVal)) sel.value = currentVal;
}

export function isVoiceDownloaded(dir) {
  const sel = document.getElementById('cfg-voice-' + dir);
  const lang =
    dir === 'out'
      ? document.getElementById('cfg-their-lang').value
      : document.getElementById('cfg-my-lang').value;
  const voices = state.allVoices[lang] || [];
  const voice = voices.find((v) => v.name === sel.value);
  return voice ? voice.downloaded : true;
}

function updateDlButton(dir) {
  const btn = document.getElementById('dl-voice-' + dir);
  const sel = document.getElementById('cfg-voice-' + dir);
  if (!sel.value || isVoiceDownloaded(dir)) {
    btn.classList.add('hidden');
  } else {
    btn.classList.remove('hidden');
  }
}

export async function showDownloadPrompt(lang, hintId) {
  const hint = document.getElementById(hintId);
  if (LANGS_NO_TTS.includes(lang)) {
    hint.innerHTML =
      '<span style="color:var(--yellow)">No TTS voice exists for ' +
      langName(lang) +
      '. Translation will work but without audio output.</span>';
    return;
  }
  hint.innerHTML =
    '<button class="sp-download-btn" onclick="downloadDefaultVoice(\'' +
    lang +
    "', '" +
    hintId +
    "')\">Download " +
    langName(lang) +
    ' default voice &amp; restart engine</button>';
  hint.style.color = '';
}

export async function downloadDefaultVoice(lang, hintId) {
  if (state.downloadingLangs.has(lang)) return;
  state.downloadingLangs.add(lang);
  const hint = document.getElementById(hintId);
  hint.innerHTML =
    '<div class="sp-progress"><div class="sp-progress-bar" id="pb-' +
    lang +
    '"></div><div class="sp-progress-text" id="pt-' +
    lang +
    '">Connecting...</div></div>';

  try {
    const resp = await fetch('/api/download-voice', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ lang }),
    });
    const reader = resp.body.getReader();
    const decoder = new TextDecoder();
    let buf = '';

    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      buf += decoder.decode(value, { stream: true });
      const lines = buf.split('\n');
      buf = lines.pop() || '';
      for (const line of lines) {
        if (!line.startsWith('data: ')) continue;
        const data = JSON.parse(line.slice(6));
        if (data.progress !== undefined) {
          const bar = document.getElementById('pb-' + lang);
          const txt = document.getElementById('pt-' + lang);
          if (bar) bar.style.width = data.progress + '%';
          if (txt) txt.textContent = data.progress + '% \u2014 ' + data.mb_done + '/' + data.mb_total + ' MB';
        }
        if (data.done) {
          hint.innerHTML = '<span style="color:var(--green)">' + langName(lang) + ' voice installed!</span>';
          showToast(langName(lang) + ' voice downloaded');
          await loadVoices();
          await saveAndRestart();
        }
        if (data.error) {
          hint.innerHTML = '<span style="color:var(--red)">' + data.error + '</span>';
        }
      }
    }
  } catch (e) {
    hint.innerHTML = '<span style="color:var(--red)">Download failed: ' + e.message + '</span>';
  }
  state.downloadingLangs.delete(lang);
}

export async function previewVoice(dir) {
  const btn = document.getElementById('preview-' + dir);
  const voiceSelect = document.getElementById('cfg-voice-' + dir);
  const voice = voiceSelect.value;
  if (!voice) {
    showToast('No voice selected');
    return;
  }
  if (!isVoiceDownloaded(dir)) {
    showToast('Download the voice first');
    return;
  }
  const lang =
    dir === 'out'
      ? document.getElementById('cfg-their-lang').value
      : document.getElementById('cfg-my-lang').value;
  btn.classList.add('loading');
  try {
    const r = await fetch('/api/tts-preview', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ lang, voice }),
    });
    const data = await r.json();
    if (data.status && data.status.startsWith('ok')) {
      showToast('Playing preview...');
      await sleep(3500);
    } else {
      showToast('Preview failed: ' + (data.status || 'engine not running'));
    }
  } catch {
    showToast('Preview error: engine not running');
  }
  btn.classList.remove('loading');
}

export async function downloadSelectedVoice(dir) {
  const sel = document.getElementById('cfg-voice-' + dir);
  const btn = document.getElementById('dl-voice-' + dir);
  const hint = document.getElementById('voice-hint-' + dir);
  const voice = sel.value;
  const lang =
    dir === 'out'
      ? document.getElementById('cfg-their-lang').value
      : document.getElementById('cfg-my-lang').value;

  if (!voice) return;
  btn.classList.add('loading');
  hint.innerHTML =
    '<div class="sp-progress"><div class="sp-progress-bar" id="pb-dl-' +
    dir +
    '"></div><div class="sp-progress-text" id="pt-dl-' +
    dir +
    '">Connecting...</div></div>';

  try {
    const resp = await fetch('/api/download-voice', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ lang, voice }),
    });
    const reader = resp.body.getReader();
    const decoder = new TextDecoder();
    let buf = '';

    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      buf += decoder.decode(value, { stream: true });
      const lines = buf.split('\n');
      buf = lines.pop() || '';
      for (const line of lines) {
        if (!line.startsWith('data: ')) continue;
        const data = JSON.parse(line.slice(6));
        if (data.progress !== undefined) {
          const bar = document.getElementById('pb-dl-' + dir);
          const txt = document.getElementById('pt-dl-' + dir);
          if (bar) bar.style.width = data.progress + '%';
          if (txt) txt.textContent = data.progress + '% \u2014 ' + data.mb_done + '/' + data.mb_total + ' MB';
        }
        if (data.done) {
          hint.innerHTML = '<span style="color:var(--green)">Downloaded!</span>';
          setTimeout(() => {
            hint.textContent = '';
          }, 3000);
          await loadVoices();
          sel.value = voice;
          updateDlButton(dir);
        }
        if (data.error) {
          hint.innerHTML = '<span style="color:var(--red)">' + data.error + '</span>';
        }
      }
    }
  } catch (e) {
    hint.innerHTML = '<span style="color:var(--red)">Download failed: ' + e.message + '</span>';
  }
  btn.classList.remove('loading');
}

export function initVoiceListeners() {
  document.getElementById('cfg-my-lang')?.addEventListener('change', updateVoiceDropdowns);
  document.getElementById('cfg-their-lang')?.addEventListener('change', updateVoiceDropdowns);
  document.getElementById('cfg-my-lang')?.addEventListener('change', () => {
    if (typeof applyTooltips === 'function') applyTooltips();
  });
  document.getElementById('cfg-voice-in')?.addEventListener('change', () => updateDlButton('in'));
  document.getElementById('cfg-voice-out')?.addEventListener('change', () => updateDlButton('out'));
}
