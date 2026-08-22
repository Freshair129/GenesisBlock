# G3 moat bench (WP-3.2, Windows) — the engine's fused vector+graph+AS-OF jobs
# vs the DIY single-SQLite-file assembly (brute f32 scan + recursive CTE +
# shared RRF glue + audit-history temporal pattern), both in-process in one
# Rust binary. Self-contained seeded corpus, no downloads.
#
#   .\benchmark\run_moat_bench.ps1
#   $env:GB_MOAT_N=100000; $env:GB_MOAT_DIM=1024; .\benchmark\run_moat_bench.ps1
#
# WP-3.3 follow-ups (both optional, off by default so the clone-and-run path
# stays self-contained):
#   $env:GB_MOAT_LIBSQL=1      also measure the libSQL/DiskANN baseline rows
#                              (compiles the `libsql-baseline` feature: ~2 min).
#   $env:GB_MOAT_VECTORS=<f32> use a REAL embedding corpus instead of synthetic
#                              vectors — build one with
#                              `python benchmark\gen_corpus_bge_m3.py` and set
#                              GB_MOAT_DIM to match its manifest.
#
# Output: benchmark\results\moat\<ts>_<commit>\{result.json,raw.log,...}
. (Join-Path (Split-Path -Parent $MyInvocation.MyCommand.Path) "_lib.ps1")

$N = 100000;  if ($env:GB_MOAT_N)    { $N    = [int]$env:GB_MOAT_N }
$dim = 1024;  if ($env:GB_MOAT_DIM)  { $dim  = [int]$env:GB_MOAT_DIM }
$runs = 30;   if ($env:GB_MOAT_RUNS) { $runs = [int]$env:GB_MOAT_RUNS }
$allowDirty = [bool]$env:GB_ALLOW_DIRTY

$dir = Gb-ResultDir "moat"
Write-Host "==> run dir: $dir (N=$N dim=$dim runs=$runs)"
Gb-CollectEnv (Join-Path $dir "env.json") $dir | Out-Null

Write-Host "==> building moat-bench (release)"
Push-Location $GbRepoRoot
try { cargo build --release --no-default-features --features bins --bin moat-bench } finally { Pop-Location }
if ($LASTEXITCODE -ne 0) { exit 2 }
$bin = Join-Path $GbRepoRoot "target/release/moat-bench.exe"

$env:GB_MOAT_OUT = $dir; $env:GB_MOAT_N = "$N"; $env:GB_MOAT_DIM = "$dim"; $env:GB_MOAT_RUNS = "$runs"
if (-not $env:GB_MOAT_K)              { $env:GB_MOAT_K = "10" }
if (-not $env:GB_MOAT_SEED)           { $env:GB_MOAT_SEED = "42" }
if (-not $env:GB_MOAT_EDGES_PER_NODE) { $env:GB_MOAT_EDGES_PER_NODE = "5" }
$proc = Start-Process -FilePath $bin `
  -RedirectStandardOutput (Join-Path $dir "raw.log") `
  -RedirectStandardError  (Join-Path $dir "stderr.log") -NoNewWindow -PassThru
$null = $proc.Handle   # cache exit code (Start-Process quirk)
$proc.WaitForExit()
Get-Content (Join-Path $dir "raw.log")

$metrics = Join-Path $dir "moat_bench_metrics.json"
if ($proc.ExitCode -ne 0 -or -not (Test-Path $metrics)) {
  Write-Error "moat-bench failed (rc=$($proc.ExitCode)) or no metrics produced"; exit 2
}

# WP-3.3 follow-up 1: libSQL/DiskANN runs as a SEPARATE process (libsql-ffi and
# rusqlite export the same sqlite3_* symbols — see benches/moat_libsql.rs).
if ($env:GB_MOAT_LIBSQL -eq "1") {
  Write-Host "==> building moat-libsql (release, +libsql-baseline)"
  Push-Location $GbRepoRoot
  try { cargo build --release --no-default-features --features "bins,libsql-baseline" --bin moat-libsql } finally { Pop-Location }
  if ($LASTEXITCODE -ne 0) { exit 2 }
  $lbin = Join-Path $GbRepoRoot "target/release/moat-libsql.exe"
  $lproc = Start-Process -FilePath $lbin `
    -RedirectStandardOutput (Join-Path $dir "raw_libsql.log") `
    -RedirectStandardError  (Join-Path $dir "stderr_libsql.log") -NoNewWindow -PassThru
  $null = $lproc.Handle
  $lproc.WaitForExit()
  Get-Content (Join-Path $dir "raw_libsql.log")
  if ($lproc.ExitCode -ne 0 -or -not (Test-Path (Join-Path $dir "moat_libsql_metrics.json"))) {
    Write-Error "moat-libsql failed (rc=$($lproc.ExitCode)) or no metrics produced"; exit 2
  }
}
Gb-AssembleAndVerify -Bid "moat" -Dir $dir -Metrics $metrics -PeakMb $null -AllowDirty $allowDirty
