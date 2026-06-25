#!/usr/bin/env bash
# check-bins-feature.sh — guard for the run-bench-audit skill.
#
# The GenesisBlockDB benchmark/audit [[bin]] targets are gated behind the
# off-by-default `bins` feature. A `cargo run --bin <name>` WITHOUT
# `--features bins` fails with exit 101 ("target requires the features: bins").
# This script validates a cargo command string before you trust its output.
#
# Usage:  check-bins-feature.sh "cargo run --release --features bins --bin industrial-audit"
# Exit 0 = OK (or not a bin run), 1 = a `--bin` run is missing the bins feature.

set -euo pipefail

cmd="${*:-}"

if [[ -z "$cmd" ]]; then
  echo "usage: check-bins-feature.sh \"<cargo command>\"" >&2
  exit 2
fi

# Only enforce on commands that actually run a binary target.
if ! grep -qE -- '--bin[ =]' <<<"$cmd"; then
  # `cargo bench --bench ldbc_lite` and similar are not gated — pass through.
  echo "OK: not a '--bin' run, no bins feature required."
  exit 0
fi

if grep -qE -- '--features[ =]"?[^"]*bins' <<<"$cmd"; then
  echo "OK: '--bin' run includes the 'bins' feature."
  exit 0
fi

echo "FAIL: '--bin' run is missing '--features bins'." >&2
echo "  cargo gates every bench/audit binary behind the 'bins' feature;" >&2
echo "  without it the build exits 101 ('target requires the features: bins')." >&2
echo "  Fix: add  --features bins  (PowerShell: --features=\"bins\")." >&2
exit 1
