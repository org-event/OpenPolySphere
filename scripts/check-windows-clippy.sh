#!/usr/bin/env bash
# Full `cargo clippy` parity with the GitHub `windows` job.
#
# Runs on native Windows only. macOS/Linux cannot cross-clippy this crate graph yet
# (OpenBLAS, sentencepiece/cmake, MSVC) even with zig — use `just check-windows-static`
# before push; zig is installed via `just install` as dev-only tooling, not a Cargo dep.
set -euo pipefail
cd "$(dirname "$0")/.."

case "$(uname -s)" in
  MINGW* | MSYS* | CYGWIN*)
    echo "[..] cargo clippy (native Windows, CI parity)..."
    cargo clippy -p translator -p audio-core@0.1.0 --all-targets -- -D warnings
    echo "[ok] Windows clippy passed"
    ;;
  *)
    echo "[skip] Full Windows clippy needs a native Windows host (OpenBLAS + sentencepiece C deps)."
    echo "       Before push: just check-windows-static  (in just prepush on every OS)"
    exit 0
    ;;
esac
