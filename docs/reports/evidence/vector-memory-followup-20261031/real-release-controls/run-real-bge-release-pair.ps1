[CmdletBinding()]
param([switch]$Run)

$ErrorActionPreference = 'Stop'
$ControllerRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoPath = 'D:\GenesisBlock_Dev\GenesisBlock'
$InputPath = 'C:\Users\suanr\genesis-vector-memory-20261031\inputs\real-11266-1024'
$BaselineExe = 'C:\Users\suanr\genesis-vector-memory-followup-20261031\baseline-release.exe'
$CandidateExe = 'C:\Users\suanr\genesis-vector-memory-followup-20261031\candidate-release.exe'
$ClearMarker = Join-Path $ControllerRoot 'vector-release-clear'
$ResultsRoot = Join-Path $ControllerRoot 'runs'

$Expected = [ordered]@{
    baselineExe = 'd820a5af8a54f9a95e2722dd953d3dc50dff074687ec361bfe1464b02f0a6c87'
    candidateExe = 'eedf2ce01df03c2ce32d629131e896166fd895166b8a3acd823cde68c5d26ff8'
    benchSource = '46694637e7e028f8a12c5c5f5eb83c8bac3ca33e7e276b1236e097f970f53c3f'
    benchEntrySource = '2f73000dfc60fd6d4e01f24b7670ff94987e4d5e76acebabf3f20ea28ee4c6c5'
    corpus = '20978d757c33a29dcd8e89e5708762738b10fc8da0f01358bcb325741f2f6057'
    queries = '6ccad031b956c6111519e6b0dfe97e7c63bd85801d9934efe4438f76d152ca68'
    truth = '2c133fd68ed2b60345715c24667a830ebf3ab303528e8eec0300385125e37c9c'
    meta = 'f79c423a069d8c4faa9a2bf8fff920d0dc4102261b28c35692544974137a89de'
}
$RssLimitBytes = [int64]2147483648
$DeadlineSeconds = 1200
$MinimumFreeBytes = [int64]6442450944
$Schedule = @(
    [pscustomobject]@{label='01-baseline';stage='baseline';exe=$BaselineExe}
    [pscustomobject]@{label='02-candidate';stage='candidate';exe=$CandidateExe}
)


