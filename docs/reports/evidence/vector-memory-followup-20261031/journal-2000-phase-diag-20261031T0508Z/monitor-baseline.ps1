$ErrorActionPreference = 'Stop'
$run = 'C:\Users\suanr\genesis-vector-memory-followup-20261031\journal-2000-phase-diag-20261031T0508Z'
$nodeExe = 'C:\Users\suanr\AppData\Local\Programs\nodejs-v24.20.0\node.exe'
$nodeScript = Join-Path $run 'journal-2000-baseline.cjs'
$addon = 'C:\Users\suanr\genesis-vector-memory-followup-20261031\baseline.node'
$dbPath = Join-Path $run 'database'
$progressPath = Join-Path $run 'progress.jsonl'
$phasesPath = Join-Path $run 'phases.jsonl'
$stdoutPath = Join-Path $run 'node.stdout.log'
$stderrPath = Join-Path $run 'node.stderr.log'
$timelinePath = Join-Path $run 'timeline.jsonl'
$summaryPath = Join-Path $run 'monitor-summary.json'
$timelineCsv = Join-Path $run 'timeline.csv'
$limitBytes = [int64]4GB
$limitSeconds = 300
foreach ($p in @($dbPath, $progressPath, $phasesPath, $stdoutPath, $stderrPath, $timelinePath, $summaryPath, $timelineCsv)) {
    if (Test-Path -LiteralPath $p) { throw "Refusing to overwrite existing output: $p" }
}
$manifest = [pscustomobject]@{
    diagnostic = 'journal_compresses_authenticated_compaction_proofs 2000 individual adds'
    addon = $addon
    addon_sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $addon).Hash
    node_exe = $nodeExe
    node_script = $nodeScript
    db_path = $dbPath
    limit_seconds = $limitSeconds
    rss_limit_bytes = $limitBytes
    start_utc = [DateTime]::UtcNow.ToString('o')
}
$manifestPath = Join-Path $run 'run-manifest.json'
if (Test-Path -LiteralPath $manifestPath) { throw "Refusing to overwrite manifest: $manifestPath" }
$manifest | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $manifestPath -Encoding utf8

$sw = [System.Diagnostics.Stopwatch]::StartNew()
$child = Start-Process -FilePath $nodeExe -ArgumentList @('--expose-gc', $nodeScript, $addon, $dbPath, $progressPath, $phasesPath) -PassThru -WindowStyle Hidden -RedirectStandardOutput $stdoutPath -RedirectStandardError $stderrPath
$ownedPid = $child.Id
$shutdownSeen = $false
$shutdownElapsed = $null
$peakRss = [int64]0
$peakCpu = 0.0
$reason = 'process_exit'
$exitCode = $null
$startUtc = [DateTime]::UtcNow.ToString('o')
$initial = Get-CimInstance Win32_Process -Filter ("ProcessId = {0}" -f $ownedPid) -ErrorAction SilentlyContinue
if ($initial -and (($initial.ExecutablePath -ne $nodeExe) -or ($initial.CommandLine -notlike "*$nodeScript*"))) {
    throw "Started process identity mismatch; refusing to monitor or terminate PID $ownedPid"
}
$csvRows = [System.Collections.Generic.List[object]]::new()
while ($true) {
    $child.Refresh()
    if ($child.HasExited) {
        $exitCode = $child.ExitCode
        break
    }
    $proc = Get-Process -Id $ownedPid -ErrorAction SilentlyContinue
    if (-not $proc) {
        $child.Refresh()
        if ($child.HasExited) { $exitCode = $child.ExitCode; break }
        Start-Sleep -Milliseconds 250
        continue
    }
    $proc.Refresh()
    $rss = [int64]$proc.WorkingSet64
    $cpu = [double]$proc.TotalProcessorTime.TotalSeconds
    if ($rss -gt $peakRss) { $peakRss = $rss }
    if ($cpu -gt $peakCpu) { $peakCpu = $cpu }
    $sample = [pscustomobject]@{
        elapsed_s = [Math]::Round($sw.Elapsed.TotalSeconds, 3)
        utc = [DateTime]::UtcNow.ToString('o')
        pid = $ownedPid
        rss_bytes = $rss
        cpu_seconds = $cpu
    }
    ($sample | ConvertTo-Json -Compress) | Add-Content -LiteralPath $timelinePath -Encoding utf8
    $csvRows.Add($sample)
    if (-not $shutdownSeen -and (Test-Path -LiteralPath $stdoutPath)) {
        $tail = Get-Content -LiteralPath $stdoutPath -Tail 20 -ErrorAction SilentlyContinue
        if ($tail -match 'shutdown: graceful shutdown, state saved') {
            $shutdownSeen = $true
            $shutdownElapsed = [Math]::Round($sw.Elapsed.TotalSeconds, 3)
        }
    }
    if ($rss -ge $limitBytes) {
        $reason = 'rss_limit_4_GiB'
        break
    }
    if ($sw.Elapsed.TotalSeconds -ge $limitSeconds) {
        $reason = 'timeout_300s'
        break
    }
    Start-Sleep -Milliseconds 1000
}
if ($reason -ne 'process_exit') {
    $live = Get-CimInstance Win32_Process -Filter ("ProcessId = {0}" -f $ownedPid) -ErrorAction SilentlyContinue
    if ($live) {
        if (($live.ExecutablePath -ne $nodeExe) -or ($live.CommandLine -notlike "*$nodeScript*")) {
            throw "Refusing to terminate PID $ownedPid because its command identity changed"
        }
        Stop-Process -Id $ownedPid -Force
        Start-Sleep -Milliseconds 500
    }
    $child.Refresh()
    if ($child.HasExited) { $exitCode = $child.ExitCode }
}
$sw.Stop()
if ($csvRows.Count -gt 0) {
    $csvRows | Export-Csv -NoTypeInformation -LiteralPath $timelineCsv -Encoding utf8
}
$lastProgress = $null
if (Test-Path -LiteralPath $progressPath) {
    $lastLine = Get-Content -LiteralPath $progressPath -Tail 1 -ErrorAction SilentlyContinue
    if ($lastLine) { try { $lastProgress = $lastLine | ConvertFrom-Json } catch { $lastProgress = $lastLine } }
}
$summary = [pscustomobject]@{
    reason = $reason
    exit_code = $exitCode
    pid = $ownedPid
    started_utc = $startUtc
    ended_utc = [DateTime]::UtcNow.ToString('o')
    elapsed_s = [Math]::Round($sw.Elapsed.TotalSeconds, 3)
    peak_rss_bytes = $peakRss
    final_cpu_seconds = $peakCpu
    storage_shutdown_marker_seen = $shutdownSeen
    storage_shutdown_marker_elapsed_s = $shutdownElapsed
    last_progress = $lastProgress
    progress_path = $progressPath
    stdout_path = $stdoutPath
    stderr_path = $stderrPath
    timeline_path = $timelinePath
}
$summary | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $summaryPath -Encoding utf8
$summary | ConvertTo-Json -Depth 8
