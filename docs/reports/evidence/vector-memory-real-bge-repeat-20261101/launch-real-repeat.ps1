param([Parameter(Mandatory=$true)][ValidateSet('before','after')][string]$Mode,
      [Parameter(Mandatory=$true)][int]$Round)
$ErrorActionPreference='Stop'
$r='C:\Users\suanr\genesis-sqlite-prepare-20261031'
if($Round -lt 1 -or $Round -gt 20){throw 'Invalid round'}
$case=Join-Path $r "real-repeat\round$Round-$Mode"
if(Test-Path -LiteralPath $case){throw 'Refuse existing case'}
New-Item -ItemType Directory -Path $case -Force | Out-Null
$c=Get-Content -Raw "$r\real-pair\$Mode\run-config.json" | ConvertFrom-Json
$c.run_root=$case; $c.repo=$case; $c.extra_env.GB_MEMORY_OUT="$case\output"
$c | ConvertTo-Json -Depth 10 | Set-Content -Encoding utf8 "$case\run-config.json"
$g=Get-Content -Raw "$r\real-$Mode-explicit-guard-config.json" | ConvertFrom-Json
$g.args[2]="$case\run-config.json"; $g.cwd=$case; $g.output="$r\real-repeat\round$Round-$Mode-guard"
$config="$r\real-repeat\round$Round-$Mode-guard-config.json"
$g | ConvertTo-Json -Depth 6 | Set-Content -Encoding utf8 $config
& "$r\await-resource-job.ps1" -Config $config -Label "real-repeat-round$Round-$Mode"
exit $LASTEXITCODE
