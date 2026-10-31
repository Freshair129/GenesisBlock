param([Parameter(Mandatory=$true)][string]$Config)
$ErrorActionPreference='Stop'
$c=Get-Content -LiteralPath $Config -Raw|ConvertFrom-Json
New-Item -ItemType Directory -Path $c.output -Force|Out-Null
$active=@(Get-CimInstance Win32_Process|Where-Object {$_.Name -match '^(cargo|rustc|vbench-genesis|sqlite-prepare-reproducer)\.exe$'})
if($active.Count){throw 'Concurrent compiler/benchmark blocks launch'}
$samples=@();for($i=0;$i -lt 11;$i++){$cpu=Get-CimInstance Win32_PerfFormattedData_PerfOS_Processor -Filter "Name='_Total'";$os=Get-CimInstance Win32_OperatingSystem;$samples+=@([pscustomobject]@{cpu=[double]$cpu.PercentProcessorTime;free=[int64]$os.FreePhysicalMemory*1024});if($i -lt 10){Start-Sleep -Seconds 1}}
$avg=($samples|Measure-Object cpu -Average).Average;$free=($samples|Measure-Object free -Minimum).Minimum
$samples|ConvertTo-Json|Set-Content (Join-Path $c.output 'admission.json')
if($c.kind -eq 'functional_test' -and ($null -eq $c.cap_seconds -or [int]$c.cap_seconds -le 0 -or [int]$c.cap_seconds -gt 300)){throw 'Functional test requires explicit positive cap <=300 seconds'}
$cargoName=([IO.Path]::GetFileName($c.exe) -eq 'cargo.exe')
$nodeName=([IO.Path]::GetFileName($c.exe) -eq 'node.exe')
$buildOnly=($c.kind -eq 'build_preparation' -and (($cargoName -and ($c.args[0] -in @('build','clippy','check') -or ($c.args[0] -eq 'test' -and $c.args -contains '--no-run'))) -or ($nodeName -and $c.args[0] -like '*@napi-rs*cli*scripts*index.js' -and $c.args[1] -eq 'build')))
$functionalOnly=($c.kind -eq 'functional_test' -and $null -ne $c.cap_seconds -and [int]$c.cap_seconds -gt 0 -and [int]$c.cap_seconds -le 300 -and (($c.exe -like (Join-Path $PSScriptRoot '*-library-tests.exe') -and $c.args -contains '--test-threads=1') -or ($cargoName -and $c.args[0] -eq 'test' -and $c.args -contains '--doc') -or ($nodeName -and ($c.args[0] -eq '--test' -or $c.args[0] -like '*.cjs' -or $c.args[0] -like '*check-dts-freshness.mjs'))))
if(((-not $buildOnly) -and (-not $functionalOnly) -and $avg -ge 20) -or $free -lt 6GB){throw "Host admission failed CPU=$avg free=$free"}
foreach($entry in $c.env.PSObject.Properties){Set-Item -LiteralPath ('Env:'+$entry.Name) -Value $entry.Value}
$p=Start-Process -FilePath $c.exe -ArgumentList $c.args -WorkingDirectory $c.cwd -WindowStyle Hidden -PassThru -RedirectStandardOutput (Join-Path $c.output 'stdout.log') -RedirectStandardError (Join-Path $c.output 'stderr.log')
$null=$p.Handle
$cap=if($c.cap_seconds){[math]::Min(1200,[int]$c.cap_seconds)}else{1200}
$start=$p.StartTime;$clock=[Diagnostics.Stopwatch]::StartNew();$peak=0L;$peakPrivate=0L;$reason=$null;$known=@{};$excluded=@{}
$monitorError=$null
try {
 $rootMeta=Get-CimInstance Win32_Process -Filter "ProcessId=$($p.Id)"
 if($rootMeta){
  $sameBirth=[math]::Abs(($rootMeta.CreationDate-$start).Ticks) -lt 10
  $sameName=$rootMeta.Name -eq [IO.Path]::GetFileName($c.exe)
  $samePath=$p.HasExited -or $rootMeta.ExecutablePath -eq $p.Path
  if($sameBirth -and $sameName -and $samePath){$known[[int]$p.Id]=$rootMeta}
  elseif(-not $p.HasExited){throw 'Root CIM identity did not match original Process handle'}
 } elseif(-not $p.HasExited){throw 'Live original root missing from CIM snapshot'}
} catch {$monitorError=$_.ToString()}
function Same-Process($a,$b){
 if($null -eq $a -or $null -eq $b){return $false}
 if(-not $b.CreationDate){throw 'Process birth unavailable; ownership uncertain'}
 if($a.CreationDate -ne $b.CreationDate){return $false}
 if($a.Name -ne $b.Name){throw 'Same-birth process name conflict; ownership uncertain'}
 if($a.ExecutablePath -and $b.ExecutablePath -and $a.ExecutablePath -ne $b.ExecutablePath){throw 'Same-birth process path conflict; ownership uncertain'}
 # A temporarily unavailable CIM path is not evidence that a known owner died.
 # The original opened handle and microsecond birth check still establish identity.
 return $true
}
function Get-OwnedProcesses {
 $all=@(Get-CimInstance Win32_Process -Property ProcessId,ParentProcessId,CreationDate,Name,ExecutablePath);$current=@{};foreach($w in $all){$current[[int]$w.ProcessId]=$w}
 $script:GuardProcessInventory=$all
 $changed=$true;while($changed){$changed=$false;foreach($w in $all){$id=[int]$w.ProcessId;$parent=[int]$w.ParentProcessId;if(((-not $known.ContainsKey($id)) -or (-not (Same-Process $known[$id] $w))) -and $known.ContainsKey($parent) -and (Same-Process $known[$parent] $current[$parent]) -and $w.CreationDate -ge $start.AddMilliseconds(-2)){
   if($buildOnly -and $w.Name -match '^(vctip|mspdbsrv)\.exe$'){$excluded[$id]=[pscustomobject]@{ProcessId=$w.ProcessId;ParentProcessId=$w.ParentProcessId;CreationDate=$w.CreationDate;Name=$w.Name;ExecutablePath=$w.ExecutablePath;parent_creation=$known[$parent].CreationDate;parent_name=$known[$parent].Name;role='non-compiling toolchain telemetry/PDB service'};continue}
   $known[$id]=$w;$changed=$true
 }}}
 $rows=@();foreach($id in $known.Keys){if(Same-Process $known[$id] $current[$id]){$proc=Get-Process -Id $id -ErrorAction SilentlyContinue;if($proc){try{$null=$proc.Handle;if([math]::Abs(($proc.StartTime-$known[$id].CreationDate).Ticks) -lt 10){$rows+=@($proc)}}catch{if(-not $proc.HasExited){throw}}}}};return @($rows)
}
[ordered]@{pid=$p.Id;start=$start.ToString('o');config=$c;query_deadline_ms=5000;cap_seconds=$cap;rss_cap=2147483648;identity='PID+CreationDate+path+name; require live matching parent and child born after root'}|ConvertTo-Json -Depth 9|Set-Content (Join-Path $c.output 'start.json')
$lastKnown=@();$procs=@();$cleanupComplete=$false
try {
 if($monitorError){throw $monitorError}
 while($true){
  $procs=@(Get-OwnedProcesses);$lastKnown=$procs
  if(-not $buildOnly -and -not $functionalOnly){
   $competition=@($script:GuardProcessInventory|Where-Object {$_.Name -match '^(cargo|rustc|clippy-driver|vbench-genesis|sqlite-prepare-reproducer)\.exe$' -and ((-not $known.ContainsKey([int]$_.ProcessId)) -or (-not (Same-Process $known[[int]$_.ProcessId] $_)))})
   if($competition.Count){$reason='resource_competition';$competition|Select-Object ProcessId,CreationDate,Name,ExecutablePath|ConvertTo-Json|Set-Content (Join-Path $c.output 'competition.json')}
  }
  $rss=0L;$private=0L
  foreach($tracked in $procs){$rss+=$tracked.WorkingSet64;$private+=$tracked.PrivateMemorySize64}
  $peak=[math]::Max($peak,$rss);$peakPrivate=[math]::Max($peakPrivate,$private)
  $record=[ordered]@{seconds=$clock.Elapsed.TotalSeconds;rss=$rss;private=$private;processes=@($procs|Select-Object Id,CPU,WorkingSet64,PrivateMemorySize64,PeakWorkingSet64)}
  [IO.File]::AppendAllText((Join-Path $c.output 'timeline.jsonl'),(($record|ConvertTo-Json -Depth 5 -Compress)+[Environment]::NewLine))
  if($rss -gt 2GB){$reason='rss_limit'};if($clock.Elapsed.TotalSeconds -ge $cap){$reason='job_timeout'}
  $p.Refresh()
  if($p.HasExited -and $p.ExitCode -ne 0 -and $procs.Count){$reason='root_exited_with_children'}
  if($reason){break}
  if($p.HasExited -and $procs.Count -eq 0){$cleanupComplete=$true;break}
  Start-Sleep -Seconds 1
 }
} catch {$monitorError=$_.ToString();$reason='monitor_error'}
if($reason){
 # Each Process object has an opened original handle; Kill never resolves a new PID.
 foreach($owned in (@($lastKnown)+@($p))){try{if(-not $owned.HasExited){$owned.Kill()}}catch{}}
 $cleanupUntil=[datetime]::UtcNow.AddSeconds(5)
 do {
  try {$procs=@(Get-OwnedProcesses)}catch{$monitorError=$_.ToString();$procs=@($lastKnown|Where-Object {-not $_.HasExited})}
  $alive=@($procs|Where-Object {-not $_.HasExited})
  if($p.HasExited -and $alive.Count -eq 0){$cleanupComplete=(-not $monitorError);break}
  foreach($owned in $alive){try{$owned.Kill()}catch{}}
  Start-Sleep -Milliseconds 100
 } while([datetime]::UtcNow -lt $cleanupUntil)
}
# Bounded termination receipt. Failed ownership discovery stays quarantined even
# if all last-known handles exited; a caller must prove a fresh stop before restore.
try {if(-not $p.WaitForExit(5000)){$cleanupComplete=$false;$reason='cleanup_incomplete'}}catch{$cleanupComplete=$false;$monitorError=$_.ToString()}
$p.Refresh();$code=if($p.HasExited){$p.ExitCode}else{$null}
$remaining=@();try{$remaining=@(Get-OwnedProcesses|Where-Object {-not $_.HasExited})}catch{$cleanupComplete=$false;$monitorError=$_.ToString()}
if(-not $p.HasExited){$remaining+=@($p)}
if($remaining.Count){$cleanupComplete=$false}
if(-not $cleanupComplete -and -not $reason){$reason='cleanup_incomplete'}
[ordered]@{exit=$code;reason=$reason;seconds=$clock.Elapsed.TotalSeconds;peak_rss=$peak;peak_private=$peakPrivate;identities=@($known.Values|Select-Object ProcessId,ParentProcessId,CreationDate,Name,ExecutablePath);excluded_build_helpers=@($excluded.Values);helper_finished_parent_condition=($cleanupComplete -and $p.HasExited -and $code -eq 0);helper_scope='Helpers are not terminated; successful Cargo completion and stopped tracked compiler clients are required before private-source edits. No shared source/target mutation.';admission_cpu=$avg;admission_free=$free;remaining=@($remaining|ForEach-Object {[pscustomobject]@{pid=$_.Id}});cleanup_complete=$cleanupComplete;monitor_error=$monitorError;rss_scope=if($buildOnly){'tracked processes; compiler telemetry/daemon helpers excluded'}else{'tracked owned process tree'}}|ConvertTo-Json -Depth 6|Set-Content (Join-Path $c.output 'status.json')

