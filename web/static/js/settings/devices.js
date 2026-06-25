import { state } from '../core/state.js';
import { t } from '../core/i18n.js';
import { startLevelMonitoring, bindLevelMeterDeviceChange } from '../audio/levels.js';

/** Show Linux virtual-sink setup hint; generic level-meter text off macOS. */
export function applyPlatformAudioHints(settings = state.currentSettings) {
  const hostOs = settings?._host_os || '';
  const linuxHint = document.getElementById('linux-virtual-audio-hint');
  if (linuxHint) {
    linuxHint.hidden = hostOs !== 'linux';
  }
  const levelHint = document.getElementById('audio-level-hint');
  if (levelHint) {
    const key = hostOs === 'linux' ? 'settings.audioLevelHintLinux' : 'settings.audioLevelHint';
    levelHint.textContent = t(key);
  }
}

export async function loadDevices() {
  try {
    const r = await fetch('/api/devices');
    const data = await r.json();
    const inputDevs = data.input || [];
    const outputDevs = data.output || [];

    function fillSelect(id, devices, current) {
      const sel = document.getElementById(id);
      sel.innerHTML = '';
      const def = document.createElement('option');
      def.value = 'default';
      def.textContent = t('settings.deviceDefault');
      sel.appendChild(def);
      devices.forEach((d) => {
        const opt = document.createElement('option');
        opt.value = d;
        opt.textContent = d;
        sel.appendChild(opt);
      });
      if (current && current !== 'default') {
        if (!devices.includes(current)) {
          const opt = document.createElement('option');
          opt.value = current;
          opt.textContent = t('settings.deviceSaved', { name: current });
          sel.appendChild(opt);
        }
        sel.value = current;
      }
    }

    fillSelect('cfg-mic', inputDevs, state.currentSettings.mic_device);
    fillSelect('cfg-speaker', outputDevs, state.currentSettings.speaker_device);
    fillSelect('cfg-meet-in', inputDevs, state.currentSettings.meet_input_device || 'default');
    fillSelect('cfg-meet-out', outputDevs, state.currentSettings.meet_output_device || 'default');
    bindLevelMeterDeviceChange();
    if (document.getElementById('sp')?.classList.contains('open')) {
      startLevelMonitoring();
    }
  } catch (e) {
    console.error('Failed to load devices', e);
  }
}

export function initDeviceListeners() {
  document.getElementById('cfg-endpointing')?.addEventListener('input', function () {
    document.getElementById('endpointing-val').textContent = this.value + 'ms';
  });
}
