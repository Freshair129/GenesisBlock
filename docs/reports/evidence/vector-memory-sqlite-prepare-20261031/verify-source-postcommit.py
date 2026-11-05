from pathlib import Path
import json,hashlib,subprocess,datetime
r=Path(__file__).parent;d=Path(r'D:\GenesisBlock_Dev\GenesisBlock');c=r.parent/'genesis-vector-memory-20261031/verify-src'
def git(*args):return subprocess.check_output(['git','-c','safe.directory=D:/GenesisBlock_Dev/GenesisBlock',*args],cwd=d)
bridge=json.loads((r/'audit-commit-receipt.json').read_text());pre=json.loads((r/'audit-commit-preflight.json').read_text())
head=git('rev-parse','HEAD').decode().strip()
if head!=bridge['commit']:
 subprocess.run(['git','-c','safe.directory=D:/GenesisBlock_Dev/GenesisBlock','merge-base','--is-ancestor',bridge['commit'],head],cwd=d,check=True)
 followup=git('diff','--name-only',bridge['commit'],head).decode().splitlines()
 assert followup and all(p.startswith('docs/reports/') for p in followup),'Unexpected runtime change after source checkpoint'
assert hashlib.sha256(git('diff','--cached','--binary')).hexdigest()==pre['protected_patch_sha256']
q=json.loads((r/'qualification-receipt.json').read_text());bad=[]
for p,h in q['full_source_manifest'].items():
 with (d/p).open('rb') as f:actual=hashlib.file_digest(f,'sha256').hexdigest()
 if actual!=h:bad.append(p)
assert not bad,bad
mirror=json.loads((r/'verify-source-manifest.json').read_text());mismatch=[]
for p,h in mirror.items():
 with (c/p).open('rb') as f:actual=hashlib.file_digest(f,'sha256').hexdigest()
 if actual!=h:mismatch.append(p)
assert not mismatch,mismatch
out={'at':datetime.datetime.now().astimezone().isoformat(),'passed':True,'commit':head,'runtime_source_checkpoint':bridge['commit'],'qualified_paths':len(q['full_source_manifest']),
 'mirror_paths':len(mirror),'protected_staged_patch_unchanged':True,'protected_staged_files':7,
 'scope':'Runtime/source identity bridge after authorized checkpoint commit; no new test execution claim; report/evidence may evolve separately.'}
(r/'verify-source-postcommit-receipt.json').write_text(json.dumps(out,indent=2)+'\n');print(json.dumps(out))
