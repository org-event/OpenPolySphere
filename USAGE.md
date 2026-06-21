# Usage Guide

Open **http://127.0.0.1:5050** after starting the server:

```bash
cargo run --release -p translator
```

First-time model download:

```bash
cargo run --release -p translator -- setup
```

## First Launch

By default **local mode** works without API keys (Whisper STT + Opus-MT translation).

1. **Languages** — set "My Language" and "Their Language"
2. **Voice** — pick TTS voices; download more from the dropdown if needed
3. **Audio Devices** — mic, speakers, and virtual cable (TranslateTelega / BlackHole)
4. Click **Save & Restart Engine**, then **Start**

Optional cloud backends in Settings:

- **Deepgram** — cloud STT (disable "Local model" under STT)
- **OpenRouter** — cloud translation (disable "Local model" under Translation)

## Controls

| Button | What it does |
|--------|-------------|
| **Start / Stop** | Start or stop translation pipelines. Pill shows **Stopped** vs **Translating** |
| **Mic Out** | Mute/unmute your microphone (outgoing translation) |
| **Mic In** | Mute/unmute incoming audio (their speech) |
| **Tab Audio** | Capture audio from a browser tab instead of a virtual cable |
| **Monitor** | Play translated audio in the browser |
| **Compact** | Toggle compact chat mode |
| **Saved** | Filter to bookmarked messages |
| **History** | Past sessions and AI summaries |
| **Export** | Download current transcript as text |
| **Clear** | Clear messages from the current view |
| **Settings** | Settings panel |

## Live Chat

- **Blue bubbles** (right) — your speech, translated to their language
- **Purple bubbles** (left) — their speech, translated to your language
- **Italic text** — original (untranslated) text
- **Timing** — STT, translation, and TTS latency per message

## Voice Management

In Settings > Voice:

- **Downloaded** voices at the top, **Available** below
- Download button for catalog voices
- Play button to preview downloaded voices

## Call History

Click **History** — full transcript per session, optional OpenRouter summary, delete.

## Audio Setup for Calls

1. Virtual cable: [BlackHole](https://existential.audio/blackhole/) or TranslateTelega
2. Call app: virtual device as mic/speakers as needed
3. Use **Google Chrome** for the call app

## Keyboard Shortcuts

Hover buttons for tooltips (multiple UI languages).
