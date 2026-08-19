#!/usr/bin/env bash
# G3 moat bench (WP-3.2) — the engine's fused vector+graph+AS-OF jobs vs the
# DIY single-SQLite-file assembly (brute f32 scan + recursive CTE + shared RRF
# glue + audit-history temporal pattern), both in-process in one Rust binary.
# Self-contained: deterministic seeded corpus, no model downloads.
#
#   benchmark/run_moat_bench.sh
#   GB_MOAT_N=100000 GB_MOAT_DIM=1024 GB_MOAT_RUNS=30 benchmark/run_moat_bench.sh
#
# Output: benchmark/results/moat/<ts>_<commit>/{result.json,raw.log,...}
set -uo pipefail
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/_lib.sh"

N="${GB_MOAT_N:-100000}"; DIM="${GB_MOAT_DIM:-1024}"; RUNS="${GB_MOAT_RUNS:-30}"
dir="$(gb_resultdir moat)"
echo "==> run dir: $dir (N=$N dim=$DIM runs=$RUNS)"

gb_collect_env "$dir/env.json" "$dir" || exit 2

echo "==> building moat-bench (release)"
( cd "$GB_REPO_ROOT" && cargo build --release --no-default-features --features bins --bin moat-bench ) || exit 2
BIN="$GB_REPO_ROOT/target/release/moat-bench"; [ -x "$BIN.exe" ] && BIN="$BIN.exe"

GB_MOAT_OUT="$dir" GB_MOAT_N="$N" GB_MOAT_DIM="$DIM" GB_MOAT_RUNS="$RUNS" \
  GB_MOAT_K="${GB_MOAT_K:-10}" GB_MOAT_SEED="${GB_MOAT_SEED:-42}" \
  GB_MOAT_EDGES_PER_NODE="${GB_MOAT_EDGES_PER_NODE:-5}" \
  "$BIN" > "$dir/raw.log" 2> "$dir/stderr.log"
rc=$?
cat "$dir/raw.log"

metrics="$dir/moat_bench_metrics.json"
if [ "$rc" -ne 0 ] || [ ! -f "$metrics" ]; then
  echo "ERROR: moat-bench failed (rc=$rc) or no metrics produced" >&2
  exit 2
fi

gb_assemble_and_verify moat "$dir" "$metrics" null "${GB_ALLOW_DIRTY:-0}"
exit $?
