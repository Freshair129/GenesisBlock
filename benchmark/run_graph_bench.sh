#!/usr/bin/env bash
# Graph traversal benchmark (k-hop neighbor latency p50/p95/p99 + BFS throughput).
# Self-contained: builds a seeded random graph, no external data needed.
#
#   benchmark/run_graph_bench.sh
#   GB_GRAPH_N=1000000 GB_GRAPH_FANOUT=8 benchmark/run_graph_bench.sh
#
# Output: benchmark/results/graph_traversal/<ts>_<commit>/{result.json,raw.log,...}
set -uo pipefail
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/_lib.sh"

N="${GB_GRAPH_N:-100000}"
FANOUT="${GB_GRAPH_FANOUT:-8}"
dir="$(gb_resultdir graph_traversal)"
echo "==> run dir: $dir (N=$N fanout=$FANOUT)"

gb_collect_env "$dir/env.json" "$dir" || exit 2

echo "==> building graph-bench (release)"
( cd "$GB_REPO_ROOT" && cargo build --release --no-default-features --features bins --bin graph-bench ) || exit 2
BIN="$GB_REPO_ROOT/target/release/graph-bench"; [ -x "$BIN.exe" ] && BIN="$BIN.exe"

GB_VBENCH="$dir" GB_GRAPH_N="$N" GB_GRAPH_FANOUT="$FANOUT" \
  "$BIN" > "$dir/raw.log" 2> "$dir/stderr.log"
rc=$?
cat "$dir/raw.log"

metrics="$dir/graph_bench_metrics.json"
if [ "$rc" -ne 0 ] || [ ! -f "$metrics" ]; then
  echo "ERROR: graph-bench failed (rc=$rc) or no metrics produced" >&2
  exit 2
fi

gb_assemble_and_verify graph_traversal "$dir" "$metrics" null "${GB_ALLOW_DIRTY:-0}"
exit $?
