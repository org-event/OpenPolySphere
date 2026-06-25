# Linux support (draft)

Target: **Ubuntu 22.04+ / Debian 12+ / Fedora 39+**. Goal: **CI builds on `ubuntu-latest`**, download the artifact or develop locally with the same `just` recipes as macOS/Windows.

## Local dev (same flow as other platforms)

After clone, use the same entry point everywhere:

```bash
git clone https://github.com/org-event/OpenPolySphere.git
cd OpenPolySphere
./scripts/bootstrap          # installs just if needed, then just install
just install-linux-deps      # one-time: apt packages (sudo)
just fetch-ort               # one-time: ONNX Runtime under ort/
# export ORT_DYLIB_PATH / LD_LIBRARY_PATH from fetch-ort output
just check                   # rustfmt + clippy + ESLint (Swift skipped on Linux)
just build
just setup                   # download models
just run                     # http://127.0.0.1:5050
```

| Recipe | Where | What |
|--------|-------|------|
| `just install` | **all hosts** | Rust, bun, hooks; prints OS-specific one-time deps |
| `just install-linux-deps` | **Linux** | `apt` packages (ALSA, OpenBLAS, espeak-ng, cmake) |
| `just fetch-ort` | **all hosts** | Download or print ONNX Runtime path |
| `just check` | **Linux** | rustfmt + clippy + ESLint |
| `just prepush` | **all hosts** | fmt + JS + `check-windows-static` cfg guards |
| `just check-linux-clippy` | **native Linux** | Full clippy parity with CI `linux` job |
| CI `linux` job | GitHub | Same deps as above. Label `ci/linux-only` skips macOS + Windows. |

**One-time apt packages (Debian/Ubuntu):**

```bash
sudo apt install cmake pkg-config libopenblas-dev libasound2-dev espeak-ng
```

Fedora: `sudo dnf install cmake pkgconfig openblas-devel alsa-lib-devel espeak-ng`

## Phase 1 — CI build (current)

- [x] GitHub Actions `linux` job: clippy + release build + artifact
- [ ] Manual: download artifact → `translator --help`

**Artifact:** `translator`, `libonnxruntime.so`, `web/`, `.env.example`, `LINUX.txt`

## Phase 2 — Audio routing

- [x] Platform defaults: `OpenPolySphere-Meet-In.monitor` / `OpenPolySphere-Meet-Out`
- [x] UI hint in Settings → Audio Devices (Linux only)
- [x] Docs below (PipeWire / PulseAudio)

### Virtual audio (PipeWire / PulseAudio)

```bash
pactl load-module module-null-sink sink_name=OpenPolySphere-Meet-Out sink_properties=device.description=OpenPolySphere-Meet-Out
pactl load-module module-null-sink sink_name=OpenPolySphere-Meet-In sink_properties=device.description=OpenPolySphere-Meet-In
```

In Settings → Audio Devices: **OpenPolySphere-Meet-In.monitor** (call input), **OpenPolySphere-Meet-Out** (call output). Or use **Tab Audio** in the toolbar for incoming audio.

## CI artifact smoke test

```bash
chmod +x translator
export ORT_DYLIB_PATH=$PWD/libonnxruntime.so
export LD_LIBRARY_PATH=$PWD:$LD_LIBRARY_PATH
./translator --help
./translator setup
./translator
```

## Out of scope (for now)

- Flatpak / Snap
- Apple Speech / Translation

See [ADR 0002](../adr/0002-ci-platform-tiers.md), [issue #29](https://github.com/org-event/OpenPolySphere/issues/29).
