from pathlib import Path
import json,hashlib,sys
r=Path(__file__).parent;binary=r/'fresh-after.exe'
with binary.open('rb') as f:sha=hashlib.file_digest(f,'sha256').hexdigest()
cfg={'mode':'prepare','case_path':str(r/'100k-fixture-preparation'),'binary_path':str(binary),'binary_sha256':sha,
 'input_path':r'C:\Users\suanr\genesis-vector-memory-20261031\inputs\synthetic-100000-1024'}
p=r/'fixture-preparation-config.json';assert not p.exists();p.write_text(json.dumps(cfg,indent=2)+'\n')
guard={'kind':'benchmark','cap_seconds':1200,'exe':sys.executable,'args':['-u',str(r/'run-fresh-workload.py'),str(p)],'cwd':str(r),
 'output':str(r/'fixture-preparation-guard'),'env':{}}
(r/'fixture-preparation-guard-config.json').write_text(json.dumps(guard,indent=2)+'\n')
print('Frozen fixture preparation config written; not launched.')
