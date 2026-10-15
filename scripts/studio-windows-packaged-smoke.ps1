param(
  [switch]$TestExitCodeGuard
)

$ErrorActionPreference = 'Stop'

function Get-VerifiedChildExitCode([System.Diagnostics.Process]$Process, [string]$Name) {
  $Process.Refresh()
  if (-not $Process.HasExited) {
    throw "$Name has not exited, so its exit code is not available"
  }
  try {
    $exitCode = $Process.ExitCode
  }
  catch {
    throw "$Name exited, but its exit code could not be read: $($_.Exception.Message)"
  }
  if ($null -eq $exitCode) {
    throw "$Name exited, but its exit code was unavailable"
  }
  return [int]$exitCode
}

function Assert-ChildProcessSucceeded([System.Diagnostics.Process]$Process, [string]$Name) {
  $exitCode = Get-VerifiedChildExitCode $Process $Name
  if ($exitCode -ne 0) {
    throw "$Name failed with exit code $exitCode"
  }
}

if ($TestExitCodeGuard) {
  $childPowerShell = (Get-Command powershell.exe -ErrorAction Stop).Source
  $failedChild = Start-Process -FilePath $childPowerShell `
    -ArgumentList @('-NoProfile', '-NonInteractive', '-Command', 'exit 7') `
    -Wait `
    -PassThru
  try {
    Assert-ChildProcessSucceeded $failedChild 'Intentional exit-code guard probe'
  }
  catch {
    if ($_.Exception.Message -notmatch 'exit code 7') { throw }
    Write-Host 'Windows packaged smoke exit-code guard rejected the intentional exit code 7.'
    return
  }
  throw 'Windows packaged smoke exit-code guard accepted the intentional nonzero exit code'
}

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$bundleDirectory = Join-Path $repoRoot 'studio\src-tauri\target\debug\bundle\msi'
$smokeMsiPath = $env:STUDIO_PACKAGED_SMOKE_MSI
if (-not $smokeMsiPath -or -not (Test-Path -LiteralPath $smokeMsiPath -PathType Leaf)) {
  throw 'The isolated Windows UI-smoke MSI path was not provided by the packaging workflow'
}

$fixtureRoot = $env:STUDIO_PACKAGED_FIXTURE_ROOT
if (-not $fixtureRoot -or -not (Test-Path -LiteralPath $fixtureRoot -PathType Container)) {
  throw 'The persisted Studio parity fixture was not seeded before the packaged smoke'
}

$evidenceDirectory = Join-Path $env:RUNNER_TEMP 'studio-packaged-evidence'
New-Item -ItemType Directory -Path $evidenceDirectory -Force | Out-Null
$signalDirectory = Join-Path $evidenceDirectory "studio-smoke-$([guid]::NewGuid().ToString('N'))"
New-Item -ItemType Directory -Path $signalDirectory | Out-Null

$installLog = Join-Path $evidenceDirectory 'studio-msi-install.log'
$nodeStdout = Join-Path $evidenceDirectory 'studio-packaged-ui-smoke.stdout.log'
$nodeStderr = Join-Path $evidenceDirectory 'studio-packaged-ui-smoke.stderr.log'
$serverStdout = Join-Path $evidenceDirectory 'studio-parity-server.stdout.log'
$serverStderr = Join-Path $evidenceDirectory 'studio-parity-server.stderr.log'
$missingRoot = Join-Path $env:RUNNER_TEMP "studio missing root - 雪 $([guid]::NewGuid().ToString('N'))"
$nonDirectoryPath = Join-Path $env:RUNNER_TEMP "studio not a folder $([guid]::NewGuid().ToString('N')).txt"
$parityToken = 'studio-ci-parity-token'

$signals = @{
  PickerCancelRequested = Join-Path $signalDirectory 'picker-cancel-requested'
  PickerCancelled = Join-Path $signalDirectory 'picker-cancelled'
  PickerSelectRequested = Join-Path $signalDirectory 'picker-select-requested'
  PickerSelected = Join-Path $signalDirectory 'picker-selected'
  LocalCaptured = Join-Path $signalDirectory 'local-captured'
  RemoteReady = Join-Path $signalDirectory 'remote-ready'
}

$install = Start-Process -FilePath 'msiexec.exe' -ArgumentList @(
  '/i', "`"$smokeMsiPath`"", '/qn', '/norestart', '/L*v', "`"$installLog`""
) -Wait -PassThru
if ($install.ExitCode -ne 0) {
  Get-Content -LiteralPath $installLog -Tail 80 -ErrorAction SilentlyContinue
  throw "The isolated Studio UI-smoke MSI installation failed with exit code $($install.ExitCode)"
}

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
  throw "Studio MSI installed, but its app executable was not found. MSI log: $installLog"
}

Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes

