from pathlib import Path
import hashlib, json
r=Path(__file__).parent
def load(p): return json.loads(p.read_text(encoding='utf-8-sig'))
out={}
for mode in ('before','after'):
    p=r/'real-repeat'/f'round2-{mode}'
    guard=load(r/'real-repeat'/f'round2-{mode}-guard'/'status.json')
    receipt=load(p/'driver-status.json'); cfg=load(p/'run-config.json'); data=load(p/'output/memory.json')
    assert guard['exit']==0 and guard['reason'] is None and guard['cleanup_complete'] and not guard['remaining']
    assert guard['seconds']<1200 and guard['peak_rss']<2*1024**3
    assert receipt['exit_code']==0 and receipt['input_unchanged']
    assert receipt['input_hashes_before']==receipt['input_hashes_after']==cfg['expected_input_hashes']
    assert data['n']==11266 and data['dim']==1024 and data['q']==24
    assert data['snapshot_error'] is None and data['reopen_error'] is None and data['fresh_reopen']['error'] is None
    for cov in (data['coverage'],data['fresh_reopen']['coverage']):
        b=next(x for x in cov if x['collection']=='bench')
        assert b['state']=='READY' and b['validated'] and b['source_count']==b['indexed_count']==11266
        assert b['missing_count']==b['extra_count']==b['pending_count']==0
    vec=p/'output/gdb/vec_bench.bin'
    digest=hashlib.file_digest(vec.open('rb'),'sha256').hexdigest()
    out[mode]={'guard':{k:guard[k] for k in ('seconds','peak_rss','admission_cpu','admission_free','cleanup_complete')},
      'input_hashes':receipt['input_hashes_before'],'vec_sha256':digest,
      'metrics':{k:data[k] for k in ('search10','search100','reopen_search10','ingest','rebuild','snapshot','reopen','fresh_reopen')}}
assert out['before']['input_hashes']==out['after']['input_hashes']
assert out['before']['vec_sha256']==out['after']['vec_sha256']
out['scope']='Single additional guarded serial pair; baseline reopen query deadline error retained; guard exit 0 is not correctness pass.'
dest=r/'real-repeat-analysis.json'; assert not dest.exists()
dest.write_text(json.dumps(out,indent=2)+'\n',encoding='utf-8')
print(json.dumps({m:{'guard':out[m]['guard'],'vec':out[m]['vec_sha256'],**{k:out[m]['metrics'][k] for k in ('search10','search100','reopen_search10')}} for m in ('before','after')},indent=2))
