# 12-HOUR heavy soak (Windows). Long-running, disk- and RAM-heavy — run on a
# machine you can leave alone for half a day, NOT a GitHub-hosted runner.
#
#   .\benchmark\run_soak_12h.ps1                          # full 12h (43200s)
#   $env:SOAK_DURATION_SEC=3600; .\benchmark\run_soak_12h.ps1   # 1-hour soak
#   $env:SOAK_TMPDIR="D:\gsoak"; .\benchmark\run_soak_12h.ps1   # route DB to fast disk
#
# Output: benchmark\results\soak_heavy_12h\<ts>_<commit>\{result.json,raw.log,...}
# Verify later: python benchmark\verify_report.py <dir>\result.json
# Disk: ensure ~50+ GB free on SOAK_TMPDIR (or the repo drive) for a full run.
. (Join-Path (Split-Path -Parent $MyInvocation.MyCommand.Path) "_lib.ps1")

$duration = 43200
if ($env:SOAK_DURATION_SEC) { $duration = [int]$env:SOAK_DURATION_SEC }
$allowDirty = [bool]$env:GB_ALLOW_DIRTY

if ($duration -ge 43200) { $bid = "soak_heavy_12h" }
elseif ($duration -eq 3600) { $bid = "soak_1h" }
else { $bid = "soak_custom_${duration}s" }

Write-Host "Starting soak '$bid' for ${duration}s at $((Get-Date).ToUniversalTime())."
Write-Host "This will run for a long time. Logs stream to the run directory."
$script:GbExitCode = 1
Gb-RunSoak -Bid $bid -DurationSec $duration -AllowDirty $allowDirty
exit $script:GbExitCode
