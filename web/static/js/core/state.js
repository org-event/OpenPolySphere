/** Shared application state (single source of truth). */

export const state = {
  stats: { stt: [], trl: [], tts: [], lat: [], count: 0 },
  muteState: { outgoing: false, incoming: true },
  pending: { direction: null, transcript: null, translation: null },
  lastRenderedDirection: null,
  lastMsgEl: null,
  lastMsgTime: 0,
  sessionStart: Date.now(),
  compactMode: false,
  bookmarkFilterOn: false,
  allMessages: [],
  currentSettings: {},
  engineRunning: false,
  engineBusy: false,
  timerPaused: true,
  timerPausedAt: 0,
  timerOffset: 0,
  logEntries: [],
  unreadLogs: 0,
  downloadingLangs: new Set(),
  translationModelsCache: [],
  allVoices: {},
  levelPollTimer: null,
  levelMonitoring: false,
  evtSource: null,
};

export const MAX_LOGS = 200;
export const LANGS_NO_TTS = [];

export const LANG_NAMES = {
  ar: 'Arabic', ca: 'Catalan', cs: 'Czech', da: 'Danish', de: 'German', el: 'Greek',
  en: 'English', es: 'Spanish', fa: 'Persian', fi: 'Finnish', fr: 'French',
  hi: 'Hindi', hu: 'Hungarian', id: 'Indonesian', it: 'Italian', ja: 'Japanese',
  ko: 'Korean', lv: 'Latvian', nl: 'Dutch', no: 'Norwegian', pl: 'Polish',
  pt: 'Portuguese', ro: 'Romanian', ru: 'Russian', sv: 'Swedish', tr: 'Turkish',
  uk: 'Ukrainian', vi: 'Vietnamese', zh: 'Chinese',
};
