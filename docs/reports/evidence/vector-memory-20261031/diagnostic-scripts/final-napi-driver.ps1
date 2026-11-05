$r='C:\Users\suanr\genesis-vector-memory-20261031'
Set-Location 'D:\GenesisBlock_Dev\GenesisBlock'
$env:CARGO_TARGET_DIR="$r\napi-target"
$env:CARGO_PROFILE_DEV_DEBUG='0';$env:CARGO_BUILD_JOBS='1';$env:CARGO_NET_OFFLINE='true'
.\node_modules\.bin\napi.cmd build --platform --target=x86_64-pc-windows-msvc --js=index.js --dts=index.d.ts --cargo-flags=--locked target/vector-memory-napi > "$r\proof-final-napi-build.log" 2>&1
$code=$LASTEXITCODE
$code | Set-Content "$r\proof-final-napi-build.status"
if($code -ne 0){exit $code}
Copy-Item -LiteralPath 'target\vector-memory-napi\index.win32-x64-msvc.node' -Destination "$r\proof-after\index.win32-x64-msvc.node"
Copy-Item -LiteralPath 'target\vector-memory-napi\index.win32-x64-msvc.node' -Destination "$r\verify-src\index.win32-x64-msvc.node"
node scripts/check-dts-freshness.mjs . target/vector-memory-napi > "$r\final-dts-check.log" 2>&1
$LASTEXITCODE | Set-Content "$r\final-dts-check.status"
Copy-Item -LiteralPath "$r\napi-smoke.json" -Destination "$r\napi-smoke-phase1.json"
node "$r\napi-smoke.cjs" > "$r\final-napi-smoke.log" 2>&1
$LASTEXITCODE | Set-Content "$r\final-napi-smoke.status"
Set-Location "$r\verify-src"
node --test --test-concurrency=1 __test__/*.mjs > "$r\final-node-tests.log" 2>&1
$LASTEXITCODE | Set-Content "$r\final-node-tests.status"
