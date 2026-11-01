$ErrorActionPreference='Stop'
$result=Join-Path $PSScriptRoot 'stable-wait.jsonl'
$deadline=[DateTime]::UtcNow.AddMinutes(10)
$clearSince=$null
while([DateTime]::UtcNow -lt $deadline){
 $active=@(Get-CimInstance Win32_Process|Where-Object {$_.Name -match '^(cargo|rustc|clippy-driver|vbench-genesis|sqlite-prepare-reproducer)\.exe$'})
 $now=[DateTime]::UtcNow
 if($active.Count){$clearSince=$null} elseif($null -eq $clearSince){$clearSince=$now}
 $elapsed=if($clearSince){($now-$clearSince).TotalSeconds}else{0}
 $entry=[ordered]@{at=$now.ToString('o');compiler_count=$active.Count;clear_seconds=$elapsed;scope='Pre-wait only; original admission and in-run guard remain mandatory'}
 [IO.File]::AppendAllText($result,(($entry|ConvertTo-Json -Compress)+[Environment]::NewLine))
 if($elapsed -ge 60){exit 0}
 Start-Sleep -Seconds 10
}
exit 3
