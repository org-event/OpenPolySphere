# Windows support (draft)

Target: **Windows 10/11 x64**. Goal for now: **CI builds on `windows-latest`**, you download the artifact and smoke-test without a dev machine.

## Phase 1 — CI build (current)

- [x] `windows` branch
- [ ] GitHub Actions job: compile `translator.exe`, upload artifact
- [ ] Download artifact → run `translator.exe --help`

**CI installs (runner only):** OpenBLAS (vcpkg), ONNX Runtime zip, espeak-ng (Chocolatey).

**Artifact contains:** `translator.exe`, `onnxruntime.dll`, `web/`, `.env.example`, `WINDOWS.txt` (quick start).

## Phase 2 — Make the binary runnable

- [ ] Default `ORT_DYLIB_PATH` / load `onnxruntime.dll` next to exe
- [ ] Find `espeak-ng` on Windows (Chocolatey path)
- [ ] Platform defaults instead of BlackHole (`cfg(windows)` in settings/engine)
- [ ] Document virtual audio: VB-Audio Cable or VoiceMeeter

## Phase 3 — Full call translation

- [ ] `translator setup` on Windows (model download)
- [ ] Mic + virtual cable routing documented and tested
- [ ] Optional: `just install` hints for native Windows

## Phase 4 — Polish

- [ ] README Windows section
- [ ] MSI installer (out of scope until MVP works)

## Manual smoke test (after downloading CI artifact)

1. Unzip artifact to a folder, e.g. `C:\banyan\`
2. Open **PowerShell** in that folder
3. `.\translator.exe --help` — should print usage
4. `.\translator.exe setup` — downloads models (needs network, disk ~1GB+)
5. `.\translator.exe` — open http://127.0.0.1:5050 in Chrome
6. Install [VB-Audio Virtual Cable](https://vb-audio.com/Cable/) for call routing (Phase 2+)

## Local dev (optional)

Native build needs: Rust, CMake, vcpkg OpenBLAS (`x64-windows-static`, matches ct2rs `/MT`), ONNX Runtime, espeak-ng, Bun (for ESLint). `.cargo/config.toml` sets `+crt-static` on `x86_64-pc-windows-msvc` — required by ct2rs.

**Local Windows CI parity:**

| Command | Where | What |
|---------|-------|------|
| `just check-windows-static` | **all hosts** (macOS, Linux, Windows) | In `just prepush`. Fast cfg/import guards — everyone editing cross-platform code. |
| `just check-windows-clippy` | **native Windows only** | Same as CI `cargo clippy -p translator -p audio-core`. Skips on macOS/Linux. |
| CI `windows` job | GitHub | Full clippy + release build on `windows-latest`. |

**macOS-only dev extras** (`just install`): **zig** (brew, dev-only cross toolchain, not a Cargo dep). Windows and Linux developers do not install zig or the `x86_64-pc-windows-msvc` rustup target.

See [issue #3](https://github.com/org-event/Banyan/issues/3).
