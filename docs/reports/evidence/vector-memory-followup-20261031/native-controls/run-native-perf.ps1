$ErrorActionPreference = 'Stop'

$Bundle = 'C:\Users\suanr\genesis-vector-memory-followup-20261031\followuproot\native-perf-abbaab'
$Marker = Join-Path $Bundle 'native-perf-clear'
$RunsRoot = Join-Path $Bundle 'runs'
$NodeExe = 'C:\Users\suanr\AppData\Local\Programs\nodejs-v24.20.0\node.exe'
$WriterScript = Join-Path $Bundle 'write-profiled.cjs'
$VerifyScript = Join-Path $Bundle 'verify-profiled.cjs'
$MaxSeconds = 300.0
$MaxRssBytes = [int64](4GB)
$MinFreeRamBytes = [int64](6GB)
$Utf8NoBom = New-Object System.Text.UTF8Encoding($false)

$Profiles = @{
  A = @{
    name = 'baseline'
    addon = 'C:\Users\suanr\genesis-vector-memory-followup-20261031\baseline-profiled.node'
    sha256 = '8BDE8388D401F82CB03D4A7C039EF45963AC77B8B39F7741DAECC260D08FF4C2'
  }
  B = @{
    name = 'candidate'
    addon = 'C:\Users\suanr\genesis-vector-memory-followup-20261031\candidate-profiled.node'
    sha256 = '8E3AAC3A8BD6A33EADB5B170F27D1A8C345FD08EFFAD13E4396190B23F9CFBB5'
  }
}
$Order = @('A', 'B', 'B', 'A', 'A', 'B')

function Write-JsonLine {
  param([string]$Path, [object]$Value)
  $line = ConvertTo-Json -InputObject $Value -Depth 24 -Compress
  [System.IO.File]::AppendAllText($Path, $line + [Environment]::NewLine, $Utf8NoBom)
}

function Write-JsonFile {
  param([string]$Path, [object]$Value)
  $json = ConvertTo-Json -InputObject $Value -Depth 24
  [System.IO.File]::WriteAllText($Path, $json + [Environment]::NewLine, $Utf8NoBom)
}

function Get-HostCpuPercent {
  $record = Get-CimInstance -ClassName Win32_PerfFormattedData_PerfOS_Processor -Filter "Name='_Total'" -ErrorAction Stop
  return [double]$record.PercentProcessorTime
}

function Get-FreeRamBytes {
  $os = Get-CimInstance -ClassName Win32_OperatingSystem -ErrorAction Stop
  return [int64]$os.FreePhysicalMemory * 1024
}

function Get-BlockingOwnedProcesses {
  $items = Get-CimInstance -ClassName Win32_Process -ErrorAction Stop
  $blocked = @()
  foreach ($item in $items) {
    $cmd = [string]$item.CommandLine
    if ($item.Name -match '^(cargo|rustc|rust-lld|link)\.exe$') {
      $blocked += [pscustomobject]@{ pid = $item.ProcessId; name = $item.Name; command_line = $cmd; reason = 'compiler_or_build_driver' }
      continue
    }
    if ($cmd -match '(?i)D:\\GenesisBlock_Dev\\GenesisBlock\\target\\(debug|release)\\deps\\[^"]*(test|tests)[^"]*\.exe') {
      $blocked += [pscustomobject]@{ pid = $item.ProcessId; name = $item.Name; command_line = $cmd; reason = 'repo_test_executable' }
      continue
    }
    if ($item.Name -ieq 'node.exe' -and $cmd -match '(?i)(baseline-profiled|candidate-profiled|write-profiled\.cjs|verify-profiled\.cjs)') {
      $blocked += [pscustomobject]@{ pid = $item.ProcessId; name = $item.Name; command_line = $cmd; reason = 'native_profile_benchmark' }
    }
  }
  return $blocked
}

