param([Parameter(Mandatory=$true)][ValidateSet('before','after')][string]$Mode,
      [Parameter(Mandatory=$true)][string]$Label,
      [Parameter(Mandatory=$true)][uint64]$Seed,
      [switch]$Smoke)
$ErrorActionPreference='Stop'
$r='C:\Users\suanr\genesis-sqlite-prepare-20261031'
$base=Join-Path $r 'seeded-topology-20261101'
if($Label -notmatch '^[a-z0-9-]+$'){throw 'Invalid label'}
$case=Join-Path $base "runs\$Label"
if(Test-Path -LiteralPath $case){throw 'Refuse existing case'}
New-Item -ItemType Directory -Path (Join-Path $case 'topology') -Force | Out-Null
$c=Get-Content -Raw "$r\real-pair\$Mode\run-config.json" | ConvertFrom-Json
$c.run_root=$case; $c.repo=$case; $c.extra_env.GB_MEMORY_OUT="$case\output"
$c.extra_env | Add-Member -NotePropertyName GB_MEMORY_QUERY_EVIDENCE -NotePropertyValue '1'
$c.extra_env | Add-Member -NotePropertyName GB_MEMORY_HNSW_SEED -NotePropertyValue "$Seed"
$c.extra_env | Add-Member -NotePropertyName GB_MEMORY_TOPOLOGY_DIR -NotePropertyValue "$case\topology"
$c.binary="$base\seeded-$Mode.exe"
$c.binary_sha256=(Get-FileHash -LiteralPath $c.binary -Algorithm SHA256).Hash.ToLowerInvariant()
if($Smoke){
 $inputPath='C:\Users\suanr\genesis-vector-memory-20261031\inputs\synthetic-1000-768'
 $c.input_dir=$inputPath; $c.env_whitelist.GB_VBENCH=$inputPath; $c.env_whitelist.GB_MEMORY_INPUT=$inputPath
 foreach($pair in @(@('corpus','corpus.f32'),@('queries','queries.f32'),@('truth','truth.json'),@('meta','meta.json'))){
  $c.expected_input_hashes.($pair[0])=(Get-FileHash -LiteralPath (Join-Path $inputPath $pair[1]) -Algorithm SHA256).Hash.ToLowerInvariant()
 }
}
$c | ConvertTo-Json -Depth 10 | Set-Content -Encoding utf8 "$case\run-config.json"
$g=Get-Content -Raw "$r\real-$Mode-explicit-guard-config.json" | ConvertFrom-Json
$g.args[2]="$case\run-config.json"; $g.cwd=$case; $g.output="$base\runs\$Label-guard"
$config="$base\runs\$Label-guard-config.json"
$g | ConvertTo-Json -Depth 6 | Set-Content -Encoding utf8 $config
& "$r\await-resource-job.ps1" -Config $config -Label "seeded-$Label"
exit $LASTEXITCODE
