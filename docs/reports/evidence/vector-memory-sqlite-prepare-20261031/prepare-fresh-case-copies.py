from pathlib import Path
import runpy,json,shutil,hashlib,sys
r=Path(__file__).parent
api=runpy.run_path(str(r/'prepare-100k-cases.py'))
source=r/'100k-fixture-preparation/gdb';cases=r/'100k-fresh-cases'
prep=json.loads((r/'100k-fixture-preparation/result.json').read_text());assert prep['state']=='fixture_prepared' and prep['queries_executed']==0
receipt=json.loads((r/'100k-fixture-preparation/run-receipt.json').read_text());assert receipt['input_unchanged'] and receipt['engine_exit']==0
wal=source/'projection.sqlite-wal';assert not wal.exists() or wal.stat().st_size==0
assert shutil.disk_usage(r).free>=50*1024**3,'Below original copy free-space threshold'
assert not cases.exists();cases.mkdir()
first=api['tree_manifest'](source)
copies=[]
for name in ('before','after'):
 case=cases/name;case.mkdir();dest=api['require_under'](case/'gdb',r)
 shutil.copytree(source,dest,copy_function=shutil.copy2)
 copies.append(api['tree_manifest'](dest))
last=api['tree_manifest'](source)
assert first==last==copies[0]==copies[1]
manifest={'source_db':str(source),'source':first,'source_after':last,'before':copies[0],'after':copies[1],
 'marker_id':prep['marker_id'],'copies_equal':True,'source_matches_both_copies':True,'sqlite_opened_by_copy':False,
 'fixture_preparation_queries':0,'preparation_guard':'fixture-preparation-guard/status.json','full_two_open_result':'TIMEOUT retained separately'}
api['write_json'](cases/'copy-manifest.json',manifest)
for mode in ('before','after'):
 binary=r/f'fresh-{mode}.exe'
 with binary.open('rb') as f:digest=hashlib.file_digest(f,'sha256').hexdigest()
 cfg={'mode':mode,'case_path':str(cases/mode),'binary_path':str(binary),'binary_sha256':digest,
 'input_path':r'C:\Users\suanr\genesis-vector-memory-20261031\inputs\synthetic-100000-1024'}
 p=r/f'fresh-{mode}-config.json';assert not p.exists();p.write_text(json.dumps(cfg,indent=2)+'\n')
 guard={'kind':'benchmark','cap_seconds':1200,'exe':sys.executable,'args':['-u',str(r/'run-fresh-workload.py'),str(p)],'cwd':str(r),
 'output':str(r/f'fresh-{mode}-guard'),'env':{}}
 (r/f'fresh-{mode}-guard-config.json').write_text(json.dumps(guard,indent=2)+'\n')
print(json.dumps({'copies_equal':True,'bytes_each':first['byte_count'],'files_each':first['file_count'],'source_unchanged':True,'sqlite_opened':False}))
