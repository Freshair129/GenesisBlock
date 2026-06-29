# Short soak SMOKE test (Windows) — proves the pipeline end-to-end in ~2 minutes.
#
#   .\benchmark\run_smoke.ps1
#   $env:SOAK_DURATION_SEC=60; .\benchmark\run_smoke.ps1
#   $env:GB_ALLOW_DIRTY=1; .\benchmark\run_smoke.ps1   # accept a dirty tree (dev only)
#
# Output: benchmark\results\soak_smoke\<ts>_<commit>\{result.json,raw.log,...}
. (Join-Path (Split-Path -Parent $MyInvocation.MyCommand.Path) "_lib.ps1")

$duration = 120
if ($env:SOAK_DURATION_SEC) { $duration = [int]$env:SOAK_DURATION_SEC }
$allowDirty = [bool]$env:GB_ALLOW_DIRTY

$script:GbExitCode = 1
Gb-RunSoak -Bid "soak_smoke" -DurationSec $duration -AllowDirty $allowDirty
exit $script:GbExitCode
