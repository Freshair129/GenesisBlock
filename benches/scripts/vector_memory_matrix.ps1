param(
    [Parameter(Mandatory=$true)][string]$Exe,
    [Parameter(Mandatory=$true)][string]$Evidence,
    [ValidateSet('prepare','baseline','candidate')][string]$Stage = 'prepare',
    [int[]]$Dims = @(768,1024,1536),
    [int]$Count = 10000,
    [string]$RealCorpus = '',
    [int]$MinFreeMemoryGB = 6,
    [string[]]$SelectedCases = @('none-r0','sq8-r0','sq8-r1','bq-r0','bq-r1','f16-r0')
)
$ErrorActionPreference = 'Stop'
$Exe = (Resolve-Path -LiteralPath $Exe).Path
$Evidence = [IO.Path]::GetFullPath($Evidence)
New-Item -ItemType Directory -Path $Evidence -Force | Out-Null
$env:GB_MEMORY_AUDIT = '1'
$env:GB_EF = '200'
$env:GB_VEC_Q = '24'
$env:RAYON_NUM_THREADS = '4'
$cases = @(@('none','0'),@('sq8','0'),@('sq8','1'),@('bq','0'),@('bq','1'),@('f16','0'))
foreach ($dim in $Dims) {
    $name = if ($RealCorpus) { "real-$Count-$dim" } else { "synthetic-$Count-$dim" }
    $inputPath = Join-Path $Evidence "inputs/$name"
    $env:GB_VBENCH = $inputPath
    if ($Stage -eq 'prepare') {
        $env:GB_MEMORY_PREPARE = '1'
        $env:GB_VEC_N = "$Count"
        $env:GB_VEC_DIM = "$dim"
        if ($RealCorpus) { $env:GB_MEMORY_CORPUS = (Resolve-Path -LiteralPath $RealCorpus).Path }
        else { Remove-Item Env:GB_MEMORY_CORPUS -ErrorAction SilentlyContinue }
        & $Exe > (Join-Path $Evidence "prepare-$name.log") 2>&1
        if ($LASTEXITCODE -ne 0) { throw "prepare failed: $name" }
        continue
    }
    Remove-Item Env:GB_MEMORY_PREPARE -ErrorAction SilentlyContinue
    foreach ($case in $cases) {
        if ("$($case[0])-r$($case[1])" -notin $SelectedCases) { continue }
        $free = (Get-CimInstance Win32_OperatingSystem).FreePhysicalMemory * 1KB
        if ($free -lt ($MinFreeMemoryGB * 1GB)) { throw "Less than $MinFreeMemoryGB GiB free RAM; matrix stopped before starting another process" }
        $env:GB_QUANT = $case[0]
        $env:GB_RERANK = $case[1]
        $caseName = "$name-$($case[0])-r$($case[1])"
        $env:GB_MEMORY_OUT = Join-Path $Evidence "$Stage/$caseName"
        if (Test-Path -LiteralPath $env:GB_MEMORY_OUT) { throw "Refuse to overwrite $env:GB_MEMORY_OUT" }
        New-Item -ItemType Directory -Path $env:GB_MEMORY_OUT | Out-Null
        $stdout = Join-Path $env:GB_MEMORY_OUT 'stdout.log'
        $stderr = Join-Path $env:GB_MEMORY_OUT 'stderr.log'
        $process = Start-Process -FilePath $Exe -PassThru -WindowStyle Hidden -RedirectStandardOutput $stdout -RedirectStandardError $stderr
        while (-not $process.HasExited) {
            Start-Sleep -Milliseconds 500
            $process.Refresh()
            if (-not $process.HasExited -and $process.WorkingSet64 -gt 4GB) {
                # Only this newly-created benchmark process, never a live DB/server.
                Stop-Process -Id $process.Id
                throw "Benchmark $caseName exceeded the 4 GiB working set budget; artifacts retained"
            }
        }
        $process.WaitForExit()
        if ($process.ExitCode -ne 0) { throw "benchmark failed: $caseName ($($process.ExitCode))" }
        Write-Output "$Stage $caseName complete"
    }
}
