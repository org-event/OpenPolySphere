#!/usr/bin/env bash
# Cross-platform lint guardrails without a Windows runner.
set -euo pipefail
cd "$(dirname "$0")/.."

errors=0

for f in crates/audio-core/src/stt/mod.rs crates/audio-core/src/translation/mod.rs; do
  if rg -q '#\[cfg\(target_os' "$f"; then
    echo "[fail] $f: platform cfg must not appear in orchestrators (use platform/ + apple imp)"
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
