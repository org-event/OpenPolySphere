# Linux support (draft)

Target: **Ubuntu 22.04+ / Debian 12+ / Fedora 39+**. Goal for now: **CI builds on `ubuntu-latest`**, you download the artifact and smoke-test without a dev machine.

## Phase 1 — CI build (current)

- [x] `feat/linux` branch
- [x] GitHub Actions job: `cargo clippy`, `cargo build --release -p translator`
- [ ] Download artifact → run `translator --help` (manual smoke test)

**CI installs (runner only):** OpenBLAS (`libopenblas-dev`), ONNX Runtime zip, espeak-ng (`apt`).

**Artifact contains:** `translator`, `libonnxruntime.so`, `web/`, `.env.example`, `LINUX.txt` (quick start).

## Phase 2 — Make the binary runnable

- [x] Default `ORT_DYLIB_PATH` / `libonnxruntime.so` bundled in CI artifact
- [x] Find `espeak-ng` on Linux via `platform::find_espeak_ng` (PATH + `/usr/bin`)
- [x] Platform defaults instead of BlackHole (`OpenPolySphere-Meet-In.monitor` / `OpenPolySphere-Meet-Out`)
- [x] Document virtual audio: PipeWire / PulseAudio null sinks (below)
- [x] Short UI hint for virtual sink setup on Linux (Settings → Audio Devices)

### Virtual audio (PipeWire / PulseAudio)

Create null sinks for call routing (names match server defaults):

```bash
# PulseAudio (also works on many PipeWire setups via pactl)
pactl load-module module-null-sink sink_name=OpenPolySphere-Meet-Out sink_properties=device.description=OpenPolySphere-Meet-Out
pactl load-module module-null-sink sink_name=OpenPolySphere-Meet-In sink_properties=device.description=OpenPolySphere-Meet-In

# In the call app:
# - Speakers → OpenPolySphere-Meet-In
# - Microphone → OpenPolySphere-Meet-Out (monitor of the out sink, or a loopback — tune per app)
```

In Settings → Audio Devices, pick **OpenPolySphere-Meet-In.monitor** for meet input and **OpenPolySphere-Meet-Out** for meet output if they appear in the list. Otherwise use **Tab Audio** capture in Chrome for incoming audio.

## Phase 3 — Full call translation

- [ ] `translator setup` on Linux (model download)
- [ ] Mic + virtual sink routing documented and tested
- [ ] `just install` hints for native Linux (see `Justfile`)

## Phase 4 — Polish

- [ ] README Linux section
- [ ] Flatpak / Snap packaging (out of scope until MVP works)

## Manual smoke test (after downloading CI artifact)

1. Unzip artifact to a folder, e.g. `~/OpenPolySphere/`
2. In a terminal in that folder:
   ```bash
   chmod +x translator
   export ORT_DYLIB_PATH=$PWD/libonnxruntime.so
   export LD_LIBRARY_PATH=$PWD:$LD_LIBRARY_PATH
   ./translator --help
   ./translator setup    # downloads models (network, ~1GB+ disk)
   ./translator          # open http://127.0.0.1:5050 in Chrome
   ```
3. Install virtual sinks above for call routing (Phase 2+).

## Local dev (optional)

Native build needs: Rust, CMake, `libopenblas-dev`, ONNX Runtime (`ORT_DYLIB_PATH`), espeak-ng, Bun (for ESLint).

```bash
sudo apt install cmake pkg-config libopenblas-dev espeak-ng
# ONNX Runtime: download linux x64 tgz from GitHub releases or distro package
export ORT_DYLIB_PATH=/path/to/libonnxruntime.so
cargo build --release -p translator
```

**Local Linux CI parity:**

| Command | Where | What |
|---------|-------|------|
| `just check` | **Linux host** | rustfmt + clippy + ESLint (Swift checks skipped) |
| `just prepush` | **Linux host** | fmt + JS + `check-windows-static` cfg guards |
| CI `linux` job | GitHub | Skipped on docs-only PRs; rust-cache on `main`. PR label `ci/linux-only` skips macOS + Windows. |

See [ADR 0002](../adr/0002-ci-platform-tiers.md) for CI tiering.

See [issue #29](https://github.com/org-event/OpenPolySphere/issues/29).
