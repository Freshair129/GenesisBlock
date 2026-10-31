$ErrorActionPreference='Stop'
Set-Location 'C:\Users\suanr\genesis-vector-memory-20261031\verify-src'
$root='C:\Users\suanr\genesis-vector-memory-20261031'
$exe="$root\napi-target\x86_64-pc-windows-msvc\debug\deps\concurrency-948a29ec82ad9112.exe"
$results=@()
foreach($name in @('concurrent_read_while_write','concurrent_search_while_write','concurrent_node_writes','concurrent_edge_writes')) {
 $p=Start-Process -FilePath $exe -ArgumentList @('--exact',$name,'--test-threads=1') -WindowStyle Hidden -PassThru -RedirectStandardOutput "$root\final-concurrent-$name.log" -RedirectStandardError "$root\final-concurrent-$name.err"
 $start=Get-Date; $peak=0L; $reason=$null
 while(-not $p.HasExited) {
  Start-Sleep -Milliseconds 200; $p.Refresh()
  if(-not $p.HasExited) {
   $peak=[math]::Max($peak,$p.WorkingSet64)
   if($peak -gt 4GB) {$reason='rss_limit_4_GiB';Stop-Process -Id $p.Id}
   elseif(((Get-Date)-$start).TotalSeconds -gt 180) {$reason='timeout_180s';Stop-Process -Id $p.Id}
  }
 }
 $p.WaitForExit()
 $results+=@{test=$name;exit_code=$p.ExitCode;peak_rss_bytes=$peak;reason=$reason;seconds=((Get-Date)-$start).TotalSeconds}
 $results | ConvertTo-Json -Depth 3 | Set-Content "$root\final-concurrent-focused.json"
}
'complete' | Set-Content "$root\final-concurrent-focused.status"
