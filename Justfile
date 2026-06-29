# Call Translator — dev tasks (like package.json scripts).
# After clone:  ./scripts/bootstrap   or   just install

default:
    @just --list

# Bootstrap dev environment for this machine (like bun install).
install: install-rust install-system install-js install-git install-hooks
    @echo ""
    @echo "Dev environment ready."
    @echo "  just check                    # lint before commit (platform-aware)"
    @echo "  just prepush                  # fmt + JS + static cfg guards (all OS)"
    @echo "  just check-linux-clippy       # full Linux clippy (native Linux only)"
    @echo "  just check-windows-clippy     # full Windows clippy (native Windows only)"
    @echo "  just fetch-ort                # ONNX Runtime path hints / download"
    @echo "  just test                     # unit tests (audio-core)"
    @echo "  just setup                    # download models (first run)"
    @echo "  just run                      # start server (browser)"
    @echo "  just app                      # desktop window (dev, needs: just build)"

# All pre-commit checks in one command.
check: check-rust check-js check-swift

# Fast gate before git push — same for everyone (git hook via pre-commit).
# fmt + JS lint + Windows static guards (catches cfg/import bugs before CI).
prepush: prepush-fmt check-js check-windows-static

build:
    cargo build --release -p translator -p openpolysphere-app

test:
    cargo test -p audio-core@0.1.0 --lib

run:
    cargo run --release -p translator

# Desktop shell in dev: repo-root .env, target/release/translator, embedded WebView.
app: build
    cargo run --release -p openpolysphere-app

setup:
    cargo run --release -p translator -- setup

# Remove Swift build caches and legacy Banyan-era artifacts (e.g. after repo rename).
clean:
    #!/usr/bin/env bash
    set -euo pipefail
    for pkg in tools/polysphere-speech-auth tools/polysphere-translate tools/polysphere-speech; do
      rm -rf "$pkg/.build"
    done
    rm -rf tools/polysphere-speech-auth/PolySphereSpeech.app
    rm -rf target/debug/BanyanSpeech.app target/release/BanyanSpeech.app
    rm -f target/debug/banyan-translate target/release/banyan-translate
    echo "[ok] cleaned Swift .build caches and legacy Banyan artifacts"

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
    # Cross-target for macOS R&D only; native Windows/Linux use host triple.
    case "$(uname -s)" in
      Darwin) rustup target add x86_64-pc-windows-msvc 2>/dev/null || true ;;
    esac

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
        # Dev-only cross toolchain (not a Cargo dep — rust-first). Used for Windows R&D; full
        # `cargo clippy --target windows-msvc` still needs native Windows (OpenBLAS, sentencepiece).
        brew list zig &>/dev/null || brew install zig
        command -v just >/dev/null || brew install just
        command -v pre-commit >/dev/null || brew install pre-commit
        command -v bun >/dev/null || brew install oven-sh/bun/bun
        xcode-select -p &>/dev/null || echo "[!] Run: xcode-select --install (for Swift)"
        echo "[ok] macOS system deps"
        ;;
      Linux)
        echo "[i] Linux one-time system packages:"
        echo "    ./scripts/install-linux-deps.sh"
        echo "    (or: sudo apt install cmake pkg-config libopenblas-dev libasound2-dev espeak-ng)"
        echo "    just fetch-ort              # ONNX Runtime for local build"
        echo "    just check-linux-clippy       # same as CI linux job"
        echo "    PolySphere Speech / Swift checks are skipped on Linux (CI runs them on macOS)."
        ;;
      MINGW*|MSYS*|CYGWIN*)
        echo "[i] Windows one-time: OpenBLAS (vcpkg), ONNX Runtime, espeak-ng (choco) — see docs/windows.md"
        echo "    just fetch-ort                # download ONNX Runtime zip"
        echo "    just check-windows-clippy     # same as CI windows job"
        echo "    just prepush                  # fmt + JS + static cfg guards"
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
    cargo clippy -p translator -p audio-core@0.1.0 --all-targets -- -D warnings

check-rust:
    @just prepush-rust

check-js:
    bun run lint:js

# Merge scripts/i18n-settings-*.json patches into web/static/locales/
i18n-merge-settings:
    bun run i18n:merge-settings

check-windows-static:
    #!/usr/bin/env bash
    set -euo pipefail
    chmod +x scripts/check-windows-lint.sh
    ./scripts/check-windows-lint.sh

check-windows-clippy:
    #!/usr/bin/env bash
    set -euo pipefail
    chmod +x scripts/check-windows-clippy.sh
    ./scripts/check-windows-clippy.sh

check-linux-clippy:
    #!/usr/bin/env bash
    set -euo pipefail
    chmod +x scripts/check-linux-clippy.sh
    ./scripts/check-linux-clippy.sh

fetch-ort:
    #!/usr/bin/env bash
    set -euo pipefail
    chmod +x scripts/fetch-onnxruntime.sh
    ./scripts/fetch-onnxruntime.sh

install-linux-deps:
    #!/usr/bin/env bash
    set -euo pipefail
    chmod +x scripts/install-linux-deps.sh
    ./scripts/install-linux-deps.sh

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
    # PolySphere Translate APIs need the macOS 15+ SDK (Xcode 16+); TranslationSession needs macOS 26 SDK.
    if ! xcrun --sdk macosx --show-sdk-version 2>/dev/null | awk -F. '{ exit !($1 >= 15) }'; then
      echo "[skip] Swift Translation checks need macOS 15+ SDK (install Xcode 16+)"
      exit 0
    fi
    for pkg in tools/polysphere-speech-auth tools/polysphere-translate tools/polysphere-speech; do
      echo "==> swift build $pkg"
      if [[ "$pkg" == "tools/polysphere-translate" ]]; then
        if ! xcrun --sdk macosx --show-sdk-version 2>/dev/null | awk -F. '{ exit !($1 >= 26) }'; then
          echo "[skip] polysphere-translate needs macOS 26 SDK (Xcode 26+) for TranslationSession"
          continue
        fi
      fi
      swift build -c release --package-path "$pkg"
    done
