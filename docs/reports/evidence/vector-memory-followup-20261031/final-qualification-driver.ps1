$r='C:\Users\suanr\genesis-vector-memory-followup-20261031'
Set-Location 'D:\GenesisBlock_Dev\GenesisBlock'
$env:CARGO_TARGET_DIR='C:\Users\suanr\genesis-vector-memory-20261031\napi-target';$env:CARGO_PROFILE_DEV_DEBUG='0';$env:CARGO_PROFILE_TEST_DEBUG='0';$env:CARGO_BUILD_JOBS='1';$env:CARGO_NET_OFFLINE='true'
cargo test --locked --offline --target x86_64-pc-windows-msvc --no-default-features --no-run -j1 --message-format=json-render-diagnostics > "$r\final-core-build.jsonl" 2> "$r\final-core-build.err"
$LASTEXITCODE|Set-Content "$r\final-core-build.status"
cargo clippy --locked --offline --target x86_64-pc-windows-msvc --no-default-features --all-targets -j1 -- -D warnings > "$r\final-clippy-core.log" 2>&1
$core=$LASTEXITCODE
cargo clippy --locked --offline --target x86_64-pc-windows-msvc --all-targets -j1 -- -D warnings > "$r\final-clippy-default.log" 2>&1
$default=$LASTEXITCODE
cargo check --locked --offline --target x86_64-pc-windows-msvc --no-default-features --features mobile -j1 > "$r\final-mobile-check.log" 2>&1
$mobile=$LASTEXITCODE
@{core=$core;default=$default;mobile=$mobile}|ConvertTo-Json|Set-Content "$r\final-qualification.status"
.\node_modules\.bin\napi.cmd build --platform --target=x86_64-pc-windows-msvc --js=index.js --dts=index.d.ts --cargo-flags=--locked target/vector-memory-followup-napi > "$r\final-napi-build.log" 2>&1
$code=$LASTEXITCODE;$code|Set-Content "$r\final-napi-build.status"
if($code -eq 0){Copy-Item -LiteralPath 'target\vector-memory-followup-napi\index.win32-x64-msvc.node' -Destination "$r\candidate-profiled.node"}
node scripts/check-dts-freshness.mjs . target/vector-memory-followup-napi > "$r\final-dts.log" 2>&1
$LASTEXITCODE|Set-Content "$r\final-dts.status"
