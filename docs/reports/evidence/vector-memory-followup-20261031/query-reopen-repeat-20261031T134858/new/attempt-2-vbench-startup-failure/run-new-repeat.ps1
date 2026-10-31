$ErrorActionPreference='Stop'
$run='C:\Users\suanr\genesis-vector-memory-followup-20261031\query-reopen-repeat-20261031T134858\new'
$db=Join-Path $run 'gdb'
$exe='C:\Users\suanr\genesis-vector-memory-followup-20261031\candidate-repeat-build-20261031T131156\candidate-vbench-genesis-repeat.exe'
$input='C:\Users\suanr\genesis-vector-memory-20261031\inputs\synthetic-100000-1024'
$expected='959DFA8F6348082046BCBFAD744A422E2BD7F5A43C702C2DBA7D2D86D26AEAF1'
$actual=(Get-FileHash -LiteralPath $exe -Algorithm SHA256).Hash
if($actual -ne $expected){throw "candidate binary hash mismatch: $actual"}
$admission=Get-Content -LiteralPath 'C:\Users\suanr\genesis-vector-memory-followup-20261031\query-reopen-repeat-20261031T134858\new\new-preflight-attempt3.json' -Raw | ConvertFrom-Json
if(-not $admission.preflight_pass){throw 'candidate admission preflight did not pass'}
$env:GB_MEMORY_OPEN_ONLY=$db
$env:GB_MEMORY_OPEN_OUT=Join-Path $run 'repeat-open-only.json'
$env:GB_MEMORY_OPEN_REPEAT='2'
$env:GB_MEMORY_OPEN_READ_WRITE='1'
$env:GB_MEMORY_OPEN_QUERY_SAMPLES='120'
$env:GB_MEMORY_INPUT=$input
$env:GB_VBENCH=$input
$env:GB_MEMORY_RSS_LIMIT_BYTES='2147483648'
$env:GB_VECTOR_QUERY_DIAGNOSTICS='1'
$env:RAYON_NUM_THREADS='4'
$env:GB_EF='200'
$stdout=Join-Path $run 'stdout.log'
$stderr=Join-Path $run 'stderr.log'
$monitor=Join-Path $run 'resource-monitor.jsonl'
$start=Get-Date
$proc=Start-Process -FilePath $exe -WindowStyle Hidden -PassThru -RedirectStandardOutput $stdout -RedirectStandardError $stderr
$pidOwned=$proc.Id
$started=(Get-Date).ToString('o')
@{pid=$pidOwned;started_local=$started;binary_sha256=$actual;query_budget_default_elapsed_ms=5000;rss_limit_bytes=2147483648;repeat=2;page_cache_mb=64;read_only=$false;vector_dim=1024;retention='full';rayon_num_threads=4;ef_search=200;k=10;alpha=0;sample_count=120;input=$input;db=$db} | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $run 'process-start.json') -Encoding utf8
Write-Output "CANDIDATE_STARTED pid=$pidOwned binary=$actual time=$started"
$clock=[Diagnostics.Stopwatch]::StartNew()
$peakWs=0L;$peakPrivate=0L;$killed=$false;$killReason=$null;$lastCpu=-30;$cpuSamples=[System.Collections.Generic.List[object]]::new()
while($true){
  $p=Get-Process -Id $pidOwned -ErrorAction SilentlyContinue
  if($null -eq $p){break}
  $ws=[long]$p.WorkingSet64;$private=[long]$p.PrivateMemorySize64
  if($ws -gt $peakWs){$peakWs=$ws};if($private -gt $peakPrivate){$peakPrivate=$private}
  $ram=[double](Get-CimInstance Win32_OperatingSystem).FreePhysicalMemory*1024
  $entry=[pscustomobject]@{timestamp=(Get-Date).ToString('o');elapsed_seconds=[math]::Round($clock.Elapsed.TotalSeconds,1);pid=$pidOwned;working_set_bytes=$ws;private_bytes=$private;available_memory_bytes=$ram}
  Add-Content -LiteralPath $monitor -Value ($entry|ConvertTo-Json -Compress)
  if($ws -gt 2147483648){$killed=$true;$killReason='external_working_set_guard';Stop-Process -Id $pidOwned -Force;break}
  if($clock.Elapsed.TotalSeconds -ge 1200){$killed=$true;$killReason='20_minute_run_cap';Stop-Process -Id $pidOwned -Force;break}
  if(($clock.Elapsed.TotalSeconds-$lastCpu) -ge 30){
    try{$cs=Get-Counter -Counter '\Processor(_Total)\% Processor Time' -MaxSamples 1;$v=[math]::Round($cs.CounterSamples[0].CookedValue,3);$cpuSamples.Add([pscustomobject]@{timestamp=$cs.CounterSamples[0].Timestamp.ToString('o');cpu_percent=$v});$lastCpu=$clock.Elapsed.TotalSeconds}catch{}
  }
  Start-Sleep -Seconds 5
}
try{Wait-Process -Id $pidOwned -Timeout 30 -ErrorAction SilentlyContinue}catch{}
$proc.Refresh();$exit=$null;try{$exit=$proc.ExitCode}catch{}
$clock.Stop()
$status=[pscustomobject]@{pid=$pidOwned;exit_code=$exit;killed_by_guard=$killed;kill_reason=$killReason;elapsed_seconds=[math]::Round($clock.Elapsed.TotalSeconds,3);peak_working_set_seen_bytes=$peakWs;peak_private_seen_bytes=$peakPrivate;binary_sha256=$actual;query_budget_default_elapsed_ms=5000;rss_limit_bytes=2147483648;global_cpu_samples=$cpuSamples;global_cpu_average_percent=if($cpuSamples.Count){[math]::Round((($cpuSamples|Measure-Object cpu_percent -Average).Average),3)}else{$null}}
$status | ConvertTo-Json -Depth 7 | Set-Content -LiteralPath (Join-Path $run 'process-status.json') -Encoding utf8
Write-Output ($status|ConvertTo-Json -Compress -Depth 7)




