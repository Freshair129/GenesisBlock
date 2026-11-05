Set-Location 'C:\Users\suanr\genesis-vector-memory-20261031\verify-src'
$env:CARGO_TARGET_DIR='C:\Users\suanr\genesis-vector-memory-20261031\napi-target'
$env:CARGO_PROFILE_DEV_DEBUG='0';$env:CARGO_PROFILE_TEST_DEBUG='0'
cargo test --locked --offline --target x86_64-pc-windows-msvc --no-default-features --no-run -j1 > 'C:\Users\suanr\genesis-vector-memory-20261031\final-core-build.log' 2>&1
$LASTEXITCODE | Set-Content 'C:\Users\suanr\genesis-vector-memory-20261031\final-core-build.status'
