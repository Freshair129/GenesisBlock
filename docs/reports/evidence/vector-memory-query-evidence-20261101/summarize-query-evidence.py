from pathlib import Path
import hashlib,json
r=Path(__file__).parent/'query-evidence-runs'
pairs=[json.loads((r/f'pair{i}-analysis.json').read_text()) for i in range(1,4)]
expected=None; hashes={}; summary=[]
for i,d in enumerate(pairs,1):
    row={'pair':i}
    for mode in ('before','after'):
        run=r/f'pair{i}-{mode}'
        with (run/'output/gdb/vec_bench.bin').open('rb') as f: vec=hashlib.file_digest(f,'sha256').hexdigest()
        if expected is None: expected=vec
        assert expected==vec
        s=d[mode]['search']; hashes[f'{i}-{mode}']=d[mode]['input_hashes']
        row[mode]={'rss':d[mode]['guard']['peak_rss'],'search10_recall':s['search10']['recall'],
          'search100_recall':s['search100']['recall'],'reopen_query0_ms':s['reopen_search10']['per_query'][0]['elapsed_us']/1000,
          'reopen_error':s['reopen_search10'].get('error'),
          'search10_p_ms':[s['search10'][k]/1000 for k in ('p50_us','p95_us','p99_us')],
          'search100_p_ms':[s['search100'][k]/1000 for k in ('p50_us','p95_us','p99_us')]}
    summary.append(row)
assert all(h==next(iter(hashes.values())) for h in hashes.values())
variation={}
for mode in ('before','after'):
    variation[mode]=[{'query':i,'hits':[d[mode]['search']['search10']['per_query'][i]['hit_count'] for d in pairs]}
      for i in range(24) if len({d[mode]['search']['search10']['per_query'][i]['hit_count'] for d in pairs})>1]
out={'pairs':summary,'same_vector_sha256':expected,'same_inputs':True,'within_variant_query_hit_variation':variation,
     'scope':'Three serial guarded pairs, independent random HNSW graphs; 24 queries per sweep; no deterministic causality or nonregression claim.'}
(r/'summary.json').write_text(json.dumps(out,indent=2)+'\n',encoding='utf-8')
print(json.dumps(out,indent=2))
