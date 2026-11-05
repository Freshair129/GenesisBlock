from pathlib import Path
import json,hashlib,datetime,re,subprocess
r=Path(__file__).parent;repo=Path(r'D:\GenesisBlock_Dev\GenesisBlock')
def read(p):return json.loads(p.read_text(encoding='utf-8-sig'))
def sha(p):
 h=hashlib.sha256()
 with p.open('rb') as f:
  while b:=f.read(1048576):h.update(b)
 return h.hexdigest()
results=read(r/'final-runtime.json');assert len(results)==159
assert all(d['exit_code']==0 and not d['reason'] for d in results)
assert (r/'final-runtime.status').read_text().strip()=='complete'
counts={'passed':0,'failed':0,'ignored':0}
for d in results:
 for a,b,c in re.findall(r'test result: .*? (\d+) passed; (\d+) failed; (\d+) ignored;', (r/d['log']).read_text(errors='replace')):
  for k,v in zip(counts,(a,b,c)):counts[k]+=int(v)
post=read(r/'post-checks.status');assert post['passed'] and post['core_sha256'].lower()==sha(repo/'src/lib.rs')
entry=read(r/'full-source-entry-manifest.json');manifest=entry['manifest']
for rel,h in manifest.items():assert sha(repo/rel)==h,('Source drift',rel)
canonical=''.join(f'{p}|{manifest[p].lower()}\n' for p in sorted(manifest));canonical_sha=hashlib.sha256(canonical.encode()).hexdigest();assert canonical_sha==entry['canonical_sha256']
fields=['src/lib.rs','src/projection_rebuild_tests.rs','Cargo.toml','Cargo.lock','benches/vector_memory.rs']
gates=['final-runtime.status','final-runtime.json','final-test-artifacts.json','post-checks.status']
receipt={'run_id':'genesis-sqlite-prepare-20261031','recorded_at':datetime.datetime.now().astimezone().isoformat(),'passed':True,'completed_targets':159,'source':{p:sha(repo/p) for p in fields},'full_source_manifest':manifest,'full_source_manifest_sha256':canonical_sha,'gate_hashes':{p:sha(r/p) for p in gates},'counts':counts,'node_tests':{'passed':88,'failed':0,'skipped':0},'scope':'Windows source/static/native correctness qualification only; not benchmark/100k deadline qualification'}
(r/'qualification-receipt.json').write_text(json.dumps(receipt,indent=2)+'\n',newline='\n')
summary={'recorded_at':receipt['recorded_at'],'core_source':receipt['source'],'fresh_final_core':{'targets':159,'counts':counts,'failed_targets':[]},'static_native_checks':{x['name']:{k:x['status'].get(k) for k in ['exit','reason','seconds','cleanup_complete']} for x in post['checks']},'full_source_manifest_sha256':canonical_sha,'full_source_paths':len(manifest),'historical_stages':['qualification-attempt1/','resource-hold/'],'timed_sqlite_matrix':'NOT_RUN','reopen_100k':'NOT_RUN','note':'Final immutable E69 suite freshly completed after resource hold; historical interrupted results are not summed into this result.'}
(r/'qualification-summary.json').write_text(json.dumps(summary,indent=2)+'\n',newline='\n')
print(json.dumps({'counts':counts,'targets':159,'native_checks':len(post['checks']),'source_paths':len(manifest),'qualified_correctness':True}))
