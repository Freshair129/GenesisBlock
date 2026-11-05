from pathlib import Path
import re
r=Path(__file__).parent
base=(r/'run-full-core-final.py').read_text(encoding='utf-8-sig')
header=base[:base.index("logs=r/'final-runtime'")]
body=base[base.index('for exe in sorted(dep.glob'):]
body=body.replace("'log':log.name", "'log':str(log.relative_to(r))")
body=body.replace("for exe in sorted(dep.glob('*.exe')):", """for ordinal,exe in enumerate(sorted(dep.glob('*.exe'))):
 if ordinal % 10 == 0:
  check=subprocess.run(['pwsh','-NoProfile','-Command',"@(Get-CimInstance Win32_Process|Where-Object {$_.Name -match '^(cargo|rustc)\\\\.exe$'}).Count"],capture_output=True,text=True,creationflags=subprocess.CREATE_NO_WINDOW)
  if check.returncode or int(check.stdout.strip()) != 0:
   (r/'final-runtime.status').write_text('resource_hold_before_next_target')
   (r/'functional-resume-hold.json').write_text(json.dumps({'completed':len(results),'next_target':exe.name,'compiler_check':check.stdout,'stderr':check.stderr,'reason':'New competing compiler; no current test was interrupted'},indent=2))
   raise SystemExit(75)
""")
# Use the same unchanged per-executable 300s/4GiB verifier on immutable binaries.
setup=r'''import hashlib,datetime,sys
repo=pathlib.Path(r'D:\GenesisBlock_Dev\GenesisBlock')
expected=json.loads((r/'final-test-artifacts.json').read_text())
assert expected['source']['src/lib.rs']=='5b8bb9bed5dde5fa6e7d4d255da3acce6c1bfe892092456133adb65a78998fdd'
assert expected['source']['src/projection_rebuild_tests.rs']=='e69dc7dd5cbc5043aa5ed9c717e449e50246a8fbd8ad56ac95ed135718c99be2'
def sha(p):
 h=hashlib.sha256()
 with p.open('rb') as f:
  while b:=f.read(1048576):h.update(b)
 return h.hexdigest()
manifest=json.loads((r/'verify-source-manifest.json').read_text())
for rel,h in manifest.items():
 assert sha(repo/rel)==h,('D source drift',rel)
 assert sha(work/rel)==h,('C source drift',rel)
for entry in expected['artifacts']:
 assert sha(dep/entry['binary'])==entry['sha256'],entry['binary']
# Capture the complete eligible private-clone input set before the fresh run.
full=dict(manifest)
for root in ('src','benches','include'):
 for p in (repo/root).rglob('*'):
  rel=p.relative_to(repo).as_posix();parts=rel.split('/')
  if p.is_symlink():raise AssertionError(('Source reparse point',rel))
  if not p.is_file() or any(x in ('.git','target','node_modules','temp_db','temp_db_bulk','database','databases') for x in parts):continue
  if p.suffix.lower() in ('.db','.sqlite','.sqlite3','.wal','.shm','.gwal','.gseg','.bin'):continue
  if re.search(r'(?i)(\.sqlite3?|\.db|\.wal|\.shm|\.gwal|\.gseg)(?:-|$)|(?:^|[-_.])(wal|shm)$',p.name):continue
  full[rel]=sha(p)
for rel in ('Cargo.toml','Cargo.lock','build.rs'):full[rel]=sha(repo/rel)
canonical=''.join(f'{p}|{full[p].lower()}\n' for p in sorted(full))
full_receipt={'recorded_at':datetime.datetime.now().astimezone().isoformat(),'manifest':full,'canonical_sha256':hashlib.sha256(canonical.encode()).hexdigest(),'qualified':False,'note':'Frozen before fresh immutable E69 runtime; becomes qualified only after complete successful suite and post checks'}
(r/'full-source-entry-manifest.json').write_text(json.dumps(full_receipt,indent=2)+'\n')
logs=r/('final-runtime-fresh-'+datetime.datetime.now().strftime('%Y%m%dT%H%M%S'));logs.mkdir()
(r/'final-runtime.status').write_text('running-fresh-final-E69')
(r/'final-runtime.json').write_text('[]\n')
'''
(r/'run-immutable-core.py').write_text(header+setup+'\n'+body,newline='\n')
