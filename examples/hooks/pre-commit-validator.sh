#!/usr/bin/env bash
# MSP pre-commit hook — runs npm run msp:validate on staged atom files.
# Installed by examples/hooks/install.sh.
# Skip with: git commit --no-verify
# msp:hook-marker:pre-commit-validator-v1

set -euo pipefail

# Resolve repo root so the hook works no matter where git invokes it from.
REPO_ROOT=$(git rev-parse --show-toplevel)
cd "$REPO_ROOT"

# 1. Collect staged atom files (added / copied / modified / renamed).
STAGED=$(git diff --cached --name-only --diff-filter=ACMR \
  | grep -E '^(gks/.*\.md|\.brain/msp/projects/[^/]+/inbound/.*\.md)$' || true)

# 2. Zero-cost happy path — nothing to check.
if [ -z "$STAGED" ]; then
  exit 0
fi

# 3. Validate each. Re-run on failure to capture the human-readable error.
FAIL=0
COUNT=0
while IFS= read -r f; do
  COUNT=$((COUNT + 1))
  if ! npm run msp:validate --silent -- "$f" >/dev/null 2>&1; then
    FAIL=$((FAIL + 1))
    npm run msp:validate --silent -- "$f" 2>&1 | sed 's/^/  /'
  fi
done <<< "$STAGED"

# 4. Summary + exit code.
if [ "$FAIL" -gt 0 ]; then
  echo "✗ MSP validator: $FAIL of $COUNT file(s) failed. Fix and re-stage, or use --no-verify to skip."
  exit 1
fi

if [ "$COUNT" -gt 0 ]; then
  echo "✓ MSP validator: $COUNT file(s) passed."
fi

# 5. Hotfix gate (ADR--HOTFIX-ESCAPE-HATCH).
# For staged paths outside gks/ + .brain/ + infra dirs, ask GKS whether any
# overdue HOTFIX-- atom blocks them. Single invocation per commit.
hotfix_paths=$(git diff --cached --name-only --diff-filter=ACMR \
  | grep -v -E '^(gks/|\.brain/|\.github/|examples/|scripts/|test/|node_modules/|dist/)' \
  || true)

if [ -n "$hotfix_paths" ]; then
  args=()
  while IFS= read -r p; do
    [ -z "$p" ] && continue
    args+=(--file="$p")
  done <<< "$hotfix_paths"
  output=$(npx gks hotfix check "${args[@]}" 2>&1) && rc=0 || rc=$?
  if [ "$rc" -ne 0 ]; then
    # shellcheck disable=SC2001
    echo "$output" | sed 's/^/  /'
    echo "✗ MSP hotfix check failed. Backfill the HOTFIX-- atom or use --no-verify."
    exit 1
  fi
fi

exit 0
