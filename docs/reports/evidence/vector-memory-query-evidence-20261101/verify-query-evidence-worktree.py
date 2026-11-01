from pathlib import Path
import hashlib,json,subprocess
r=Path(__file__).parent; d=Path(r'D:\GenesisBlock_Dev\GenesisBlock')
q=json.loads((r/'qualification-receipt.json').read_text()); pre=json.loads((r/'audit-commit-preflight.json').read_text())
bad=[]
for path, expected in q['full_source_manifest'].items():
    if path=='benches/vector_memory.rs': continue
    with (d/path).open('rb') as f: actual=hashlib.file_digest(f,'sha256').hexdigest()
    if actual!=expected: bad.append(path)
assert not bad,bad
staged=subprocess.check_output(['git','-c','safe.directory=D:/GenesisBlock_Dev/GenesisBlock','diff','--cached','--binary'],cwd=d)
assert hashlib.sha256(staged).hexdigest()==pre['protected_patch_sha256']
manifest=json.loads((r/'query-evidence-build-manifest.json').read_text())
for name,key in [('vector_memory.rs','bench'),('query_evidence.rs','helper')]:
    with (d/'benches'/name).open('rb') as f: assert hashlib.file_digest(f,'sha256').hexdigest()==manifest[key]
out={'qualified_files_unchanged':len(q['full_source_manifest'])-1,'explicit_harness_files_match_build':True,'protected_staged_patch_unchanged':True,'engine_checkpoint':'f98a324c6ac9c16bbd98679622012519107628a3'}
(r/'query-evidence-source-check.json').write_text(json.dumps(out,indent=2)+'\n'); print(json.dumps(out))
