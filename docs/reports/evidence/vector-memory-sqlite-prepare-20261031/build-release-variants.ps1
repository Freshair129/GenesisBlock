[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

# [ASSUMPTIONS] This is a deferred preparation script. It builds only after the
# exact qualification receipt and all four receipt-bound gate files pass.
$R = 'C:\Users\suanr\genesis-sqlite-prepare-20261031'
$D = 'D:\GenesisBlock_Dev\GenesisBlock'
$Guard = Join-Path $R 'guard.ps1'
$ReceiptPath = Join-Path $R 'qualification-receipt.json'
$VerifiedSourceManifestPath = Join-Path $R 'verify-source-manifest.json'
$LockPath = Join-Path $R 'release-variants.lock'
$CargoExe = 'C:\Users\suanr\.cargo\bin\cargo.exe'
$BeforeLib = Join-Path $R 'before-traced-lib.rs'
$AfterLib = Join-Path $R 'after-traced-lib.rs'
$ExternalBench = Join-Path $R 'reopen100k-vector_memory.rs'
$GatePaths = [ordered]@{
    'final-runtime.status' = (Join-Path $R 'final-runtime.status')
    'final-runtime.json' = (Join-Path $R 'final-runtime.json')
    'final-test-artifacts.json' = (Join-Path $R 'final-test-artifacts.json')
    'post-checks.status' = (Join-Path $R 'post-checks.status')
}
$ExpectedBefore = 'CE0F4A77A33A8D1EA2DFB1A7F8F3A1EA4DF18042D5EB08E59DF969A0B7E71CCF'
$ExpectedAfter = '74975827C95C17A783A1A26F3BA2A5439C6B198A8EA1608AF504F23577715859'
$ExpectedQualifiedSources = [ordered]@{
    'src/lib.rs' = '5B8BB9BED5DDE5FA6E7D4D255DA3ACCE6C1BFE892092456133ADB65A78998FDD'
    'src/projection_rebuild_tests.rs' = 'E69DC7DD5CBC5043AA5ED9C717E449E50246A8FBD8AD56AC95ED135718C99BE2'
    'Cargo.toml' = '00550FBAC8EDB5C1316B5EB21983B078039604ED3C1460689515BA5539F30E25'
    'Cargo.lock' = 'EDA5BFA67C1077C96F2B84E44F740273E695C87A1A51B352CF18F2A031A4F8D2'
    'benches/vector_memory.rs' = '46694637E7E028F8A12C5C5F5EB83C8BAC3CA33E7E276B1236E097F970F53C3F'
}
$RequiredRootFiles = @('Cargo.toml','Cargo.lock','build.rs')
$CopyDirectories = @('src','benches','include')
$OutputPaths = @(
    (Join-Path $R 'diagnostic-before.exe'),
    (Join-Path $R 'diagnostic-after.exe'),
    (Join-Path $R 'candidate-untraced-release.exe')
)
$script:ReceiptSha = $null
$script:RunId = $null
$script:RunDir = $null
$script:SourceClone = $null
$script:TargetDir = $null
$script:TargetExe = $null
$script:FrozenSource = @()
$script:FrozenProductionLib = $null
$script:FrozenProductionBench = $null
$script:FrozenBenchPath = $null
$script:FrozenBenchSha = $null
$script:QualifiedManifest = @()
$script:QualifiedManifestSha = $null
$script:QualifiedCloneManifestSha = $null
$script:QualifiedCloneFiles = @()
$script:GuardRuns = [System.Collections.Generic.List[object]]::new()
$script:BuildRecords = [System.Collections.Generic.List[object]]::new()
$script:lockStream = $null

function Get-Sha([string]$Path) {
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) { throw "File missing for SHA256: $Path" }
    return (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash.ToUpperInvariant()
}
function Get-BytesSha([byte[]]$Bytes) {
    $sha = [Security.Cryptography.SHA256]::Create()
    try { return ([BitConverter]::ToString($sha.ComputeHash($Bytes))).Replace('-','').ToUpperInvariant() }
    finally { $sha.Dispose() }
}
function Get-ManifestEntries($Object,[string]$Label) {
    if ($null -eq $Object) { throw "$Label is missing" }
    $entries = [System.Collections.Generic.List[object]]::new()
    $seen = [System.Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
    foreach ($property in $Object.PSObject.Properties) {
        $path = [string]$property.Name
        $hash = [string]$property.Value
        if ($path -notmatch '^[\x20-\x7E]+$' -or $path -match '\\' -or $path.StartsWith('/') -or $path -match '(^|/)\.\.?(/|$)' -or $path -match '(^/)|//|/$') { throw "$Label contains a noncanonical or unsafe path: $path" }
        if (-not $seen.Add($path)) { throw "$Label contains a duplicate case-insensitive path: $path" }
        if ($hash -notmatch '^[A-Fa-f0-9]{64}$') { throw "$Label has invalid SHA256 for $path" }
        $entries.Add([ordered]@{ relative=$path; sha256=$hash.ToUpperInvariant() })
    }
    $array = @($entries.ToArray() | Sort-Object -Property { [string]$_.relative } -CaseSensitive)
    return ,$array
}
function Get-CanonicalManifestSha([object[]]$Entries) {
    # Canonical file-set SHA256: ASCII forward-slash relative paths, ordinal
    # case-sensitive path order, lowercase SHA256 values, UTF-8 without BOM,
    # and one LF after each path|sha256 row (including the final row).
    $map = @{}
    foreach ($entry in $Entries) { $map[[string]$entry.relative] = ([string]$entry.sha256).ToLowerInvariant() }
    [string[]]$paths = @($map.Keys)
    [Array]::Sort($paths,[StringComparer]::Ordinal)
    $lines = [System.Collections.Generic.List[string]]::new()
    foreach ($path in $paths) { $lines.Add($path + '|' + $map[$path]) }
    $payload = [Text.UTF8Encoding]::new($false).GetBytes(([string]::Join("`n",$lines)) + "`n")
    return Get-BytesSha $payload
}
function Read-Json([string]$Path) { Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json }
function Write-Json([string]$Path, $Value) {
    $json = ConvertTo-Json -InputObject $Value -Depth 18
    [IO.File]::WriteAllText($Path, $json + [Environment]::NewLine, [Text.UTF8Encoding]::new($false))
}
function Get-ExactProperty($Object, [string]$Name) {
    if ($null -eq $Object) { throw "Missing object while reading exact property '$Name'" }
    $matches = @($Object.PSObject.Properties | Where-Object { [string]::Equals($_.Name,$Name,[StringComparison]::Ordinal) })
    if ($matches.Count -ne 1) { throw "Expected exactly one case-sensitive property '$Name'" }
    return $matches[0]
}
function Assert-Hash([string]$Path,[string]$Expected,[string]$Label) {
    $actual = Get-Sha $Path
    if (-not [string]::Equals($actual,$Expected,[StringComparison]::OrdinalIgnoreCase)) { throw "$Label SHA256 mismatch: expected $Expected got $actual" }
    return $actual
}
function Get-GateSnapshot {
    foreach ($key in $GatePaths.Keys) {
        $path = $GatePaths[$key]
        if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "Required qualification gate is missing: $path" }
    }
    $runtimeStatus = [IO.File]::ReadAllText($GatePaths['final-runtime.status']).Trim()
    $runtimeComplete = [string]::Equals($runtimeStatus,'complete',[StringComparison]::OrdinalIgnoreCase)
    if (-not $runtimeComplete) {
        try { $statusJson = $runtimeStatus | ConvertFrom-Json } catch { throw 'final-runtime.status must say complete or contain complete status JSON' }
        foreach ($name in @('status','state')) {
            $p = @($statusJson.PSObject.Properties | Where-Object { $_.Name -ceq $name })
            if ($p.Count -eq 1 -and [string]::Equals([string]$p[0].Value,'complete',[StringComparison]::Ordinal)) { $runtimeComplete = $true }
        }
        foreach ($name in @('complete','completed')) {
            $p = @($statusJson.PSObject.Properties | Where-Object { $_.Name -ceq $name })
            if ($p.Count -eq 1 -and $p[0].Value -is [bool] -and $p[0].Value) { $runtimeComplete = $true }
        }
    }
    if (-not $runtimeComplete) { throw 'final-runtime.status is not complete' }
    $post = Read-Json $GatePaths['post-checks.status']
    $passed = Get-ExactProperty $post 'passed'
    if ($passed.Value -isnot [bool] -or $passed.Value -ne $true) { throw 'post-checks.status JSON must contain passed:true' }
    $hashes = [ordered]@{}
    foreach ($key in $GatePaths.Keys) { $hashes[$key] = Get-Sha $GatePaths[$key] }
    return [ordered]@{ hashes=$hashes; runtime_status=$runtimeStatus; post_checks_passed=$true }
}
function Assert-Qualification {
    $gates = Get-GateSnapshot
    if (-not (Test-Path -LiteralPath $ReceiptPath -PathType Leaf)) { throw "Missing qualification receipt: $ReceiptPath" }
    $receiptHash = Get-Sha $ReceiptPath
    if ($script:ReceiptSha -and $receiptHash -ne $script:ReceiptSha) { throw 'qualification-receipt.json changed during the variant run' }
    $receipt = Read-Json $ReceiptPath
    $runId = Get-ExactProperty $receipt 'run_id'
    if (-not [string]::Equals([string]$runId.Value,'genesis-sqlite-prepare-20261031',[StringComparison]::Ordinal)) { throw 'Qualification receipt run_id mismatch' }
    $passed = Get-ExactProperty $receipt 'passed'
    if ($passed.Value -isnot [bool] -or $passed.Value -ne $true) { throw 'Qualification receipt passed is not true' }
    $targets = Get-ExactProperty $receipt 'completed_targets'
    if ([int]$targets.Value -ne 159) { throw "Qualification receipt completed_targets is not 159: $($targets.Value)" }
    $source = (Get-ExactProperty $receipt 'source').Value
    foreach ($relative in $ExpectedQualifiedSources.Keys) {
        $sourceHash = [string](Get-ExactProperty $source $relative).Value
        if (-not [string]::Equals($sourceHash,$ExpectedQualifiedSources[$relative],[StringComparison]::OrdinalIgnoreCase)) { throw "Qualification source SHA mismatch: $relative" }
        Assert-Hash (Join-Path $D ($relative -replace '/', '\')) $ExpectedQualifiedSources[$relative] "Qualified D source $relative" | Out-Null
    }
    $fullManifestEntries = Get-ManifestEntries ((Get-ExactProperty $receipt 'full_source_manifest').Value) 'Qualification full_source_manifest'
    $declaredFullManifestSha = [string](Get-ExactProperty $receipt 'full_source_manifest_sha256').Value
    if ($declaredFullManifestSha -notmatch '^[A-Fa-f0-9]{64}$') { throw 'Qualification full_source_manifest_sha256 is invalid' }
    $actualFullManifestSha = Get-CanonicalManifestSha $fullManifestEntries
    if (-not [string]::Equals($declaredFullManifestSha,$actualFullManifestSha,[StringComparison]::OrdinalIgnoreCase)) { throw 'Qualification full source manifest canonical hash mismatch' }
    $baselineManifestEntries = Get-ManifestEntries (Read-Json $VerifiedSourceManifestPath) 'verify-source-manifest.json'
    $fullByPath = @{}
    foreach ($entry in $fullManifestEntries) { $fullByPath[[string]$entry.relative] = [string]$entry.sha256 }
    foreach ($entry in $baselineManifestEntries) {
        $path = [string]$entry.relative
        if (-not $fullByPath.ContainsKey($path) -or $fullByPath[$path] -ne [string]$entry.sha256) { throw "Full source manifest omitted/changed verified provenance path: $path" }
    }
    foreach ($entry in $fullManifestEntries) {
        $dPath = Join-Path $D ($entry.relative -replace '/', '\')
        Assert-Hash $dPath $entry.sha256 "Full-manifest D source $($entry.relative)" | Out-Null
    }
    $actualCloneFiles = Get-SourceFileList
    $actualCloneByPath = @{}
    foreach ($entry in $actualCloneFiles) { $actualCloneByPath[[string]$entry.relative] = [string]$entry.sha256 }
    $manifestCloneEntries = @($fullManifestEntries | Where-Object { (Test-CloneScopePath ([string]$_.relative)) -and -not (Test-DatabaseArtifact ([string]$_.relative)) })
    $manifestCloneByPath = @{}
    foreach ($entry in $manifestCloneEntries) { $manifestCloneByPath[[string]$entry.relative] = [string]$entry.sha256 }
    if ($actualCloneByPath.Count -ne $manifestCloneByPath.Count) { throw "Qualified clone source scope differs from D: actual=$($actualCloneByPath.Count) receipt=$($manifestCloneByPath.Count)" }
    foreach ($path in $actualCloneByPath.Keys) {
        if (-not $manifestCloneByPath.ContainsKey($path)) { throw "D clone input is absent from qualified full manifest: $path" }
        if ($actualCloneByPath[$path] -ne $manifestCloneByPath[$path]) { throw "Qualified clone input SHA mismatch: $path" }
    }
    $actualCloneSha = Get-CanonicalManifestSha $actualCloneFiles
    $manifestCloneSha = Get-CanonicalManifestSha $manifestCloneEntries
    if ($actualCloneSha -ne $manifestCloneSha) { throw 'Qualified clone file-set canonical hash mismatch' }
    if ($script:QualifiedManifestSha -and $script:QualifiedManifestSha -ne $actualFullManifestSha) { throw 'Qualification full source manifest changed during the variant run' }
    if ($script:QualifiedCloneManifestSha -and $script:QualifiedCloneManifestSha -ne $actualCloneSha) { throw 'Qualified clone file-set changed during the variant run' }
    $script:QualifiedManifest = $fullManifestEntries
    $script:QualifiedManifestSha = $actualFullManifestSha
    $script:QualifiedCloneManifestSha = $actualCloneSha
    $script:QualifiedCloneFiles = $actualCloneFiles
    $receiptGateHashes = (Get-ExactProperty $receipt 'gate_hashes').Value
    foreach ($key in $GatePaths.Keys) {
        $hash = [string](Get-ExactProperty $receiptGateHashes $key).Value
        if (-not [string]::Equals($hash,$gates.hashes[$key],[StringComparison]::OrdinalIgnoreCase)) { throw "Qualification gate hash mismatch: $key" }
    }
    if (-not $script:ReceiptSha) { $script:ReceiptSha = $receiptHash }
    return [ordered]@{ run_id=[string]$runId.Value; receipt_sha256=$receiptHash; completed_targets=[int]$targets.Value; passed=$true; gate_hashes=$gates.hashes; full_source_manifest_sha256=$actualFullManifestSha; clone_source_manifest_sha256=$actualCloneSha; full_source_manifest_paths=$fullManifestEntries.Count; clone_source_files=$actualCloneFiles.Count }
}

function Get-ActiveAdmissionProcesses {
    $all = @(Get-CimInstance Win32_Process -Property ProcessId,Name,ExecutablePath,CommandLine,CreationDate)
    return @($all | Where-Object {
        $identity = "$($_.Name) $($_.ExecutablePath) $($_.CommandLine)"
        $_.Name -match '^(cargo|rustc|clippy-driver|vbench-genesis|sqlite-prepare-reproducer)\.exe$' -or
        $identity -match '(?i)run-full-core(?:-waiting-attempt1)?\.py|cargo(?:\.exe)?\s+(?:test|bench|run|build|clippy|check)|sqlite-prepare-reproducer|vbench-genesis|final-test-bin|[\\/]target[\\/](?:debug|release)[\\/]deps[\\/][^\s"]+\.exe'
    })
}
function Assert-NoActiveProcesses([string]$Where) {
    $active = @(Get-ActiveAdmissionProcesses)
    if ($active.Count) {
        $details = ($active | ForEach-Object { "$($_.ProcessId):$($_.Name):$($_.ExecutablePath)" }) -join ', '
        throw "Active compiler/test/benchmark process blocks admission for $Where`: $details"
    }
    return [ordered]@{ checked_at_utc=[DateTime]::UtcNow.ToString('o'); blocked_count=0; excludes=@('vctip.exe','mspdbsrv.exe') }
}

function Test-DatabaseArtifact([string]$Relative) {
    $leaf = [IO.Path]::GetFileName($Relative).ToLowerInvariant()
    $ext = [IO.Path]::GetExtension($leaf)
    $parts = $Relative.Split('/')
    if (@($parts | Where-Object { $_ -match '^(?i:temp_db|temp_db_bulk|database|databases)$' }).Count) { return $true }
    if ($ext -in @('.db','.sqlite','.sqlite3','.wal','.shm','.gwal','.gseg','.bin')) { return $true }
    if ($leaf -match '(?i)(\.sqlite3?|\.db|\.wal|\.shm|\.gwal|\.gseg)(?:-|$)' -or $leaf -match '(?i)(?:^|[-_.])(wal|shm)$') { return $true }
    return $false
}
function Test-CloneScopePath([string]$Relative) {
    $parts = $Relative.Split('/')
    if (@($parts | Where-Object { $_ -in @('.git','target','node_modules') }).Count) { return $false }
    return ($Relative -in @('Cargo.toml','Cargo.lock','build.rs') -or $Relative.StartsWith('src/',[StringComparison]::Ordinal) -or $Relative.StartsWith('benches/',[StringComparison]::Ordinal) -or $Relative.StartsWith('include/',[StringComparison]::Ordinal))
}
function Get-SourceFileList {
    $files = [System.Collections.Generic.List[object]]::new()
    $seen = [System.Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
    foreach ($rootFile in $RequiredRootFiles) {
        $path = Join-Path $D $rootFile
        if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "Required D source file missing: $path" }
        if (-not $seen.Add($rootFile)) { throw "Duplicate source path: $rootFile" }
        $item = Get-Item -LiteralPath $path -Force
        if (($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { throw "Refusing reparse point source: $path" }
        $files.Add([ordered]@{ relative=$rootFile; path=$path; sha256=(Get-Sha $path); bytes=$item.Length })
    }
    foreach ($relativeRoot in $CopyDirectories) {
        $base = Join-Path $D $relativeRoot
        if (-not (Test-Path -LiteralPath $base -PathType Container)) {
            if ($relativeRoot -eq 'include') { continue }
            throw "Required D source directory missing: $base"
        }
        $baseItem = Get-Item -LiteralPath $base -Force
        if (($baseItem.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { throw "Refusing reparse point source directory: $base" }
        foreach ($item in Get-ChildItem -LiteralPath $base -Force -Recurse) {
            $relative = $item.FullName.Substring($D.Length).TrimStart([char[]]@('\','/')).Replace('\','/')
            $parts = $relative.Split('/')
            if (@($parts | Where-Object { $_ -in @('.git','target','node_modules') }).Count) { continue }
            if (($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { throw "Refusing reparse point in source tree: $relative" }
            if ($item.PSIsContainer) { continue }
            if (-not (Test-CloneScopePath $relative) -or (Test-DatabaseArtifact $relative)) { continue }
            if (-not $seen.Add($relative)) { throw "Case-insensitive duplicate source path: $relative" }
            $files.Add([ordered]@{ relative=$relative; path=$item.FullName; sha256=(Get-Sha $item.FullName); bytes=$item.Length })
        }
    }
    $array = @($files.ToArray() | Sort-Object -Property { [string]$_.relative } -CaseSensitive)
    return ,$array
}
function Get-SnapshotIndex([object[]]$Snapshot) {
    $index = @{}
    foreach ($entry in $Snapshot) { $index[[string]$entry.relative] = $entry }
    return $index
}
function Assert-DSourceSnapshot([object[]]$Snapshot,[string]$Where) {
    $current = Get-SourceFileList
    if ($current.Count -ne $Snapshot.Count) { throw "D source file set changed at ${Where}: $($Snapshot.Count) to $($current.Count)" }
    for ($i=0; $i -lt $Snapshot.Count; $i++) {
        if ($current[$i].relative -cne $Snapshot[$i].relative -or $current[$i].sha256 -cne $Snapshot[$i].sha256 -or $current[$i].bytes -ne $Snapshot[$i].bytes) {
            throw "Frozen D source changed at ${Where}: $($Snapshot[$i].relative)"
        }
    }
}
function Copy-FrozenSourceToClone([object[]]$Snapshot,[string]$Destination) {
    if (Test-Path -LiteralPath $Destination) { throw "Refusing to reuse source clone path: $Destination" }
    New-Item -ItemType Directory -Path $Destination | Out-Null
    foreach ($entry in $Snapshot) {
        $dest = Join-Path $Destination ([string]$entry.relative -replace '/', '\')
        $parent = Split-Path -Parent $dest
        New-Item -ItemType Directory -Path $parent -Force | Out-Null
        if ((Get-Sha ([string]$entry.path)) -ne [string]$entry.sha256) { throw "D source changed while cloning: $($entry.relative)" }
        Copy-Item -LiteralPath ([string]$entry.path) -Destination $dest
        if ((Get-Sha $dest) -ne [string]$entry.sha256) { throw "Private source clone SHA mismatch: $($entry.relative)" }
    }
}
function Assert-CloneSnapshot([object[]]$Snapshot,[string]$ExpectedLib,[string]$ExpectedBench,[string]$Where) {
    $index = Get-SnapshotIndex $Snapshot
    foreach ($entry in $Snapshot) {
        $expected = [string]$entry.sha256
        if ($entry.relative -ceq 'src/lib.rs') { $expected = $ExpectedLib }
        if ($entry.relative -ceq 'benches/vector_memory.rs') { $expected = $ExpectedBench }
        $path = Join-Path $script:SourceClone ([string]$entry.relative -replace '/', '\')
        $actual = Get-Sha $path
        if ($actual -ne $expected) { throw "Private clone drift at ${Where}: $($entry.relative) expected $expected got $actual" }
    }
    $actualFiles = @(Get-ChildItem -LiteralPath $script:SourceClone -File -Force -Recurse | ForEach-Object { $_.FullName.Substring($script:SourceClone.Length).TrimStart([char[]]@('\','/')).Replace('\','/') } | Sort-Object -CaseSensitive)
    $expectedFiles = @($Snapshot | ForEach-Object { [string]$_.relative } | Sort-Object -CaseSensitive)
    if ($actualFiles.Count -ne $expectedFiles.Count) { throw "Private clone file set changed at $Where" }
    for ($i=0; $i -lt $expectedFiles.Count; $i++) { if ($actualFiles[$i] -cne $expectedFiles[$i]) { throw "Private clone has unexpected/missing file at ${Where}: $($actualFiles[$i])" } }
}
function Convert-BirthToUtc($Value) {
    if ($null -eq $Value) { throw 'Process identity birth time is missing' }
    if ($Value -is [DateTime]) { return $Value.ToUniversalTime() }
    $text = [string]$Value
    if ($text -match '^/?Date\((-?\d+)(?:[+-]\d+)?\)/?$') { return [DateTimeOffset]::FromUnixTimeMilliseconds([int64]$Matches[1]).UtcDateTime }
    return [DateTime]::Parse($text,[Globalization.CultureInfo]::InvariantCulture,[Globalization.DateTimeStyles]::AssumeUniversal).ToUniversalTime()
}
function Test-SameBirth($A,$B) {
    return ([Math]::Abs(((Convert-BirthToUtc $A) - (Convert-BirthToUtc $B)).TotalMilliseconds) -lt 2)
}
function Assert-GuardRunsStopped([object[]]$Entries,[string]$Where) {
    $live = @(Get-CimInstance Win32_Process -Property ProcessId,ParentProcessId,CreationDate,Name,ExecutablePath,CommandLine)
    $helperProofs = [System.Collections.Generic.List[object]]::new()
    $ownedCount = 0
    foreach ($entry in $Entries) {
        $status = $entry.status
        $clean = (Get-ExactProperty $status 'cleanup_complete').Value
        if ($clean -isnot [bool] -or -not $clean) { throw "Guard cleanup_complete is not true at $Where for $($entry.stage)" }
        $remaining = (Get-ExactProperty $status 'remaining').Value
        if ($null -eq $remaining -or $remaining -isnot [array] -or @($remaining).Count -ne 0) { throw "Guard remaining must be an explicit empty array at $Where for $($entry.stage)" }
        $exitCode = (Get-ExactProperty $status 'exit').Value
        if ($null -eq $exitCode -or [int]$exitCode -ne 0) { throw "Cargo did not exit successfully at $Where for $($entry.stage)" }
        $helperFinished = (Get-ExactProperty $status 'helper_finished_parent_condition').Value
        if ($helperFinished -isnot [bool] -or -not $helperFinished) { throw "Guard did not attest successful Cargo completion for toolchain helpers at $Where" }
        $identities = @((Get-ExactProperty $status 'identities').Value)
        if (-not $identities.Count) { throw "Guard identity receipt is empty at $Where for $($entry.stage)" }
        foreach ($identity in $identities) {
            $ownedCount++
            $pidValue = [int](Get-ExactProperty $identity 'ProcessId').Value
            $birth = (Get-ExactProperty $identity 'CreationDate').Value
            $name = [string](Get-ExactProperty $identity 'Name').Value
            $path = [string](Get-ExactProperty $identity 'ExecutablePath').Value
            if ($pidValue -le 0 -or [string]::IsNullOrWhiteSpace($name) -or [string]::IsNullOrWhiteSpace($path)) { throw "Guard identity is incomplete at $Where" }
            $samePid = @($live | Where-Object { [int]$_.ProcessId -eq $pidValue })
            foreach ($candidate in $samePid) {
                if (Test-SameBirth $birth $candidate.CreationDate) {
                    $sameName = [string]::Equals([string]$candidate.Name,$name,[StringComparison]::OrdinalIgnoreCase)
                    $samePath = [string]::Equals([string]$candidate.ExecutablePath,$path,[StringComparison]::OrdinalIgnoreCase)
                    if (-not $sameName -or (-not [string]::IsNullOrWhiteSpace([string]$candidate.ExecutablePath) -and -not $samePath)) { throw "Same-birth PID identity conflict at $Where for PID $pidValue" }
                    throw "Guard-owned compiler process is still live at ${Where}: PID=$pidValue Name=$name Path=$path"
                }
            }
        }
        $helpers = @((Get-ExactProperty $status 'excluded_build_helpers').Value)
        foreach ($helper in $helpers) {
            $helperPid = [int](Get-ExactProperty $helper 'ProcessId').Value
            $parentPid = [int](Get-ExactProperty $helper 'ParentProcessId').Value
            $helperBirth = (Get-ExactProperty $helper 'CreationDate').Value
            $helperName = [string](Get-ExactProperty $helper 'Name').Value
            $helperPath = [string](Get-ExactProperty $helper 'ExecutablePath').Value
            $parentBirth = (Get-ExactProperty $helper 'parent_creation').Value
            $parentName = [string](Get-ExactProperty $helper 'parent_name').Value
            if ($helperName -notmatch '^(vctip|mspdbsrv)\.exe$' -or $helperPid -le 0 -or $parentPid -le 0 -or [string]::IsNullOrWhiteSpace($helperPath)) { throw "Excluded toolchain-helper identity is incomplete at $Where" }
            $parentIdentity = @($identities | Where-Object { [int](Get-ExactProperty $_ 'ProcessId').Value -eq $parentPid })
            if ($parentIdentity.Count -ne 1) { throw "Excluded helper parent PID $parentPid is not bound to one tracked compiler identity at $Where" }
            $parentIdentity = $parentIdentity[0]
            if (-not (Test-SameBirth $parentBirth ((Get-ExactProperty $parentIdentity 'CreationDate').Value)) -or -not [string]::Equals($parentName,[string](Get-ExactProperty $parentIdentity 'Name').Value,[StringComparison]::OrdinalIgnoreCase)) { throw "Excluded helper parent birth/name does not match tracked compiler at $Where" }
            $helperNow = @($live | Where-Object { [int]$_.ProcessId -eq $helperPid -and (Test-SameBirth $helperBirth $_.CreationDate) })
            foreach ($candidate in $helperNow) {
                if (-not [string]::Equals([string]$candidate.Name,$helperName,[StringComparison]::OrdinalIgnoreCase) -or (-not [string]::IsNullOrWhiteSpace([string]$candidate.ExecutablePath) -and -not [string]::Equals([string]$candidate.ExecutablePath,$helperPath,[StringComparison]::OrdinalIgnoreCase))) { throw "Excluded helper identity conflict at $Where for PID $helperPid" }
            }
            $helperProofs.Add([ordered]@{
                helper=[ordered]@{ pid=$helperPid; birth=(Convert-BirthToUtc $helperBirth).ToString('o'); name=$helperName; path=$helperPath; still_live_as_excluded_service=($helperNow.Count -gt 0) }
                parent=[ordered]@{ pid=$parentPid; birth=(Convert-BirthToUtc $parentBirth).ToString('o'); name=$parentName; tracked_compiler_client_stopped=$true }
                safe_finished_parent=$true
                justification='Guard reports Cargo exit 0 and stopped tracked compiler clients; this vctip/mspdbsrv helper is excluded noncompiling telemetry/PDB service. Its exact parent PID+birth+name matches a tracked identity proven absent in the fresh stop snapshot. Helper is not killed and is not claimed terminated.'
            })
        }
    }
    return [ordered]@{ checked_at_utc=[DateTime]::UtcNow.ToString('o'); guard_runs_checked=$Entries.Count; tracked_compiler_clients_checked=$ownedCount; active_tracked_clients=0; identity='PID+CreationDate+ExecutablePath+Name'; excluded_helpers=@($helperProofs) }
}

function Set-CloneOverlay([string]$Relative,[byte[]]$Bytes,[string]$ExpectedSha,[string]$Name) {
    if ((Get-BytesSha $Bytes) -ne $ExpectedSha) { throw "$Name frozen overlay bytes have unexpected SHA256" }
    $path = Join-Path $script:SourceClone ($Relative -replace '/', '\')
    [IO.File]::WriteAllBytes($path,$Bytes)
    if ((Get-Sha $path) -ne $ExpectedSha) { throw "$Name private clone overlay verification failed" }
}
function Invoke-ReleaseStage([string]$Name,[byte[]]$LibBytes,[string]$ExpectedLib,[byte[]]$BenchBytes,[string]$ExpectedBench,[string]$OutputExe) {
    $qualification = Assert-Qualification
    $admission = Assert-NoActiveProcesses $Name
    if ($script:GuardRuns.Count) { Assert-GuardRunsStopped -Entries @($script:GuardRuns) -Where "$Name pre-overlay" | Out-Null }
    Assert-DSourceSnapshot -Snapshot $script:FrozenSource -Where "$Name pre-overlay"
    Assert-CloneSnapshot -Snapshot $script:FrozenSource -ExpectedLib $script:ExpectedCloneLib -ExpectedBench $script:ExpectedCloneBench -Where "$Name before overlay"
    if ((Get-Sha $ExternalBench) -ne $script:FrozenBenchSha -or (Get-Sha $script:FrozenBenchPath) -ne $script:FrozenBenchSha) { throw 'External reopen100k benchmark changed after its run-time freeze' }
    if (Test-Path -LiteralPath $OutputExe) { throw "Refusing to overwrite existing output: $OutputExe" }

    Set-CloneOverlay 'src/lib.rs' $LibBytes $ExpectedLib "$Name lib.rs"
    Set-CloneOverlay 'benches/vector_memory.rs' $BenchBytes $ExpectedBench "$Name vector_memory.rs"
    $script:ExpectedCloneLib = $ExpectedLib
    $script:ExpectedCloneBench = $ExpectedBench
    Assert-CloneSnapshot -Snapshot $script:FrozenSource -ExpectedLib $ExpectedLib -ExpectedBench $ExpectedBench -Where "$Name after overlay"
    Assert-DSourceSnapshot -Snapshot $script:FrozenSource -Where "$Name after overlay"

    $stageDir = Join-Path $script:RunDir $Name
    New-Item -ItemType Directory -Path $stageDir | Out-Null
    $outputDir = Join-Path $stageDir 'guard-output'
    $configPath = Join-Path $stageDir 'guard-config.json'
    $envConfig = [ordered]@{
        CARGO_TARGET_DIR=$script:TargetDir
        RELEASE_LTO='false'; CODEGEN_UNITS='16'; BUILD_JOBS='1'; CARGO_NET_OFFLINE='true'
        CARGO_PROFILE_RELEASE_LTO='false'; CARGO_PROFILE_RELEASE_CODEGEN_UNITS='16'; CARGO_BUILD_JOBS='1'
    }
    $args = @('build','--manifest-path',(Join-Path $script:SourceClone 'Cargo.toml'),'--locked','--offline','--release','--no-default-features','--features','bins','--bin','vbench-genesis','-j1')
    $config = [ordered]@{ kind='build_preparation'; exe=$CargoExe; args=$args; cwd=$script:SourceClone; output=$outputDir; cap_seconds=1200; env=$envConfig }
    Write-Json $configPath $config
    $context = [ordered]@{
        stage=$Name; qualification=$qualification; source_clone=$script:SourceClone; manifest_path=(Join-Path $script:SourceClone 'Cargo.toml'); target_dir=$script:TargetDir
        source_manifest_path=(Join-Path $script:RunDir 'source-snapshot.json'); source_manifest_sha256=(Get-Sha (Join-Path $script:RunDir 'source-snapshot.json'))
        source=[ordered]@{ lib_sha256=$ExpectedLib; vector_memory_sha256=$ExpectedBench; Cargo_toml_sha256=(Get-Sha (Join-Path $script:SourceClone 'Cargo.toml')); Cargo_lock_sha256=(Get-Sha (Join-Path $script:SourceClone 'Cargo.lock')); projection_rebuild_tests_sha256=(Get-Sha (Join-Path $script:SourceClone 'src/projection_rebuild_tests.rs')); build_rs_sha256=(Get-Sha (Join-Path $script:SourceClone 'build.rs')); external_bench_sha256=$script:FrozenBenchSha }
        config=$config; config_path=$configPath; output_dir=$outputDir; admission=$admission
        resource_contract=[ordered]@{ max_seconds=1200; rss_bytes=2147483648; min_free_bytes=6442450944; query_deadline_ms=5000; compile_only_cpu_admission_exemption=$true }
    }
    Write-Json (Join-Path $stageDir 'build-context.json') $context
    Assert-Qualification | Out-Null
    Assert-DSourceSnapshot -Snapshot $script:FrozenSource -Where "$Name immediately before guard"
    Assert-CloneSnapshot -Snapshot $script:FrozenSource -ExpectedLib $ExpectedLib -ExpectedBench $ExpectedBench -Where "$Name immediately before guard"
    Assert-NoActiveProcesses "$Name immediately before guard" | Out-Null

    try { & $Guard -Config $configPath | Out-Null }
    catch {
        Write-Json (Join-Path $stageDir 'guard-invocation-error.json') ([ordered]@{ error=$_.Exception.ToString(); owned_stop_proof='missing'; next_stage='forbidden until root obtains fresh stop proof' })
        throw
    }
    $statusPath = Join-Path $outputDir 'status.json'
    if (-not (Test-Path -LiteralPath $statusPath -PathType Leaf)) { throw "$Name guard returned without status.json; no next source edit is permitted" }
    $status = Read-Json $statusPath
    $entry = [ordered]@{ stage=$Name; status_path=$statusPath; status=$status }
    $script:GuardRuns.Add($entry)
    $stopProof = Assert-GuardRunsStopped -Entries @($entry) -Where "$Name post-build stop proof"

    $exit = (Get-ExactProperty $status 'exit').Value
    $reason = (Get-ExactProperty $status 'reason').Value
    $seconds = [double](Get-ExactProperty $status 'seconds').Value
    $peakRss = [int64](Get-ExactProperty $status 'peak_rss').Value
    $free = [int64](Get-ExactProperty $status 'admission_free').Value
    $passed = ([int]$exit -eq 0 -and $null -eq $reason -and $seconds -le 1200 -and $peakRss -le 2147483648 -and $free -ge 6442450944)
    $record = [ordered]@{
        stage=$Name; status=if($passed){'passed'}else{'failed'}; actual_exit=$exit; reason=$reason; seconds=$seconds; peak_rss=$peakRss; admission_free=$free
        qualification=(Assert-Qualification); source=$context.source; source_clone=$script:SourceClone; target_dir=$script:TargetDir
        guard_status_path=$statusPath; fresh_owned_stop_proof=$stopProof; output_exe=$OutputExe; output_exe_sha256=$null; target_exe_sha256=$null
    }
    if ($passed) {
        if (-not (Test-Path -LiteralPath $script:TargetExe -PathType Leaf)) { throw "$Name completed without private target executable: $($script:TargetExe)" }
        $record.target_exe_sha256 = Get-Sha $script:TargetExe
        Copy-Item -LiteralPath $script:TargetExe -Destination $OutputExe
        $record.output_exe_sha256 = Get-Sha $OutputExe
    }
    Write-Json (Join-Path $stageDir 'build-receipt.json') $record
    $script:BuildRecords.Add($record)
    if (-not $passed) { throw "$Name guarded build failed; see $statusPath" }
    return $record
}

$lock = $null
$failure = $null
try {
    $lock = [IO.File]::Open($LockPath,[IO.FileMode]::OpenOrCreate,[IO.FileAccess]::ReadWrite,[IO.FileShare]::None)
    $lockBytes = [Text.UTF8Encoding]::new($false).GetBytes("$([DateTime]::UtcNow.ToString('o')) $PID`n")
    $lock.SetLength(0); $lock.Write($lockBytes,0,$lockBytes.Length); $lock.Flush($true)

    foreach ($path in @($VerifiedSourceManifestPath,$Guard,$BeforeLib,$AfterLib,$ExternalBench,$CargoExe)) { if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "Required build input missing: $path" } }
    $preQualification = Assert-Qualification
    $preAdmission = Assert-NoActiveProcesses 'locked preflight'
    foreach ($path in $OutputPaths) { if (Test-Path -LiteralPath $path) { throw "Refusing to overwrite existing release variant: $path" } }
    Assert-Hash $BeforeLib $ExpectedBefore 'before diagnostic source' | Out-Null
    Assert-Hash $AfterLib $ExpectedAfter 'after diagnostic source' | Out-Null

    $script:RunId = [DateTime]::UtcNow.ToString('yyyyMMddTHHmmssZ') + '-' + [Guid]::NewGuid().ToString('N').Substring(0,8)
    $script:RunDir = Join-Path $R ("release-variants-" + $script:RunId)
    $script:SourceClone = Join-Path (Join-Path $R 'release-src') $script:RunId
    $script:TargetDir = Join-Path (Join-Path $R 'release-target') $script:RunId
    $script:TargetExe = Join-Path $script:TargetDir 'release\vbench-genesis.exe'
    foreach ($path in @($script:RunDir,$script:SourceClone,$script:TargetDir)) { if (Test-Path -LiteralPath $path) { throw "Refusing to reuse per-run path: $path" } }
    New-Item -ItemType Directory -Path $script:RunDir | Out-Null
    New-Item -ItemType Directory -Path (Split-Path -Parent $script:SourceClone) -Force | Out-Null
    New-Item -ItemType Directory -Path (Split-Path -Parent $script:TargetDir) -Force | Out-Null

    $script:FrozenSource = @($script:QualifiedCloneFiles)
    if (-not $script:FrozenSource.Count) { throw 'D source snapshot is empty' }
    Assert-DSourceSnapshot -Snapshot $script:FrozenSource -Where 'initial frozen snapshot'
    foreach ($relative in $ExpectedQualifiedSources.Keys) {
        $entry = @($script:FrozenSource | Where-Object { $_.relative -ceq $relative })
        if ($entry.Count -ne 1 -or $entry[0].sha256 -ne $ExpectedQualifiedSources[$relative]) { throw "Source snapshot does not contain qualified file/hash: $relative" }
    }
    $script:FrozenProductionLib = [IO.File]::ReadAllBytes((Join-Path $D 'src/lib.rs'))
    $script:FrozenProductionBench = [IO.File]::ReadAllBytes((Join-Path $D 'benches/vector_memory.rs'))
    $script:FrozenBenchSha = Get-Sha $ExternalBench
    $script:FrozenBenchPath = Join-Path $script:RunDir 'externalreopen100k-vector_memory.frozen.rs'
    [IO.File]::WriteAllBytes($script:FrozenBenchPath,[IO.File]::ReadAllBytes($ExternalBench))
    if ((Get-Sha $script:FrozenBenchPath) -ne $script:FrozenBenchSha) { throw 'External benchmark freeze SHA mismatch' }
    Copy-FrozenSourceToClone -Snapshot $script:FrozenSource -Destination $script:SourceClone
    $script:ExpectedCloneLib = $ExpectedQualifiedSources['src/lib.rs']
    $script:ExpectedCloneBench = $ExpectedQualifiedSources['benches/vector_memory.rs']
    Assert-CloneSnapshot -Snapshot $script:FrozenSource -ExpectedLib $script:ExpectedCloneLib -ExpectedBench $script:ExpectedCloneBench -Where 'initial clone'
    Assert-DSourceSnapshot -Snapshot $script:FrozenSource -Where 'post-clone verification'

    $snapshotManifest = [ordered]@{
        run_id=$script:RunId; source_root=$D; clone=$script:SourceClone; captured_at_utc=[DateTime]::UtcNow.ToString('o')
        exclusion_policy='Clone-source closed set is qualified by full_source_manifest. No .git, target, node_modules, reparse points, .db/.sqlite*/WAL/SHM/GWAL/GSEG/.bin files, or temp_db/database directories; copy eligible src/**, benches/**, include/**, Cargo.toml, Cargo.lock, build.rs.'
        file_count=$script:FrozenSource.Count; files=@($script:FrozenSource | ForEach-Object { [ordered]@{ relative=$_.relative; sha256=$_.sha256; bytes=$_.bytes } })
        qualified_source_hashes=$ExpectedQualifiedSources; qualified_full_source_manifest_sha256=$script:QualifiedManifestSha; clone_source_manifest_sha256=$script:QualifiedCloneManifestSha; frozen_external_bench_sha256=$script:FrozenBenchSha
    }
    $snapshotPath = Join-Path $script:RunDir 'source-snapshot.json'
    Write-Json $snapshotPath $snapshotManifest
    $manifest = [ordered]@{
        status='running'; run_id=$script:RunId; qualification=$preQualification; admission=$preAdmission
        paths=[ordered]@{ D=$D; source_clone=$script:SourceClone; cargo_manifest=(Join-Path $script:SourceClone 'Cargo.toml'); private_target_dir=$script:TargetDir; guard=$Guard; lock=$LockPath; snapshot=$snapshotPath; frozen_bench=$script:FrozenBenchPath }
        hashes=[ordered]@{ before_lib=$ExpectedBefore; after_lib=$ExpectedAfter; production_lib=$ExpectedQualifiedSources['src/lib.rs']; production_bench=$ExpectedQualifiedSources['benches/vector_memory.rs']; frozen_external_bench=$script:FrozenBenchSha; qualified_full_source_manifest_sha256=$script:QualifiedManifestSha; clone_source_manifest_sha256=$script:QualifiedCloneManifestSha }
        caps=[ordered]@{ max_seconds_per_build=1200; rss_bytes=2147483648; min_free_bytes=6442450944; query_deadline_ms=5000; compile_only_cpu_admission_exemption=$true }
        build_args=@('build','--manifest-path',(Join-Path $script:SourceClone 'Cargo.toml'),'--locked','--offline','--release','--no-default-features','--features','bins','--bin','vbench-genesis','-j1')
        variant_order=@('diagnostic-before','diagnostic-after','candidate-untraced-release')
        shared_manifest_and_target_for_all_variants=$true; repo_source_mutation='none'; repository_target_mutation='none'
    }
    Write-Json (Join-Path $script:RunDir 'run-manifest.json') $manifest

    try {
        $beforeBytes = [IO.File]::ReadAllBytes($BeforeLib)
        $afterBytes = [IO.File]::ReadAllBytes($AfterLib)
        $benchBytes = [IO.File]::ReadAllBytes($script:FrozenBenchPath)
        Invoke-ReleaseStage 'diagnostic-before' $beforeBytes $ExpectedBefore $benchBytes $script:FrozenBenchSha $OutputPaths[0] | Out-Null
        Invoke-ReleaseStage 'diagnostic-after' $afterBytes $ExpectedAfter $benchBytes $script:FrozenBenchSha $OutputPaths[1] | Out-Null
        Invoke-ReleaseStage 'candidate-untraced-release' $script:FrozenProductionLib $ExpectedQualifiedSources['src/lib.rs'] $script:FrozenProductionBench $ExpectedQualifiedSources['benches/vector_memory.rs'] $OutputPaths[2] | Out-Null
    } catch { $failure = $_.Exception.ToString() }

    $finalQualification = $null
    try { $finalQualification = Assert-Qualification } catch { if (-not $failure) { $failure = "Final gate/receipt recheck failed: $($_.Exception.Message)" } }
    $result = [ordered]@{
        status=if($failure){'failed'}elseif($script:BuildRecords.Count -eq 3){'complete'}else{'incomplete'}
        run_id=$script:RunId; failure=$failure; qualification_receipt_sha256=$script:ReceiptSha; final_qualification=$finalQualification
        guard_runs=@($script:GuardRuns); builds=@($script:BuildRecords)
        paths=[ordered]@{ source_clone=$script:SourceClone; private_target_dir=$script:TargetDir; source_snapshot=$snapshotPath; run_dir=$script:RunDir }
        cleanup_policy='No C or D source/target files are edited, so no repository restoration is performed. Private clone and private target are preserved as run artifacts; if guard stop proof is incomplete, stop and require root-owned cleanup evidence before any later source edit.'
    }
    Write-Json (Join-Path $script:RunDir 'result.json') $result
    if ($failure) { throw $failure }
    if ($script:BuildRecords.Count -ne 3) { throw 'Not all three guarded release variants completed' }
} finally {
    if ($null -ne $lock) { $lock.Dispose() }
}
