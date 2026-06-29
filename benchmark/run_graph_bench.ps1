# Graph traversal benchmark (Windows). Self-contained seeded random graph.
#
#   .\benchmark\run_graph_bench.ps1
#   $env:GB_GRAPH_N=1000000; .\benchmark\run_graph_bench.ps1
#
# Output: benchmark\results\graph_traversal\<ts>_<commit>\{result.json,raw.log,...}
. (Join-Path (Split-Path -Parent $MyInvocation.MyCommand.Path) "_lib.ps1")

$N = 100000; if ($env:GB_GRAPH_N) { $N = [int]$env:GB_GRAPH_N }
$fanout = 8; if ($env:GB_GRAPH_FANOUT) { $fanout = [int]$env:GB_GRAPH_FANOUT }
$allowDirty = [bool]$env:GB_ALLOW_DIRTY

$dir = Gb-ResultDir "graph_traversal"
Write-Host "==> run dir: $dir (N=$N fanout=$fanout)"
Gb-CollectEnv (Join-Path $dir "env.json") $dir | Out-Null

Write-Host "==> building graph-bench (release)"
Push-Location $GbRepoRoot
try { cargo build --release --no-default-features --features bins --bin graph-bench } finally { Pop-Location }
if ($LASTEXITCODE -ne 0) { exit 2 }
$bin = Join-Path $GbRepoRoot "target/release/graph-bench.exe"

$env:GB_VBENCH = $dir; $env:GB_GRAPH_N = "$N"; $env:GB_GRAPH_FANOUT = "$fanout"
$proc = Start-Process -FilePath $bin `
  -RedirectStandardOutput (Join-Path $dir "raw.log") `
  -RedirectStandardError  (Join-Path $dir "stderr.log") -NoNewWindow -PassThru
$null = $proc.Handle   # cache exit code (Start-Process quirk)
$proc.WaitForExit()
Get-Content (Join-Path $dir "raw.log")

$metrics = Join-Path $dir "graph_bench_metrics.json"
if ($proc.ExitCode -ne 0 -or -not (Test-Path $metrics)) {
  Write-Error "graph-bench failed (rc=$($proc.ExitCode)) or no metrics produced"; exit 2
}
Gb-AssembleAndVerify -Bid "graph_traversal" -Dir $dir -Metrics $metrics -PeakMb $null -AllowDirty $allowDirty
exit $script:GbVerifyExit
