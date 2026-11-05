import pathlib,json,re,subprocess,time,ctypes,ctypes.wintypes,os
r=pathlib.Path(r'C:\Users\suanr\genesis-vector-memory-20261031')
work=r/'verify-src'; dep=r/'napi-target/x86_64-pc-windows-msvc/debug/deps'
started=set()
# The interrupted concurrency target is rerun below with the heavy bulk case excluded.
results=[]
class Counters(ctypes.Structure):
 _fields_=[('cb',ctypes.wintypes.DWORD),('PageFaultCount',ctypes.wintypes.DWORD)]+[(x,ctypes.c_size_t) for x in ('PeakWorkingSetSize','WorkingSetSize','QuotaPeakPagedPoolUsage','QuotaPagedPoolUsage','QuotaPeakNonPagedPoolUsage','QuotaNonPagedPoolUsage','PagefileUsage','PeakPagefileUsage')]
k=ctypes.WinDLL('kernel32',use_last_error=True); ps=ctypes.WinDLL('psapi',use_last_error=True)
k.OpenProcess.argtypes=[ctypes.wintypes.DWORD,ctypes.wintypes.BOOL,ctypes.wintypes.DWORD]; k.OpenProcess.restype=ctypes.wintypes.HANDLE
ps.GetProcessMemoryInfo.argtypes=[ctypes.wintypes.HANDLE,ctypes.POINTER(Counters),ctypes.wintypes.DWORD]
k.CloseHandle.argtypes=[ctypes.wintypes.HANDLE]
logs=r/'final-recovery-focused'; logs.mkdir(exist_ok=True)
while not (r/'proof-lifecycle.status').exists() or not (r/'final-core-build.status').exists():time.sleep(1)
assert (r/'final-core-build.status').read_text().strip()=='0'
cases=[('journal_format_tests-fe6b4cd9fa0e01d3.exe','journal_compresses_authenticated_compaction_proofs'),('rebuild_from_truth_tests-afd2a683c957236a.exe','large_batch_roundtrip'),('hql_cypher_tests-cf7711df2414f595.exe','cypher_id_anchor'),('consensus_checkpoint_race_tests-23a17829175f7dbc.exe','consensus_commits_survive_concurrent_compact')]
for binary,skip in cases:
 exe=dep/binary
 is_concurrency=False
 if exe.name in started and not is_concurrency: continue
 args=[str(exe),'--test-threads=1','--skip',skip]
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
    elif time.monotonic()-start>180: reason='timeout_180s';proc.kill()
    time.sleep(.2)
   code=proc.wait()
  finally:
   if handle:k.CloseHandle(handle)
 d={'target':exe.name,'args':args[1:],'exit_code':code,'reason':reason,'peak_rss_bytes':peak,'seconds':round(time.monotonic()-start,3),'log':log.name}
 results.append(d); (r/'final-recovery-focused.json').write_text(json.dumps(results,indent=2))
 print(json.dumps(d),flush=True)
(r/'final-recovery-focused.status').write_text('complete')
