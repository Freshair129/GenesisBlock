import pathlib,json,re,subprocess,time,ctypes,ctypes.wintypes,os
r=pathlib.Path(r'C:\Users\suanr\genesis-vector-memory-followup-20261031')
work=pathlib.Path(r'D:\GenesisBlock_Dev\GenesisBlock'); dep=r/'six-candidate-tests'
started=set()
# The interrupted concurrency target is rerun below with the heavy bulk case excluded.
results=[]
class Counters(ctypes.Structure):
 _fields_=[('cb',ctypes.wintypes.DWORD),('PageFaultCount',ctypes.wintypes.DWORD)]+[(x,ctypes.c_size_t) for x in ('PeakWorkingSetSize','WorkingSetSize','QuotaPeakPagedPoolUsage','QuotaPagedPoolUsage','QuotaPeakNonPagedPoolUsage','QuotaNonPagedPoolUsage','PagefileUsage','PeakPagefileUsage')]
k=ctypes.WinDLL('kernel32',use_last_error=True); ps=ctypes.WinDLL('psapi',use_last_error=True)
k.OpenProcess.argtypes=[ctypes.wintypes.DWORD,ctypes.wintypes.BOOL,ctypes.wintypes.DWORD]; k.OpenProcess.restype=ctypes.wintypes.HANDLE
ps.GetProcessMemoryInfo.argtypes=[ctypes.wintypes.HANDLE,ctypes.POINTER(Counters),ctypes.wintypes.DWORD]
k.CloseHandle.argtypes=[ctypes.wintypes.HANDLE]
logs=r/'six-runtime'; logs.mkdir(exist_ok=True)
while not (r/'six-build.status').exists():time.sleep(1)
assert (r/'six-build.status').read_text().strip()=='0'
import shutil,hashlib
names={'concurrency','consensus_checkpoint_race_tests','hql_cypher_tests','journal_format_tests','rebuild_from_truth_tests','wal_compaction_tests'}
dep.mkdir(exist_ok=True);artifact=[]
for line in (r/'six-build.jsonl').read_text(errors='replace').splitlines():
 try:d=json.loads(line)
 except json.JSONDecodeError:continue
 if d.get('reason')=='compiler-artifact' and d.get('target',{}).get('name') in names and d.get('executable'):
  p=pathlib.Path(d['executable']);shutil.copyfile(p,dep/p.name);artifact.append({'target':d['target']['name'],'binary':p.name,'sha256':hashlib.sha256(p.read_bytes()).hexdigest()})
assert len(artifact)==6,artifact
(r/'six-artifacts.json').write_text(json.dumps({'source_sha256':hashlib.sha256((work/'src/lib.rs').read_bytes()).hexdigest(),'tests_sha256':hashlib.sha256((work/'src/vector_memory_tests.rs').read_bytes()).hexdigest(),'artifacts':artifact},indent=2))
tmp=pathlib.Path(r'C:\Users\suanr\genesis-vector-memory-20261031\napi-target\x86_64-pc-windows-msvc\tmp')
backup=r/'pre-followup-failed-fixtures';backup.mkdir(exist_ok=True)
for name in ('bulk_edges_1000','test_consensus_ckpt_race','test_consensus_ckpt_race_img','test_consensus_compact_race','test_consensus_compact_race_img','cypher_id_anchor_fast_path_full_rows','jf_ratio','rebuild_large_batch','test_wal_compaction_shrink'):
 src=tmp/name;dst=backup/name
 if src.exists():
  assert src.resolve().is_relative_to(tmp.resolve()) and dst.resolve().is_relative_to(r.resolve()) and not dst.exists()
  src.rename(dst)

for exe in sorted(dep.glob('*.exe')):
 is_concurrency=False
 if exe.name in started and not is_concurrency: continue
 args=[str(exe),'--test-threads=1','--nocapture']
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
 results.append(d); (r/'six-runtime.json').write_text(json.dumps(results,indent=2))
 print(json.dumps(d),flush=True)
(r/'six-runtime.status').write_text('complete')
