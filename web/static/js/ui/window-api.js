/** Global handlers for inline onclick attributes in index.html. */

import { toggleTheme } from '../core/theme.js';
import { toggleCompact, toggleBookmarkFilter, exportChat, clearAll } from '../chat/messages.js';
import { openLogs, closeLogs, clearLogs } from '../logs/panel.js';
import { toggleEngine, toggleMute } from '../engine/control.js';
import { saveAndRestart } from '../engine/restart.js';
import { toggleMonitor } from '../audio/monitor.js';
import { toggleTabCapture } from '../audio/tab-capture.js';
import { openSettings, closeSettings, toggleSection } from '../settings/panel.js';
import { downloadWhisperModel, requestAppleSpeechAuth } from '../settings/stt.js';
import {
  downloadTranslationModels,
  downloadPolishModel,
  testLocalTranslation,
  loadTranslationModels,
} from '../settings/translation.js';
import { testKey } from '../settings/keys.js';
import {
  previewVoice,
  downloadSelectedVoice,
  downloadDefaultVoice,
} from '../settings/voices.js';

const handlers = {
  toggleTheme,
  toggleCompact,
  toggleBookmarkFilter,
  exportChat,
  clearAll,
  openLogs,
  closeLogs,
  clearLogs,
  toggleEngine,
  toggleMute,
  saveAndRestart,
  toggleMonitor,
  toggleTabCapture,
  openSettings,
  closeSettings,
  toggleSection,
  downloadWhisperModel,
  requestAppleSpeechAuth,
  downloadTranslationModels,
  downloadPolishModel,
  testLocalTranslation,
  loadTranslationModels,
  testKey,
  previewVoice,
  downloadSelectedVoice,
  downloadDefaultVoice,
};

export function exposeWindowApi() {
  Object.assign(window, handlers);
}
