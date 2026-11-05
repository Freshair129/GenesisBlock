$r='C:\Users\suanr\genesis-vector-memory-followup-20261031'
Set-Location 'D:\GenesisBlock_Dev\GenesisBlock'
$env:CARGO_TARGET_DIR='C:\Users\suanr\genesis-vector-memory-20261031\napi-target'
$env:CARGO_PROFILE_DEV_DEBUG='0';$env:CARGO_PROFILE_TEST_DEBUG='0'
cargo test --locked --offline --target x86_64-pc-windows-msvc --no-default-features --no-run -j1 --message-format=json-render-diagnostics --test concurrency --test consensus_checkpoint_race_tests --test hql_cypher_tests --test journal_format_tests --test rebuild_from_truth_tests --test wal_compaction_tests > "$r\six-build.jsonl" 2> "$r\six-build.err"
$LASTEXITCODE | Set-Content "$r\six-build.status"
