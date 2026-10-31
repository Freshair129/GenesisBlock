$ErrorActionPreference='Stop'
Set-StrictMode -Version Latest
$root='C:\Users\suanr\genesis-sqlite-prepare-20261031'
$builder=Join-Path $root 'build-release-variants.ps1'
$text=[IO.File]::ReadAllText($builder)
$anchor='$lock = $null'
if(($text.Split(@($anchor),[StringSplitOptions]::None)).Count -ne 2){throw 'Builder definitions anchor ambiguous'}
. ([scriptblock]::Create($text.Substring(0,$text.IndexOf($anchor))))
$prior=Join-Path $R 'release-variants-20261031T150004Z-b5e55f51'
$priorManifest=Read-Json (Join-Path $prior 'run-manifest.json')
$stop=Read-Json (Join-Path $R 'release-build-empty-tree-stop-proof.json')
if($stop.remaining_owned -ne 0 -or $stop.global_compiler_benchmark_count -ne 0){throw 'Missing fresh original-build stop proof'}
$oldStatus=Read-Json (Join-Path $prior 'diagnostic-before/guard-output/status.json')
$live=@(Get-CimInstance Win32_Process)
foreach($identity in $oldStatus.identities){
 if(@($live|Where-Object {[int]$_.ProcessId -eq [int]$identity.ProcessId -and (Test-SameBirth $_.CreationDate $identity.CreationDate)}).Count){throw 'Original compiler identity remains'}
}
$lock=[IO.File]::Open($LockPath,[IO.FileMode]::OpenOrCreate,[IO.FileAccess]::ReadWrite,[IO.FileShare]::None)
try {
 $qualification=Assert-Qualification
 Assert-NoActiveProcesses 'private resume preflight'|Out-Null
 foreach($p in $OutputPaths){if(Test-Path -LiteralPath $p){throw "Refusing existing variant $p"}}
 $script:RunId='resume-'+[DateTime]::UtcNow.ToString('yyyyMMddTHHmmssZ')
 $script:RunDir=Join-Path $R ('release-variants-'+$script:RunId)
 New-Item -ItemType Directory -Path $script:RunDir|Out-Null
 $script:SourceClone=$priorManifest.paths.source_clone
 $script:TargetDir=$priorManifest.paths.private_target_dir
 $script:TargetExe=Join-Path $script:TargetDir 'release/vbench-genesis.exe'
 $script:FrozenSource=@($script:QualifiedCloneFiles)
 $script:FrozenProductionLib=[IO.File]::ReadAllBytes((Join-Path $D 'src/lib.rs'))
 $script:FrozenProductionBench=[IO.File]::ReadAllBytes((Join-Path $D 'benches/vector_memory.rs'))
 $script:FrozenBenchSha=Get-Sha $ExternalBench
 $script:FrozenBenchPath=Join-Path $prior 'externalreopen100k-vector_memory.frozen.rs'
 $script:ExpectedCloneLib=$ExpectedBefore
 $script:ExpectedCloneBench=$script:FrozenBenchSha
 Assert-CloneSnapshot $script:FrozenSource $script:ExpectedCloneLib $script:ExpectedCloneBench 'fresh proven private resume'
 Assert-DSourceSnapshot $script:FrozenSource 'fresh proven private resume'
 Copy-Item -LiteralPath (Join-Path $prior 'source-snapshot.json') -Destination (Join-Path $script:RunDir 'source-snapshot.json')
 Write-Json (Join-Path $script:RunDir 'resume-context.json') ([ordered]@{prior_run=$prior;source_clone=$script:SourceClone;target=$script:TargetDir;qualification=$qualification;builder_sha256=(Get-Sha $builder);guard_sha256=(Get-Sha $Guard);stop_proof=$stop;old_monitor_result='preserved failed; cached Cargo completion rerun required';repository_mutation='none'})
 $bench=[IO.File]::ReadAllBytes($script:FrozenBenchPath)
 Invoke-ReleaseStage 'diagnostic-before' ([IO.File]::ReadAllBytes($BeforeLib)) $ExpectedBefore $bench $script:FrozenBenchSha $OutputPaths[0]|Out-Null
 Invoke-ReleaseStage 'diagnostic-after' ([IO.File]::ReadAllBytes($AfterLib)) $ExpectedAfter $bench $script:FrozenBenchSha $OutputPaths[1]|Out-Null
 Invoke-ReleaseStage 'candidate-untraced-release' $script:FrozenProductionLib $ExpectedQualifiedSources['src/lib.rs'] $script:FrozenProductionBench $ExpectedQualifiedSources['benches/vector_memory.rs'] $OutputPaths[2]|Out-Null
 $final=Assert-Qualification
 Write-Json (Join-Path $script:RunDir 'result.json') ([ordered]@{status='complete';qualification=$final;builds=@($script:BuildRecords);guard_runs=@($script:GuardRuns);prior_failed_monitor_run=$prior;paths=[ordered]@{source_clone=$script:SourceClone;private_target=$script:TargetDir};repo_mutation='none'})
} finally {$lock.Dispose()}
