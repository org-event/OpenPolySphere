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

Heavy jobs (macOS / Windows build) run only when Rust, workflows, or related scripts change — not on docs-only PRs. Security audits (RustSec, CodeQL) still run separately.

| Label | Effect |
|-------|--------|
| `ci/windows-only` | Skip macOS job on this PR (Windows CI still runs). Remove before merging to `main`. |

Manual full platform run: **Actions → CI → Run workflow** (`workflow_dispatch`).

See [ADR 0002](docs/adr/0002-ci-platform-tiers.md).


```bash
git clone https://github.com/org-event/OpenPolySphere.git
cd OpenPolySphere
./scripts/bootstrap    # installs `just` if needed, then `just install`
just check             # same checks that run before each commit
cp .env.example .env   # optional: cloud API keys
just setup             # download models (first time)
just run               # start server
```

**`./scripts/bootstrap` vs `just install`:** bootstrap is the entry point after clone (like `bun install`). It ensures `just` is on your PATH, then runs `just install`. If you already have `just`, `just install` alone is enough.

**`just install` installs:** Rust rustfmt/clippy; on macOS also Homebrew packages (espeak-ng, onnxruntime, bun, pre-commit); `bun install --frozen-lockfile`; git pre-commit hook.

| Recipe | What it runs |
|--------|----------------|
| `just` / `just --list` | Show all recipes |
| `just install` | Dev environment bootstrap |
| `just check` | rustfmt + clippy (`-D warnings`) + ESLint + Swift release build (macOS) |
| `just build` | Release build of `translator` |
| `just run` | `cargo run --release -p translator` |
| `just setup` | Download Whisper, Opus-MT, default Piper voices |

On **Linux/Windows** Swift and Apple-only checks are skipped automatically; full macOS build is verified in CI.

### Code style

- **All**: `just check` (rustfmt, clippy, eslint, Swift on macOS)
- **Rust**: `cargo fmt` + `cargo clippy`
- **JS**: `bun run lint:js`
- Pre-commit hook runs `just check` automatically after `just install`

### What we especially welcome

- Windows / Linux audio backend support
- New TTS voice contributions
- Latency optimizations
- Documentation improvements
- Bug fixes with reproduction steps

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
