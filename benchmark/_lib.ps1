# Shared helpers for the Independent Benchmark Suite runner scripts (Windows).
# Dot-sourced by run_smoke.ps1 / run_soak_12h.ps1 / run_graph_bench.ps1 /
# run_vector_bench.ps1. Requires git, cargo, and python on PATH.

# NOTE: do NOT set $ErrorActionPreference = "Stop" globally. Under Windows
# PowerShell 5.1, native tools (cargo, git) write progress to stderr, which Stop
# turns into terminating NativeCommandError records. We check $LASTEXITCODE /
# Test-Path explicitly instead.
$GbBenchDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$GbRepoRoot = Split-Path -Parent $GbBenchDir
$GbPython = (Get-Command python -ErrorAction SilentlyContinue).Source
if (-not $GbPython) { $GbPython = (Get-Command python3 -ErrorAction SilentlyContinue).Source }

function Gb-Ts    { (Get-Date).ToUniversalTime().ToString("yyyyMMddTHHmmssZ") }
function Gb-Short {
  $s = (git -C $GbRepoRoot rev-parse --short HEAD 2>$null)
  if (-not $s) { return "nogit" }
  return $s.Trim()
}

function Gb-ResultDir([string]$Bid) {
  $dir = Join-Path $GbRepoRoot "benchmark/results/$Bid/$(Gb-Ts)_$(Gb-Short)"
  New-Item -ItemType Directory -Force -Path $dir | Out-Null
  return $dir
}

function Gb-CollectEnv([string]$OutJson, [string]$DiskTarget) {
  & $GbPython (Join-Path $GbBenchDir "collect_env.py") --out $OutJson --disk-target $DiskTarget
}

# Assembles result.json + summary.md, then verifies. Stores the verifier exit
# code in $script:GbVerifyExit (NOT a function return value: PowerShell functions
# emit the whole success stream as their return, so returning the int alongside
# python's stdout would yield an array and break `exit`). Python output streams
# straight to the console.
function Gb-AssembleAndVerify {
  param([string]$Bid, [string]$Dir, [string]$Metrics, $PeakMb, [bool]$AllowDirty = $false)
  $script:GbVerifyExit = 1
  $peakArg = if ($null -eq $PeakMb) { "null" } else { "$PeakMb" }
  & $GbPython (Join-Path $GbBenchDir "assemble_result.py") `
    --metrics $Metrics --env (Join-Path $Dir "env.json") --out (Join-Path $Dir "result.json") `
    --benchmark-id $Bid --repo-root $GbRepoRoot --peak-ram-mb $peakArg `
    --raw-log (Join-Path $Dir "raw.log") --stderr-log (Join-Path $Dir "stderr.log") `
    --summary (Join-Path $Dir "summary.md")
  if ($LASTEXITCODE -ne 0) { $script:GbVerifyExit = 3; return }
  $vargs = @((Join-Path $Dir "result.json"))
  if ($AllowDirty) { $vargs += "--allow-dirty" }
  & $GbPython (Join-Path $GbBenchDir "verify_report.py") @vargs
  $script:GbVerifyExit = $LASTEXITCODE
}

# Builds the soak_tests binary (release, --no-default-features) and returns its path.
function Gb-BuildSoakExe {
  Write-Host "==> building soak_tests (release, --no-default-features)"
  Push-Location $GbRepoRoot
  try {
    # 2>$null drops cargo's human-readable stderr progress; --message-format=json
    # keeps the machine-readable target records on stdout for parsing.
    $json = cargo test --no-default-features --test soak_tests --release --no-run --message-format=json 2>$null
  } finally { Pop-Location }
  foreach ($line in $json) {
    $t = $line.Trim()
    if (-not $t.StartsWith("{")) { continue }
    try { $m = $t | ConvertFrom-Json } catch { continue }
    if ($m.target.name -eq "soak_tests" -and $m.executable) { return $m.executable }
  }
  return $null
}

# Drives a duration-bounded soak (the `soak_heavy` test). Returns an exit code:
# non-zero if the soak binary failed or the verifier rejected the report.
function Gb-RunSoak {
  param([string]$Bid, [int]$DurationSec, [bool]$AllowDirty = $false)
  $dir = Gb-ResultDir $Bid
  Write-Host "==> run dir: $dir"
  Write-Host "==> benchmark_id=$Bid duration_target=${DurationSec}s"

  Gb-CollectEnv (Join-Path $dir "env.json") $dir | Out-Null

  $exe = Gb-BuildSoakExe
  if (-not $exe -or -not (Test-Path $exe)) {
    Write-Error "could not build/locate soak_tests binary"; $script:GbExitCode = 2; return
  }
  Write-Host "==> soak binary: $exe"

  $env:SOAK_DURATION_SEC = "$DurationSec"
  $env:SOAK_BENCHMARK_ID = $Bid
  $env:SOAK_RESULT_JSON  = (Join-Path $dir "metrics.json")

  $proc = Start-Process -FilePath $exe `
    -ArgumentList "--ignored","--nocapture","soak_heavy" `
    -RedirectStandardOutput (Join-Path $dir "raw.log") `
    -RedirectStandardError  (Join-Path $dir "stderr.log") `
    -NoNewWindow -PassThru
  # Touch .Handle so the process object caches the exit code (Start-Process
  # otherwise leaves $proc.ExitCode null once the handle is released).
  $null = $proc.Handle

  $interval = 10
  if ($env:GB_PEAK_INTERVAL) { $interval = [int]$env:GB_PEAK_INTERVAL }
  $peakBytes = 0
  while (-not $proc.HasExited) {
    try { $proc.Refresh(); if ($proc.PeakWorkingSet64 -gt $peakBytes) { $peakBytes = $proc.PeakWorkingSet64 } } catch {}
    Start-Sleep -Seconds $interval
  }
  $proc.WaitForExit()
  $rc = $proc.ExitCode
  $peakMb = if ($peakBytes -gt 0) { [math]::Round($peakBytes / 1MB, 1) } else { $null }

  Write-Host "==> soak exit=$rc peak_ram_mb=$peakMb"
  if (Test-Path (Join-Path $dir "raw.log")) { Get-Content (Join-Path $dir "raw.log") -Tail 15 }

  if (-not (Test-Path (Join-Path $dir "metrics.json"))) {
    Write-Error "no metrics.json produced (soak crashed before writing)"; $script:GbExitCode = 2; return
  }

  Gb-AssembleAndVerify -Bid $Bid -Dir $dir -Metrics (Join-Path $dir "metrics.json") -PeakMb $peakMb -AllowDirty $AllowDirty
  $vrc = $script:GbVerifyExit
  Write-Host "==> result: $(Join-Path $dir 'result.json')  (soak rc=$rc, verify rc=$vrc)"
  # Fail on EITHER a failed soak process or a rejected report.
  if ($rc -ne 0) { $script:GbExitCode = $rc } else { $script:GbExitCode = $vrc }
}
