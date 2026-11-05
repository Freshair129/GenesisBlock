from pathlib import Path
import json,os,sys,hashlib,subprocess,time,datetime
r=Path(__file__).parent
cfg=json.loads(Path(sys.argv[1]).read_text(encoding='utf-8-sig'))
case=Path(cfg['case_path']).resolve();case.relative_to(r.resolve())
binary=Path(cfg['binary_path']);inp=Path(cfg['input_path'])
def sha(p):
 with p.open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
assert sha(binary)==cfg['binary_sha256'],'Frozen binary changed'
assert (case/'gdb/projection.sqlite').is_file()
mode=cfg['mode'];assert mode in ('prepare','before','after')
for n in ('result.json','run-receipt.json','stdout.log','stderr.log'):assert not (case/n).exists(),f'Existing run artifact {n}'
def hashes():return {n:sha(inp/n) for n in ('meta.json','corpus.f32','queries.f32','truth.json')}
meta=json.loads((inp/'meta.json').read_text());assert meta['n']==100000 and meta['dim']==1024
before=hashes()
env=os.environ.copy()
for k in list(env):
 if k.upper().startswith('GB_') or k.upper() in ('RETENTION','RAYON_NUM_THREADS'):env.pop(k)
profile={'GB_VBENCH':str(inp),'GB_MEMORY_AUDIT':'1','GB_MEMORY_INPUT':str(inp),'GB_MEMORY_OPEN_ONLY':str(case/'gdb'),
 'GB_MEMORY_OPEN_OUT':str(case/'result.json'),'GB_MEMORY_OPEN_READ_WRITE':'1','GB_MEMORY_OPEN_REPEAT':'1',
 'GB_MEMORY_OPEN_QUERY_SAMPLES':'24','GB_MEMORY_OPEN_MUTATE_BEFORE_REOPEN':'1','GB_VECTOR_QUERY_DIAGNOSTICS':'1',
 'GB_EF':'200','GB_QUANT':'bq','GB_RERANK':'0','GB_VEC_Q':'24','GB_MEMORY_RETENTION':'full','RETENTION':'full',
 'RAYON_NUM_THREADS':'4','GB_MEMORY_RSS_LIMIT_BYTES':str(2*1024**3)}
if mode=='prepare':profile['GB_MEMORY_PREPARE_MARKER_ONLY']='1'
else:
 fixture=json.loads((r/'100k-fixture-preparation-nov01/result.json').read_text());assert fixture['state']=='fixture_prepared' and fixture['queries_executed']==0
 profile['GB_MEMORY_PREPARED_MARKER_ID']=fixture['marker_id']
 manifest=json.loads((r/'100k-fresh-cases/copy-manifest.json').read_text())
 assert manifest['copies_equal'] and manifest['source_matches_both_copies']
env.update(profile)
receipt={'mode':mode,'started':datetime.datetime.now().astimezone().isoformat(),'binary':str(binary),'binary_sha256':cfg['binary_sha256'],
 'production_commit':'f98a324c6ac9c16bbd98679622012519107628a3','environment':profile,'input_before':before,'input_after':None,
 'method':'Separate setup or fresh-process RW reopen; no query warmup; parent retains1200s/2GiB; query5000ms',
 'not_original_two_open_qualification':True,'query_or_quality_qualification_claimed':False}
def save():(case/'run-receipt.json').write_text(json.dumps(receipt,indent=2)+'\n')
save();started=time.monotonic()
with (case/'stdout.log').open('wb',buffering=0) as out,(case/'stderr.log').open('wb',buffering=0) as err:
 p=subprocess.Popen([str(binary)],cwd=case,env=env,stdin=subprocess.DEVNULL,stdout=out,stderr=err,shell=False)
 receipt['engine_pid']=p.pid;save();code=p.wait()
after=hashes();receipt.update(input_after=after,input_unchanged=(before==after),engine_exit=code,seconds=time.monotonic()-started)
result=json.loads((case/'result.json').read_text()) if (case/'result.json').exists() else {}
receipt['engine_state']=result.get('state');receipt['query0']=result.get('query0');save()
assert before==after,'Frozen input changed'
assert code==0,code
if mode=='prepare':assert result.get('state')=='fixture_prepared' and result.get('queries_executed')==0
print(json.dumps({'mode':mode,'engine_exit':code,'state':result.get('state'),'seconds':receipt['seconds'],'input_unchanged':True}))
