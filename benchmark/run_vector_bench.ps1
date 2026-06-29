# Vector search benchmark (Windows). k-NN latency + REAL recall@k vs exact
# brute-force ground truth. Self-contained seeded random vectors, no downloads.
#
#   .\benchmark\run_vector_bench.ps1
#   $env:GB_VEC_N=200000; $env:GB_VEC_DIM=256; .\benchmark\run_vector_bench.ps1
#
# Output: benchmark\results\vector_search\<ts>_<commit>\{result.json,raw.log,...}
. (Join-Path (Split-Path -Parent $MyInvocation.MyCommand.Path) "_lib.ps1")

$N = 50000;  if ($env:GB_VEC_N)   { $N   = [int]$env:GB_VEC_N }
$dim = 128;  if ($env:GB_VEC_DIM) { $dim = [int]$env:GB_VEC_DIM }
$Q = 1000;   if ($env:GB_VEC_Q)   { $Q   = [int]$env:GB_VEC_Q }
$allowDirty = [bool]$env:GB_ALLOW_DIRTY

$dir = Gb-ResultDir "vector_search"
Write-Host "==> run dir: $dir (N=$N dim=$dim Q=$Q)"
Gb-CollectEnv (Join-Path $dir "env.json") $dir | Out-Null

Write-Host "==> building vector-bench (release)"
Push-Location $GbRepoRoot
try { cargo build --release --no-default-features --features bins --bin vector-bench } finally { Pop-Location }
if ($LASTEXITCODE -ne 0) { exit 2 }
$bin = Join-Path $GbRepoRoot "target/release/vector-bench.exe"

$env:GB_VEC_OUT = $dir; $env:GB_VEC_N = "$N"; $env:GB_VEC_DIM = "$dim"; $env:GB_VEC_Q = "$Q"
if (-not $env:GB_VEC_K)    { $env:GB_VEC_K = "10" }
if (-not $env:GB_VEC_EF)   { $env:GB_VEC_EF = "200" }
if (-not $env:GB_VEC_SEED) { $env:GB_VEC_SEED = "42" }
$proc = Start-Process -FilePath $bin `
  -RedirectStandardOutput (Join-Path $dir "raw.log") `
  -RedirectStandardError  (Join-Path $dir "stderr.log") -NoNewWindow -PassThru
$null = $proc.Handle   # cache exit code (Start-Process quirk)
$proc.WaitForExit()
Get-Content (Join-Path $dir "raw.log")

$metrics = Join-Path $dir "vector_bench_metrics.json"
if ($proc.ExitCode -ne 0 -or -not (Test-Path $metrics)) {
  Write-Error "vector-bench failed (rc=$($proc.ExitCode)) or no metrics produced"; exit 2
}
Gb-AssembleAndVerify -Bid "vector_search" -Dir $dir -Metrics $metrics -PeakMb $null -AllowDirty $allowDirty
exit $script:GbVerifyExit
