import pathlib,json,re,subprocess,time,ctypes,ctypes.wintypes,os
r=pathlib.Path(r'C:\Users\suanr\genesis-sqlite-prepare-20261031')
work=pathlib.Path(r'C:\Users\suanr\genesis-vector-memory-20261031\verify-src'); dep=r/'final-test-bin'
started=set()
# Same whole-executable acceptance protocol: no filters or skipped targets.
results=[]
class Counters(ctypes.Structure):
 _fields_=[('cb',ctypes.wintypes.DWORD),('PageFaultCount',ctypes.wintypes.DWORD)]+[(x,ctypes.c_size_t) for x in ('PeakWorkingSetSize','WorkingSetSize','QuotaPeakPagedPoolUsage','QuotaPagedPoolUsage','QuotaPeakNonPagedPoolUsage','QuotaNonPagedPoolUsage','PagefileUsage','PeakPagefileUsage')]
k=ctypes.WinDLL('kernel32',use_last_error=True); ps=ctypes.WinDLL('psapi',use_last_error=True)
k.OpenProcess.argtypes=[ctypes.wintypes.DWORD,ctypes.wintypes.BOOL,ctypes.wintypes.DWORD]; k.OpenProcess.restype=ctypes.wintypes.HANDLE
ps.GetProcessMemoryInfo.argtypes=[ctypes.wintypes.HANDLE,ctypes.POINTER(Counters),ctypes.wintypes.DWORD]
k.CloseHandle.argtypes=[ctypes.wintypes.HANDLE]
import hashlib,datetime,sys
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

for ordinal,exe in enumerate(sorted(dep.glob('*.exe'))):
 if ordinal % 10 == 0:
  check=subprocess.run(['pwsh','-NoProfile','-Command',"@(Get-CimInstance Win32_Process|Where-Object {$_.Name -match '^(cargo|rustc)\\.exe$'}).Count"],capture_output=True,text=True,creationflags=subprocess.CREATE_NO_WINDOW)
  if check.returncode or int(check.stdout.strip()) != 0:
   (r/'final-runtime.status').write_text('resource_hold_before_next_target')
   (r/'functional-resume-hold.json').write_text(json.dumps({'completed':len(results),'next_target':exe.name,'compiler_check':check.stdout,'stderr':check.stderr,'reason':'New competing compiler; no current test was interrupted'},indent=2))
   raise SystemExit(75)

 args=[str(exe),'--test-threads=1']
 log=logs/(exe.stem+'.log'); peak=0; reason=None; start=time.monotonic()
 with log.open('w') as f:
  proc=subprocess.Popen(args,cwd=work,stdout=f,stderr=subprocess.STDOUT,creationflags=subprocess.CREATE_NO_WINDOW)
  handle=k.OpenProcess(0x1000|0x10,False,proc.pid)
  try:
   while proc.poll() is None:
    c=Counters(); c.cb=ctypes.sizeof(c)
    if handle and ps.GetProcessMemoryInfo(handle,ctypes.byref(c),ctypes.sizeof(c)): peak=max(peak,c.WorkingSetSize)
    if peak>4*1024**3: reason='rss_limit_4_GiB';proc.kill()
    elif time.monotonic()-start>300: reason='timeout_300s';proc.kill()
    time.sleep(.2)
   code=proc.wait()
  finally:
   if handle:k.CloseHandle(handle)
 d={'target':exe.name,'args':args[1:],'exit_code':code,'reason':reason,'peak_rss_bytes':peak,'seconds':round(time.monotonic()-start,3),'log':str(log.relative_to(r))}
 results.append(d); (r/'final-runtime.json').write_text(json.dumps(results,indent=2))
 print(json.dumps(d),flush=True)
(r/'final-runtime.status').write_text('complete')

