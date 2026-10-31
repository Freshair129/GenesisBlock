$ErrorActionPreference='Stop'
$run='C:\Users\suanr\genesis-vector-memory-followup-20261031\query-reopen-repeat-20261031T134858\new'
$oldContextPath='C:\Users\suanr\genesis-vector-memory-followup-20261031\query-reopen-repeat-20261031T134858\old\run-context.json'
$planPath='C:\Users\suanr\genesis-vector-memory-followup-20261031\candidate-repeat-build-20261031T131156\main-lifecycle-reproduction-plan.json'
$db=Join-Path $run 'gdb'
$exe='C:\Users\suanr\genesis-vector-memory-followup-20261031\candidate-repeat-build-20261031T131156\candidate-vbench-genesis-repeat.exe'
$benchSource='C:\Users\suanr\genesis-vector-memory-followup-20261031\candidate-repeat-build-20261031T131156\candidate-repeat-vector_memory.rs'
$libSource='C:\Users\suanr\genesis-vector-memory-followup-20261031\candidate-repeat-build-20261031T131156\candidate-querytrace-lib.rs'
$input='C:\Users\suanr\genesis-vector-memory-20261031\inputs\synthetic-100000-1024'
$outJson=Join-Path $run 'repeat-open-only.json'
$expectedExeSha='959DFA8F6348082046BCBFAD744A422E2BD7F5A43C702C2DBA7D2D86D26AEAF1'
$expectedBenchSha='5BFDE2C3B897C316BED557FC3037AE9A44D9EEAD07053DB4F835BF1BD7D27C62'
$expectedMetaSha=''
$sourceHash=(Get-FileHash -LiteralPath $benchSource -Algorithm SHA256).Hash
$libHash=(Get-FileHash -LiteralPath $libSource -Algorithm SHA256).Hash
$exeHash=(Get-FileHash -LiteralPath $exe -Algorithm SHA256).Hash
if($exeHash -ne $expectedExeSha){throw "candidate binary hash mismatch: $exeHash"}
if($sourceHash -ne $expectedBenchSha){throw "benchmark source hash mismatch: $sourceHash"}
$old=Get-Content -LiteralPath $oldContextPath -Raw | ConvertFrom-Json
$plan=Get-Content -LiteralPath $planPath -Raw | ConvertFrom-Json
$meta=Get-Content -LiteralPath (Join-Path $input 'meta.json') -Raw | ConvertFrom-Json
if($meta.n -ne 100000 -or $meta.dim -ne 1024 -or $meta.q -ne 24){throw 'input metadata mismatch'}
$envSpec=[ordered]@{
  GB_VBENCH=$input
  GB_QUANT='bq'
  GB_MEMORY_RETENTION='full'
  GB_VEC_Q='24'
  GB_MEMORY_AUDIT='1'
  GB_MEMORY_OPEN_QUERY_SAMPLES='120'
  RAYON_NUM_THREADS='4'
  GB_VECTOR_QUERY_DIAGNOSTICS='1'
  GB_MEMORY_OPEN_ONLY=$db
  GB_MEMORY_OPEN_READ_WRITE='1'
  GB_EF='200'
  GB_MEMORY_OPEN_OUT=$outJson
  GB_MEMORY_RSS_LIMIT_BYTES='2147483648'
  GB_MEMORY_INPUT=$input
  GB_MEMORY_OPEN_REPEAT='2'
  GB_RERANK='0'
}
$whitelist=@($envSpec.Keys)
$oldEnv=[ordered]@{}
foreach($prop in $old.env.PSObject.Properties){$oldEnv[$prop.Name]=[string]$prop.Value}
if($oldEnv.Count -ne 16){throw "old run-context has $($oldEnv.Count) env keys, expected 16"}
if(Compare-Object @($oldEnv.Keys|Sort-Object) @($envSpec.Keys|Sort-Object)){throw 'old and candidate environment whitelist keys differ'}
$oldEnvMatches=$true
foreach($key in $whitelist){
  $expected=[string]$envSpec[$key]
  if($key -in @('GB_MEMORY_OPEN_ONLY','GB_MEMORY_OPEN_OUT')){$expected=$expected.Replace('\new\','\old\')}
  if([string]$oldEnv[$key] -ne $expected){$oldEnvMatches=$false;throw "old run-context mismatch for $key"}
}
$clearNames=@('GB_MEMORY_PREPARE','GB_MEMORY_COMPACT','GB_MEMORY_CORPUS')
foreach($name in $clearNames){Remove-Item -LiteralPath "Env:$name" -ErrorAction SilentlyContinue}
foreach($key in $whitelist){[Environment]::SetEnvironmentVariable($key,[string]$envSpec[$key],'Process')}
$effectiveEnv=[ordered]@{}
foreach($key in $whitelist){$effectiveEnv[$key]=[Environment]::GetEnvironmentVariable($key,'Process')}
$effectiveMatches=$true
foreach($key in $whitelist){if([string]$effectiveEnv[$key] -ne [string]$envSpec[$key]){$effectiveMatches=$false}}
$conflictsAbsent=$true
foreach($name in $clearNames){if($null -ne [Environment]::GetEnvironmentVariable($name,'Process')){$conflictsAbsent=$false}}
if(-not $oldEnvMatches -or -not $effectiveMatches -or -not $conflictsAbsent){throw 'environment equivalence or clearing check failed'}
$envKeys=@($effectiveEnv.Keys|Sort-Object)
if($envKeys.Count -ne 16){throw 'effective whitelist key count changed'}
$context=[ordered]@{
  status='prepared_before_final_admission'
  binary=$exe
  binary_sha256=$exeHash
  bench_source=$benchSource
  bench_sha256=$sourceHash
  querytrace_lib_source=$libSource
  querytrace_lib_sha256=$libHash
  input=$input
  input_meta=$meta
  input_files=$plan.input_files
  database=$db
  database_role='candidate equivalent disposable full copy; source database untouched'
  output_json=$outJson
  whitelist_count=16
  old_context_exact_whitelist_keys=$true
  old_context_values_match_after_path_substitution=$true
  effective_whitelist_values_match=$true
  cleared_conflicting_variable_names=$clearNames
  conflicting_variables_absent=$true
  whitelisted_env=$effectiveEnv
  query=@{k=10;ef_search=200;alpha=0.0;deadline_ms=5000;query_index=0}
  open_options=@{read_only=$false;page_cache_mb=64;vector_dim=1024;retention='full'}
  runtime_guards=@{rss_limit_bytes=2147483648;external_working_set_cap_bytes=2147483648;max_runtime_seconds=1200}
  no_budget_changes=$true
}
$context | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $run 'run-context.json') -Encoding utf8
$preflightStart=(Get-Date).ToString('o')
$cpu=Get-Counter -Counter '\Processor(_Total)\% Processor Time' -SampleInterval 1 -MaxSamples 10
$cpuSamples=@($cpu.CounterSamples|ForEach-Object{[pscustomobject]@{timestamp=$_.Timestamp.ToString('o');cpu_percent=[math]::Round($_.CookedValue,3)}})
$ramBefore=[double](Get-CimInstance Win32_OperatingSystem).FreePhysicalMemory*1024
$relatedBefore=@(Get-Process -Name cargo,rustc,vbench-genesis -ErrorAction SilentlyContinue|Select-Object Id,ProcessName)
$ramAfter=[double](Get-CimInstance Win32_OperatingSystem).FreePhysicalMemory*1024
$relatedAfter=@(Get-Process -Name cargo,rustc,vbench-genesis -ErrorAction SilentlyContinue|Select-Object Id,ProcessName)
$cpuAvg=[math]::Round((($cpuSamples|Measure-Object cpu_percent -Average).Average),3)
$freeMin=[math]::Min($ramBefore,$ramAfter)
$preflight=[ordered]@{
  sample_start_local=$cpuSamples[0].timestamp
  sample_count=$cpuSamples.Count
  duration_seconds=10
  counter='\Processor(_Total)\% Processor Time'
  cpu_samples=$cpuSamples
  global_cpu_avg_percent=$cpuAvg
  free_ram_before_bytes=$ramBefore
  free_ram_after_bytes=$ramAfter
  free_ram_min_bytes=$freeMin
  free_ram_min_gib=[math]::Round($freeMin/1GB,3)
  related_processes_before=$relatedBefore
  related_processes_after=$relatedAfter
  candidate_binary_sha256=$exeHash
  preflight_pass=($cpuAvg -lt 20 -and $freeMin -ge 6GB -and $relatedBefore.Count -eq 0 -and $relatedAfter.Count -eq 0)
}
$preflight | ConvertTo-Json -Depth 7 | Set-Content -LiteralPath (Join-Path $run 'new-preflight-final.json') -Encoding utf8
Write-Output ($preflight|ConvertTo-Json -Compress -Depth 7)
if(-not $preflight.preflight_pass){throw 'admission failed; no candidate process launched'}
if(Test-Path -LiteralPath $outJson){throw 'refusing existing query output file'}
$stdout=Join-Path $run 'stdout.log';$stderr=Join-Path $run 'stderr.log';$monitor=Join-Path $run 'resource-monitor.jsonl'
$started=Get-Date
$proc=Start-Process -FilePath $exe -WorkingDirectory 'D:\GenesisBlock_Dev\GenesisBlock' -WindowStyle Hidden -PassThru -RedirectStandardOutput $stdout -RedirectStandardError $stderr
$pidOwned=$proc.Id
$processStart=[ordered]@{pid=$pidOwned;started_local=(Get-Date).ToString('o');binary_sha256=$exeHash;query_budget_default_elapsed_ms=5000;rss_limit_bytes=2147483648;repeat=2;page_cache_mb=64;read_only=$false;vector_dim=1024;retention='full';rayon_num_threads=4;ef_search=200;k=10;alpha=0;sample_count=120;input=$input;db=$db;whitelisted_env=$effectiveEnv}
$processStart|ConvertTo-Json -Depth 7|Set-Content -LiteralPath (Join-Path $run 'process-start.json') -Encoding utf8
Write-Output "CANDIDATE_STARTED pid=$pidOwned binary=$exeHash time=$($processStart.started_local)"
$clock=[Diagnostics.Stopwatch]::StartNew();$peakWs=0L;$peakPrivate=0L;$killed=$false;$killReason=$null;$nextCpu=30.0;$cpuRun=[System.Collections.Generic.List[object]]::new()
while($true){
  $p=Get-Process -Id $pidOwned -ErrorAction SilentlyContinue
  if($null -eq $p){break}
  $ws=[long]$p.WorkingSet64;$private=[long]$p.PrivateMemorySize64
  if($ws -gt $peakWs){$peakWs=$ws};if($private -gt $peakPrivate){$peakPrivate=$private}
  $available=[double](Get-CimInstance Win32_OperatingSystem).FreePhysicalMemory*1024
  Add-Content -LiteralPath $monitor -Value ([pscustomobject]@{timestamp=(Get-Date).ToString('o');elapsed_seconds=[math]::Round($clock.Elapsed.TotalSeconds,1);pid=$pidOwned;working_set_bytes=$ws;private_bytes=$private;available_memory_bytes=$available}|ConvertTo-Json -Compress)
  if($ws -gt 2147483648){$killed=$true;$killReason='external_working_set_guard';Stop-Process -Id $pidOwned -Force;break}
  if($clock.Elapsed.TotalSeconds -ge 1200){$killed=$true;$killReason='20_minute_run_cap';Stop-Process -Id $pidOwned -Force;break}
  if($clock.Elapsed.TotalSeconds -ge $nextCpu){try{$cs=Get-Counter -Counter '\Processor(_Total)\% Processor Time' -MaxSamples 1;$cpuRun.Add([pscustomobject]@{timestamp=$cs.CounterSamples[0].Timestamp.ToString('o');cpu_percent=[math]::Round($cs.CounterSamples[0].CookedValue,3)});$nextCpu+=30.0}catch{}}
  Start-Sleep -Seconds 5
}
try{Wait-Process -Id $pidOwned -Timeout 30 -ErrorAction SilentlyContinue}catch{}
$proc.Refresh();$exit=$null;try{$exit=$proc.ExitCode}catch{};$clock.Stop()
$status=[ordered]@{pid=$pidOwned;exit_code=$exit;killed_by_guard=$killed;kill_reason=$killReason;elapsed_seconds=[math]::Round($clock.Elapsed.TotalSeconds,3);peak_working_set_seen_bytes=$peakWs;peak_private_seen_bytes=$peakPrivate;binary_sha256=$exeHash;query_budget_default_elapsed_ms=5000;rss_limit_bytes=2147483648;global_cpu_samples=$cpuRun;global_cpu_average_percent=if($cpuRun.Count){[math]::Round((($cpuRun|Measure-Object cpu_percent -Average).Average),3)}else{$null}}
$status|ConvertTo-Json -Depth 7|Set-Content -LiteralPath (Join-Path $run 'process-status.json') -Encoding utf8
Write-Output ($status|ConvertTo-Json -Compress -Depth 7)