function Wait-IdleGate {
  param([string]$Label, [string]$LogPath)
  while ($true) {
    if (-not (Test-Path -LiteralPath $Marker -PathType Leaf)) {
      Write-JsonLine $LogPath @{ phase = 'waiting_for_clear_marker'; label = $Label; marker = $Marker; utc = [DateTime]::UtcNow.ToString('o') }
      Start-Sleep -Seconds 5
      continue
    }

    $blockersBefore = @(Get-BlockingOwnedProcesses)
    if ($blockersBefore.Count -gt 0) {
      Write-JsonLine $LogPath @{ phase = 'idle_gate_blocked'; label = $Label; reason = 'owned_build_test_or_benchmark_process'; blockers = $blockersBefore; utc = [DateTime]::UtcNow.ToString('o') }
      Start-Sleep -Seconds 5
      continue
    }

    $samples = @()
    $minRam = [int64]::MaxValue
    for ($i = 0; $i -lt 10; $i++) {
      $cpu = Get-HostCpuPercent
      $ram = Get-FreeRamBytes
      $samples += [pscustomobject]@{ cpu_percent = $cpu; free_ram_bytes = $ram; utc = [DateTime]::UtcNow.ToString('o') }
      if ($ram -lt $minRam) { $minRam = $ram }
      if ($i -lt 9) { Start-Sleep -Seconds 1 }
    }
    $avgCpu = [double](($samples | Measure-Object -Property cpu_percent -Average).Average)
    $blockersAfter = @(Get-BlockingOwnedProcesses)
    if ($avgCpu -lt 20.0 -and $minRam -ge $MinFreeRamBytes -and $blockersAfter.Count -eq 0) {
      $gate = [pscustomobject]@{
        label = $Label
        cpu_samples_10s = @($samples | ForEach-Object { $_.cpu_percent })
        avg_cpu_percent_10s = $avgCpu
        min_free_ram_bytes_10s = $minRam
        blockers = @()
        marker = $Marker
        passed_utc = [DateTime]::UtcNow.ToString('o')
      }
      Write-JsonLine $LogPath @{ phase = 'idle_gate_passed'; gate = $gate }
      return $gate
    }

    Write-JsonLine $LogPath @{
      phase = 'idle_gate_blocked'
      label = $Label
      avg_cpu_percent_10s = $avgCpu
      min_free_ram_bytes_10s = $minRam
      blockers = $blockersAfter
      samples = $samples
      utc = [DateTime]::UtcNow.ToString('o')
    }
    Start-Sleep -Seconds 5
  }
}

function Get-ProcessIoSample {
  param([int]$ProcessId)
  try {
    $item = Get-CimInstance -ClassName Win32_PerfFormattedData_PerfProc_Process -Filter "IDProcess=$ProcessId" -ErrorAction Stop | Select-Object -First 1
    if ($null -eq $item) { return $null }
    return [pscustomobject]@{
      read_bytes_per_sec = [int64]$item.IOReadBytesPersec
      write_bytes_per_sec = [int64]$item.IOWriteBytesPersec
      data_bytes_per_sec = [int64]$item.IODataBytesPersec
    }
  } catch {
    return $null
  }
}

function Get-GlobalDiskSample {
  try {
    $item = Get-CimInstance -ClassName Win32_PerfFormattedData_PerfDisk_PhysicalDisk -Filter "Name='_Total'" -ErrorAction Stop | Select-Object -First 1
    if ($null -eq $item) { return $null }
    return [pscustomobject]@{
      read_bytes_per_sec = [int64]$item.DiskReadBytesPersec
      write_bytes_per_sec = [int64]$item.DiskWriteBytesPersec
      percent_disk_time = [double]$item.PercentDiskTime
    }
  } catch {
    return $null
  }
}

function Stop-OnlyOwnedChild {
  param([System.Diagnostics.Process]$Process, [string]$ScriptPath, [string]$AddonPath, [string]$DbPath, [string]$Reason)
  $pidValue = $Process.Id
  $item = Get-CimInstance -ClassName Win32_Process -Filter "ProcessId=$pidValue" -ErrorAction SilentlyContinue
  $exact = $false
  if ($item -and $item.ExecutablePath -and [string]$item.ExecutablePath -ieq $NodeExe) {
    $cmd = [string]$item.CommandLine
    $exact = $cmd.Contains($ScriptPath) -and $cmd.Contains($AddonPath) -and $cmd.Contains($DbPath)
  }
  if (-not $exact) {
    throw "cap reached but child identity did not match; refusing to stop PID $pidValue"
  }
  Stop-Process -Id $pidValue -Force -ErrorAction Stop
  Write-JsonLine $script:LauncherLog @{ phase = 'owned_child_stopped'; pid = $pidValue; reason = $Reason; script = $ScriptPath; addon = $AddonPath; db_path = $DbPath; utc = [DateTime]::UtcNow.ToString('o') }
}

