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

echo "✓ MSP validator: $COUNT file(s) passed."
exit 0
