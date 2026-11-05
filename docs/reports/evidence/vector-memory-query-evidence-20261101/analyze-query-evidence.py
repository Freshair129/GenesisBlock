from pathlib import Path
import json,sys
r=Path(__file__).parent/'query-evidence-runs'; label=sys.argv[1]
def load(p): return json.loads(p.read_text(encoding='utf-8-sig'))
out={}
for mode in ('before','after'):
    p=r/f'{label}-{mode}'; g=load(r/f'{label}-{mode}-guard/status.json'); receipt=load(p/'driver-status.json'); cfg=load(p/'run-config.json'); d=load(p/'output/memory.json')
    assert g['exit']==0 and g['reason'] is None and g['cleanup_complete'] and not g['remaining']
    assert g['seconds']<1200 and g['peak_rss']<2*1024**3
    assert receipt['exit_code']==0 and receipt['input_unchanged'] and receipt['input_hashes_before']==receipt['input_hashes_after']==cfg['expected_input_hashes']
    assert d['n']==11266 and d['dim']==1024 and d['q']==24 and d['fresh_reopen']['error'] is None
    assert d['snapshot_error'] is None and d['reopen_error'] is None
    for cov in (d['coverage'],d['fresh_reopen']['coverage']):
        c=next(x for x in cov if x['collection']=='bench'); assert c['state']=='READY' and c['validated'] and c['source_count']==c['indexed_count']==11266 and c['missing_count']==c['extra_count']==c['pending_count']==0
    for key in ('search10','search100','reopen_search10'):
        v=d[key]; records=v['per_query']; assert records and len(records)<=24
        for i,q in enumerate(records):
            assert q['query_index']==i and q['elapsed_us']>=0
            if q['error'] is None:
                assert len(q['returned_ids'])==v['k'] and len(set(q['returned_ids']))==v['k']
                assert q['hit_count']==len(set(q['returned_ids']) & set(q['truth_ids']))
            else: assert i==len(records)-1 and q['hit_count'] is None and not q['returned_ids']
        if 'error' in v: assert records[-1]['error']==v['error'] and v['completed_queries']==len(records)-1
        else: assert len(records)==24 and abs(sum(q['hit_count'] for q in records)/(24*v['k'])-v['recall'])<1e-12
    out[mode]={'guard':{k:g[k] for k in ('seconds','peak_rss','admission_cpu','admission_free')},'input_hashes':receipt['input_hashes_before'],'binary':receipt['binary_sha256'],'search':{k:d[k] for k in ('search10','search100','reopen_search10')}}
assert out['before']['input_hashes']==out['after']['input_hashes']
diffs={}
for key in ('search10','search100','reopen_search10'):
    b=out['before']['search'][key]['per_query']; a=out['after']['search'][key]['per_query']; rows=[]
    for x,y in zip(b,a):
        assert x['query_index']==y['query_index'] and x['truth_ids']==y['truth_ids']
        if x['error'] is None and y['error'] is None:
            if x['hit_count']!=y['hit_count'] or x['returned_ids']!=y['returned_ids']:
                rows.append({'query_index':x['query_index'],'before_hits':x['hit_count'],'after_hits':y['hit_count'],
                             'before_only':sorted(set(x['returned_ids'])-set(y['returned_ids'])),
                             'after_only':sorted(set(y['returned_ids'])-set(x['returned_ids']))})
        else: rows.append({'query_index':x['query_index'],'before_error':x['error'],'after_error':y['error']})
    diffs[key]=rows
out['differences']=diffs
dest=r/f'{label}-analysis.json'; assert not dest.exists(); dest.write_text(json.dumps(out,indent=2)+'\n')
print(json.dumps({m:{'rss':out[m]['guard']['peak_rss'],**{k:{z:v for z,v in out[m]['search'][k].items() if z!='per_query'} for k in out[m]['search']}} for m in ('before','after')},indent=2))
print(json.dumps({'differences':diffs},indent=2))
