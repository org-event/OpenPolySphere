# Contributing

Thanks for your interest in contributing to Realtime Call Translator!

## How to contribute

### Bug reports

Open an [issue](../../issues/new?template=bug_report.md) with:
- What you expected vs what happened
- Steps to reproduce
- macOS version, browser, audio setup

### Feature requests

Open an [issue](../../issues/new?template=feature_request.md) describing the use case and why it matters.

### Pull requests

1. Fork the repo
2. Create a branch: `git checkout -b feature/your-feature`
3. Make your changes
4. Test locally with a real call (Google Meet, Zoom, etc.)
5. Keep your branch up to date with **`git rebase origin/main`** (not `git merge`)
6. Submit a PR and merge with **Rebase and merge** (merge commits are disabled on this repo)

### Git workflow (rebase, not merge)

`main` requires a **linear history**. Merge commits are not allowed in PRs or on long-lived branches.

**One-time setup** (also run by `just install` for this clone):

```bash
git config pull.rebase true
git config rebase.autoStash true
git config branch.autoSetupRebase always
git config fetch.prune true
```

**Sync your branch with `main`:**

```bash
git fetch origin
git rebase origin/main
git push --force-with-lease   # after rebase, if the branch was already pushed
```

Do **not** use `git merge origin/main` or `git pull` without rebase.

**Merge PRs on GitHub:** use **Rebase and merge** (or Squash and merge for a single commit). **Create a merge commit** is disabled.

**Direct push to `main` is disallowed** — open a PR. Apply once per repo: `./scripts/apply-main-branch-protection.sh`

### CI on pull requests

Heavy jobs (macOS / Windows / Linux build) run only when Rust, workflows, or related scripts change — not on docs-only PRs. Security audits (RustSec, CodeQL) still run separately.

| Label | Effect |
|-------|--------|
| `ci/windows-only` | Skip macOS job on this PR (Windows + Linux still run). Remove before merging to `main`. |
| `ci/linux-only` | Skip macOS and Windows on this PR (Linux still runs). Remove before merging to `main`. |

Manual full platform run: **Actions → CI → Run workflow** (`workflow_dispatch`: `all` / `macos` / `windows` / `linux`).

Platform port notes: [docs/windows.md](docs/windows.md), [docs/linux.md](docs/linux.md).

See [ADR 0002](docs/adr/0002-ci-platform-tiers.md).


```bash
git clone https://github.com/org-event/OpenPolySphere.git
cd OpenPolySphere
./scripts/bootstrap    # installs `just` if needed, then `just install`
just check             # lint before commit (Swift only on macOS)
cp .env.example .env   # optional: cloud API keys
just setup             # download models (first time)
just run               # start server
```

**Linux / Windows one-time deps** (after `just install`): see [docs/linux.md](docs/linux.md) or [docs/windows.md](docs/windows.md) — `just install-linux-deps`, `just fetch-ort`, then `just check-linux-clippy` or `just check-windows-clippy` on native hosts.

**`./scripts/bootstrap` vs `just install`:** bootstrap is the entry point after clone (like `bun install`). It ensures `just` is on your PATH, then runs `just install`. If you already have `just`, `just install` alone is enough.

**`just install` installs:** Rust rustfmt/clippy; on macOS also Homebrew packages (espeak-ng, onnxruntime, bun, pre-commit); `bun install --frozen-lockfile`; git pre-commit hook. On Linux/Windows it prints one-time system dependency steps.

| Recipe | What it runs |
|--------|----------------|
| `just` / `just --list` | Show all recipes |
| `just install` | Dev environment bootstrap |
| `just check` | rustfmt + clippy + ESLint + Swift (macOS only) |
| `just prepush` | fmt + JS + Windows static cfg guards (all OS) |
| `just check-linux-clippy` | Full Linux clippy (native Linux, CI parity) |
| `just check-windows-clippy` | Full Windows clippy (native Windows, CI parity) |
| `just fetch-ort` | ONNX Runtime download / path hints |
| `just build` | Release build of `translator` |
| `just test` | Unit tests (`audio-core` VAD/downsample) |
| `just run` | Start server |
| `just setup` | Download Whisper, Opus-MT, default Piper voices |

On **Linux/Windows** Swift and Apple-only checks are skipped automatically; platform builds are verified in CI.

### Code style

- **All**: `just check` (rustfmt, clippy, eslint, Swift on macOS)
- **Rust**: `cargo fmt` + `cargo clippy`
- **JS**: `bun run lint:js`
- Pre-commit hook runs `just check` automatically after `just install`

### Automated tests

- **Run unit tests:** `just test` or `cargo test -p audio-core@0.1.0 --lib`
- **Major new features:** add or update unit tests in the automated suite when practical (see `crates/audio-core/src/vad/mod.rs` for examples)
- **User-facing audio changes:** also verify with a real call (Google Meet, Zoom, etc.) before merging
- **CI:** macOS, Windows, and Linux jobs run `cargo test -p audio-core@0.1.0 --lib` on Rust changes

### What we especially welcome

- Windows / Linux audio backend support
- New TTS voice contributions
- Latency optimizations
- Documentation improvements
- Bug fixes with reproduction steps

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
