#!/usr/bin/env bash
# Full `cargo clippy` parity with the GitHub `linux` job.
#
# Runs on native Linux only. macOS/Windows use their own CI jobs or static guards.
set -euo pipefail
cd "$(dirname "$0")/.."

case "$(uname -s)" in
  Linux)
    echo "[..] cargo clippy (native Linux, CI parity)..."
    cargo clippy -p translator -p audio-core@0.1.0 --all-targets -- -D warnings
    echo "[ok] Linux clippy passed"
    ;;
  *)
    echo "[skip] Full Linux clippy needs a native Linux host (ALSA, OpenBLAS, sentencepiece)."
    echo "       Before push: just prepush  (fmt + JS + Windows static cfg guards on every OS)"
    exit 0
    ;;
esac