function Invoke-MonitoredChild {
  param(
    [string]$Label,
    [string]$ScriptPath,
    [string]$AddonPath,
    [string]$DbPath,
    [string[]]$Arguments,
    [string]$JobDir
  )
  $stdoutPath = Join-Path $JobDir ($Label + '.stdout.log')
  $stderrPath = Join-Path $JobDir ($Label + '.stderr.log')
  $timelinePath = Join-Path $JobDir ($Label + '.timeline.jsonl')
  $proc = Start-Process -FilePath $NodeExe -ArgumentList $Arguments -WorkingDirectory $JobDir -PassThru -WindowStyle Hidden -RedirectStandardOutput $stdoutPath -RedirectStandardError $stderrPath
  $watch = [System.Diagnostics.Stopwatch]::StartNew()
  $peakRss = [int64]0
  $peakPrivate = [int64]0
  $lastCpuSeconds = 0.0
  $stopReason = $null
  $sampleNo = 0

  while ($true) {
    $proc.Refresh()
    if ($proc.HasExited) { break }
    $elapsed = $watch.Elapsed.TotalSeconds
    $osProcess = Get-Process -Id $proc.Id -ErrorAction SilentlyContinue
    if ($null -eq $osProcess) { break }
    $rss = [int64]$osProcess.WorkingSet64
    $peak = [int64]$osProcess.PeakWorkingSet64
    $private = [int64]$osProcess.PrivateMemorySize64
    if ($rss -gt $peakRss) { $peakRss = $rss }
    if ($peak -gt $peakRss) { $peakRss = $peak }
    if ($private -gt $peakPrivate) { $peakPrivate = $private }
    $cpuSeconds = [double]$osProcess.TotalProcessorTime.TotalSeconds
    $lastCpuSeconds = $cpuSeconds
    $hostCpu = Get-HostCpuPercent
    $freeRam = Get-FreeRamBytes
    $procIo = Get-ProcessIoSample -ProcessId $proc.Id
    $disk = Get-GlobalDiskSample
    $sampleNo++
    Write-JsonLine $timelinePath @{
      sample = $sampleNo
      elapsed_s = $elapsed
      utc = [DateTime]::UtcNow.ToString('o')
      pid = $proc.Id
      process_cpu_seconds = $cpuSeconds
      process_working_set_bytes = $rss
      process_os_peak_working_set_bytes = $peak
      process_private_bytes = $private
      host_cpu_percent = $hostCpu
      host_free_ram_bytes = $freeRam
      process_io = $procIo
      global_disk = $disk
    }

    if ($elapsed -ge $MaxSeconds) {
      $stopReason = 'wall_time_cap_300s'
      break
    }
    if ($peakRss -ge $MaxRssBytes) {
      $stopReason = 'rss_cap_4gib'
      break
    }
    Start-Sleep -Seconds 1
  }

  if ($stopReason) {
    Stop-OnlyOwnedChild -Process $proc -ScriptPath $ScriptPath -AddonPath $AddonPath -DbPath $DbPath -Reason $stopReason
  }
  try { $proc.WaitForExit() } catch {}
  try {
    $proc.Refresh()
    $osPeakAfterExit = [int64]$proc.PeakWorkingSet64
    if ($osPeakAfterExit -gt $peakRss) { $peakRss = $osPeakAfterExit }
    $lastCpuSeconds = [double]$proc.TotalProcessorTime.TotalSeconds
  } catch {}
  $exitCode = $null
  try { $exitCode = $proc.ExitCode } catch {}
  $watch.Stop()

  $finalProc = Get-Process -Id $proc.Id -ErrorAction SilentlyContinue
  if ($finalProc) {
    if ([int64]$finalProc.PeakWorkingSet64 -gt $peakRss) { $peakRss = [int64]$finalProc.PeakWorkingSet64 }
    if ([int64]$finalProc.PrivateMemorySize64 -gt $peakPrivate) { $peakPrivate = [int64]$finalProc.PrivateMemorySize64 }
    $lastCpuSeconds = [double]$finalProc.TotalProcessorTime.TotalSeconds
  }

  $corePercent = 0.0
  if ($watch.Elapsed.TotalSeconds -gt 0 -and [Environment]::ProcessorCount -gt 0) {
    $corePercent = 100.0 * $lastCpuSeconds / ($watch.Elapsed.TotalSeconds * [Environment]::ProcessorCount)
  }
  return [pscustomobject]@{
    label = $Label
    pid = $proc.Id
    script = $ScriptPath
    addon = $AddonPath
    db_path = $DbPath
    elapsed_s = [Math]::Round($watch.Elapsed.TotalSeconds, 3)
    process_cpu_seconds = [Math]::Round($lastCpuSeconds, 3)
    process_cpu_percent_of_host = [Math]::Round($corePercent, 3)
    os_peak_rss_bytes = $peakRss
    sampled_peak_rss_bytes = $peakRss
    sampled_peak_private_bytes = $peakPrivate
    exit_code = $exitCode
    stop_reason = $stopReason
    stdout_path = $stdoutPath
    stderr_path = $stderrPath
    timeline_path = $timelinePath
    sample_count = $sampleNo
  }
}

