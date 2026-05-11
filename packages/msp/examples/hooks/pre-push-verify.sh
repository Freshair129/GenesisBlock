#!/usr/bin/env bash
# MSP pre-push hook — runs `gks verify-flow` on every FEAT touched in the push range.
# Installed by examples/hooks/install.sh.
# Skip with: git push --no-verify
# msp:hook-marker:pre-push-verify-v1

set -euo pipefail

REPO_ROOT=$(git rev-parse --show-toplevel)
cd "$REPO_ROOT"

ZERO_SHA="0000000000000000000000000000000000000000"
FAIL=0
COUNT=0

while read -r _local_ref local_sha _remote_ref remote_sha; do
  [ -z "${local_sha:-}" ] && continue
  [ "$local_sha" = "$ZERO_SHA" ] && continue   # branch deletion

  if [ "$remote_sha" = "$ZERO_SHA" ]; then
    base=$(git merge-base "$local_sha" origin/main 2>/dev/null || echo "${local_sha}~1")
  else
    base="$remote_sha"
  fi

  changed=$(git diff --name-only "$base" "$local_sha" -- 'gks/feat/FEAT--*.md' 2>/dev/null || true)
  [ -z "$changed" ] && continue

  while IFS= read -r path; do
    [ -z "$path" ] && continue
    feat=$(basename "$path" .md)
    COUNT=$((COUNT + 1))
    output=$(npx gks verify-flow "$feat" --root="$REPO_ROOT" 2>&1) && rc=0 || rc=$?
    if [ "$rc" -ne 0 ]; then
      FAIL=$((FAIL + 1))
      # shellcheck disable=SC2001
      echo "$output" | sed 's/^/  /'
    fi
  done <<< "$changed"
done

if [ "$FAIL" -gt 0 ]; then
  echo "✗ MSP pre-push: $FAIL of $COUNT FEAT(s) failed verify-flow. Fix and re-push, or use --no-verify."
  exit 1
fi

if [ "$COUNT" -gt 0 ]; then
  echo "✓ MSP pre-push: $COUNT FEAT(s) verified."
fi
exit 0
