#!/usr/bin/env bash
# Create CI-related GitHub labels (idempotent).
set -euo pipefail
REPO="${GITHUB_REPO:-org-event/Banyan}"

create() {
  local name="$1" color="$2" desc="$3"
  if gh label list --repo "$REPO" --search "$name" --json name --jq '.[].name' | rg -qx "$name"; then
    echo "[ok] label exists: $name"
  else
    gh label create "$name" --repo "$REPO" --color "$color" --description "$desc"
    echo "[ok] created: $name"
  fi
}

create "ci/windows-only" "1D76DB" "Skip macOS CI on this PR; Windows jobs still run"
