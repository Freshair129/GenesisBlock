import pathlib,json,re,subprocess,time,ctypes,ctypes.wintypes,os
r=pathlib.Path(r'C:\Users\suanr\genesis-vector-memory-followup-20261031')
work=pathlib.Path(r'D:\GenesisBlock_Dev\GenesisBlock'); dep=r/'final-test-bin'
started=set()
# The interrupted concurrency target is rerun below with the heavy bulk case excluded.
results=[]
class Counters(ctypes.Structure):
 _fields_=[('cb',ctypes.wintypes.DWORD),('PageFaultCount',ctypes.wintypes.DWORD)]+[(x,ctypes.c_size_t) for x in ('PeakWorkingSetSize','WorkingSetSize','QuotaPeakPagedPoolUsage','QuotaPagedPoolUsage','QuotaPeakNonPagedPoolUsage','QuotaNonPagedPoolUsage','PagefileUsage','PeakPagefileUsage')]
k=ctypes.WinDLL('kernel32',use_last_error=True); ps=ctypes.WinDLL('psapi',use_last_error=True)
k.OpenProcess.argtypes=[ctypes.wintypes.DWORD,ctypes.wintypes.BOOL,ctypes.wintypes.DWORD]; k.OpenProcess.restype=ctypes.wintypes.HANDLE
ps.GetProcessMemoryInfo.argtypes=[ctypes.wintypes.HANDLE,ctypes.POINTER(Counters),ctypes.wintypes.DWORD]
k.CloseHandle.argtypes=[ctypes.wintypes.HANDLE]
logs=r/'final-runtime'; logs.mkdir(exist_ok=True)
while not (r/'final-core-build.status').exists() or not (r/'six2-runtime.status').exists():time.sleep(1)
assert (r/'final-core-build.status').read_text().strip()=='0'
import shutil,hashlib
artifact={};profiles=[]
for line in (r/'final-core-build.jsonl').read_text(errors='replace').splitlines():
 try:d=json.loads(line)
 except json.JSONDecodeError:continue
 if d.get('reason')=='compiler-artifact':
  if d.get('target',{}).get('name') in ('sha2','curve25519_dalek','lz4_flex'):profiles.append({'name':d['target']['name'],'profile':d['profile']})
  if d.get('profile',{}).get('test') and d.get('executable'):artifact[d['executable']]=d
assert len(artifact)==159,len(artifact)
dep.mkdir(exist_ok=True);binaries=[]
for path,d in artifact.items():
 p=pathlib.Path(path);shutil.copyfile(p,dep/p.name);binaries.append({'target':d['target']['name'],'binary':p.name,'sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'profile':d['profile']})
(r/'final-test-artifacts.json').write_text(json.dumps({'source':{f:hashlib.sha256((work/f).read_bytes()).hexdigest() for f in ('src/lib.rs','src/vector_memory_tests.rs','Cargo.toml','Cargo.lock')},'artifacts':binaries,'dependency_profiles':profiles},indent=2))

for exe in sorted(dep.glob('*.exe')):
 is_concurrency=False
 if exe.name in started and not is_concurrency: continue
 args=[str(exe),'--test-threads=1']
 if is_concurrency: args+=['--skip','bulk_add_edges_1000']
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
 d={'target':exe.name,'args':args[1:],'exit_code':code,'reason':reason,'peak_rss_bytes':peak,'seconds':round(time.monotonic()-start,3),'log':log.name}
 results.append(d); (r/'final-runtime.json').write_text(json.dumps(results,indent=2))
 print(json.dumps(d),flush=True)
(r/'final-runtime.status').write_text('complete')
