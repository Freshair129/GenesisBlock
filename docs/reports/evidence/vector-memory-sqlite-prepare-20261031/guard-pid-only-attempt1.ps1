param([Parameter(Mandatory=$true)][string]$Config)
$ErrorActionPreference='Stop'
$c=Get-Content -LiteralPath $Config -Raw|ConvertFrom-Json
New-Item -ItemType Directory -Path $c.output -Force|Out-Null
$active=@(Get-CimInstance Win32_Process | Where-Object {$_.Name -match '^(cargo|rustc|vbench-genesis|sqlite-prepare-reproducer)\.exe$'})
if($active.Count){throw 'Owned runtime admission requires no concurrent cargo/rustc/benchmark'}
$samples=@();for($i=0;$i -lt 11;$i++){$cpu=Get-CimInstance Win32_PerfFormattedData_PerfOS_Processor -Filter "Name='_Total'";$os=Get-CimInstance Win32_OperatingSystem;$samples+=@([pscustomobject]@{cpu=[double]$cpu.PercentProcessorTime;free=[int64]$os.FreePhysicalMemory*1024});if($i -lt 10){Start-Sleep -Seconds 1}}
$avg=($samples|Measure-Object cpu -Average).Average;$free=($samples|Measure-Object free -Minimum).Minimum
$samples|ConvertTo-Json|Set-Content (Join-Path $c.output 'admission.json')
$buildOnly=($c.kind -eq 'build_preparation' -and [IO.Path]::GetFileName($c.exe) -eq 'cargo.exe' -and ($c.args[0] -eq 'build' -or ($c.args[0] -eq 'test' -and $c.args -contains '--no-run')))
$functionalOnly=($c.kind -eq 'functional_test' -and $c.exe -like (Join-Path $PSScriptRoot '*-library-tests.exe') -and $c.args -contains '--test-threads=1' -and $c.cap_seconds -le 300)
if(((-not $buildOnly) -and (-not $functionalOnly) -and $avg -ge 20) -or $free -lt 6GB){throw "Host admission failed CPU=$avg free=$free"}
foreach($p in $c.env.PSObject.Properties){Set-Item -LiteralPath ('Env:'+ $p.Name) -Value $p.Value}
$p=Start-Process -FilePath $c.exe -ArgumentList $c.args -WorkingDirectory $c.cwd -WindowStyle Hidden -PassThru -RedirectStandardOutput (Join-Path $c.output 'stdout.log') -RedirectStandardError (Join-Path $c.output 'stderr.log')
$cap=if($c.cap_seconds){[math]::Min(1200,[int]$c.cap_seconds)}else{1200}
$start=Get-Date;$clock=[Diagnostics.Stopwatch]::StartNew();$peak=0L;$peakPrivate=0L;$reason=$null;$known=@($p.Id)
[ordered]@{pid=$p.Id;start=$start.ToString('o');config=$c;query_deadline_ms=5000;cap_seconds=$cap;rss_cap=2147483648}|ConvertTo-Json -Depth 9|Set-Content (Join-Path $c.output 'start.json')
while($true){
 $all=@(Get-CimInstance Win32_Process);$changed=$true;while($changed){$changed=$false;foreach($w in $all){if($known -contains [int]$w.ParentProcessId -and $known -notcontains [int]$w.ProcessId){$known+=@([int]$w.ProcessId);$changed=$true}}}
 $procs=@($known|ForEach-Object {Get-Process -Id $_ -ErrorAction SilentlyContinue});$rss=[int64](($procs|Measure-Object WorkingSet64 -Sum).Sum);$private=[int64](($procs|Measure-Object PrivateMemorySize64 -Sum).Sum);$peak=[math]::Max($peak,$rss);$peakPrivate=[math]::Max($peakPrivate,$private)
 $record=[ordered]@{seconds=$clock.Elapsed.TotalSeconds;rss=$rss;private=$private;processes=@($procs|Select-Object Id,CPU,WorkingSet64,PrivateMemorySize64,PeakWorkingSet64)}
 [IO.File]::AppendAllText((Join-Path $c.output 'timeline.jsonl'),(($record|ConvertTo-Json -Depth 5 -Compress)+[Environment]::NewLine))
 if($rss -gt 2GB){$reason='rss_limit'};if($clock.Elapsed.TotalSeconds -ge $cap){$reason='job_timeout'}
 if($reason){foreach($ownedId in ($known|Sort-Object -Descending)){Stop-Process -Id $ownedId -Force -ErrorAction SilentlyContinue};break}
 $p.Refresh();if($p.HasExited -and $procs.Count -eq 0){break};Start-Sleep -Seconds 1
}
$p.WaitForExit();$p.Refresh();[ordered]@{exit=$p.ExitCode;reason=$reason;seconds=$clock.Elapsed.TotalSeconds;peak_rss=$peak;peak_private=$peakPrivate;pids=$known;admission_cpu=$avg;admission_free=$free;remaining=@($known|ForEach-Object {Get-Process -Id $_ -ErrorAction SilentlyContinue}|Select-Object Id,ProcessName)}|ConvertTo-Json -Depth 6|Set-Content (Join-Path $c.output 'status.json')




