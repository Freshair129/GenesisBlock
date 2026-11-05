$ErrorActionPreference='Stop'
Set-StrictMode -Version Latest
$root='C:\Users\suanr\genesis-sqlite-prepare-20261031'
$builder=Join-Path $root 'build-release-variants.ps1'
$text=[IO.File]::ReadAllText($builder);$anchor='$lock = $null'
if(($text.Split(@($anchor),[StringSplitOptions]::None)).Count -ne 2){throw 'Definitions anchor ambiguous'}
. ([scriptblock]::Create($text.Substring(0,$text.IndexOf($anchor))))
$metadata=Read-Json (Join-Path $R 'fresh-reopen-variant-metadata.json')
$BeforeLib=$metadata.sources.before.path;$ExpectedBefore=$metadata.sources.before.sha256
$AfterLib=$metadata.sources.after.path;$ExpectedAfter=$metadata.sources.after.sha256
$ExternalBench=$metadata.bench.path
$OutputPaths=@((Join-Path $R 'fresh-before.exe'),(Join-Path $R 'fresh-after.exe'))
$prior=Join-Path $R 'release-variants-20261031T150004Z-b5e55f51'
$priorManifest=Read-Json (Join-Path $prior 'run-manifest.json')
$lock=[IO.File]::Open($LockPath,[IO.FileMode]::OpenOrCreate,[IO.FileAccess]::ReadWrite,[IO.FileShare]::None)
try {
 $oldStatus=Read-Json (Join-Path $R '100k-before-guard/status.json')
 if(-not $oldStatus.cleanup_complete -or @($oldStatus.remaining).Count){throw 'Old timed workload not cleaned up'}
 $live=@(Get-CimInstance Win32_Process)
 foreach($identity in $oldStatus.identities){if(@($live|Where-Object {[int]$_.ProcessId -eq [int]$identity.ProcessId -and (Test-SameBirth $_.CreationDate $identity.CreationDate)}).Count){throw 'Old workload identity remains'}}
 $q=Assert-Qualification
 Assert-NoActiveProcesses 'fresh workload variants'|Out-Null
 foreach($p in $OutputPaths){if(Test-Path -LiteralPath $p){throw "Refusing existing variant $p"}}
 $script:RunId='fresh-reopen-'+[DateTime]::UtcNow.ToString('yyyyMMddTHHmmssZ')
 $script:RunDir=Join-Path $R ('release-variants-'+$script:RunId)
 New-Item -ItemType Directory -Path $script:RunDir|Out-Null
 $script:SourceClone=$priorManifest.paths.source_clone
 $script:TargetDir=$priorManifest.paths.private_target_dir
 $script:TargetExe=Join-Path $script:TargetDir 'release/vbench-genesis.exe'
 $script:FrozenSource=@($script:QualifiedCloneFiles)
 $script:FrozenBenchSha=$metadata.bench.sha256
 $script:FrozenBenchPath=Join-Path $script:RunDir 'fresh-reopen-vector_memory.frozen.rs'
 Copy-Item -LiteralPath $ExternalBench -Destination $script:FrozenBenchPath
 $script:ExpectedCloneLib=$ExpectedQualifiedSources['src/lib.rs']
 $script:ExpectedCloneBench=$ExpectedQualifiedSources['benches/vector_memory.rs']
 Assert-CloneSnapshot $script:FrozenSource $script:ExpectedCloneLib $script:ExpectedCloneBench 'before fresh variants'
 Assert-DSourceSnapshot $script:FrozenSource 'before fresh variants'
 Copy-Item -LiteralPath (Join-Path $prior 'source-snapshot.json') -Destination (Join-Path $script:RunDir 'source-snapshot.json')
 Write-Json (Join-Path $script:RunDir 'context.json') ([ordered]@{qualification=$q;metadata=$metadata;production_commit='f98a324c6ac9c16bbd98679622012519107628a3';source_clone=$script:SourceClone;target=$script:TargetDir;old_workload='two-open1200s TIMEOUT retained, not relabeled';new_workload='separate fixture setup and fresh RW reopen, original per-job/query/RSS caps';prod_source_mutation='none'})
 $bench=[IO.File]::ReadAllBytes($script:FrozenBenchPath)
 Invoke-ReleaseStage 'fresh-before' ([IO.File]::ReadAllBytes($BeforeLib)) $ExpectedBefore $bench $script:FrozenBenchSha $OutputPaths[0]|Out-Null
 Invoke-ReleaseStage 'fresh-after' ([IO.File]::ReadAllBytes($AfterLib)) $ExpectedAfter $bench $script:FrozenBenchSha $OutputPaths[1]|Out-Null
 $final=Assert-Qualification
 Write-Json (Join-Path $script:RunDir 'result.json') ([ordered]@{status='complete';qualification=$final;builds=@($script:BuildRecords);guard_runs=@($script:GuardRuns);repo_mutation='none'})
} finally {$lock.Dispose()}
