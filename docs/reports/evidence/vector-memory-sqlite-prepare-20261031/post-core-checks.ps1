$ErrorActionPreference='Stop'
$r=$PSScriptRoot
$cdir='C:\Users\suanr\genesis-vector-memory-20261031\verify-src'
while((-not (Test-Path (Join-Path $r 'final-runtime.status'))) -or ((Get-Content (Join-Path $r 'final-runtime.status') -Raw).Trim() -ne 'complete')){Start-Sleep -Seconds 2}
$results=@(Get-Content (Join-Path $r 'final-runtime.json') -Raw|ConvertFrom-Json)
if($results.Count -ne 159 -or @($results|Where-Object {$_.exit_code -ne 0 -or $_.reason}).Count){@{passed=$false;reason='Core suite failed or incomplete'}|ConvertTo-Json|Set-Content (Join-Path $r 'post-checks.status');exit 1}
$compiled=Get-Content (Join-Path $r 'final-test-artifacts.json') -Raw|ConvertFrom-Json
if($compiled.source.'src/projection_rebuild_tests.rs' -ne 'e69dc7dd5cbc5043aa5ed9c717e449e50246a8fbd8ad56ac95ed135718c99be2'){throw 'Stale test-source qualification'}
if((Get-FileHash (Join-Path $cdir 'src\lib.rs')).Hash -ne '5B8BB9BED5DDE5FA6E7D4D255DA3ACCE6C1BFE892092456133ADB65A78998FDD'){throw 'UnexpectedCsourcebeforechecks'}
$baseEnv=@{CARGO_TARGET_DIR='C:\Users\suanr\genesis-vector-memory-20261031\napi-target';CARGO_PROFILE_DEV_DEBUG='0';CARGO_PROFILE_TEST_DEBUG='0';CARGO_BUILD_JOBS='1';CARGO_NET_OFFLINE='true'}
$receipts=@()
function Invoke-Check([string]$Name,[string]$Exe,[string[]]$Arguments,[string]$Kind,[int]$Cap=1200){
 $config=[ordered]@{kind=$Kind;exe=$Exe;args=$Arguments;cwd=$cdir;output=(Join-Path $r $Name);env=$baseEnv;cap_seconds=$Cap}
 $configPath=Join-Path $r ($Name+'.json');$config|ConvertTo-Json -Depth 6|Set-Content $configPath
 & (Join-Path $r 'guard.ps1') -Config $configPath
 $status=Get-Content (Join-Path $config.output 'status.json') -Raw|ConvertFrom-Json
 $script:receipts+=@([ordered]@{name=$Name;status=$status})
 if($status.exit -ne 0 -or $status.reason){throw "Check failed: $Name"}
}
try {
 $cargo=(Get-Command cargo.exe).Source;$node=(Get-Command node.exe).Source
 Invoke-Check 'clippy-core' $cargo @('clippy','--locked','--offline','--target','x86_64-pc-windows-msvc','--no-default-features','--all-targets','-j1','--','-D','warnings') 'build_preparation'
 Invoke-Check 'clippy-default' $cargo @('clippy','--locked','--offline','--target','x86_64-pc-windows-msvc','--all-targets','-j1','--','-D','warnings') 'build_preparation'
 Invoke-Check 'mobile-host-check' $cargo @('check','--locked','--offline','--target','x86_64-pc-windows-msvc','--no-default-features','--features','mobile','-j1') 'build_preparation'
 Invoke-Check 'doc-tests' $cargo @('test','--locked','--offline','--target','x86_64-pc-windows-msvc','--no-default-features','--doc','-j1') 'functional_test' 300
 $cli=Join-Path $cdir 'node_modules\@napi-rs\cli\scripts\index.js'
 Invoke-Check 'napi-build' $node @($cli,'build','--platform','--target=x86_64-pc-windows-msvc','--js=index.js','--dts=index.d.ts','--cargo-flags=--locked','target/sqlite-prepare-napi') 'build_preparation'
 $addon=Join-Path $cdir 'target\sqlite-prepare-napi\index.win32-x64-msvc.node'
 Copy-Item -LiteralPath $addon -Destination (Join-Path $r 'candidate-profiled.node')
 Copy-Item -LiteralPath $addon -Destination (Join-Path $cdir 'index.win32-x64-msvc.node')
 Invoke-Check 'dts-freshness' $node @((Join-Path $cdir 'scripts\check-dts-freshness.mjs'),$cdir,(Join-Path $cdir 'target\sqlite-prepare-napi')) 'functional_test' 300
 Invoke-Check 'napi-smoke' $node @((Join-Path $r 'napi-smoke.cjs')) 'functional_test' 300
 Invoke-Check 'node-full' $node @('--test','--test-concurrency=1','__test__/*.mjs') 'functional_test' 300
 @{passed=$true;checks=$receipts;addon_sha256=(Get-FileHash $addon).Hash;core_sha256=(Get-FileHash (Join-Path $cdir 'src\lib.rs')).Hash}|ConvertTo-Json -Depth 8|Set-Content (Join-Path $r 'post-checks.status')
} catch {
 @{passed=$false;error=$_.ToString();checks=$receipts}|ConvertTo-Json -Depth 8|Set-Content (Join-Path $r 'post-checks.status');throw
}
