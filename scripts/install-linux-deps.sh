#!/usr/bin/env bash
# One-time native deps for local Linux builds (same families as CI linux job).
# Usage: ./scripts/install-linux-deps.sh
set -euo pipefail

if [[ "$(uname -s)" != "Linux" ]]; then
  echo "[skip] Linux deps script — not on Linux"
  exit 0
fi

if ! command -v apt-get >/dev/null; then
  echo "[!] apt-get not found. Install manually: cmake pkg-config libopenblas-dev libasound2-dev espeak-ng"
  exit 1
fi

echo "[..] Installing Linux build deps (sudo)..."
sudo apt-get update
sudo apt-get install -y cmake pkg-config libopenblas-dev libasound2-dev espeak-ng
echo "[ok] System packages installed"
echo ""
echo "Next:"
echo "  just fetch-ort          # download ONNX Runtime (or set ORT_DYLIB_PATH yourself)"
echo "  just check-linux-clippy # full clippy parity with CI"
echo "  just build && just setup && just run"
