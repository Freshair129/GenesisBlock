$ErrorActionPreference = 'Stop'

$normalPackageDirectory = Join-Path $env:RUNNER_TEMP 'studio-normal-package'
$evidenceDirectory = Join-Path $env:RUNNER_TEMP 'studio-packaged-evidence'
New-Item -ItemType Directory -Path $evidenceDirectory -Force | Out-Null

$msiFiles = @(Get-ChildItem -LiteralPath $normalPackageDirectory -Filter '*.msi' -File -ErrorAction Stop)
if ($msiFiles.Count -ne 1) {
  throw "Expected exactly one ordinary MSI in $normalPackageDirectory, found $($msiFiles.Count)"
}
$msi = $msiFiles[0]

$evidencePath = Join-Path $evidenceDirectory 'studio-windows-normal-msi-startup.json'
$installLog = Join-Path $evidenceDirectory 'studio-windows-normal-msi-install.log'
$uninstallLog = Join-Path $evidenceDirectory 'studio-windows-normal-msi-uninstall.log'
$startupWindowSeconds = 10
$evidence = [ordered]@{
  package = $msi.Name
  package_sha256 = (Get-FileHash -LiteralPath $msi.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
  installed_executable = $null
  startup_window_seconds = $startupWindowSeconds
  process_id = $null
  process_alive = $false
  visible_ui_verified = $false
  fixture_parity = $false
  install_exit_code = $null
  uninstall_exit_code = $null
  cleanup_complete = $false
  failure = $null
}
$installed = $false
$appProcess = $null
$failure = $null

function Invoke-Msi([string[]]$MsiArgs) {
  $process = Start-Process -FilePath 'msiexec.exe' -ArgumentList $MsiArgs -PassThru -ErrorAction Stop
  if (-not $process.WaitForExit(300000)) {
    try { $process.Kill() } catch { }
    throw 'MSI operation exceeded the five-minute limit'
  }
  $process.Refresh()
  return [int]$process.ExitCode
}

try {
  $installArguments = @('/i', "`"$($msi.FullName)`"", '/qn', '/norestart', '/L*v', "`"$installLog`"")
  $evidence.install_exit_code = Invoke-Msi -MsiArgs $installArguments
  if ($evidence.install_exit_code -notin @(0, 3010)) {
    throw "Ordinary Studio MSI installation failed with exit code $($evidence.install_exit_code)"
  }
  $installed = $true

  $uninstallRoots = @(
    'Registry::HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall',
    'Registry::HKEY_LOCAL_MACHINE\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall',
    'Registry::HKEY_CURRENT_USER\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall'
  )
  $installedRecords = foreach ($registryRoot in $uninstallRoots) {
    if (Test-Path -LiteralPath $registryRoot) {
      Get-ChildItem -LiteralPath $registryRoot -ErrorAction SilentlyContinue |
        ForEach-Object { Get-ItemProperty -LiteralPath $_.PSPath -ErrorAction SilentlyContinue } |
        Where-Object { $_.DisplayName -like 'Genesis Studio*' }
    }
  }
  $searchRoots = @(
    $installedRecords | ForEach-Object { $_.InstallLocation }
    "$env:ProgramFiles\Genesis Studio"
    "${env:ProgramFiles(x86)}\Genesis Studio"
    "$env:LOCALAPPDATA\Programs\Genesis Studio"
    "$env:LOCALAPPDATA\Genesis Studio"
  ) | Where-Object { $_ -and (Test-Path -LiteralPath $_ -PathType Container) } | Select-Object -Unique

  $appExecutable = $null
  foreach ($searchRoot in $searchRoots) {
    $appExecutable = Get-ChildItem -LiteralPath $searchRoot -Filter '*.exe' -File -Recurse -ErrorAction SilentlyContinue |
      Where-Object { $_.Name -in @('Genesis Studio.exe', 'genesis-studio.exe') } |
      Select-Object -First 1
    if ($appExecutable) { break }
  }
  if (-not $appExecutable) {
    throw "Ordinary Studio MSI installed, but its app executable was not found. MSI log: $installLog"
  }

  $evidence.installed_executable = $appExecutable.FullName
  $appProcess = Start-Process -FilePath $appExecutable.FullName -PassThru -ErrorAction Stop
  $evidence.process_id = $appProcess.Id
  $exitedDuringWindow = $appProcess.WaitForExit($startupWindowSeconds * 1000)
  $appProcess.Refresh()
  $evidence.process_alive = -not $exitedDuringWindow
  if (-not $evidence.process_alive) {
    throw "Ordinary packaged Studio process exited during the $startupWindowSeconds-second startup window (code $($appProcess.ExitCode))"
  }
}
catch {
  $failure = $_.Exception.Message
}
finally {
  if ($appProcess) {
    try {
      $appProcess.Refresh()
      if (-not $appProcess.HasExited) { Stop-Process -Id $appProcess.Id -Force -ErrorAction SilentlyContinue }
    }
    catch { }
  }

  if ($installed) {
    try {
      $uninstallArguments = @('/x', "`"$($msi.FullName)`"", '/qn', '/norestart', '/L*v', "`"$uninstallLog`"")
      $evidence.uninstall_exit_code = Invoke-Msi -MsiArgs $uninstallArguments
      if ($evidence.uninstall_exit_code -notin @(0, 3010)) {
        $failure = "Ordinary Studio MSI uninstall failed with exit code $($evidence.uninstall_exit_code)"
      }
      else {
        $evidence.cleanup_complete = $true
      }
    }
    catch {
      $failure = "Ordinary Studio MSI cleanup failed: $($_.Exception.Message)"
    }
  }

  $evidence.failure = $failure
  $evidenceJson = ConvertTo-Json -InputObject $evidence -Depth 5
  [System.IO.File]::WriteAllText($evidencePath, $evidenceJson, [System.Text.UTF8Encoding]::new($false))
}

if ($failure) {
  throw "Ordinary packaged Windows MSI smoke failed: $failure (evidence: $evidencePath)"
}

Write-Host "Ordinary packaged MSI install/launch evidence: $evidencePath"
