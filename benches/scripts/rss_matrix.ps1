# MARK XV P1 — RSS + on-disk footprint x quant x scale matrix runner.
# Sweeps {none,sq8,bq} x {rerank 0/1} over the 500k and 1M corpora, recording for
# each config: engine RSS (MB, corpus streamed so no harness buffer), the persisted
# gdb snapshot size on disk (MB), and free space on C:. Results -> rss_disk_matrix.txt.
#
# RAM note: the bin streams corpus.f32 from disk (no 8 GB upfront load), so quantized
# configs fit a 32 GB box at 1M; f32 @1M (~11 GB engine) may still OOM and is recorded
# as FAILED rather than aborting the sweep.
# Disk note: each run wipes+rebuilds gdb, so peak disk = corpus + one gdb snapshot.
$ErrorActionPreference = "Stop"
Write-Host "Building release bin..."
cargo build --release --features bins --bin vbench-genesis
if ($LASTEXITCODE -ne 0) { throw "build failed" }
$exe = "target\release\vbench-genesis.exe"
$env:GB_EF = "200"
Remove-Item Env:\GB_LIMIT -ErrorAction SilentlyContinue

$scales = @(
  @{ name = "500k"; dir = "C:\Users\freshair\gb_vbench_500k" },
  @{ name = "1m";   dir = "C:\Users\freshair\gb_vbench_1m" }
)
$configs = @(@("none","0"), @("sq8","0"), @("sq8","1"), @("bq","0"), @("bq","1"))
$out = "C:\Users\freshair\rss_disk_matrix.txt"
"# scale quant rerank disk_mb freeC_gb | <bin summary line>  (MARK XV P1, main 1c030b9 + harness ext, streamed corpus)" | Out-File -FilePath $out -Encoding utf8

foreach ($s in $scales) {
  $env:GB_VBENCH = $s.dir
  $gdb = Join-Path $s.dir "gdb"
  if (-not (Test-Path (Join-Path $s.dir "corpus.f32"))) { Write-Host "skip $($s.name): no corpus"; continue }
  foreach ($c in $configs) {
    $env:GB_QUANT = $c[0]; $env:GB_RERANK = $c[1]
    Write-Host "RUN $($s.name) quant=$($c[0]) rerank=$($c[1])"
    $line = ""
    try { $line = (& $exe | Select-String "^GenesisBlockDB:").Line } catch { $line = "" }
    $diskMB = 0
    if (Test-Path $gdb) {
      $sum = (Get-ChildItem $gdb -Recurse -File -ErrorAction SilentlyContinue | Measure-Object Length -Sum).Sum
      if ($sum) { $diskMB = [math]::Round($sum / 1MB, 0) }
    }
    $freeC = [math]::Round((Get-PSDrive C).Free / 1GB, 1)
    if (-not $line) { $line = "FAILED (likely OOM)" }
    "$($s.name) $($c[0]) $($c[1]) disk_mb=$diskMB freeC_gb=$freeC | $line" | Out-File -FilePath $out -Append -Encoding utf8
    Write-Host "  disk=$diskMB MB, freeC=$freeC GB"
  }
}
Write-Host "MATRIX DONE -> $out"
Get-Content $out
