$ErrorActionPreference='Stop'
Set-StrictMode -Version Latest
$root='C:\Users\suanr\genesis-sqlite-prepare-20261031'
$text=[IO.File]::ReadAllText((Join-Path $root 'build-release-variants.ps1'));$anchor='$lock = $null'
. ([scriptblock]::Create($text.Substring(0,$text.IndexOf($anchor))))
$metadata=Read-Json (Join-Path $R 'fresh-reopen-variant-metadata.json')
$BeforeLib=$metadata.sources.before.path;$ExpectedBefore=$metadata.sources.before.sha256
$AfterLib=$metadata.sources.after.path;$ExpectedAfter=$metadata.sources.after.sha256
$ExternalBench=$metadata.bench.path
$prior=Join-Path $R 'release-variants-fresh-reopen-20261031T170202Z'
$oldStatus=Read-Json (Join-Path $prior 'fresh-before/guard-output/status.json')
Assert-GuardRunsStopped @([ordered]@{stage='fresh-before';status=$oldStatus}) 'before fresh-after resume'|Out-Null
$until=[DateTime]::UtcNow.AddMinutes(10)
do {
 $active=@(Get-ActiveAdmissionProcesses)
 Write-Json (Join-Path $R 'fresh-after-build-wait.json') ([ordered]@{at=[DateTime]::UtcNow.ToString('o');active=$active.Count;state=if($active.Count){'waiting_external_compilers'}else{'ready'};no_owned_child_build_started=$true})
 if(-not $active.Count){break}
 if([DateTime]::UtcNow -ge $until){throw 'Resource wait ended; no build launched'}
 Start-Sleep -Seconds 15
}while($true)
$lock=[IO.File]::Open($LockPath,[IO.FileMode]::OpenOrCreate,[IO.FileAccess]::ReadWrite,[IO.FileShare]::None)
try {
 $q=Assert-Qualification
 Assert-NoActiveProcesses 'fresh-after resume'|Out-Null
 $CargoExe='C:\Users\suanr\.rustup\toolchains\1.96.0-x86_64-pc-windows-msvc\bin\cargo.exe'
 $env:RUSTUP_TOOLCHAIN='1.96.0-x86_64-pc-windows-msvc'
 $env:RUSTC='C:\Users\suanr\.rustup\toolchains\1.96.0-x86_64-pc-windows-msvc\bin\rustc.exe'
 $env:RUSTDOC='C:\Users\suanr\.rustup\toolchains\1.96.0-x86_64-pc-windows-msvc\bin\rustdoc.exe'
 if(Test-Path -LiteralPath (Join-Path $R 'fresh-after.exe')){throw 'Refusing existing output'}
 $script:RunId='fresh-after-resume-'+[DateTime]::UtcNow.ToString('yyyyMMddTHHmmssZ')
 $script:RunDir=Join-Path $R ('release-variants-'+$script:RunId)
 New-Item -ItemType Directory -Path $script:RunDir|Out-Null
 $base=Read-Json (Join-Path $R 'release-variants-20261031T150004Z-b5e55f51/run-manifest.json')
 $script:SourceClone=$base.paths.source_clone;$script:TargetDir=$base.paths.private_target_dir
 $script:TargetExe=Join-Path $script:TargetDir 'release/vbench-genesis.exe'
 $script:FrozenSource=@($script:QualifiedCloneFiles)
 $script:FrozenBenchSha=$metadata.bench.sha256
 $script:FrozenBenchPath=Join-Path $prior 'fresh-reopen-vector_memory.frozen.rs'
 $script:ExpectedCloneLib=$ExpectedBefore;$script:ExpectedCloneBench=$script:FrozenBenchSha
 Assert-CloneSnapshot $script:FrozenSource $script:ExpectedCloneLib $script:ExpectedCloneBench 'after external resource hold'
 Copy-Item -LiteralPath (Join-Path $prior 'source-snapshot.json') -Destination (Join-Path $script:RunDir 'source-snapshot.json')
 Write-Json (Join-Path $script:RunDir 'context.json') ([ordered]@{qualification=$q;toolchain='1.96.0-x86_64-pc-windows-msvc';cargo=$CargoExe;rustc=$env:RUSTC;before_status=(Join-Path $prior 'fresh-before/guard-output/status.json');repo_mutation='none'})
 Invoke-ReleaseStage 'fresh-after' ([IO.File]::ReadAllBytes($AfterLib)) $ExpectedAfter ([IO.File]::ReadAllBytes($script:FrozenBenchPath)) $script:FrozenBenchSha (Join-Path $R 'fresh-after.exe')|Out-Null
 Write-Json (Join-Path $script:RunDir 'result.json') ([ordered]@{status='complete';qualification=(Assert-Qualification);builds=@($script:BuildRecords);guard_runs=@($script:GuardRuns);before_status=(Join-Path $prior 'fresh-before/guard-output/status.json')})
}finally{$lock.Dispose()}
