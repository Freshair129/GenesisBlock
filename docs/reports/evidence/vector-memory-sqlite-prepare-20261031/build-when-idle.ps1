$ErrorActionPreference='Stop'
$r=$PSScriptRoot
for($attempt=5;$attempt -le 10;$attempt++) {
 $c=Get-Content (Join-Path $r 'build-attempt3.json') -Raw|ConvertFrom-Json
 $c.output=Join-Path $r ("reproducer-build-attempt$attempt")
 $config=Join-Path $r ("build-attempt$attempt.json")
 $c|ConvertTo-Json -Depth 5|Set-Content $config
 try { & (Join-Path $r 'guard.ps1') -Config $config } catch { $_|Out-String|Set-Content (Join-Path $r ("build-controller-attempt$attempt.err")) }
 if(Test-Path (Join-Path $c.output 'status.json')) { $status=Get-Content (Join-Path $c.output 'status.json') -Raw|ConvertFrom-Json; if($status.exit -eq 0 -and -not $status.reason){ @{status='built';attempt=$attempt}|ConvertTo-Json|Set-Content (Join-Path $r 'build-queue-status.json');exit 0 };throw 'A real build ran and failed; stop for diagnosis' }
 if($attempt -lt 10){Start-Sleep -Seconds 55}
}
@{status='all_admissions_rejected_no_build_started';attempts=10}|ConvertTo-Json|Set-Content (Join-Path $r 'build-queue-status.json')
