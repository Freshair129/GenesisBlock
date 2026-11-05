from pathlib import Path
import subprocess,json,hashlib,datetime
r=Path(__file__).parent;d=Path(r'D:\GenesisBlock_Dev\GenesisBlock')
def git(*args):return subprocess.check_output(['git','-c','safe.directory=D:/GenesisBlock_Dev/GenesisBlock',*args],cwd=d)
q=json.loads((r/'qualification-receipt.json').read_text())
assert q['passed'] and q['completed_targets']==159
for p,h in q['full_source_manifest'].items():assert hashlib.file_digest((d/p).open('rb'),'sha256').hexdigest()==h,p
protected=json.loads((r.parent/'genesis-vector-memory-20261031/baseline-source.json').read_text())['preserved_staged']
staged=git('diff','--cached','--name-only').decode().splitlines();assert sorted(staged)==sorted(protected)
for p,h in protected.items():assert hashlib.file_digest((d/p).open('rb'),'sha256').hexdigest()==h,p
owned=['Cargo.toml','benches/vbench_genesis.rs','benches/vector_memory.rs','benches/scripts/vector_memory_matrix.ps1','index.d.ts','src/lib.rs','src/projection_rebuild_tests.rs','src/router.rs','src/vector_memory_tests.rs','tests/rest_api_tests.rs','tests/vector_memory_metrics_tests.rs','docs/reports']
assert not set(owned)&set(protected)
base_test=git('show','HEAD:tests/hql2_g4_scalar_counter_tests.rs').replace(b'\r\n',b'\n')
current_test=(d/'tests/hql2_g4_scalar_counter_tests.rs').read_bytes().replace(b'\r\n',b'\n')
assert current_test.startswith(base_test),'Pre-existing H2a tests must be append-only for source-coverage bridge'
patch=git('diff','--cached','--binary');(r/'pre-audit-commit-staged.patch').write_bytes(patch)
out={'recorded_at':datetime.datetime.now().astimezone().isoformat(),'authorization':'User: Commit แล้วทำต่อ','parent':git('rev-parse','HEAD').decode().strip(),'branch':git('branch','--show-current').decode().strip(),'owned_paths':owned,'protected_staged':protected,'protected_patch_sha256':hashlib.sha256(patch).hexdigest(),'qualified_manifest_sha256':q['full_source_manifest_sha256'],'source_manifest':q['full_source_manifest'],'validation_scope':'Existing source-bound working-tree qualification includes append-only pre-existing H2a tests; those7paths are excluded from audit commit. All original H2a tests remain unchanged as a prefix. No claim of fresh CI or exact-commit full-suite rerun.'}
(r/'audit-commit-preflight.json').write_text(json.dumps(out,indent=2)+'\n')
print(json.dumps({'parent':out['parent'],'owned_pathspecs':len(owned),'protected_staged_paths':len(protected),'full_source_paths_verified':len(q['full_source_manifest']),'h2a_original_tests_unchanged_prefix':True}))
