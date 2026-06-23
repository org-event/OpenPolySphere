# Call Translator — dev tasks (like package.json scripts).
# After clone:  ./scripts/bootstrap   or   just install

default:
    @just --list

# Bootstrap dev environment for this machine (like bun install).
install: install-rust install-system install-js install-git install-hooks
    @echo ""
    @echo "Dev environment ready."
    @echo "  just check                              # lint before commit"
    @echo "  just prepush                            # lint before git push"
    @echo "  just check-windows-static               # cross-platform cfg guard (no MSVC)"
    @echo "  cargo run --release -p translator -- setup   # download models (first run)"
    @echo "  cargo run --release -p translator            # start server"

# All pre-commit checks in one command.
check: check-rust check-js check-swift

# Fast gate before git push — same for everyone (git hook via pre-commit).
# fmt + JS lint; catches the mistakes that broke CI in seconds.
prepush: prepush-fmt check-js check-windows-static

build:
    cargo build --release -p translator

run:
    cargo run --release -p translator

setup:
    cargo run --release -p translator -- setup

# --- install steps ---

install-rust:
    #!/usr/bin/env bash
    set -euo pipefail
    if command -v rustc >/dev/null; then
      echo "[ok] Rust $(rustc --version)"
    elif command -v rustup >/dev/null; then
      echo "[..] Installing stable Rust toolchain..."
      rustup default stable
    else
      echo "[!] Rust not found. Install from https://rustup.rs then re-run: just install"
      exit 1
    fi
    rustup component add rustfmt clippy 2>/dev/null || true

install-system:
    #!/usr/bin/env bash
    set -euo pipefail
    case "$(uname -s)" in
      Darwin)
        if ! command -v brew >/dev/null; then
          echo "[!] Homebrew required on macOS: https://brew.sh"
          exit 1
        fi
        echo "[..] macOS system packages (brew)..."
        brew list espeak-ng &>/dev/null || brew install espeak-ng
        brew list onnxruntime &>/dev/null || brew install onnxruntime
        command -v just >/dev/null || brew install just
        command -v pre-commit >/dev/null || brew install pre-commit
        command -v bun >/dev/null || brew install oven-sh/bun/bun
        xcode-select -p &>/dev/null || echo "[!] Run: xcode-select --install (for Swift)"
        echo "[ok] macOS system deps"
        ;;
      Linux)
        echo "[i] Linux: optional runtime libs for local build:"
        echo "    Debian/Ubuntu: sudo apt install espeak-ng libonnxruntime-dev pkg-config"
        echo "    Banyan Speech / Swift checks are skipped on Linux (CI runs them on macOS)."
        ;;
      MINGW*|MSYS*|CYGWIN*)
        echo "[i] Windows: use WSL or wait for native port. Rust/JS hooks still work where applicable."
        ;;
      *)
        echo "[i] Unknown OS — install espeak-ng and onnxruntime manually if you build locally."
        ;;
    esac

install-js:
    #!/usr/bin/env bash
    set -euo pipefail
    if ! command -v bun >/dev/null; then
      echo "[!] bun not found. Install from https://bun.sh then re-run: just install"
      exit 1
    fi
    echo "[..] bun install --frozen-lockfile..."
    bun install --frozen-lockfile
    echo "[ok] JS dev dependencies"

install-git:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "[..] git rebase workflow (local config for this repo)..."
    git config pull.rebase true
    git config rebase.autoStash true
    git config branch.autoSetupRebase always
    git config fetch.prune true
    echo "[ok] pull.rebase, rebase.autoStash, branch.autoSetupRebase, fetch.prune"

install-hooks:
    #!/usr/bin/env bash
    set -euo pipefail
    if ! command -v pre-commit >/dev/null; then
      echo "[!] pre-commit not found."
      echo "    macOS: brew install pre-commit"
      echo "    other: pip install pre-commit   or   pipx install pre-commit"
      exit 1
    fi
    pre-commit install
    pre-commit install --hook-type pre-push
    echo "[ok] git pre-commit hook → just check"
    echo "[ok] git pre-push hook → just prepush"

# --- check steps ---

prepush-fmt:
    cargo fmt --all -- --check

prepush-rust:
    @just prepush-fmt
    cargo clippy -p translator -p audio-core --all-targets -- -D warnings

check-rust:
    @just prepush-rust

check-js:
    bun run lint:js

check-windows-static:
    #!/usr/bin/env bash
    set -euo pipefail
    chmod +x scripts/check-windows-lint.sh
    ./scripts/check-windows-lint.sh

check-swift:
    #!/usr/bin/env bash
    set -euo pipefail
    if [[ "$(uname -s)" != "Darwin" ]]; then
      echo "[skip] Swift (not macOS)"
      exit 0
    fi
    if ! command -v swift >/dev/null; then
      echo "[!] Swift not found. Run: xcode-select --install"
      exit 1
    fi
    # Banyan Translate APIs need the macOS 15+ SDK (Xcode 16+); TranslationSession needs macOS 26 SDK.
    if ! xcrun --sdk macosx --show-sdk-version 2>/dev/null | awk -F. '{ exit !($1 >= 15) }'; then
      echo "[skip] Swift Translation checks need macOS 15+ SDK (install Xcode 16+)"
      exit 0
    fi
    for pkg in tools/banyan-speech-auth tools/banyan-translate tools/banyan-speech; do
      echo "==> swift build $pkg"
      if [[ "$pkg" == "tools/banyan-translate" ]]; then
        if ! xcrun --sdk macosx --show-sdk-version 2>/dev/null | awk -F. '{ exit !($1 >= 26) }'; then
          echo "[skip] banyan-translate needs macOS 26 SDK (Xcode 26+) for TranslationSession"
          continue
        fi
      fi
      swift build -c release --package-path "$pkg"
    done