function Get-Hash([string]$Path) {
    (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

function Write-JsonFile([string]$Path, $Value) {
    $Value | ConvertTo-Json -Depth 18 | Set-Content -LiteralPath $Path -Encoding utf8
}

function Write-JsonLine([string]$Path, $Value) {
    $line = ($Value | ConvertTo-Json -Depth 12 -Compress) + [Environment]::NewLine
    [System.IO.File]::AppendAllText($Path,$line,[System.Text.UTF8Encoding]::new($false))
}

function Get-InputHashes {
    [ordered]@{
        corpus = Get-Hash (Join-Path $InputPath 'corpus.f32')
        queries = Get-Hash (Join-Path $InputPath 'queries.f32')
        truth = Get-Hash (Join-Path $InputPath 'truth.json')
        meta = Get-Hash (Join-Path $InputPath 'meta.json')
    }
}

function Test-ExpectedHashes($Hashes) {
    foreach ($name in @('corpus','queries','truth','meta')) {
        if ($Hashes[$name] -ne $Expected[$name]) {
            throw "Frozen input hash mismatch for $($name): $($Hashes[$name])"
        }
    }
}

function Get-HostSample {
    $cpuRecord = Get-CimInstance -ClassName Win32_PerfFormattedData_PerfOS_Processor -Filter "Name='_Total'" -ErrorAction Stop
    $osRecord = Get-CimInstance -ClassName Win32_OperatingSystem -ErrorAction Stop
    [pscustomobject]@{
        at_utc = (Get-Date).ToUniversalTime().ToString('o')
        total_cpu_percent = [double]$cpuRecord.PercentProcessorTime
        free_physical_bytes = [int64]$osRecord.FreePhysicalMemory * 1024
    }
}

function Get-HostEnvironment {
    $computer = Get-CimInstance -ClassName Win32_ComputerSystem -ErrorAction Stop
    $osRecord = Get-CimInstance -ClassName Win32_OperatingSystem -ErrorAction Stop
    $cpuRecords = @(Get-CimInstance -ClassName Win32_Processor -ErrorAction Stop)
    $dataDisk = Get-CimInstance -ClassName Win32_LogicalDisk -Filter "DeviceID='C:'" -ErrorAction SilentlyContinue
    $diskFreeBytes = $null
    if ($null -ne $dataDisk) { $diskFreeBytes = [int64]$dataDisk.FreeSpace }
    [ordered]@{
        captured_at_utc = (Get-Date).ToUniversalTime().ToString('o')
        hostname = $env:COMPUTERNAME
        os_caption = [string]$osRecord.Caption
        os_version = [string]$osRecord.Version
        os_build = [string]$osRecord.BuildNumber
        cpu_name = [string]$cpuRecords[0].Name
        logical_cpu_count = [int]$computer.NumberOfLogicalProcessors
        physical_memory_bytes = [int64]$computer.TotalPhysicalMemory
        free_memory_bytes = [int64]$osRecord.FreePhysicalMemory * 1024
        data_disk = 'C:'
        data_disk_free_bytes = $diskFreeBytes
    }
}
function Get-Admission([string]$TimelinePath) {
    $samples = @()
    for ($index = 0; $index -le 10; $index++) {
        $sample = Get-HostSample
        $samples += $sample
        Write-JsonLine $TimelinePath $sample
        if ($index -lt 10) { Start-Sleep -Seconds 1 }
    }
    $cpuAverage = ($samples | Measure-Object -Property total_cpu_percent -Average).Average
    $freeMinimum = ($samples | Measure-Object -Property free_physical_bytes -Minimum).Minimum
    $result = [ordered]@{
        sample_count = $samples.Count
        observed_window_seconds = 10
        average_total_cpu_percent = [double]$cpuAverage
        minimum_free_physical_bytes = [int64]$freeMinimum
        cpu_gate_passed = ([double]$cpuAverage -lt 20.0)
        memory_gate_passed = ([int64]$freeMinimum -ge $MinimumFreeBytes)
        samples = $samples
    }
    if (-not $result.cpu_gate_passed -or -not $result.memory_gate_passed) {
        throw "Host admission failed: avg CPU=$cpuAverage%; minimum free RAM=$freeMinimum bytes"
    }
    $result
}

function Get-ProcessTreeSnapshot([int]$RootProcessId) {
    $all = @(Get-CimInstance -ClassName Win32_Process -Property ProcessId,ParentProcessId,Name,ExecutablePath,ReadTransferCount,WriteTransferCount -ErrorAction Stop)
    $known = [System.Collections.Generic.List[int]]::new()
    $known.Add($RootProcessId)
    $cursor = 0
    while ($cursor -lt $known.Count) {
        $parentId = $known[$cursor]
        foreach ($entry in $all) {
            if ([int]$entry.ParentProcessId -eq $parentId -and -not $known.Contains([int]$entry.ProcessId)) {
                $known.Add([int]$entry.ProcessId)
            }
        }
        $cursor++
    }
    $rows = @()
    foreach ($id in $known) {
        $wm = $all | Where-Object { [int]$_.ProcessId -eq $id } | Select-Object -First 1
        $process = Get-Process -Id $id -ErrorAction SilentlyContinue
        if ($null -ne $wm -and $null -ne $process) {
            $cpuSeconds = 0.0
            if ($null -ne $process.CPU) { $cpuSeconds = [double]$process.CPU }
            $rows += [pscustomobject]@{
                pid = [int]$id
                parent_pid = [int]$wm.ParentProcessId
                name = [string]$wm.Name
                executable_path = [string]$wm.ExecutablePath
                working_set_bytes = [int64]$process.WorkingSet64
                private_bytes = [int64]$process.PrivateMemorySize64
                peak_working_set_bytes = [int64]$process.PeakWorkingSet64
                cpu_seconds = $cpuSeconds
                io_read_bytes = [int64]$wm.ReadTransferCount
                io_write_bytes = [int64]$wm.WriteTransferCount
            }
        }
    }
    $rows
}

function Stop-OwnedProcessTree([int]$RootProcessId, [string]$ExpectedRootExe) {
    $rootInfo = Get-CimInstance -ClassName Win32_Process -Filter "ProcessId = $RootProcessId" -ErrorAction SilentlyContinue
    if ($null -eq $rootInfo) { return }
    if (-not [string]::Equals([string]$rootInfo.ExecutablePath, $ExpectedRootExe, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Refusing to stop PID ${RootProcessId}: executable path no longer matches the launched binary"
    }
    $tree = @(Get-ProcessTreeSnapshot $RootProcessId)
    foreach ($entry in ($tree | Sort-Object { if ($_.pid -eq $RootProcessId) { 0 } else { 1 } } -Descending)) {
        try { Stop-Process -Id $entry.pid -Force -ErrorAction Stop } catch {}
    }
}

function Get-DirectoryBytes([string]$Path) {
    $files = @(Get-ChildItem -LiteralPath $Path -File -Recurse -ErrorAction SilentlyContinue)
    [int64](($files | Measure-Object -Property Length -Sum).Sum)
}

function Get-Median($Values) {
    $sorted = @($Values | Where-Object { $null -ne $_ } | ForEach-Object { [double]$_ } | Sort-Object)
    if ($sorted.Count -eq 0) { return $null }
    $middle = [int][Math]::Floor($sorted.Count / 2)
    if (($sorted.Count % 2) -eq 1) { return $sorted[$middle] }
    return ($sorted[$middle - 1] + $sorted[$middle]) / 2.0
}

function Get-StageObservation($Jobs, [string]$Stage) {
    $selected = @($Jobs | Where-Object { $_.stage -eq $Stage -and $_.status -eq 'complete' })
    $fields = @('ingest_s','rebuild_s','compaction_s','snapshot_s','reopen_s','fresh_read_only_reopen_s','search10_recall','search10_p50_us','search100_recall','search100_p50_us')
    $values = [ordered]@{}
    if ($selected.Count -eq 1) {
        foreach ($field in $fields) { $values[$field] = ($selected[0].metrics)[$field] }
    }
    [ordered]@{completed_runs=$selected.Count;single_observation=$values}
}

function Get-OutputManifest([string]$DatabasePath) {
    $statePath = Join-Path $DatabasePath 'state.json'
    if (-not (Test-Path -LiteralPath $statePath)) { return $null }
    $state = Get-Content -Raw -LiteralPath $statePath | ConvertFrom-Json
    [ordered]@{
        state_sha256 = Get-Hash $statePath
        p6_manifest_sha256 = $state.p6.manifest.manifest_sha256
        components = @($state.p6.manifest.components | ForEach-Object {
            [ordered]@{path=$_.path;bytes=$_.bytes;sha256=$_.sha256}
        })
    }
}

if (-not $Run) {
    [pscustomobject]@{
        prepared_controller = $MyInvocation.MyCommand.Path
        action = 'No benchmark will run unless -Run is supplied.'
        required_clear_marker = $ClearMarker
        schedule = @($Schedule | ForEach-Object { "$($_.label) $($_.stage)" })
        input_path = $InputPath
        case = 'real-11266-1024-none-r0; quant=none; rerank=0; compact=1; retention=full; RAYON_NUM_THREADS=4; ef=200; q=24'
        caps = '2 GiB process-tree working set; 1200 seconds per job; 10-second host admission, average CPU <20%, free RAM >=6 GiB'
    } | ConvertTo-Json -Depth 6
    return
}

if (-not (Test-Path -LiteralPath $ClearMarker -PathType Leaf)) {
    throw "Runtime-clear marker is missing. Expected $ClearMarker to contain query_reopen_clear."
}
$markerText = (Get-Content -Raw -LiteralPath $ClearMarker).Trim()
if ($markerText -ne 'query_reopen_clear') {
    throw "Runtime-clear marker has unexpected content; expected exactly query_reopen_clear."
}
foreach ($path in @($InputPath,(Split-Path -Parent $BaselineExe),(Split-Path -Parent $CandidateExe),$RepoPath)) {
    if (-not (Test-Path -LiteralPath $path)) { throw "Required path is missing: $path" }
}
foreach ($exe in @($BaselineExe,$CandidateExe)) {
    if (-not (Test-Path -LiteralPath $exe -PathType Leaf)) { throw "Missing release binary: $exe" }
}
$baselineHash = Get-Hash $BaselineExe
$candidateHash = Get-Hash $CandidateExe
$sourceHash = Get-Hash (Join-Path $RepoPath 'benches\vector_memory.rs')
$entrySourceHash = Get-Hash (Join-Path $RepoPath 'benches\vbench_genesis.rs')
if ($baselineHash -ne $Expected.baselineExe -or $candidateHash -ne $Expected.candidateExe -or $sourceHash -ne $Expected.benchSource -or $entrySourceHash -ne $Expected.benchEntrySource) {
    throw 'Pinned executable or benchmark source hash changed; refusing to run this prepared comparison.'
}
$initialInputHashes = Get-InputHashes
Test-ExpectedHashes $initialInputHashes
$meta = Get-Content -Raw -LiteralPath (Join-Path $InputPath 'meta.json') | ConvertFrom-Json
if ($meta.n -ne 11266 -or $meta.dim -ne 1024 -or $meta.q -ne 24 -or $meta.model -ne 'bge-m3-repository-fixture') {
    throw 'Frozen BGE-M3 input metadata does not match the prepared case.'
}

$repoHead = (& git -c safe.directory=D:/GenesisBlock_Dev/GenesisBlock -C $RepoPath rev-parse HEAD 2>&1 | Out-String).Trim()
if ($LASTEXITCODE -ne 0) { throw 'Could not record source revision for run provenance.' }
$repoStatus = @(& git -c safe.directory=D:/GenesisBlock_Dev/GenesisBlock -C $RepoPath status --short --untracked-files=no)
$lockPath = Join-Path $ControllerRoot 'runtime.lock'
$lockStream = [System.IO.File]::Open($lockPath,[System.IO.FileMode]::OpenOrCreate,[System.IO.FileAccess]::ReadWrite,[System.IO.FileShare]::None)
try {
    $existingRunExe = @(Get-CimInstance -ClassName Win32_Process -Property ProcessId,ExecutablePath -ErrorAction Stop | Where-Object {
        [string]::Equals([string]$_.ExecutablePath,$BaselineExe,[StringComparison]::OrdinalIgnoreCase) -or
        [string]::Equals([string]$_.ExecutablePath,$CandidateExe,[StringComparison]::OrdinalIgnoreCase)
    })
    if ($existingRunExe.Count -gt 0) {
        throw 'A pinned release benchmark executable is already running; refusing to overlap runs.'
    }
    if (-not (Test-Path -LiteralPath $ResultsRoot)) {
        New-Item -ItemType Directory -Path $ResultsRoot | Out-Null
    }
    $runId = (Get-Date).ToUniversalTime().ToString('yyyyMMddTHHmmssZ')
    $RunRoot = Join-Path $ResultsRoot "real-bge-release-pair-$runId"
    if (Test-Path -LiteralPath $RunRoot) { throw "Refuse to overwrite $RunRoot" }
    New-Item -ItemType Directory -Path $RunRoot | Out-Null
    $manifest = [ordered]@{
        status = 'running'
        run_id = $runId
        started_at_utc = (Get-Date).ToUniversalTime().ToString('o')
        repo_path = $RepoPath
        source_provenance = [ordered]@{repo_head=$repoHead;dirty_paths=$repoStatus}
        input_path = $InputPath
        input_hashes = $initialInputHashes
        baseline_exe = [ordered]@{path=$BaselineExe;sha256=$baselineHash}
        candidate_exe = [ordered]@{path=$CandidateExe;sha256=$candidateHash}
        benchmark_source = [ordered]@{path=(Join-Path $RepoPath 'benches\vector_memory.rs');sha256=$sourceHash}
        benchmark_entry_source = [ordered]@{path=(Join-Path $RepoPath 'benches\vbench_genesis.rs');sha256=$entrySourceHash}
        case = [ordered]@{n=11266;dim=1024;q=24;model=$meta.model;quant='none';rerank=$false;compact=$true;retention='full';ef_construction=200;ef_search=200;rayon_num_threads=4;batch_rows=256;search_k=@(10,100)}
        order = @($Schedule | ForEach-Object { "$($_.label) $($_.stage)" })
        limits = [ordered]@{process_tree_working_set_bytes=$RssLimitBytes;per_job_deadline_seconds=$DeadlineSeconds;host_admission_window_seconds=10;host_cpu_average_must_be_below_percent=20;minimum_free_ram_bytes=$MinimumFreeBytes}
        host_at_start = Get-HostSample
        host_environment_at_start = Get-HostEnvironment
        jobs = @()
    }
    Write-JsonFile (Join-Path $RunRoot 'run-manifest.json') $manifest

    $jobs = @()
    foreach ($slot in $Schedule) {
        $jobPath = Join-Path $RunRoot $slot.label
        if (Test-Path -LiteralPath $jobPath) { throw "Refuse to overwrite $jobPath" }
        New-Item -ItemType Directory -Path $jobPath | Out-Null
        $entry = [ordered]@{
            label = $slot.label
            stage = $slot.stage
            binary = $slot.exe
            status = 'starting'
            input_hashes_before = $null
            input_hashes_after = $null
            admission = $null
            exit_code = $null
            timeout = $false
            rss_limit_hit = $false
            process_timeline = (Join-Path $jobPath 'process-timeline.jsonl')
            stdout = (Join-Path $jobPath 'stdout.log')
            stderr = (Join-Path $jobPath 'stderr.log')
            metrics = $null
            output_manifest = $null
            error = $null
        }
        try {
            $entry.admission = Get-Admission (Join-Path $jobPath 'host-admission.jsonl')
            $entry.input_hashes_before = Get-InputHashes
            Test-ExpectedHashes $entry.input_hashes_before
            if (Test-Path -LiteralPath (Join-Path $jobPath 'gdb')) { throw 'Database path already exists; refusing overwrite.' }

            foreach ($name in @('GB_MEMORY_PREPARE','GB_MEMORY_CORPUS','GB_LIMIT','GB_EF_SWEEP','GB_MEMORY_OPEN_ONLY','GB_MEMORY_OPEN_OUT','GB_MEMORY_OPEN_REPEAT','GB_MEMORY_OPEN_READ_WRITE','GB_MEMORY_INPUT','GB_VECTOR_QUERY_DIAGNOSTICS')) {
                Remove-Item -LiteralPath "Env:$name" -ErrorAction SilentlyContinue
            }
            $env:GB_MEMORY_AUDIT = '1'
            $env:GB_MEMORY_COMPACT = '1'
            $env:GB_MEMORY_RETENTION = 'full'
            $env:GB_MEMORY_RSS_LIMIT_BYTES = "$RssLimitBytes"
            $env:GB_VBENCH = $InputPath
            $env:GB_MEMORY_OUT = $jobPath
            $env:GB_QUANT = 'none'
            $env:GB_RERANK = '0'
            $env:GB_EF = '200'
            $env:GB_VEC_Q = '24'
            $env:RAYON_NUM_THREADS = '4'

            $child = Start-Process -FilePath $slot.exe -PassThru -WindowStyle Hidden -WorkingDirectory $RepoPath -RedirectStandardOutput $entry.stdout -RedirectStandardError $entry.stderr
            $clock = [System.Diagnostics.Stopwatch]::StartNew()
            $peakTreeWorkingSet = [int64]0
            $peakTreePrivate = [int64]0
            $sampleIndex = 0
            $timedOut = $false
            $rssHit = $false
            while (-not $child.HasExited) {
                $tree = @(Get-ProcessTreeSnapshot $child.Id)
                $treeWorkingSet = [int64](($tree | Measure-Object -Property working_set_bytes -Sum).Sum)
                $treePrivate = [int64](($tree | Measure-Object -Property private_bytes -Sum).Sum)
                $peakTreeWorkingSet = [Math]::Max($peakTreeWorkingSet,$treeWorkingSet)
                $peakTreePrivate = [Math]::Max($peakTreePrivate,$treePrivate)
                $cpuSeconds = [double](($tree | Measure-Object -Property cpu_seconds -Sum).Sum)
                $readBytes = [int64](($tree | Measure-Object -Property io_read_bytes -Sum).Sum)
                $writeBytes = [int64](($tree | Measure-Object -Property io_write_bytes -Sum).Sum)
                $diskBytes = $null
                if (($sampleIndex % 30) -eq 0) { $diskBytes = Get-DirectoryBytes $jobPath }
                $hostSample = Get-HostSample
                Write-JsonLine $entry.process_timeline ([ordered]@{
                    at_utc = $hostSample.at_utc
                    elapsed_seconds = [Math]::Round($clock.Elapsed.TotalSeconds,3)
                    process_tree = $tree
                    tree_working_set_bytes = $treeWorkingSet
                    tree_private_bytes = $treePrivate
                    tree_cpu_seconds = $cpuSeconds
                    tree_io_read_bytes = $readBytes
                    tree_io_write_bytes = $writeBytes
                    output_disk_bytes = $diskBytes
                    host_cpu_percent = $hostSample.total_cpu_percent
                    host_free_physical_bytes = $hostSample.free_physical_bytes
                })
                if ($treeWorkingSet -gt $RssLimitBytes) {
                    $rssHit = $true
                    Stop-OwnedProcessTree $child.Id $slot.exe
                    break
                }
                if ($clock.Elapsed.TotalSeconds -ge $DeadlineSeconds) {
                    $timedOut = $true
                    Stop-OwnedProcessTree $child.Id $slot.exe
                    break
                }
                $sampleIndex++
                Start-Sleep -Seconds 1
                $child.Refresh()
            }
            $child.WaitForExit()
            $entry.exit_code = $child.ExitCode
            $entry.timeout = $timedOut
            $entry.rss_limit_hit = $rssHit
            $entry.peak_process_tree_working_set_bytes = $peakTreeWorkingSet
            $entry.peak_process_tree_private_bytes = $peakTreePrivate
            $entry.elapsed_seconds = [Math]::Round($clock.Elapsed.TotalSeconds,3)

            $entry.input_hashes_after = Get-InputHashes
            Test-ExpectedHashes $entry.input_hashes_after
            if (($entry.input_hashes_before | ConvertTo-Json -Compress) -ne ($entry.input_hashes_after | ConvertTo-Json -Compress)) {
                throw 'Frozen input hashes changed during this job.'
            }

            $memoryPath = Join-Path $jobPath 'memory.json'
            if (Test-Path -LiteralPath $memoryPath) {
                $result = Get-Content -Raw -LiteralPath $memoryPath | ConvertFrom-Json
                $databasePath = Join-Path $jobPath 'gdb'
                $entry.metrics = [ordered]@{
                    n = $result.n
                    dim = $result.dim
                    q = $result.q
                    quant = $result.quant
                    rerank = $result.rerank
                    retention = $result.retention
                    ingest_s = $result.ingest.seconds
                    ingest_peak_rss_bytes = $result.ingest.peak_rss_bytes
                    rebuild_s = $result.rebuild.seconds
                    rebuild_peak_rss_bytes = $result.rebuild.peak_rss_bytes
                    search10_recall = $result.search10.recall
                    search10_p50_us = $result.search10.p50_us
                    search10_p95_us = $result.search10.p95_us
                    search10_p99_us = $result.search10.p99_us
                    search100_recall = $result.search100.recall
                    search100_p50_us = $result.search100.p50_us
                    search100_p95_us = $result.search100.p95_us
                    search100_p99_us = $result.search100.p99_us
                    compaction_s = $result.compaction.seconds
                    compaction_peak_rss_bytes = $result.compaction.peak_rss_bytes
                    snapshot_s = $result.snapshot.seconds
                    snapshot_peak_rss_bytes = $result.snapshot.peak_rss_bytes
                    snapshot_error = $result.snapshot_error
                    reopen_s = $result.reopen.seconds
                    reopen_peak_rss_bytes = $result.reopen.peak_rss_bytes
                    reopen_error = $result.reopen_error
                    fresh_read_only_reopen_s = $result.fresh_reopen.memory.seconds
                    fresh_read_only_reopen_peak_rss_bytes = $result.fresh_reopen.memory.peak_rss_bytes
                    fresh_read_only_reopen_error = $result.fresh_reopen.error
                    fresh_read_only_coverage = $result.fresh_reopen.coverage
                    reopen_search10_recall = $result.reopen_search10.recall
                    reopen_search10_p50_us = $result.reopen_search10.p50_us
                    disk_bytes = $result.disk_bytes
                    coverage = $result.coverage
                    errors = @($result.snapshot_error,$result.reopen_error,$result.fresh_reopen.error,$result.search10.error,$result.search100.error,$result.reopen_search10.error) | Where-Object { $null -ne $_ }
                }
                $entry.output_manifest = Get-OutputManifest $databasePath
                $coverageGood = (@($entry.metrics.coverage | Where-Object { $_.collection -eq 'bench' -and $_.state -eq 'READY' -and $_.validated -eq $true -and $_.indexed_count -eq 11266 -and $_.missing_count -eq 0 -and $_.extra_count -eq 0 }).Count -gt 0)
                $freshCoverageGood = (@($entry.metrics.fresh_read_only_coverage | Where-Object { $_.collection -eq 'bench' -and $_.state -eq 'READY' -and $_.validated -eq $true -and $_.indexed_count -eq 11266 -and $_.missing_count -eq 0 -and $_.extra_count -eq 0 }).Count -gt 0)
                $searchContractGood = ($null -ne $entry.metrics.search10_recall -and $entry.metrics.search10_recall -ge 0.0 -and $entry.metrics.search10_recall -le 1.0 -and $null -ne $entry.metrics.search100_recall -and $entry.metrics.search100_recall -ge 0.0 -and $entry.metrics.search100_recall -le 1.0 -and $null -ne $entry.metrics.reopen_search10_recall -and $entry.metrics.reopen_search10_recall -ge 0.0 -and $entry.metrics.reopen_search10_recall -le 1.0)
                $good = ($entry.exit_code -eq 0 -and -not $entry.timeout -and -not $entry.rss_limit_hit -and
                    $entry.metrics.n -eq 11266 -and $entry.metrics.dim -eq 1024 -and $entry.metrics.q -eq 24 -and
                    $entry.metrics.quant -eq 'none' -and $entry.metrics.rerank -eq $false -and $entry.metrics.retention -eq 'full' -and
                    $entry.metrics.snapshot_error -eq $null -and $entry.metrics.reopen_error -eq $null -and
                    $entry.metrics.fresh_read_only_reopen_error -eq $null -and $entry.metrics.errors.Count -eq 0 -and $coverageGood -and $freshCoverageGood -and $searchContractGood)
                $entry.status = if ($good) { 'complete' } else { 'failed_contract_or_engine_error' }
                if (-not $good) { $entry.error = 'Process or benchmark result failed a correctness, persistence, retention, or resource check.' }
            } else {
                $entry.status = 'failed_no_memory_json'
                $entry.error = 'memory.json was not produced; partial logs and DB artifacts are retained.'
            }
        } catch {
            $entry.status = 'failed_controller'
            $entry.error = ($_ | Out-String).Trim()
        }
        $jobs += [pscustomobject]$entry
        $summary = [ordered]@{
            status = if (@($jobs | Where-Object { $_.status -ne 'complete' }).Count -eq 0) { 'running' } else { 'incomplete' }
            run_id = $runId
            order = @($Schedule | ForEach-Object { "$($_.label) $($_.stage)" })
            completed_jobs = @($jobs | Where-Object { $_.status -eq 'complete' }).Count
            jobs = $jobs
        }
        Write-JsonFile (Join-Path $RunRoot 'run-summary.json') $summary
        if ($entry.status -ne 'complete') { break }
    }

    $summary = [ordered]@{
        status = if ($jobs.Count -eq $Schedule.Count -and @($jobs | Where-Object { $_.status -ne 'complete' }).Count -eq 0) { 'complete' } else { 'incomplete' }
        run_id = $runId
        started_at_utc = $manifest.started_at_utc
        finished_at_utc = (Get-Date).ToUniversalTime().ToString('o')
        order = @($Schedule | ForEach-Object { "$($_.label) $($_.stage)" })
        completed_jobs = @($jobs | Where-Object { $_.status -eq 'complete' }).Count
        baseline = Get-StageObservation $jobs 'baseline'
        candidate = Get-StageObservation $jobs 'candidate'
        jobs = $jobs
    }
    Write-JsonFile (Join-Path $RunRoot 'run-summary.json') $summary
    $manifest.status = $summary.status
    $manifest.finished_at_utc = $summary.finished_at_utc
    $manifest.jobs = $jobs
    Write-JsonFile (Join-Path $RunRoot 'run-manifest.json') $manifest
    Write-Output "Run artifacts: $RunRoot"
    if ($summary.status -ne 'complete') { exit 1 }
}
finally {
    if ($null -ne $lockStream) { $lockStream.Dispose() }
}











