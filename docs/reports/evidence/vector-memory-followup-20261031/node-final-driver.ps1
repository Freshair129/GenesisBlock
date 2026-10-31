$r='C:\Users\suanr\genesis-vector-memory-followup-20261031'
Set-Location 'C:\Users\suanr\genesis-vector-memory-20261031\verify-src'
node "$r\napi-smoke.cjs" > "$r\native-smoke.log" 2>&1
$LASTEXITCODE | Set-Content "$r\native-smoke.status"
node --test --test-concurrency=1 __test__/*.mjs > "$r\node-full.log" 2>&1
$LASTEXITCODE | Set-Content "$r\node-full.status"
