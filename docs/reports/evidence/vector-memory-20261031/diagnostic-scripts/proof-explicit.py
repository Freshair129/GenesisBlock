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

import shutil,hashlib
results=[]
for version in ('before','after'):
 addon=r/('proof-'+version)/'index.win32-x64-msvc.node'
 while not addon.exists():time.sleep(1)
 for n in (128,1000):
  case=r/'proof-explicit-results'/f'{version}-{n}';case.mkdir(parents=True,exist_ok=False);db=case/'gdb'
  for mode in ('write','open'):
   output=case/(mode+'.json');log=case/(mode+'.log');start=time.monotonic();peak=0;ospeak=0;reason=None
   with log.open('w') as f:
    proc=subprocess.Popen([shutil.which('node'),str(r/'proof-explicit.cjs'),mode,str(addon),str(db),str(n),str(output)],stdout=f,stderr=subprocess.STDOUT,creationflags=subprocess.CREATE_NO_WINDOW)
    h=k.OpenProcess(0x1000|0x10,False,proc.pid)
    if not h:raise RuntimeError('cannot open owned diagnostic process')
    try:
     while proc.poll() is None:
      c=Counters();c.cb=ctypes.sizeof(c)
      if ps.GetProcessMemoryInfo(h,ctypes.byref(c),ctypes.sizeof(c)):peak=max(peak,c.WorkingSetSize);ospeak=max(ospeak,c.PeakWorkingSetSize)
      if peak>4*1024**3:reason='rss_limit_4_GiB';proc.kill()
      elif time.monotonic()-start>180:reason='timeout_180s';proc.kill()
      time.sleep(.02)
     code=proc.wait()
    finally:k.CloseHandle(h)
   detail=json.loads(output.read_text()) if output.exists() else None
   d={'version':version,'n':n,'mode':mode,'exit_code':code,'reason':reason,'sampled_peak_rss_bytes':peak,'os_peak_working_set_bytes':ospeak,'seconds':round(time.monotonic()-start,3),'addon_sha256':hashlib.sha256(addon.read_bytes()).hexdigest(),'detail':detail}
   results.append(d);(r/'proof-explicit.json').write_text(json.dumps(results,indent=2));print(json.dumps(d),flush=True)
(r/'proof-explicit.status').write_text('complete')
