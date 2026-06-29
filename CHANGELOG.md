# Changelog

## [Unreleased]

### Added

- Cross-platform desktop shell (`openpolysphere`): embedded WebView on macOS, Windows, and Linux; server stops when the window closes

### Fixed

- Desktop shell stops the translator server when the window closes (all platforms)

## [0.4.3] - 2026-06-28

### Fixed

- Linux release: nfpm version injection and `.pack-root` paths in `nfpm.yaml`
- macOS Intel release: migrate `macos-13` → `macos-15-intel` (retired runner)

### Added

- CI smoke tests for `package-linux.sh` and `package-macos-app.sh` on PR builds

## [0.4.2] - 2026-06-28

### Added

- macOS: `OpenPolySphere.app` + `.dmg` / `.zip` (Apple Silicon and Intel x64)
- Linux: `.deb`, `.rpm`, and portable `.zip`
- Windows: Inno Setup installer + portable `.zip`

### Fixed

- Release packaging: macOS zip output path, Linux nfpm download URL, Windows Inno Setup script

## [0.4.0] - 2026-06-26

### Added

- Linux CI build and GitHub Release zip (`openpolysphere-*-linux-x64`)
- Linux virtual-sink defaults and Settings UI hint (PulseAudio/PipeWire)
- `just fetch-ort`, `just check-linux-clippy`, `just install-linux-deps`
- `docs/linux.md` and unified `just`-based dev workflow across macOS / Linux / Windows

### Changed

- OpenPolySphere rebrand (logos, PolySphere Speech/Translate helpers)
- CONTRIBUTING and platform docs aligned on `just` recipes

## [0.3.0] - 2026-06-23

_Pre-0.4.0 tags shipped Windows CI artifacts and macOS bundles; see GitHub Releases._

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
