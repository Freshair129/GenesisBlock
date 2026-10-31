param([Parameter(Mandatory=$true)][string]$Config,[Parameter(Mandatory=$true)][string]$Label)
$ErrorActionPreference='Stop'
Set-StrictMode -Version Latest
$r='C:\Users\suanr\genesis-sqlite-prepare-20261031'
$cfg=Get-Content -LiteralPath $Config -Raw|ConvertFrom-Json
$root=(Resolve-Path -LiteralPath $r).ProviderPath.TrimEnd('\')+'\'
if($Label -notmatch '^[a-z0-9-]+$'){throw 'Unsafe resource-job label'}
if(-not ([IO.Path]::GetFullPath($cfg.output).StartsWith($root,[StringComparison]::OrdinalIgnoreCase))){throw 'Guard output escaped evidence root'}
$until=[DateTime]::UtcNow.AddMinutes(10);$attempt=0
function Save-Wait($state,$active,$errorText){
 [ordered]@{at=[DateTime]::UtcNow.ToString('o');state=$state;active_compilers=$active;attempt=$attempt;config=$Config;guard_output=$cfg.output;error=$errorText;scope='Resource wait only; actual job retains1200s/2GiB and query5000ms; no automatic retry after possible launch'}|ConvertTo-Json|Set-Content -LiteralPath (Join-Path $r "$Label-wait-status.json")
}
while([DateTime]::UtcNow -lt $until){
 $active=@(Get-CimInstance Win32_Process|Where-Object {$_.Name -match '^(cargo|rustc|clippy-driver|vbench-genesis|sqlite-prepare-reproducer)\.exe$'})
 if($active.Count){Save-Wait 'waiting_external_compiler' $active.Count $null;Start-Sleep -Seconds 15;continue}
 if(Test-Path -LiteralPath $cfg.output){
  $possible=@('start.json','status.json','stdout.log','stderr.log')|Where-Object {Test-Path -LiteralPath (Join-Path $cfg.output $_)}
  if(@($possible).Count){throw 'Existing possible launched-job artifacts; root must inspect before retry'}
  $archive=Join-Path $r ($Label+'-admission-reject-'+[DateTime]::UtcNow.ToString('yyyyMMddTHHmmssfffZ'))
  if(-not ([IO.Path]::GetFullPath($archive).StartsWith($root,[StringComparison]::OrdinalIgnoreCase))){throw 'Archive target escaped evidence root'}
  Move-Item -LiteralPath $cfg.output -Destination $archive
 }
 $attempt++;Save-Wait 'checking_original_admission' 0 $null
 try { & (Join-Path $r 'guard.ps1') -Config $Config }
 catch {
  $failureText=$_.ToString()
  $possible=@('start.json','status.json','stdout.log','stderr.log')|Where-Object {Test-Path -LiteralPath (Join-Path $cfg.output $_)}
  if(@($possible).Count){Save-Wait 'launch_possible_requires_inspection' 0 $failureText;throw}
  if($failureText -notmatch 'Concurrent compiler/benchmark blocks launch|Host admission failed'){
   Save-Wait 'infrastructure_error_no_launch' 0 $failureText;throw
  }
  Save-Wait 'admission_rejected_no_launch' 0 $failureText;Start-Sleep -Seconds 20;continue
 }
 $status=Get-Content -LiteralPath (Join-Path $cfg.output 'status.json') -Raw|ConvertFrom-Json
 $pass=($status.exit -eq 0 -and -not $status.reason -and $status.cleanup_complete -and @($status.remaining).Count -eq 0)
 Save-Wait $(if($pass){'job_finished'}else{'job_failed_no_retry'}) 0 $null
 if(-not $pass){exit 2}
 exit 0
}
Save-Wait 'resource_wait_expired_no_launch' 0 $null
exit 3
