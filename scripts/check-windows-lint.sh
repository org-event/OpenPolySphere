#!/usr/bin/env bash
# Cross-platform lint guardrails without a Windows runner.
set -euo pipefail
cd "$(dirname "$0")/.."

errors=0

# Orchestrators must not branch on target_os — use platform/ + imp/stub facades.
for f in \
  crates/audio-core/src/stt/mod.rs \
  crates/audio-core/src/translation/mod.rs \
  crates/audio-core/src/stt/local/mod.rs \
  crates/translator/src/main.rs; do
  if rg -q '#\[cfg\(target_os' "$f"; then
    echo "[fail] $f: platform cfg must not appear in orchestrators (use platform/ + imp)"
    errors=$((errors + 1))
  fi
done

# Platform-specific backends must expose macOS imp + non-macOS stub.
for f in \
  crates/audio-core/src/stt/apple.rs \
  crates/audio-core/src/translation/apple.rs \
  crates/audio-core/src/stt/local/metal.rs; do
  if ! rg -q '#\[cfg\(target_os = "macos"\)\]' "$f"; then
    echo "[fail] $f: missing macOS imp module"
    errors=$((errors + 1))
  fi
  if ! rg -q '#\[cfg\(not\(target_os = "macos"\)\)\]' "$f"; then
    echo "[fail] $f: missing non-macOS stub imp module"
    errors=$((errors + 1))
  fi
done

for f in crates/audio-core/src/stt/apple.rs crates/audio-core/src/translation/apple.rs; do
  if rg '^use anyhow::\{.*Context' "$f" >/dev/null 2>&1; then
    echo "[fail] $f: import Context only inside macOS imp module"
    errors=$((errors + 1))
  fi
done

if [[ "$errors" -gt 0 ]]; then
  echo ""
  echo "$errors cross-platform lint issue(s)."
  exit 1
fi

echo "[ok] cross-platform static lint checks passed"