function Find-StudioFolderDialog {
  $desktop = [System.Windows.Automation.AutomationElement]::RootElement
  $windows = $desktop.FindAll(
    [System.Windows.Automation.TreeScope]::Children,
    [System.Windows.Automation.Condition]::TrueCondition
  )
  foreach ($window in $windows) {
    if ($window.Current.Name -like '*Select a GenesisBlockDB data root*') {
      return $window
    }
  }
  return $null
}

function Wait-ForSignal([string]$Path, [System.Diagnostics.Process]$UiProcess, [int]$Seconds, [string]$Description) {
  $deadline = (Get-Date).AddSeconds($Seconds)
  while ((Get-Date) -lt $deadline) {
    if (Test-Path -LiteralPath $Path) { return }
    if ($UiProcess.HasExited) {
      Get-Content -LiteralPath $nodeStdout -ErrorAction SilentlyContinue
      Get-Content -LiteralPath $nodeStderr -ErrorAction SilentlyContinue
      throw "Packaged UI smoke exited before $Description (code $($UiProcess.ExitCode))"
    }
    Start-Sleep -Milliseconds 200
  }
  throw "Timed out waiting for packaged UI smoke to $Description"
}

function Wait-ForStudioFolderDialog([System.Diagnostics.Process]$UiProcess, [int]$Seconds) {
  $deadline = (Get-Date).AddSeconds($Seconds)
  while ((Get-Date) -lt $deadline) {
    if ($UiProcess.HasExited) {
      Get-Content -LiteralPath $nodeStdout -ErrorAction SilentlyContinue
      Get-Content -LiteralPath $nodeStderr -ErrorAction SilentlyContinue
      throw "Packaged UI smoke exited before opening the native folder picker (code $($UiProcess.ExitCode))"
    }
    $dialog = Find-StudioFolderDialog
    if ($dialog) { return $dialog }
    Start-Sleep -Milliseconds 200
  }
  throw "Packaged Studio did not open its titled Windows native folder picker within $Seconds seconds"
}

