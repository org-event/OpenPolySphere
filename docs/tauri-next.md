# Tauri `.app` wrapper (Phase 5)

The All-Rust stack runs as a single `translator` binary (Axum on `127.0.0.1:5050`). A future Tauri shell can wrap it without changing the audio engine.

## Recommended approach

1. **Sidecar binary** — bundle `target/release/translator` inside the `.app` Resources folder.
2. **WebView** — load `http://127.0.0.1:5050` after spawning the sidecar, or embed `web/static/` and proxy API calls to the sidecar.
3. **Models path** — set `TRANSLATOR_MODELS_DIR` to  
   `~/Library/Application Support/CallTranslator/models/`  
   so user data survives app updates.
4. **Dock icon** — Tauri `tauri.conf.json` icon + `LSUIElement` / activation policy as needed.

## Why Axum-first

- Same binary for `cargo run`, CI, and the packaged app.
- No Elixir/Flask/TCP :5051 IPC.
- SSE and `/cmd` stay identical for `app.js`.

## Not in scope yet

- Code signing / notarization
- GPU Whisper / whisper-tiny optimizations

Auto-update: tracked publicly in [#37 — in-app update check and self-update on all platforms](https://github.com/org-event/OpenPolySphere/issues/37). Phase 1 — GitHub Releases check on the Axum stack; full signed self-update (Sparkle / `tauri-plugin-updater`) later.

When starting Phase 5, add a `crates/translator-tauri/` workspace member or a top-level `src-tauri/` that depends on the existing static assets and sidecar build.
