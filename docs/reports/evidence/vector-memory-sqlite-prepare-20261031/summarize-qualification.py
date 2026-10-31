from pathlib import Path
import json,re,hashlib,datetime
r=Path(__file__).parent
counts={'passed':0,'failed':0,'ignored':0}
for p in (r/'qualification-attempt1/final-runtime').glob('*.log'):
 for a,b,c in re.findall(r'test result: .*? (\d+) passed; (\d+) failed; (\d+) ignored;',p.read_text(errors='replace')):
  for k,v in zip(counts,(a,b,c)):counts[k]+=int(v)
prior=json.loads((r/'qualification-attempt1/final-runtime.json').read_text(encoding='utf-8-sig'))
partial=json.loads((r/'resource-hold/partial-runtime.json').read_text(encoding='utf-8-sig'))
src=Path(r'D:\GenesisBlock_Dev\GenesisBlock')
files=['src/lib.rs','src/projection_rebuild_tests.rs','Cargo.toml','Cargo.lock','benches/vector_memory.rs']
source={p:hashlib.sha256((src/p).read_bytes()).hexdigest() for p in files}
unit=[]
for p in (r/'final-runtime').glob('genesis_block_native*.log'):
 unit.append({'log':str(p.relative_to(r)),'result':re.findall(r'test result: .*',p.read_text(errors='replace'))})
summary={'recorded_at':datetime.datetime.now().astimezone().isoformat(),'production_core_sha256':source['src/lib.rs'],'prelint_test_sha256':'40c4a131c4137304764b38a61d5d4a1482893796fd5e032d8292653a966cd342','prelint_full_suite':{'targets':len(prior),'counts':counts,'failed_targets':[x for x in prior if x['exit_code'] or x['reason']]},'final_test_sha256':source['src/projection_rebuild_tests.rs'],'final_partial_suite':{'completed_targets':len(partial),'failed_targets':[x for x in partial if x['exit_code'] or x['reason']],'qualified':False,'reason':'resource_hold; interrupted current hql_cypher target excluded'},'final_unit_results':unit,'timed_reproducer':'NOT_RUN; host CPU admission rejected','reopen_100k':'NOT_RUN; host CPU admission rejected','strict_clippy':'prelint failure preserved; literal fixed; final rerun pending resource window','node_napi':'final source not rebuilt/run yet'}
(r/'qualification-summary.json').write_text(json.dumps(summary,indent=2)+'\n',newline='\n')
receipt={'run_id':'genesis-sqlite-prepare-20261031','recorded_at':summary['recorded_at'],'passed':False,'completed_targets':len(partial),'source':source,'reason':'Final immutable E69 suite interrupted by external resource pressure; final static/native and benchmark gates open; cannot authorize release-variant build'}
(r/'qualification-receipt.json').write_text(json.dumps(receipt,indent=2)+'\n',newline='\n')
(r/'post-checks-after-lint.status').write_text(json.dumps({'passed':False,'reason':'NOT_RUN after literal fix; external compiler/resource gate','historical_failure':'qualification-attempt1/post-checks.status','source':source},indent=2)+'\n',newline='\n')
print(json.dumps(summary,indent=2))
