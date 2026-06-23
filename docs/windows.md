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

Native build needs: Rust, CMake, vcpkg OpenBLAS, ONNX Runtime, espeak-ng, Bun (for ESLint). Prefer CI artifact until Phase 2 is done.

See [issue #3](https://github.com/org-event/Banyan/issues/3).
