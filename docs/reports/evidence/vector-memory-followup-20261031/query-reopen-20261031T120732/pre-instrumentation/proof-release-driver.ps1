Set-Location 'D:\GenesisBlock_Dev\GenesisBlock'
$env:CARGO_PROFILE_RELEASE_LTO='false';$env:CARGO_PROFILE_RELEASE_CODEGEN_UNITS='16'
cargo build --locked --offline --release --no-default-features --features bins --bin vbench-genesis -j2 > 'C:\Users\suanr\genesis-vector-memory-20261031\proof-final-release-build.log' 2>&1
$code=$LASTEXITCODE
$code | Set-Content 'C:\Users\suanr\genesis-vector-memory-20261031\proof-final-release-build.status'
if($code -eq 0){Copy-Item -LiteralPath 'target\release\vbench-genesis.exe' -Destination 'C:\Users\suanr\genesis-vector-memory-20261031\candidate-proof-sharing.exe'}
