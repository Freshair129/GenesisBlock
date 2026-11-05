Set-Location 'C:\Users\suanr\genesis-vector-memory-20261031\verify-src'
$env:CARGO_TARGET_DIR='C:\Users\suanr\genesis-vector-memory-20261031\napi-target'
$env:CARGO_PROFILE_DEV_DEBUG='0';$env:CARGO_PROFILE_TEST_DEBUG='0'
cargo clippy --locked --offline --target x86_64-pc-windows-msvc --no-default-features --all-targets -j1 -- -D warnings > 'C:\Users\suanr\genesis-vector-memory-20261031\final-clippy-core.log' 2>&1
$core=$LASTEXITCODE
cargo clippy --locked --offline --target x86_64-pc-windows-msvc --all-targets -j1 -- -D warnings > 'C:\Users\suanr\genesis-vector-memory-20261031\final-clippy-default.log' 2>&1
$default=$LASTEXITCODE
cargo check --locked --offline --target x86_64-pc-windows-msvc --no-default-features --features mobile -j1 > 'C:\Users\suanr\genesis-vector-memory-20261031\final-mobile-check.log' 2>&1
$mobile=$LASTEXITCODE
@{core=$core;default=$default;mobile=$mobile}|ConvertTo-Json|Set-Content 'C:\Users\suanr\genesis-vector-memory-20261031\final-qualification.status'
