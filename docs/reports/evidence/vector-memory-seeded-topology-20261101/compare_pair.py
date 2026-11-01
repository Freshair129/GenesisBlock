import hashlib,json,sys
from pathlib import Path
from compare_topology import compare

r=Path(__file__).parent
def digest(path):
    return hashlib.file_digest(path.open('rb'),'sha256').hexdigest()
def load(label):
    case=r/'runs'/label
    cfg=json.loads((case/'run-config.json').read_text(encoding='utf-8-sig'))
    guard=json.loads((r/'runs'/(label+'-guard')/'status.json').read_text(encoding='utf-8-sig'))
    driver=json.loads((case/'driver-status.json').read_text(encoding='utf-8-sig'))
    if guard['exit'] != 0 or guard['reason'] or not guard['cleanup_complete'] or not driver['input_unchanged']:
        raise ValueError('failed guard or input identity')
    data=json.loads((case/'output/memory.json').read_text())
    graphs={}
    for phase in ('initial','reopen'):
        found=list((case/'topology').glob('*-'+phase+'.txt'))
        if len(found)!=1: raise ValueError('missing or duplicate graph for '+phase)
        graphs[phase]=found[0]
    return cfg,guard,data,graphs

def analyze(before,after):
    bc,bg,bd,bt=load(before); ac,ag,ad,at=load(after)
    if bc['extra_env']['GB_MEMORY_HNSW_SEED'] != ac['extra_env']['GB_MEMORY_HNSW_SEED']:
        raise ValueError('seed mismatch')
    if bc['expected_input_hashes'] != ac['expected_input_hashes']: raise ValueError('input mismatch')
    if bd['n'] != ad['n'] or bd['q'] != ad['q']: raise ValueError('workload mismatch')
    result={'seed':bc['extra_env']['GB_MEMORY_HNSW_SEED'],'before':before,'after':after,'graphs':{},'queries':{},'rss_bytes':{'before':bg['peak_rss'],'after':ag['peak_rss']}}
    for phase in ('initial','reopen'):
        graph=compare(bt[phase],at[phase])
        if graph['before_nodes'] != bd['n'] or graph['after_nodes'] != ad['n']: raise ValueError('graph count mismatch')
        result['graphs'][phase]=graph
    for sweep in ('search10','search100','reopen_search10'):
        b=bd[sweep];a=ad[sweep]
        bq=b['per_query']; aq=a['per_query']
        zipped=list(zip(bq,aq))
        for x,y in zipped:
            if x['query_index'] != y['query_index'] or x['truth_ids'] != y['truth_ids']:
                raise ValueError('query identity or truth mismatch')
        result['queries'][sweep]={'before_error':b.get('error'),'after_error':a.get('error'),
           'before_recall':b.get('recall'),'after_recall':a.get('recall'),
           'before_completed':len([q for q in bq if not q['error']]),'after_completed':len([q for q in aq if not q['error']]),
           'both_complete':len(bq)==bd['q'] and len(aq)==ad['q'] and not b.get('error') and not a.get('error'),
           'before_p50_us':b.get('p50_us'),'before_p95_us':b.get('p95_us'),'before_p99_us':b.get('p99_us'),
           'after_p50_us':a.get('p50_us'),'after_p95_us':a.get('p95_us'),'after_p99_us':a.get('p99_us'),
           'first_query_elapsed_us':{'before':bq[0]['elapsed_us'] if bq else None,'after':aq[0]['elapsed_us'] if aq else None},
           'different_returned_indices':[x['query_index'] for x,y in zipped if x['returned_ids']!=y['returned_ids']],
           'different_hit_indices':[x['query_index'] for x,y in zipped if x['hit_count']!=y['hit_count']]}
    return result

if __name__=='__main__':
    print(json.dumps(analyze(*sys.argv[1:]),indent=2))
