#!/usr/bin/env bash
# Build OpenPolySphere macOS GUI shell (WKWebView + server lifecycle).
# Usage: ./scripts/build-macos-shell.sh [output-path]
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TOOL="$ROOT/tools/openpolysphere-shell"
OUT="${1:-$ROOT/packaging/macos/OpenPolySphere}"
SCRATCH="$ROOT/target/swift-openpolysphere-shell"

cd "$TOOL"
swift build -c release --product OpenPolySphere --scratch-path "$SCRATCH"
BIN="$(swift build -c release --product OpenPolySphere --scratch-path "$SCRATCH" --show-bin-path)/OpenPolySphere"
mkdir -p "$(dirname "$OUT")"
cp "$BIN" "$OUT"
chmod +x "$OUT"
echo "Wrote $OUT"
