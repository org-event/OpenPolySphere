# Windows support (draft)

Target: **Windows 10/11 x64**. Goal: **CI builds on `windows-latest`**, download the artifact or develop locally with the same `just` recipes as macOS/Linux.

## Local dev (same flow as other platforms)

After clone, use the same entry point everywhere:

```powershell
git clone https://github.com/org-event/OpenPolySphere.git
cd OpenPolySphere
./scripts/bootstrap          # Git Bash: installs just if needed, then just install
just fetch-ort               # one-time: ONNX Runtime under ort/
just check-windows-clippy    # full clippy parity with CI (native Windows)
just build
just setup                   # download models
just run                     # http://127.0.0.1:5050
```

On every OS before push: `just prepush` (fmt + JS + static cfg guards).

| Recipe | Where | What |
|--------|-------|------|
| `just install` | **all hosts** | Rust, bun, hooks; prints OS-specific one-time deps |
| `just fetch-ort` | **all hosts** | Download or print ONNX Runtime path |
| `just check` | **macOS** | rustfmt + clippy + ESLint + Swift |
| `just prepush` | **all hosts** | fmt + JS + `check-windows-static` |
| `just check-windows-clippy` | **native Windows** | Full clippy parity with CI `windows` job |
| `just check-windows-static` | **all hosts** | Fast cfg/import guards (in `prepush`) |
| CI `windows` job | GitHub | vcpkg OpenBLAS, ORT zip, espeak-ng. Label `ci/windows-only` skips macOS. |

**One-time native Windows deps:** Rust, CMake, [vcpkg](https://vcpkg.io) OpenBLAS (`x64-windows-static`), espeak-ng (`choco install espeak-ng`), Bun. `.cargo/config.toml` sets `+crt-static` on MSVC — required by ct2rs.

## Phase 1 — CI build (current)

- [x] GitHub Actions `windows` job: clippy + release build + artifact
- [ ] Manual: download artifact → `translator.exe --help`

**Artifact:** `translator.exe`, `onnxruntime.dll`, `web/`, `.env.example`, `WINDOWS.txt`

## Phase 2 — Audio routing

- [ ] Platform defaults for virtual cable (VB-Audio / VoiceMeeter)
- [ ] Document call routing in README

Install [VB-Audio Virtual Cable](https://vb-audio.com/Cable/) for call ↔ translator routing.

## CI artifact smoke test

```powershell
.\translator.exe --help
.\translator.exe setup
.\translator.exe
```

Open http://127.0.0.1:5050 in Chrome.

## Out of scope (for now)

- MSI installer / code signing
- Apple Speech / Translation

See [ADR 0002](../adr/0002-ci-platform-tiers.md), [issue #3](https://github.com/org-event/OpenPolySphere/issues/3).