function Get-JsonFromFile {
  param([string]$Path)
  if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) { return $null }
  return Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json
}

function Get-Median {
  param([double[]]$Values)
  if ($Values.Count -eq 0) { return $null }
  $sorted = @($Values | Sort-Object)
  $middle = [int][Math]::Floor($sorted.Count / 2)
  if (($sorted.Count % 2) -eq 1) { return [double]$sorted[$middle] }
  return ([double]$sorted[$middle - 1] + [double]$sorted[$middle]) / 2.0
}

if (-not (Test-Path -LiteralPath $NodeExe -PathType Leaf)) { throw "Node executable missing: $NodeExe" }
foreach ($path in @($WriterScript, $VerifyScript)) {
  if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "benchmark script missing: $path" }
}
$hostInfo = Get-CimInstance -ClassName Win32_OperatingSystem -ErrorAction Stop
$computerInfo = Get-CimInstance -ClassName Win32_ComputerSystem -ErrorAction Stop
$bundleStamp = [DateTime]::UtcNow.ToString('yyyyMMddTHHmmssZ')
$RunDir = Join-Path $RunsRoot ('native-perf-' + $bundleStamp)
if (Test-Path -LiteralPath $RunDir) { throw "refusing to overwrite run directory: $RunDir" }
[void][System.IO.Directory]::CreateDirectory($RunDir)
$script:LauncherLog = Join-Path $RunDir 'launcher.jsonl'

$profileManifest = @{}
foreach ($key in @('A', 'B')) {
  $profile = $Profiles[$key]
  if (-not (Test-Path -LiteralPath $profile.addon -PathType Leaf)) { throw "addon missing: $($profile.addon)" }
  $actualHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $profile.addon).Hash.ToUpperInvariant()
  if ($actualHash -ne $profile.sha256) { throw "addon hash mismatch for $($profile.name): $actualHash" }
  $file = Get-Item -LiteralPath $profile.addon
  $profileManifest[$profile.name] = @{
    addon_path = $profile.addon
    sha256 = $actualHash
    bytes = $file.Length
    last_write_utc = $file.LastWriteTimeUtc.ToString('o')
    optimized_dependencies = 'same dev package on both; engineopt0 checks true, per parent'
  }
}

