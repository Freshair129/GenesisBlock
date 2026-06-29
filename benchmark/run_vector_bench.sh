#!/usr/bin/env bash
# Vector search benchmark — k-NN latency (p50/p95/p99) + REAL recall@k vs an exact
# brute-force ground truth. Self-contained: deterministic seeded random vectors,
# no model download required.
#
#   benchmark/run_vector_bench.sh
#   GB_VEC_N=200000 GB_VEC_DIM=256 GB_VEC_Q=2000 benchmark/run_vector_bench.sh
#
# Output: benchmark/results/vector_search/<ts>_<commit>/{result.json,raw.log,...}
set -uo pipefail
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/_lib.sh"

N="${GB_VEC_N:-50000}"; DIM="${GB_VEC_DIM:-128}"; Q="${GB_VEC_Q:-1000}"
dir="$(gb_resultdir vector_search)"
echo "==> run dir: $dir (N=$N dim=$DIM Q=$Q)"

gb_collect_env "$dir/env.json" "$dir" || exit 2

echo "==> building vector-bench (release)"
( cd "$GB_REPO_ROOT" && cargo build --release --no-default-features --features bins --bin vector-bench ) || exit 2
BIN="$GB_REPO_ROOT/target/release/vector-bench"; [ -x "$BIN.exe" ] && BIN="$BIN.exe"

GB_VEC_OUT="$dir" GB_VEC_N="$N" GB_VEC_DIM="$DIM" GB_VEC_Q="$Q" \
  GB_VEC_K="${GB_VEC_K:-10}" GB_VEC_EF="${GB_VEC_EF:-200}" GB_VEC_SEED="${GB_VEC_SEED:-42}" \
  "$BIN" > "$dir/raw.log" 2> "$dir/stderr.log"
rc=$?
cat "$dir/raw.log"

metrics="$dir/vector_bench_metrics.json"
if [ "$rc" -ne 0 ] || [ ! -f "$metrics" ]; then
  echo "ERROR: vector-bench failed (rc=$rc) or no metrics produced" >&2
  exit 2
fi

gb_assemble_and_verify vector_search "$dir" "$metrics" null "${GB_ALLOW_DIRTY:-0}"
exit $?
