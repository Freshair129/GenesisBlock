#!/usr/bin/env bash
# 12-HOUR heavy soak. Long-running, disk- and RAM-heavy — run on a machine you
# can leave alone for half a day (a self-hosted runner or a spare box), NOT a
# GitHub-hosted runner (6h job cap, shared noisy I/O).
#
#   benchmark/run_soak_12h.sh                       # full 12h (43200s)
#   SOAK_DURATION_SEC=3600 benchmark/run_soak_12h.sh   # 1-hour soak instead
#   SOAK_TMPDIR=/mnt/ssd/gsoak benchmark/run_soak_12h.sh   # route DB to fast disk
#
# Output: benchmark/results/soak_heavy_12h/<ts>_<commit>/{result.json,raw.log,
#         stderr.log,env.json,summary.md}. Verify later with:
#         python3 benchmark/verify_report.py <dir>/result.json
#
# Disk: a full 12h run at defaults ingests tens of millions of nodes; ensure
# ~50+ GB free on SOAK_TMPDIR (or the repo drive). See BENCHMARKING.md.
set -uo pipefail
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/_lib.sh"

DURATION="${SOAK_DURATION_SEC:-43200}"
# Pick a stable id: a sub-12h duration is reported under soak_1h / soak_custom so
# the verifier does not hold it to the 12h >=43200s rule.
if [ "$DURATION" -ge 43200 ]; then
  BID="soak_heavy_12h"
elif [ "$DURATION" -eq 3600 ]; then
  BID="soak_1h"
else
  BID="soak_custom_${DURATION}s"
fi

echo "Starting soak '$BID' for ${DURATION}s at $(date -u)."
echo "This will run for a long time. Logs stream to the run directory."
gb_run_soak "$BID" "$DURATION"
exit $?