$manifest = @{
  created_utc = [DateTime]::UtcNow.ToString('o')
  run_dir = $RunDir
  source_comparison = @{
    relationship = 'per parent: source-before/after differ only in binding/caches'
    baseline_source = @{
      path = 'C:\Users\suanr\genesis-vector-memory-followup-20261031\baseline-addon-profile-20261031T130406\source-lib.rs'
      sha256 = '60DB70FBC9FE2B133BD345DC697A1E0D823654617F27F9754328B7DE19FF61FE'
      source_head = 'ad94aa2e46025b7729d117ff376ab281ac15c49d'
    }
    candidate_source = @{
      path = 'D:\GenesisBlock_Dev\GenesisBlock\src\lib.rs'
      sha256 = '812AFDBE74E96AEE4F0C08B93488DDA4AC5A6C0E4784B72BB9F615D30C7877A9'
      source_head = 'ad94aa2e46025b7729d117ff376ab281ac15c49d'
    }
  }
  dependency_provenance = @{
    cargo_manifest_sha256_both = '00550FBAC8EDB5C1316B5EB21983B078039604ED3C1460689515BA5539F30E25'
    cargo_lock_sha256_both = 'EDA5BFA67C1077C96F2B84E44F740273E695C87A1A51B352CF18F2A031A4F8D2'
    engine_code_opt_level = 0
    optimized_dependency_packages = @('sha2', 'curve25519-dalek', 'lz4_flex')
    engineopt0_checks = 'true, per parent'
  }
  measurement_class = 'controlled native checkpoint benchmark; diagnostic, not release qualification'
  node_executable = $NodeExe
  node_version = 'recorded by each child from process.version'
  profiles = $profileManifest
  schedule = @('A-baseline', 'B-candidate', 'B-candidate', 'A-baseline', 'A-baseline', 'B-candidate')
  workload = @{
    node_count = 2000
    operations = 'individual addNode calls, one at a time'
    id_format = 'mem:{i}'
    labels = @('Memory', 'Episodic')
    props = @{
      text = 'observation {i}: the user asked about journal segment retention and folding'
      source = 'session-2026-08-17'
      confidence = 0.87
      tags = @('journal', 'retention', 'wp-1.2')
    }
    lang = 'en'
    vector_dim = 4
    embeddings = 'none'
    retention = 'default'
    page_cache_mb = 32
  }
  caps = @{ wall_seconds_per_child = 300; rss_bytes_per_child = $MaxRssBytes }
  gates = @{ clear_marker = $Marker; global_cpu_average_10s_percent_below = 20; free_ram_bytes_at_least = $MinFreeRamBytes; no_other_owned_build_test_benchmark = $true }
  host = @{
    computer_name = $computerInfo.Name
    os_caption = $hostInfo.Caption
    os_version = $hostInfo.Version
    os_build = $hostInfo.BuildNumber
    logical_processors = [Environment]::ProcessorCount
    total_ram_bytes = [int64]$computerInfo.TotalPhysicalMemory
    power_shell_version = $PSVersionTable.PSVersion.ToString()
    startup_free_ram_bytes = Get-FreeRamBytes
  }
}
Write-JsonFile (Join-Path $RunDir 'manifest.json') $manifest

$results = New-Object System.Collections.ArrayList
$success = $true
for ($index = 0; $index -lt $Order.Count; $index++) {
  $key = $Order[$index]
  $profile = $Profiles[$key]
  $jobName = ('{0:D2}-{1}' -f ($index + 1), $profile.name)
  $gate = Wait-IdleGate -Label ($jobName + '-writer') -LogPath $script:LauncherLog
  $jobDir = Join-Path $RunDir $jobName
  if (Test-Path -LiteralPath $jobDir) { throw "refusing to overwrite job directory: $jobDir" }
  [void][System.IO.Directory]::CreateDirectory($jobDir)
  $dbPath = Join-Path $jobDir 'database'
  Write-JsonLine $script:LauncherLog @{ phase = 'job_started'; index = $index + 1; profile = $profile.name; gate = $gate; db_path = $dbPath; utc = [DateTime]::UtcNow.ToString('o') }

  $writerPhasePath = Join-Path $jobDir 'writer-progress.jsonl'
  $writerResultPath = Join-Path $jobDir 'writer-result.json'
  $writerArgs = @($WriterScript, $profile.addon, $dbPath, $writerPhasePath, $writerResultPath)
  $writerRun = Invoke-MonitoredChild -Label 'writer' -ScriptPath $WriterScript -AddonPath $profile.addon -DbPath $dbPath -Arguments $writerArgs -JobDir $jobDir
  $writerResult = Get-JsonFromFile $writerResultPath

  $verifyRun = $null
  $verifyResult = $null
  if (Test-Path -LiteralPath $dbPath -PathType Container) {
    $verifyGate = Wait-IdleGate -Label ($jobName + '-read-only-verify') -LogPath $script:LauncherLog
    $verifyResultPath = Join-Path $jobDir 'verify-result.json'
    $verifyArgs = @($VerifyScript, $profile.addon, $dbPath, $verifyResultPath)
    $verifyRun = Invoke-MonitoredChild -Label 'read-only-verify' -ScriptPath $VerifyScript -AddonPath $profile.addon -DbPath $dbPath -Arguments $verifyArgs -JobDir $jobDir
    $verifyResult = Get-JsonFromFile $verifyResultPath
  }

  $serializationLines = @()
  foreach ($logPath in @($writerRun.stdout_path, $writerRun.stderr_path, $writerPhasePath)) {
    if (Test-Path -LiteralPath $logPath -PathType Leaf) {
      $serializationLines += @(Get-Content -LiteralPath $logPath | Where-Object { $_ -match '(?i)(SERIALIZATION_(FAILED|ERROR)|(?:failed|invalid|error).{0,60}(serializ|serde|json)|(?:serializ|serde|json).{0,60}(failed|error|invalid))' })
    }
  }
  $jobSummary = @{
    index = $index + 1
    profile = $profile.name
    binary_sha256 = $profile.sha256
    writer = $writerRun
    writer_result = $writerResult
    read_only_verification = $verifyRun
    verification_result = $verifyResult
    observed_serialization_error_lines = $serializationLines
    gate_before_writer = $gate
    db_path = $dbPath
    status = if ($writerRun.exit_code -eq 0 -and $writerResult.status -eq 'complete' -and $verifyRun -and $verifyRun.exit_code -eq 0 -and $verifyResult.status -eq 'complete') { 'pass' } else { 'fail' }
  }
  Write-JsonFile (Join-Path $jobDir 'job-summary.json') $jobSummary
  [void]$results.Add([pscustomobject]$jobSummary)
  Write-JsonLine $script:LauncherLog @{ phase = 'job_complete'; index = $index + 1; profile = $profile.name; status = $jobSummary.status; writer_exit = $writerRun.exit_code; verify_exit = if ($verifyRun) { $verifyRun.exit_code } else { $null }; utc = [DateTime]::UtcNow.ToString('o') }

  if ($jobSummary.status -ne 'pass') {
    $success = $false
    break
  }
}

