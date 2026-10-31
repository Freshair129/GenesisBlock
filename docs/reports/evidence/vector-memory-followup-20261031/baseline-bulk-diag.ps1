$r='C:\Users\suanr\genesis-vector-memory-followup-20261031'
Set-Location 'C:\Users\suanr\genesis-vector-memory-20261031\verify-src'
$p=Start-Process -FilePath "$r\baseline-tests\concurrency-948a29ec82ad9112.exe" -ArgumentList @('--exact','bulk_add_edges_1000','--nocapture','--test-threads=1') -WindowStyle Hidden -PassThru -RedirectStandardOutput "$r\baseline-bulk-edges.log" -RedirectStandardError "$r\baseline-bulk-edges.err"
$start=Get-Date;$a=@();$reason=$null
while(-not $p.HasExited){Start-Sleep -Milliseconds 500;$p.Refresh();if(-not $p.HasExited){$seconds=((Get-Date)-$start).TotalSeconds;$a+=@{seconds=$seconds;cpu_seconds=$p.TotalProcessorTime.TotalSeconds;rss=$p.WorkingSet64;log_bytes=(Get-Item "$r\baseline-bulk-edges.log").Length};if($p.WorkingSet64 -gt 4GB){$reason='rss_limit_4_GiB';Stop-Process -Id $p.Id}elseif($seconds -gt 60){$reason='diagnostic_stop_60s';Stop-Process -Id $p.Id}}}
$p.WaitForExit();@{exit_code=$p.ExitCode;reason=$reason;samples=$a}|ConvertTo-Json -Depth 4|Set-Content "$r\baseline-bulk-edges-timeline.json"
