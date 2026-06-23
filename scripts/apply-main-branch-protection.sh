#!/usr/bin/env bash
# Require pull requests before merging into main (no direct pushes).
# Linear history is already enabled on org-event/Banyan.
#
# Optional: set REQUIRED_CHECKS to a comma-separated list of GitHub check contexts
# (e.g. "macos,windows,rustfmt,js,cargo audit (RustSec),bun audit").
#
# Usage:
#   ./scripts/apply-main-branch-protection.sh          # apply
#   ./scripts/apply-main-branch-protection.sh --dry-run
set -euo pipefail

REPO="${GITHUB_REPO:-org-event/Banyan}"
DRY=false
[[ "${1:-}" == "--dry-run" ]] && DRY=true

status_checks_json="null"
if [[ -n "${REQUIRED_CHECKS:-}" ]]; then
  IFS=',' read -ra checks <<<"$REQUIRED_CHECKS"
  contexts=$(printf '%s\n' "${checks[@]}" | jq -R . | jq -s .)
  status_checks_json=$(jq -n --argjson contexts "$contexts" \
    '{strict: false, contexts: $contexts}')
fi

payload=$(jq -n \
  --argjson status_checks "$status_checks_json" \
  '{
    required_status_checks: $status_checks,
    enforce_admins: false,
    required_pull_request_reviews: {
      dismiss_stale_reviews: false,
      require_code_owner_reviews: false,
      required_approving_review_count: 0
    },
    restrictions: null,
    required_linear_history: true,
    allow_force_pushes: false,
    allow_deletions: false
  }')

echo "[..] Target: $REPO branch main"
echo "$payload" | jq .

if $DRY; then
  echo "[dry-run] Would PUT /repos/$REPO/branches/main/protection"
  exit 0
fi

gh api \
  --method PUT \
  -H "Accept: application/vnd.github+json" \
  "/repos/$REPO/branches/main/protection" \
  --input - <<<"$payload"

echo "[ok] main: pull request required before merge (0 approvals minimum)"
if [[ "$status_checks_json" == "null" ]]; then
  echo "[i] No required status checks (set REQUIRED_CHECKS to enable)."
else
  echo "[ok] Required checks: $REQUIRED_CHECKS"
fi
echo "[i] Admins can still bypass unless you set enforce_admins true in this script."