function Invoke-FolderDialogButton(
  [System.Windows.Automation.AutomationElement]$Dialog,
  [string[]]$Names
) {
  $buttonCondition = [System.Windows.Automation.PropertyCondition]::new(
    [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
    [System.Windows.Automation.ControlType]::Button
  )
  $buttons = $Dialog.FindAll([System.Windows.Automation.TreeScope]::Descendants, $buttonCondition)
  foreach ($button in $buttons) {
    if ($button.Current.Name -in $Names) {
      $pattern = [System.Windows.Automation.InvokePattern]$button.GetCurrentPattern(
        [System.Windows.Automation.InvokePattern]::Pattern
      )
      $pattern.Invoke()
      return $button.Current.Name
    }
  }
  $buttonNames = @($buttons | ForEach-Object { $_.Current.Name }) -join ', '
  throw "Windows folder picker button '$($Names -join ' / ')' was not available. Buttons: $buttonNames"
}

$smokeId = [guid]::NewGuid().ToString('N')
$appProcess = $null
$uiSmokeProcess = $null
$serverProcess = $null

try {
  $cdpPort = [int]$env:STUDIO_CDP_PORT
  if ($cdpPort -le 0) {
    throw 'The Windows UI-smoke build did not provide its WebView2 CDP port'
  }
  $cdpProbe = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback, $cdpPort)
  $cdpProbe.Start()
  $cdpProbe.Stop()

  $env:STUDIO_CDP_PORT = "$cdpPort"
  $env:STUDIO_MISSING_ROOT = $missingRoot
  $env:STUDIO_NON_DIRECTORY_PATH = $nonDirectoryPath
  $env:STUDIO_FIXTURE_ROOT = $fixtureRoot
  $env:STUDIO_PARITY_TOKEN = $parityToken
  $env:STUDIO_EVIDENCE_DIRECTORY = $evidenceDirectory
  $env:STUDIO_PICKER_CANCEL_REQUESTED_FILE = $signals.PickerCancelRequested
  $env:STUDIO_PICKER_CANCELLED_FILE = $signals.PickerCancelled
  $env:STUDIO_PICKER_SELECT_REQUESTED_FILE = $signals.PickerSelectRequested
  $env:STUDIO_PICKER_SELECTED_FILE = $signals.PickerSelected
  $env:STUDIO_LOCAL_CAPTURED_FILE = $signals.LocalCaptured
  $env:STUDIO_REMOTE_READY_FILE = $signals.RemoteReady

  $appProcess = Start-Process -FilePath $appExecutable.FullName -PassThru
  Start-Sleep -Seconds 8
  if ($appProcess.HasExited) {
    throw "Installed Studio exited during its startup window with code $($appProcess.ExitCode)"
  }

  $uiSmokeScript = Join-Path $repoRoot 'scripts\studio-windows-packaged-ui-smoke.mjs'
  $node = (Get-Command node.exe -ErrorAction Stop).Source
  $uiSmokeProcess = Start-Process -FilePath $node `
    -ArgumentList @("`"$uiSmokeScript`"") `
    -WorkingDirectory $repoRoot `
    -PassThru `
    -NoNewWindow `
    -RedirectStandardOutput $nodeStdout `
    -RedirectStandardError $nodeStderr

  Wait-ForSignal $signals.PickerCancelRequested $uiSmokeProcess 120 'request the native picker cancellation probe'
  $cancelDialog = Wait-ForStudioFolderDialog $uiSmokeProcess 45
  $cancelledButton = Invoke-FolderDialogButton $cancelDialog @('Cancel')
  $dialogCloseDeadline = (Get-Date).AddSeconds(10)
  while ((Get-Date) -lt $dialogCloseDeadline -and (Find-StudioFolderDialog)) {
    Start-Sleep -Milliseconds 200
  }
  if (Find-StudioFolderDialog) {
    throw 'The native folder picker did not close after the Cancel button was invoked'
  }
  Set-Content -LiteralPath $signals.PickerCancelled -Value $cancelledButton -NoNewline

  Wait-ForSignal $signals.PickerSelectRequested $uiSmokeProcess 45 'request successful folder selection'
  $selectDialog = Wait-ForStudioFolderDialog $uiSmokeProcess 45
  $shell = New-Object -ComObject WScript.Shell
  if (-not $shell.AppActivate($selectDialog.Current.Name)) {
    throw "Could not activate native folder picker '$($selectDialog.Current.Name)'"
  }
  Start-Sleep -Milliseconds 250
  $shell.SendKeys('%d')
  $shell.SendKeys('^a')
  $shell.SendKeys($fixtureRoot)
  $shell.SendKeys('{ENTER}')
  Start-Sleep -Seconds 2
  $selectDialog = Find-StudioFolderDialog
  if (-not $selectDialog) {
    throw 'The native folder picker closed while navigating to the persisted fixture'
  }
  $selectedButton = Invoke-FolderDialogButton $selectDialog @('Select Folder')
  $dialogCloseDeadline = (Get-Date).AddSeconds(10)
  while ((Get-Date) -lt $dialogCloseDeadline -and (Find-StudioFolderDialog)) {
    Start-Sleep -Milliseconds 200
  }
  if (Find-StudioFolderDialog) {
    throw "The native folder picker did not close after '$selectedButton' was invoked"
  }
  Set-Content -LiteralPath $signals.PickerSelected -Value $selectedButton -NoNewline

  Wait-ForSignal $signals.LocalCaptured $uiSmokeProcess 120 'complete packaged local fixture reads and disconnect'

  $serverExecutable = Join-Path $repoRoot 'target\debug\genesis-db-server.exe'
  if (-not (Test-Path -LiteralPath $serverExecutable -PathType Leaf)) {
    throw "The packaged parity REST server executable was not built: $serverExecutable"
  }
  $portProbe = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback, 0)
  $portProbe.Start()
  $restPort = $portProbe.LocalEndpoint.Port
  $portProbe.Stop()
  $restBaseUrl = "http://127.0.0.1:$restPort"
  $env:GENESIS_DATA_DIR = $fixtureRoot
  $env:GENESIS_HOST = '127.0.0.1'
  $env:GENESIS_PORT = "$restPort"
  $env:GENESIS_API_KEY = $parityToken
  $serverProcess = Start-Process -FilePath $serverExecutable `
    -WorkingDirectory $repoRoot `
    -PassThru `
    -NoNewWindow `
    -RedirectStandardOutput $serverStdout `
    -RedirectStandardError $serverStderr
  Remove-Item Env:GENESIS_DATA_DIR -ErrorAction SilentlyContinue
  Remove-Item Env:GENESIS_HOST -ErrorAction SilentlyContinue
  Remove-Item Env:GENESIS_PORT -ErrorAction SilentlyContinue
  Remove-Item Env:GENESIS_API_KEY -ErrorAction SilentlyContinue

  $headers = @{ Authorization = "Bearer $parityToken" }
  $serverDeadline = (Get-Date).AddSeconds(60)
  $serverReady = $false
  while ((Get-Date) -lt $serverDeadline) {
    if ($serverProcess.HasExited) {
      Get-Content -LiteralPath $serverStdout -ErrorAction SilentlyContinue
      Get-Content -LiteralPath $serverStderr -ErrorAction SilentlyContinue
      throw "Packaged parity REST server exited during startup (code $($serverProcess.ExitCode))"
    }
    try {
      $capabilities = Invoke-RestMethod -Uri "$restBaseUrl/v1/studio/capabilities" `
        -Headers $headers -TimeoutSec 2 -ErrorAction Stop
      if ($capabilities.mode -eq 'remote') {
        $serverReady = $true
        break
      }
    }
    catch {
      Start-Sleep -Milliseconds 250
    }
  }
  if (-not $serverReady) {
    Get-Content -LiteralPath $serverStdout -ErrorAction SilentlyContinue
    Get-Content -LiteralPath $serverStderr -ErrorAction SilentlyContinue
    throw "Packaged parity REST server did not become ready at $restBaseUrl"
  }
  Set-Content -LiteralPath $signals.RemoteReady -Value $restBaseUrl -NoNewline

  if (-not $uiSmokeProcess.WaitForExit(180000)) {
    throw 'Packaged local/remote parity UI probe did not finish within 180 seconds'
  }
  $uiSmokeProcess.Refresh()
  Get-Content -LiteralPath $nodeStdout -ErrorAction SilentlyContinue
  Get-Content -LiteralPath $nodeStderr -ErrorAction SilentlyContinue
  Assert-ChildProcessSucceeded $uiSmokeProcess 'Packaged local/remote parity UI probe'
  if ($appProcess.HasExited) {
    throw "Installed Studio exited during the picker/parity smoke with code $($appProcess.ExitCode)"
  }

  $summary = Get-Content -LiteralPath (Join-Path $evidenceDirectory 'studio-packaged-parity.json') -Raw | ConvertFrom-Json
  $summary | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $evidenceDirectory 'studio-windows-smoke-result.json')
  @(
    '## Genesis Studio packaged Windows evidence'
    ''
    '- MSI installed and remained open during packaged UI checks.'
    '- Missing and non-directory local root paths displayed actionable errors.'
    '- Native folder picker opened, cancelled, then selected the persisted parity fixture.'
    "- Packaged local/remote parity passed for: $($summary.comparedOperations -join ', ')."
    "- Fixture identifier: $($summary.fixtureId)."
  ) | Add-Content -LiteralPath $env:GITHUB_STEP_SUMMARY
  Write-Host "Installed MSI startup passed: $($appExecutable.FullName)"
  Write-Host "Packaged folder picker cancellation and selection passed: $fixtureRoot"
  Write-Host "Packaged local/remote fixture parity passed for: $($summary.comparedOperations -join ', ')"
}
catch {
  Get-Content -LiteralPath $nodeStdout -ErrorAction SilentlyContinue
  Get-Content -LiteralPath $nodeStderr -ErrorAction SilentlyContinue
  Get-Content -LiteralPath $serverStdout -ErrorAction SilentlyContinue
  Get-Content -LiteralPath $serverStderr -ErrorAction SilentlyContinue
  throw
}
finally {
  if ($uiSmokeProcess -and -not $uiSmokeProcess.HasExited) {
    Stop-Process -Id $uiSmokeProcess.Id -Force -ErrorAction SilentlyContinue
  }
  if ($serverProcess -and -not $serverProcess.HasExited) {
    Stop-Process -Id $serverProcess.Id -Force -ErrorAction SilentlyContinue
  }
  if ($appProcess -and -not $appProcess.HasExited) {
    Stop-Process -Id $appProcess.Id -Force -ErrorAction SilentlyContinue
  }
  Remove-Item Env:STUDIO_CDP_PORT -ErrorAction SilentlyContinue
  Remove-Item Env:STUDIO_PACKAGED_SMOKE_MSI -ErrorAction SilentlyContinue
  Remove-Item Env:STUDIO_MISSING_ROOT -ErrorAction SilentlyContinue
  Remove-Item Env:STUDIO_NON_DIRECTORY_PATH -ErrorAction SilentlyContinue
  Remove-Item Env:STUDIO_FIXTURE_ROOT -ErrorAction SilentlyContinue
  Remove-Item Env:STUDIO_PARITY_TOKEN -ErrorAction SilentlyContinue
  Remove-Item Env:STUDIO_EVIDENCE_DIRECTORY -ErrorAction SilentlyContinue
  Remove-Item Env:STUDIO_PICKER_CANCEL_REQUESTED_FILE -ErrorAction SilentlyContinue
  Remove-Item Env:STUDIO_PICKER_CANCELLED_FILE -ErrorAction SilentlyContinue
  Remove-Item Env:STUDIO_PICKER_SELECT_REQUESTED_FILE -ErrorAction SilentlyContinue
  Remove-Item Env:STUDIO_PICKER_SELECTED_FILE -ErrorAction SilentlyContinue
  Remove-Item Env:STUDIO_LOCAL_CAPTURED_FILE -ErrorAction SilentlyContinue
  Remove-Item Env:STUDIO_REMOTE_READY_FILE -ErrorAction SilentlyContinue
  Remove-Item -LiteralPath $smokeMsiPath -Force -ErrorAction SilentlyContinue
}