$baseline = @($results | Where-Object { $_.profile -eq 'baseline' -and $_.status -eq 'pass' })
$candidate = @($results | Where-Object { $_.profile -eq 'candidate' -and $_.status -eq 'pass' })
$summary = @{
  finished_utc = [DateTime]::UtcNow.ToString('o')
  run_dir = $RunDir
  requested_order = @('baseline', 'candidate', 'candidate', 'baseline', 'baseline', 'candidate')
  completed_jobs = @($results.Count)
  all_six_passed = ($success -and $results.Count -eq 6)
  note = 'Drop is intentionally not measured: the addon autonomic Arc outlives its wrapper; every writer exits after explicit saveState.'
  raw_results = @($results)
}
if ($baseline.Count -eq 3) {
  $summary.baseline_medians = @{
    open_ms = Get-Median @($baseline | ForEach-Object { [double]$_.writer_result.open_ms })
    ingest_ms = Get-Median @($baseline | ForEach-Object { [double]$_.writer_result.ingest_elapsed_ms })
    checkpoint_ms = Get-Median @($baseline | ForEach-Object { [double]$_.writer_result.explicit_checkpoint_elapsed_ms })
    wall_s = Get-Median @($baseline | ForEach-Object { [double]$_.writer.elapsed_s })
    process_cpu_s = Get-Median @($baseline | ForEach-Object { [double]$_.writer.process_cpu_seconds })
    peak_rss_bytes = Get-Median @($baseline | ForEach-Object { [double]$_.writer.os_peak_rss_bytes })
  }
}
if ($candidate.Count -eq 3) {
  $summary.candidate_medians = @{
    open_ms = Get-Median @($candidate | ForEach-Object { [double]$_.writer_result.open_ms })
    ingest_ms = Get-Median @($candidate | ForEach-Object { [double]$_.writer_result.ingest_elapsed_ms })
    checkpoint_ms = Get-Median @($candidate | ForEach-Object { [double]$_.writer_result.explicit_checkpoint_elapsed_ms })
    wall_s = Get-Median @($candidate | ForEach-Object { [double]$_.writer.elapsed_s })
    process_cpu_s = Get-Median @($candidate | ForEach-Object { [double]$_.writer.process_cpu_seconds })
    peak_rss_bytes = Get-Median @($candidate | ForEach-Object { [double]$_.writer.os_peak_rss_bytes })
  }
}
Write-JsonFile (Join-Path $RunDir 'run-summary.json') $summary
Write-JsonLine $script:LauncherLog @{ phase = 'run_finished'; all_six_passed = $summary.all_six_passed; completed_jobs = $results.Count; utc = [DateTime]::UtcNow.ToString('o') }
if (-not $summary.all_six_passed) { exit 2 }
exit 0
