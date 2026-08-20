#!/usr/bin/env bash
# G3 moat bench (WP-3.2) — the engine's fused vector+graph+AS-OF jobs vs the
# DIY single-SQLite-file assembly (brute f32 scan + recursive CTE + shared RRF
# glue + audit-history temporal pattern), both in-process in one Rust binary.
# Self-contained: deterministic seeded corpus, no model downloads.
#
#   benchmark/run_moat_bench.sh
#   GB_MOAT_N=100000 GB_MOAT_DIM=1024 GB_MOAT_RUNS=30 benchmark/run_moat_bench.sh
#
# WP-3.3 follow-ups (both optional, both off by default so the clone-and-run
# path stays self-contained):
#   GB_MOAT_LIBSQL=1        also measure the libSQL/DiskANN baseline rows
#                           (compiles the `libsql-baseline` feature: ~2 min).
#   GB_MOAT_VECTORS=<f32>   use a REAL embedding corpus instead of synthetic
#                           vectors — build one with
#                           `python benchmark/gen_corpus_bge_m3.py` and pass
#                           GB_MOAT_DIM matching its manifest.
#
# Output: benchmark/results/moat/<ts>_<commit>/{result.json,raw.log,...}
set -uo pipefail
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/_lib.sh"

N="${GB_MOAT_N:-100000}"; DIM="${GB_MOAT_DIM:-1024}"; RUNS="${GB_MOAT_RUNS:-30}"
# Real-corpus mode is passed through the environment (the binaries read it with
# std::env::var). Export rather than inlining it into the command prefix: a
# `${VAR:+VAR=...}` expansion is NOT parsed as an assignment, it is parsed as a
# command. On Windows use a path the *binary* can open (a Git Bash `/g/...`
# path is not resolvable by a native exe) — a repo-relative path works for both.
[ -n "${GB_MOAT_VECTORS:-}" ] && export GB_MOAT_VECTORS
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

# WP-3.3 follow-up 1: the libSQL/DiskANN rows run as a SEPARATE process —
# libsql-ffi and rusqlite export the same sqlite3_* symbols, so they cannot
# share a binary soundly (see benches/moat_libsql.rs). Same host, same seeded
# corpus, same protocol; its metrics land beside the engine run's.
if [ "${GB_MOAT_LIBSQL:-0}" = "1" ]; then
  echo "==> building moat-libsql (release, +libsql-baseline)"
  ( cd "$GB_REPO_ROOT" && cargo build --release --no-default-features \
      --features "bins,libsql-baseline" --bin moat-libsql ) || exit 2
  LBIN="$GB_REPO_ROOT/target/release/moat-libsql"; [ -x "$LBIN.exe" ] && LBIN="$LBIN.exe"
  GB_MOAT_OUT="$dir" GB_MOAT_N="$N" GB_MOAT_DIM="$DIM" GB_MOAT_RUNS="$RUNS" \
    GB_MOAT_K="${GB_MOAT_K:-10}" GB_MOAT_SEED="${GB_MOAT_SEED:-42}" \
    GB_MOAT_EDGES_PER_NODE="${GB_MOAT_EDGES_PER_NODE:-5}" \
    "$LBIN" > "$dir/raw_libsql.log" 2> "$dir/stderr_libsql.log"
  lrc=$?
  cat "$dir/raw_libsql.log"
  if [ "$lrc" -ne 0 ] || [ ! -f "$dir/moat_libsql_metrics.json" ]; then
    echo "ERROR: moat-libsql failed (rc=$lrc) or no metrics produced" >&2
    exit 2
  fi
fi

gb_assemble_and_verify moat "$dir" "$metrics" null "${GB_ALLOW_DIRTY:-0}"
exit $?
