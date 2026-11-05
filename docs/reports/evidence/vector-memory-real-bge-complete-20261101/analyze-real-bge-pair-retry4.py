from pathlib import Path
import json
r = Path(__file__).parent
result = {}
hashes = None
for name, case, guard in [('before','real-pair/before-retry4','real-before-retry4-guard'),
                          ('after','real-pair/after','real-after-guard')]:
    load = lambda p: json.loads(p.read_text(encoding='utf-8-sig'))
    g = load(r/guard/'status.json'); receipt = load(r/case/'driver-status.json')
    data = load(r/case/'output/memory.json'); cfg = load(r/case/'run-config.json')
    assert g['exit']==0 and g['reason'] is None and g['cleanup_complete'] and not g['remaining']
    assert g['seconds'] < 1200 and g['peak_rss'] < 2*1024**3
    assert receipt['exit_code']==0 and receipt['input_unchanged']
    assert receipt['input_hashes_before']==receipt['input_hashes_after']==cfg['expected_input_hashes']
    if hashes is None: hashes=receipt['input_hashes_before']
    else: assert hashes==receipt['input_hashes_before']
    assert data['n']==11266 and data['dim']==1024 and data['q']==24
    assert data['snapshot_error'] is None and data['reopen_error'] is None
    assert data['fresh_reopen']['error'] is None
    for cov in (data['coverage'],data['fresh_reopen']['coverage']):
        bench=next(c for c in cov if c['collection']=='bench')
        assert bench['state']=='READY' and bench['validated'] and bench['source_count']==bench['indexed_count']==11266
        assert bench['missing_count']==bench['extra_count']==bench['pending_count']==0
    for key in ('search10','search100','reopen_search10'):
        assert 'error' not in data[key] and data[key]['recall'] is not None
    result[name]={'guard':{k:g[k] for k in ('seconds','peak_rss','admission_cpu','admission_free','cleanup_complete')},
                  'driver_binary_sha256':receipt['binary_sha256'],
                  'metrics':{k:data[k] for k in ('search10','search100','reopen_search10','ingest','rebuild','snapshot','reopen','fresh_reopen')},
                  'input_unchanged':True,'coverage_validated':True}
result['scope']='One serial pair; 24 queries per sweep; fresh child validates open and coverage, not queries. No statistical non-regression inference.'
result['input_hashes']=hashes
out=r/'real-bge-pair-retry4-analysis.json'; assert not out.exists()
out.write_text(json.dumps(result,indent=2)+'\n',encoding='utf-8')
print(json.dumps({name:{'rss':result[name]['guard']['peak_rss'],'search10':result[name]['metrics']['search10'],
                         'search100':result[name]['metrics']['search100'],'reopen_search10':result[name]['metrics']['reopen_search10']} for name in ('before','after')},indent=2))
