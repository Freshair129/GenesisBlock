import pathlib,json,re,subprocess,time,ctypes,ctypes.wintypes,os
r=pathlib.Path(r'C:\Users\suanr\genesis-vector-memory-20261031')
work=r/'verify-src'; dep=r/'napi-target/x86_64-pc-windows-msvc/debug/deps'
started={pathlib.Path(s).name for s in re.findall(r'Running .*?\((.*?\.exe)\)',(r/'full-tests-resumed.log').read_text(errors='replace'))}
# The interrupted concurrency target is rerun below with the heavy bulk case excluded.
results=[]
class Counters(ctypes.Structure):
 _fields_=[('cb',ctypes.wintypes.DWORD),('PageFaultCount',ctypes.wintypes.DWORD)]+[(x,ctypes.c_size_t) for x in ('PeakWorkingSetSize','WorkingSetSize','QuotaPeakPagedPoolUsage','QuotaPagedPoolUsage','QuotaPeakNonPagedPoolUsage','QuotaNonPagedPoolUsage','PagefileUsage','PeakPagefileUsage')]
k=ctypes.WinDLL('kernel32',use_last_error=True); ps=ctypes.WinDLL('psapi',use_last_error=True)
k.OpenProcess.argtypes=[ctypes.wintypes.DWORD,ctypes.wintypes.BOOL,ctypes.wintypes.DWORD]; k.OpenProcess.restype=ctypes.wintypes.HANDLE
ps.GetProcessMemoryInfo.argtypes=[ctypes.wintypes.HANDLE,ctypes.POINTER(Counters),ctypes.wintypes.DWORD]
k.CloseHandle.argtypes=[ctypes.wintypes.HANDLE]
logs=r/'bounded-tests'; logs.mkdir(exist_ok=True)

cases=[('bulk_add_nodes_1000','concurrency-948a29ec82ad9112.exe','bulk_nodes_1000'),('bulk_add_edges_1000','concurrency-948a29ec82ad9112.exe','bulk_edges_1000'),('index_grows_past_initial_floor','hnsw_capacity_tests-e07a5c94f455a979.exe','test_hnsw_cap_grow')]
results=[]
for version in ('before','after'):
 for name,binary,fixture in cases:
  exe=r/('proof-'+version)/binary
  while not exe.exists():time.sleep(1)
  log=r/('proof-lifecycle-'+version+'-'+name+'.log');peak=0;ospeak=0;reason=None;start=time.monotonic()
  with log.open('w') as f:
   proc=subprocess.Popen([str(exe),'--exact',name,'--test-threads=1','--nocapture'],cwd=work,stdout=f,stderr=subprocess.STDOUT,creationflags=subprocess.CREATE_NO_WINDOW)
   handle=k.OpenProcess(0x1000|0x10,False,proc.pid)
   if not handle:raise RuntimeError('cannot open owned process for RSS sampling')
   try:
    while proc.poll() is None:
     c=Counters();c.cb=ctypes.sizeof(c)
     if ps.GetProcessMemoryInfo(handle,ctypes.byref(c),ctypes.sizeof(c)):
      peak=max(peak,c.WorkingSetSize);ospeak=max(ospeak,c.PeakWorkingSetSize)
     if peak>4*1024**3:reason='rss_limit_4_GiB';proc.kill()
     elif time.monotonic()-start>180:reason='timeout_180s';proc.kill()
     time.sleep(.02)
    code=proc.wait()
   finally:k.CloseHandle(handle)
  src=r/'napi-target/x86_64-pc-windows-msvc/tmp'/fixture;dst=r/('proof-lifecycle-fixtures/'+version+'-'+fixture)
  if src.exists():
   dst.parent.mkdir(exist_ok=True)
   assert src.resolve().is_relative_to(r.resolve()) and dst.resolve().is_relative_to(r.resolve())
   assert not dst.exists();src.rename(dst)
  import hashlib
  d={'version':version,'test':name,'binary':str(exe),'binary_sha256':hashlib.sha256(exe.read_bytes()).hexdigest(),'exit_code':code,'reason':reason,'sampled_peak_rss_bytes':peak,'os_peak_working_set_bytes':ospeak,'seconds':round(time.monotonic()-start,3),'log':log.name,'checkpoint_success':'not asserted by existing test; explicit diagnostic reported separately','fixture':str(dst)}
  results.append(d);(r/'proof-lifecycle.json').write_text(json.dumps(results,indent=2));print(json.dumps(d),flush=True)
(r/'proof-lifecycle.status').write_text('complete')
