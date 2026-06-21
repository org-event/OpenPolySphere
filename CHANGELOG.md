# Changelog

## [Unreleased]

### All-Rust rewrite

- Single binary `translator` (Axum :5050 + in-process audio engine)
- Removed Elixir, Flask, Python runtime, `run.sh` / `setup.sh`
- Local Whisper STT + Opus-MT translation by default
- Model download via `translator setup` or Settings UI
- SQLite call history in-process

## [0.1.0] - 2026-04-05

### Initial open-source release

- Real-time bidirectional voice translation for video calls
- Piper TTS with local ONNX inference (29 languages)
- Web UI: live transcript, settings, call history, voice downloads
