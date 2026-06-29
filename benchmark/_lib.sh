#!/usr/bin/env bash
# Shared helpers for the Independent Benchmark Suite runner scripts (Linux/macOS).
# Sourced by run_smoke.sh / run_soak_12h.sh / run_graph_bench.sh / run_vector_bench.sh.
#
# Portable (bash 3.2+), no extra deps beyond git, cargo, and python3.
set -uo pipefail

# Repo root = parent of this script's directory (benchmark/).
GB_BENCH_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
GB_REPO_ROOT="$(cd "$GB_BENCH_DIR/.." && pwd)"

# Python interpreter: prefer python3, fall back to python (Windows/Git Bash).
if command -v python3 >/dev/null 2>&1; then GB_PY=python3
elif command -v python >/dev/null 2>&1; then GB_PY=python
else echo "ERROR: need python3 or python on PATH" >&2; GB_PY=python3; fi

gb_ts()    { date -u +%Y%m%dT%H%M%SZ; }
gb_short() { git -C "$GB_REPO_ROOT" rev-parse --short HEAD 2>/dev/null || echo "nogit"; }

# gb_resultdir <benchmark_id> -> prints (and creates) the run directory.
gb_resultdir() {
  local bid="$1"
  local dir="$GB_REPO_ROOT/benchmark/results/$bid/$(gb_ts)_$(gb_short)"
  mkdir -p "$dir"
  printf '%s\n' "$dir"
}

# gb_collect_env <out.json> <disk-target>
gb_collect_env() {
  $GB_PY "$GB_BENCH_DIR/collect_env.py" --out "$1" --disk-target "${2:-$GB_REPO_ROOT}"
}

# gb_monitor_peak <pid> <outfile> [interval_sec]
# Tracks peak RSS (MB) of <pid> while it lives. Prefers /proc VmHWM (true peak)
# and falls back to `ps rss` (sampled current). Best-effort: writes the latest
# peak to <outfile> continuously so the value survives even if killed.
gb_monitor_peak() {
  local pid="$1" out="$2" interval="${3:-5}" peak=0 rss
  while kill -0 "$pid" 2>/dev/null; do
    if [ -r "/proc/$pid/status" ]; then
      rss=$(awk '/VmHWM/{print $2}' "/proc/$pid/status" 2>/dev/null)
    else
      rss=$(ps -o rss= -p "$pid" 2>/dev/null | tr -d ' ')
    fi
    if [ -n "${rss:-}" ] && [ "$rss" -gt "$peak" ] 2>/dev/null; then peak="$rss"; fi
    echo $(( peak / 1024 )) > "$out"
    sleep "$interval"
  done
  echo $(( peak / 1024 )) > "$out"
}

# gb_build_soak_exe -> prints the compiled soak_tests binary path (release).
gb_build_soak_exe() {
  echo "==> building soak_tests (release, --no-default-features)" >&2
  ( cd "$GB_REPO_ROOT" && cargo test --no-default-features --test soak_tests --release --no-run --message-format=json ) \
    | $GB_PY -c '
import sys, json
exe = ""
for line in sys.stdin:
    line = line.strip()
    if not line.startswith("{"):
        continue
    try:
        m = json.loads(line)
    except Exception:
        continue
    if m.get("target", {}).get("name") == "soak_tests" and m.get("executable"):
        exe = m["executable"]
print(exe)
'
}

# gb_assemble_and_verify <benchmark_id> <dir> <metrics.json> <peak_mb|null> <allow_dirty:0|1>
# Assembles result.json + summary.md, then verifies. Returns verifier exit code.
gb_assemble_and_verify() {
  local bid="$1" dir="$2" metrics="$3" peak="$4" allow_dirty="${5:-0}"
  $GB_PY "$GB_BENCH_DIR/assemble_result.py" \
    --metrics "$metrics" --env "$dir/env.json" --out "$dir/result.json" \
    --benchmark-id "$bid" --repo-root "$GB_REPO_ROOT" --peak-ram-mb "$peak" \
    --raw-log "$dir/raw.log" --stderr-log "$dir/stderr.log" --summary "$dir/summary.md" || return 3
  local vargs=("$dir/result.json")
  [ "$allow_dirty" = "1" ] && vargs+=(--allow-dirty)
  $GB_PY "$GB_BENCH_DIR/verify_report.py" "${vargs[@]}"
}

# gb_run_soak <benchmark_id> <duration_sec>
# Drives a duration-bounded soak (the `soak_heavy` test) and produces the full
# run directory. Honors SOAK_* env overrides (see tests/soak_tests.rs). Exits
# non-zero if the soak binary fails OR the verifier rejects the report.
gb_run_soak() {
  local bid="$1" duration="$2"
  local dir; dir="$(gb_resultdir "$bid")"
  echo "==> run dir: $dir"
  echo "==> benchmark_id=$bid duration_target=${duration}s"

  gb_collect_env "$dir/env.json" "$dir" || { echo "env capture failed" >&2; return 2; }

  local exe; exe="$(gb_build_soak_exe)"
  if [ -z "$exe" ] || [ ! -x "$exe" ]; then
    echo "ERROR: could not build/locate soak_tests binary" >&2
    return 2
  fi
  echo "==> soak binary: $exe"

  # Route the database to a fast scratch dir if SOAK_TMPDIR is set by the caller.
  local peak_file="$dir/.peak_mb"
  echo 0 > "$peak_file"

  SOAK_DURATION_SEC="$duration" \
  SOAK_BENCHMARK_ID="$bid" \
  SOAK_RESULT_JSON="$dir/metrics.json" \
  "$exe" --ignored --nocapture soak_heavy \
    > "$dir/raw.log" 2> "$dir/stderr.log" &
  local bpid=$!
  gb_monitor_peak "$bpid" "$peak_file" "${GB_PEAK_INTERVAL:-10}" &
  local mpid=$!

  wait "$bpid"; local rc=$?
  kill "$mpid" 2>/dev/null; wait "$mpid" 2>/dev/null
  local peak; peak="$(cat "$peak_file" 2>/dev/null || echo null)"
  [ "$peak" = "0" ] && peak="null"
  rm -f "$peak_file"

  echo "==> soak exit=$rc peak_ram_mb=$peak"
  tail -n 15 "$dir/raw.log" 2>/dev/null || true

  if [ ! -f "$dir/metrics.json" ]; then
    echo "ERROR: no metrics.json produced (soak crashed before writing)" >&2
    return 2
  fi

  gb_assemble_and_verify "$bid" "$dir" "$dir/metrics.json" "$peak" "${GB_ALLOW_DIRTY:-0}"
  local vrc=$?
  echo "==> result: $dir/result.json  (soak rc=$rc, verify rc=$vrc)"
  [ "$rc" -ne 0 ] && return "$rc"
  return "$vrc"
}
