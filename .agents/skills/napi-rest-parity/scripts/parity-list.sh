#!/usr/bin/env bash
# parity-list.sh — list both front-end surfaces of GenesisBlockDB side by side
# so NAPI<->REST drift is visible. Read-only; not a pass/fail gate (intentional
# asymmetry exists, e.g. execute_batch is NAPI-only by design).
#
# Run from the repo root:  bash .claude/skills/napi-rest-parity/scripts/parity-list.sh

set -euo pipefail

ROUTER="src/router.rs"
LIB="src/lib.rs"

if [[ ! -f "$ROUTER" || ! -f "$LIB" ]]; then
  echo "error: run from the repo root (expected $ROUTER and $LIB)" >&2
  exit 2
fi

echo "=== REST routes ($ROUTER) ==="
# Pull the path + verb from each .route("/v1/...", get|post(...))
grep -oE '\.route\("[^"]+", *(get|post|put|delete|patch)' "$ROUTER" \
  | sed -E 's/\.route\("([^"]+)", *([a-z]+)/  \2\t\1/' \
  | sort -k2 || true
rest_count=$(grep -cE '\.route\("' "$ROUTER" || true)
echo "  ($rest_count routes)"
echo

echo "=== NAPI methods (#[napi] in $LIB) ==="
# Print the fn name on the line after each #[napi] attribute.
grep -A1 -E '#\[napi' "$LIB" \
  | grep -oE 'fn +[a-z_][a-z0-9_]*' \
  | sed -E 's/fn +/  /' \
  | sort -u || true
napi_count=$(grep -cE '#\[napi\]' "$LIB" || true)
echo "  ($napi_count #[napi] annotations)"
echo

echo "note: counts are NOT expected to match — status/version/swarm/consensus"
echo "      routes and NAPI-only methods (e.g. execute_batch) are legitimately"
echo "      asymmetric. Compare the SPECIFIC capability you changed, not totals."
