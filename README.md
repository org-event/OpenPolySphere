# Realtime Call Translator

[![README на русском](https://img.shields.io/badge/README-Russian-blue)](README.ru.md)

Real-time speech translator for video/voice calls. Translates both sides of the conversation live — you speak your language, the other person hears theirs, and vice versa.

**How it works:** Your mic audio goes through Speech-to-Text, gets translated by an LLM, then synthesized back to speech and routed into your call. The same happens in reverse for the other person's audio.

Supports **29 languages** with STT, translation, and TTS. Voice models from [Piper](https://github.com/rhasspy/piper) — download any language directly from the web UI.

![macOS](https://img.shields.io/badge/platform-macOS_14+-lightgrey)
![License](https://img.shields.io/badge/license-MIT-blue)
![GitHub stars](https://img.shields.io/github/stars/org-event/call-translator)

> Fork of [LetovKai/call-translator](https://github.com/LetovKai/call-translator) by Kai Letov.

> **Note:** macOS only (14+). Uses CoreAudio and cpal for audio capture. Windows/Linux support is not available yet — contributions welcome!

---

## Quick Start

```bash
git clone git@github.com:org-event/call-translator.git
cd call-translator
cargo run --release -p translator -- setup   # download models (first time)
cargo run --release -p translator            # start server
```

Open **http://127.0.0.1:5050** in **Google Chrome**.

Local mode (default): Whisper STT + Opus-MT translation — no API keys required. Cloud STT/translation optional via Settings.

---

## Architecture

Single Rust binary (`translator`): Axum web server on `:5050` + in-process audio engine (STT, translation, TTS).

```
Browser (app.js) ←SSE→ Axum ←→ audio-core Engine ←→ CoreAudio / models
```

---

## Requirements

| Dependency | Purpose | Install |
|---|---|---|
| macOS 14+ | CoreAudio for audio I/O | — |
| [Homebrew](https://brew.sh) | Package manager | see brew.sh |
| Rust | App + audio engine | `brew install rustup && rustup-init` |
| espeak-ng | TTS phonemization | `brew install espeak-ng` |
| ONNX Runtime | Model inference | `brew install onnxruntime` |
| [BlackHole](https://existential.audio/blackhole/) | Virtual audio routing | Manual download |
| Xcode CLT | C compiler | `xcode-select --install` |

**Optional API keys** (cloud STT/translation): [Deepgram](https://console.deepgram.com), [OpenRouter](https://openrouter.ai/keys)

---

## Manual Installation

If you prefer step-by-step setup:

### 1. System packages

```bash
xcode-select --install
brew install rustup espeak-ng onnxruntime
rustup-init -y --default-toolchain stable
source ~/.cargo/env
```

### 2. BlackHole audio driver

Download and install from [existential.audio/blackhole](https://existential.audio/blackhole/).

You need **both**:
- **BlackHole 16ch** — captures audio from your call app (Google Meet, Zoom, etc.)
- **BlackHole 2ch** — sends translated audio back to the call

Setup in your call app (Google Meet, Zoom, etc.):
1. Open the call in **Google Chrome** (not Safari)
2. Set **BlackHole 2ch** as the **microphone** in the call app
3. Set **BlackHole 16ch** as the **speakers** in the call app

> **Note:** Do NOT use a Multi-Output Device — it may cause audio issues. Set BlackHole devices directly in the call app settings.

### 3. Download voice models

TTS voices come from [Piper](https://github.com/rhasspy/piper). Run `cargo run --release -p translator -- setup` to download default voices, Whisper, and Opus-MT models. Additional voices can be downloaded from the web UI.

To download manually:

```bash
mkdir -p models/piper-en models/piper-ru

# English (default)
curl -sL https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/en/en_US/ryan/medium/en_US-ryan-medium.onnx \
  -o models/piper-en/en_US-ryan-medium.onnx
curl -sL https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/en/en_US/ryan/medium/en_US-ryan-medium.onnx.json \
  -o models/piper-en/en_US-ryan-medium.onnx.json

# Russian (default)
curl -sL https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/ru/ru_RU/denis/medium/ru_RU-denis-medium.onnx \
  -o models/piper-ru/ru_RU-denis-medium.onnx
curl -sL https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/ru/ru_RU/denis/medium/ru_RU-denis-medium.onnx.json \
  -o models/piper-ru/ru_RU-denis-medium.onnx.json
```

Browse all available voices at [rhasspy.github.io/piper-samples](https://rhasspy.github.io/piper-samples/).

### 4. Environment variables

```bash
cp .env.example .env
```

Edit `.env`:

```
DEEPGRAM_API_KEY=your_key_here
GROQ_API_KEY=your_key_here
ORT_DYLIB_PATH=/opt/homebrew/lib/libonnxruntime.dylib
```

### 5. Build and run

```bash
cargo run --release -p translator -- setup   # first time: download models
cargo run --release -p translator            # start server
```

Open **http://127.0.0.1:5050** in Chrome.

---

## Web UI Features

- **Live transcript** — chat-style bubbles with original text and translation
- **29 languages** — switch language pair from Settings, download voices with one click
- **Voice selection** — multiple voices per language with preview playback
- **Audio monitor** — hear translations in your browser (Chrome only)
- **Start/Stop** — control the engine without restarting
- **Mute** — independently mute outgoing or incoming pipelines
- **Bookmarks** — star important phrases, filter to show only starred
- **Export** — download the full transcript as a text file
- **Compact/Full view** — toggle between detailed and compact transcript
- **Latency metrics** — per-phrase STT, translation, TTS, and total latency
- **Dark/Light theme** — toggle with persistence

---

## Supported Languages

| Language | STT | Translation | TTS |
|----------|-----|-------------|-----|
| Arabic | + | + | + |
| Catalan | + | + | + |
| Chinese | + | + | + |
| Czech | + | + | + |
| Danish | + | + | + |
| Dutch | + | + | + |
| English | + | + | + |
| Finnish | + | + | + |
| French | + | + | + |
| German | + | + | + |
| Greek | + | + | + |
| Hindi | + | + | + |
| Hungarian | + | + | + |
| Indonesian | + | + | + |
| Italian | + | + | + |
| Japanese | + | + | — |
| Korean | + | + | — |
| Latvian | + | + | + |
| Norwegian | + | + | + |
| Persian | + | + | + |
| Polish | + | + | + |
| Portuguese | + | + | + |
| Romanian | + | + | + |
| Russian | + | + | + |
| Spanish | + | + | + |
| Swedish | + | + | + |
| Turkish | + | + | + |
| Ukrainian | + | + | + |
| Vietnamese | + | + | + |

TTS requires downloading a Piper voice model for the language (one-click from the web UI). Japanese and Korean have STT and translation but no Piper TTS voice available.

---

## Troubleshooting

**"Engine not starting"**
- Press **Start** after the page loads (server runs idle until then)
- For local mode: models in `models/` — run `cargo run --release -p translator -- setup`
- Verify `ORT_DYLIB_PATH` points to your onnxruntime library
- Run `cargo build -p translator` to check for build errors

**"No audio from call"**
- Ensure BlackHole 16ch is set up in a Multi-Output Device
- Check that your call app uses BlackHole 2ch as its microphone

**"TTS not working"**
- Verify `espeak-ng` is installed: `espeak-ng --version`
- Check that voice model files exist in `models/piper-{lang}/`
- Download voices from Settings in the web UI

**"No sound in monitor"**
- Use Chrome — Safari does not support audio output routing required for monitor
- Check your system audio output is set to speakers (not BlackHole)

**"OpenRouter key shows invalid"**
- Only needed when cloud translation is enabled
- Keys in `.env` work even if the Settings field is empty

---

## License

MIT — see [LICENSE](LICENSE). Copyright (c) 2026 Kai Letov (original author).
