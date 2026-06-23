#!/usr/bin/env bash
# Require pull requests before merging into main (no direct pushes).
# Linear history is already enabled on org-event/Banyan.
#
# Usage:
#   ./scripts/apply-main-branch-protection.sh          # apply
#   ./scripts/apply-main-branch-protection.sh --dry-run
set -euo pipefail

REPO="${GITHUB_REPO:-org-event/Banyan}"
DRY=false
[[ "${1:-}" == "--dry-run" ]] && DRY=true

payload=$(cat <<'EOF'
{
  "required_pull_request_reviews": {
    "required_approving_review_count": 0,
    "dismiss_stale_reviews": false,
    "require_code_owner_reviews": false
  },
  "required_linear_history": { "enabled": true },
  "allow_force_pushes": { "enabled": false },
  "allow_deletions": { "enabled": false },
  "enforce_admins": { "enabled": false },
  "restrictions": null
}
EOF
)

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
echo "[i] Admins can still bypass unless you set enforce_admins true in this script."
