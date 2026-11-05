from pathlib import Path
r=Path(__file__).parent;p=r/'guard.ps1';s=p.read_text()
s=s.replace('$cap=if($c.cap_seconds)', '$null=$p.Handle\n$cap=if($c.cap_seconds)',1)
a=s.index('$rootMeta=');b=s.index('function Same-Process',a)
s=s[:a]+'''$monitorError=$null
try {
 $rootMeta=Get-CimInstance Win32_Process -Filter "ProcessId=$($p.Id)"
 if($rootMeta){
  $sameBirth=[math]::Abs(($rootMeta.CreationDate-$start).TotalMilliseconds) -lt 2
  $sameName=$rootMeta.Name -eq [IO.Path]::GetFileName($c.exe)
  $samePath=$p.HasExited -or $rootMeta.ExecutablePath -eq $p.Path
  if($sameBirth -and $sameName -and $samePath){$known[[int]$p.Id]=$rootMeta}
  elseif(-not $p.HasExited){throw 'Root CIM identity did not match original Process handle'}
 } elseif(-not $p.HasExited){throw 'Live original root missing from CIM snapshot'}
} catch {$monitorError=$_.ToString()}
''' +s[b:]
s=s.replace("if($proc -and [math]::Abs(($proc.StartTime-$known[$id].CreationDate).TotalMilliseconds) -lt 2){$rows+=@($proc)}", "if($proc){try{$null=$proc.Handle;if([math]::Abs(($proc.StartTime-$known[$id].CreationDate).TotalMilliseconds) -lt 2){$rows+=@($proc)}}catch{if(-not $proc.HasExited){throw}}}")
a=s.index('while($true){\n $procs=@(Get-OwnedProcesses)')
s=s[:a]+'''$lastKnown=@();$procs=@();$cleanupComplete=$false
try {
 if($monitorError){throw $monitorError}
 while($true){
  $procs=@(Get-OwnedProcesses);$lastKnown=$procs
  $rss=[int64](($procs|Measure-Object WorkingSet64 -Sum).Sum);$private=[int64](($procs|Measure-Object PrivateMemorySize64 -Sum).Sum)
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
[ordered]@{exit=$code;reason=$reason;seconds=$clock.Elapsed.TotalSeconds;peak_rss=$peak;peak_private=$peakPrivate;identities=@($known.Values|Select-Object ProcessId,ParentProcessId,CreationDate,Name,ExecutablePath);excluded_build_helpers=@($excluded.Values|Select-Object ProcessId,CreationDate,Name);admission_cpu=$avg;admission_free=$free;remaining=@($remaining|ForEach-Object {[pscustomobject]@{pid=$_.Id}});cleanup_complete=$cleanupComplete;monitor_error=$monitorError;rss_scope=if($buildOnly){'tracked processes; compiler telemetry/daemon helpers excluded'}else{'tracked owned process tree'}}|ConvertTo-Json -Depth 6|Set-Content (Join-Path $c.output 'status.json')
'''
p.write_text(s,newline='\n')
