param([Parameter(Mandatory=$true)][string]$Config)
$ErrorActionPreference='Stop'
$c=Get-Content -LiteralPath $Config -Raw|ConvertFrom-Json
New-Item -ItemType Directory -Path $c.output -Force|Out-Null
$active=@(Get-CimInstance Win32_Process|Where-Object {$_.Name -match '^(cargo|rustc|vbench-genesis|sqlite-prepare-reproducer)\.exe$'})
if($active.Count){throw 'Concurrent compiler/benchmark blocks launch'}
$samples=@();for($i=0;$i -lt 11;$i++){$cpu=Get-CimInstance Win32_PerfFormattedData_PerfOS_Processor -Filter "Name='_Total'";$os=Get-CimInstance Win32_OperatingSystem;$samples+=@([pscustomobject]@{cpu=[double]$cpu.PercentProcessorTime;free=[int64]$os.FreePhysicalMemory*1024});if($i -lt 10){Start-Sleep -Seconds 1}}
$avg=($samples|Measure-Object cpu -Average).Average;$free=($samples|Measure-Object free -Minimum).Minimum
$samples|ConvertTo-Json|Set-Content (Join-Path $c.output 'admission.json')
$cargoName=([IO.Path]::GetFileName($c.exe) -eq 'cargo.exe')
$nodeName=([IO.Path]::GetFileName($c.exe) -eq 'node.exe')
$buildOnly=($c.kind -eq 'build_preparation' -and (($cargoName -and ($c.args[0] -in @('build','clippy','check') -or ($c.args[0] -eq 'test' -and $c.args -contains '--no-run'))) -or ($nodeName -and $c.args[0] -like '*@napi-rs*cli*scripts*index.js' -and $c.args[1] -eq 'build')))
$functionalOnly=($c.kind -eq 'functional_test' -and $c.cap_seconds -le 300 -and (($c.exe -like (Join-Path $PSScriptRoot '*-library-tests.exe') -and $c.args -contains '--test-threads=1') -or ($cargoName -and $c.args[0] -eq 'test' -and $c.args -contains '--doc') -or ($nodeName -and ($c.args[0] -eq '--test' -or $c.args[0] -like '*.cjs' -or $c.args[0] -like '*check-dts-freshness.mjs'))))
if(((-not $buildOnly) -and (-not $functionalOnly) -and $avg -ge 20) -or $free -lt 6GB){throw "Host admission failed CPU=$avg free=$free"}
foreach($entry in $c.env.PSObject.Properties){Set-Item -LiteralPath ('Env:'+$entry.Name) -Value $entry.Value}
$p=Start-Process -FilePath $c.exe -ArgumentList $c.args -WorkingDirectory $c.cwd -WindowStyle Hidden -PassThru -RedirectStandardOutput (Join-Path $c.output 'stdout.log') -RedirectStandardError (Join-Path $c.output 'stderr.log')
$cap=if($c.cap_seconds){[math]::Min(1200,[int]$c.cap_seconds)}else{1200}
$start=$p.StartTime;$clock=[Diagnostics.Stopwatch]::StartNew();$peak=0L;$peakPrivate=0L;$reason=$null;$known=@{};$excluded=@{}
$rootMeta=Get-CimInstance Win32_Process -Filter "ProcessId=$($p.Id)"
if($rootMeta){$known[[int]$p.Id]=$rootMeta}
function Same-Process($a,$b){return ($null -ne $a -and $null -ne $b -and $a.CreationDate -eq $b.CreationDate -and $a.ExecutablePath -eq $b.ExecutablePath -and $a.Name -eq $b.Name)}
function Get-OwnedProcesses {
 $all=@(Get-CimInstance Win32_Process -Property ProcessId,ParentProcessId,CreationDate,Name,ExecutablePath);$current=@{};foreach($w in $all){$current[[int]$w.ProcessId]=$w}
 $changed=$true;while($changed){$changed=$false;foreach($w in $all){$id=[int]$w.ProcessId;$parent=[int]$w.ParentProcessId;if(((-not $known.ContainsKey($id)) -or (-not (Same-Process $known[$id] $w))) -and $known.ContainsKey($parent) -and (Same-Process $known[$parent] $current[$parent]) -and $w.CreationDate -ge $start.AddMilliseconds(-2)){
   if($buildOnly -and $w.Name -match '^(vctip|mspdbsrv)\.exe$'){$excluded[$id]=$w;continue}
   $known[$id]=$w;$changed=$true
 }}}
 $rows=@();foreach($id in $known.Keys){if(Same-Process $known[$id] $current[$id]){$proc=Get-Process -Id $id -ErrorAction SilentlyContinue;if($proc -and [math]::Abs(($proc.StartTime-$known[$id].CreationDate).TotalMilliseconds) -lt 2){$rows+=@($proc)}}};return @($rows)
}
[ordered]@{pid=$p.Id;start=$start.ToString('o');config=$c;query_deadline_ms=5000;cap_seconds=$cap;rss_cap=2147483648;identity='PID+CreationDate+path+name; require live matching parent and child born after root'}|ConvertTo-Json -Depth 9|Set-Content (Join-Path $c.output 'start.json')
while($true){
 $procs=@(Get-OwnedProcesses);$rss=[int64](($procs|Measure-Object WorkingSet64 -Sum).Sum);$private=[int64](($procs|Measure-Object PrivateMemorySize64 -Sum).Sum);$peak=[math]::Max($peak,$rss);$peakPrivate=[math]::Max($peakPrivate,$private)
 $record=[ordered]@{seconds=$clock.Elapsed.TotalSeconds;rss=$rss;private=$private;processes=@($procs|Select-Object Id,CPU,WorkingSet64,PrivateMemorySize64,PeakWorkingSet64)}
 [IO.File]::AppendAllText((Join-Path $c.output 'timeline.jsonl'),(($record|ConvertTo-Json -Depth 5 -Compress)+[Environment]::NewLine))
 if($rss -gt 2GB){$reason='rss_limit'};if($clock.Elapsed.TotalSeconds -ge $cap){$reason='job_timeout'}
 if($reason){foreach($proc in $procs){try{$proc.Kill()}catch{}};break}
 $p.Refresh();if($p.HasExited -and $procs.Count -eq 0){break};Start-Sleep -Seconds 1
}
$p.WaitForExit();$p.Refresh();[ordered]@{exit=$p.ExitCode;reason=$reason;seconds=$clock.Elapsed.TotalSeconds;peak_rss=$peak;peak_private=$peakPrivate;identities=@($known.Values|Select-Object ProcessId,ParentProcessId,CreationDate,Name,ExecutablePath);excluded_build_helpers=@($excluded.Values|Select-Object ProcessId,CreationDate,Name);admission_cpu=$avg;admission_free=$free;remaining=@(Get-OwnedProcesses|Select-Object Id,ProcessName)}|ConvertTo-Json -Depth 6|Set-Content (Join-Path $c.output 'status.json')


