#!/usr/bin/env bash
# Short soak SMOKE test — proves the whole pipeline end-to-end in ~2 minutes.
# Produces benchmark/results/soak_smoke/<ts>_<commit>/{result.json,raw.log,...}.
#
#   benchmark/run_smoke.sh                 # default 120s
#   SOAK_DURATION_SEC=60 benchmark/run_smoke.sh
#   GB_ALLOW_DIRTY=1 benchmark/run_smoke.sh   # accept a dirty tree (dev only)
#
# Tune via SOAK_* env (see tests/soak_tests.rs): SOAK_NODES_PER_CYCLE, SOAK_DIM,
# SOAK_COMPACT_EVERY, SOAK_QUERY_K, SOAK_EF_SEARCH.
set -uo pipefail
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/_lib.sh"

DURATION="${SOAK_DURATION_SEC:-120}"
gb_run_soak "soak_smoke" "$DURATION"
exit $?
