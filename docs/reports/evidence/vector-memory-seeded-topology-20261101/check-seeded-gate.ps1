$ErrorActionPreference='Stop'
$r='C:\Users\suanr\genesis-sqlite-prepare-20261031'
$deadline=[DateTime]::UtcNow.AddMinutes(10)
$results=Join-Path $r 'seeded-topology-20261101\gate-attempts.jsonl'
while([DateTime]::UtcNow -lt $deadline){
 $samples=@();$free=[long]::MaxValue
 for($i=0;$i -lt 11;$i++){
  $samples+= [double](Get-CimInstance Win32_PerfFormattedData_PerfOS_Processor -Filter "Name='_Total'").PercentProcessorTime
  $bytes=[int64](Get-CimInstance Win32_OperatingSystem).FreePhysicalMemory*1024
  $free=[math]::Min($free,$bytes)
  if($i -lt 10){Start-Sleep -Seconds 1}
 }
 $active=@(Get-CimInstance Win32_Process|Where-Object {$_.Name -match '^(cargo|rustc|clippy-driver|vbench-genesis|sqlite-prepare-reproducer)\.exe$'})
 $avg=($samples|Measure-Object -Average).Average
 $pass=($avg -lt 20 -and $free -ge 6GB -and $active.Count -eq 0)
 $row=[ordered]@{at=[DateTime]::UtcNow.ToString('o');cpu_samples=$samples;cpu_average=$avg;minimum_free_bytes=$free;active_count=$active.Count;pass=$pass}
 [IO.File]::AppendAllText($results,(($row|ConvertTo-Json -Compress)+[Environment]::NewLine))
 if($pass){exit 0}
 Start-Sleep -Seconds 20
}
exit 3
