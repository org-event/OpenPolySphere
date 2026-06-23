#!/usr/bin/env bash
# Print GitHub Security & Quality posture for this repo (read-only).
set -euo pipefail

REPO="${GITHUB_REPO:-org-event/Banyan}"

echo "=== $REPO — Security & Quality ==="
echo

echo "## Branch protection (main)"
gh api "repos/$REPO/branches/main/protection" 2>/dev/null | jq '{
  pr_required: (.required_pull_request_reviews.required_approving_review_count // "none"),
  linear_history: .required_linear_history.enabled,
  status_checks: .required_status_checks,
  enforce_admins: .enforce_admins.enabled
}' || echo "  (no protection or insufficient access)"
echo

echo "## Actions default token"
gh api "repos/$REPO/actions/permissions/workflow" | jq .
echo

echo "## Enabled security features"
gh api "repos/$REPO" --jq '.security_and_analysis'
echo

echo "## Open code-scanning alerts (by rule)"
gh api "repos/$REPO/code-scanning/alerts?state=open&per_page=100" \
  | jq 'group_by(.rule.id) | map({rule: .[0].rule.id, count: length}) | sort_by(-.count)'
echo

echo "## Open Dependabot alerts"
count=$(gh api "repos/$REPO/dependabot/alerts?state=open&per_page=1" --jq 'length' 2>/dev/null || echo 0)
echo "  count (first page): $count"
echo

echo "## Workflow permissions blocks"
for f in .github/workflows/*.yml; do
  if grep -q '^permissions:' "$f" 2>/dev/null; then
    echo "  [ok] $(basename "$f")"
  else
    echo "  [--] $(basename "$f") — no top-level permissions:"
  fi
done
