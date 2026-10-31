from pathlib import Path
import subprocess,json,hashlib,datetime
r=Path(__file__).parent;d=Path(r'D:\GenesisBlock_Dev\GenesisBlock')
def git(*args):return subprocess.check_output(['git','-c','safe.directory=D:/GenesisBlock_Dev/GenesisBlock',*args],cwd=d)
p=json.loads((r/'audit-commit-preflight.json').read_text());head=git('rev-parse','HEAD').decode().strip()
assert head=='f98a324c6ac9c16bbd98679622012519107628a3'
assert git('rev-parse','HEAD^').decode().strip()==p['parent']
staged=git('diff','--cached','--name-only').decode().splitlines();assert sorted(staged)==sorted(p['protected_staged'])
assert hashlib.sha256(git('diff','--cached','--binary')).hexdigest()==p['protected_patch_sha256']
assert not git('diff','--name-only').strip(),'Unexpected unstaged change immediately after checkpoint commit'
changed=git('diff-tree','--no-commit-id','--name-only','-r','HEAD').decode().splitlines()
assert len(changed)==1742 and not set(changed)&set(p['protected_staged'])
raw_equal=0;eol_only=[];excluded=[]
for path,h in p['source_manifest'].items():
 data=(d/path).read_bytes();assert hashlib.sha256(data).hexdigest()==h,path
 committed=git('show','HEAD:'+path)
 if path in p['protected_staged']:
  excluded.append(path);continue
 if committed==data:raw_equal+=1
 else:
  assert committed.replace(b'\r\n',b'\n')==data.replace(b'\r\n',b'\n'),path
  eol_only.append(path)
out={'recorded_at':datetime.datetime.now().astimezone().isoformat(),'passed':True,'commit':head,'parent':p['parent'],
 'commit_paths':len(changed),'protected_staged_paths_unchanged':staged,'protected_patch_sha256':p['protected_patch_sha256'],
 'qualified_current_source_paths_unchanged':len(p['source_manifest']),'commit_source_raw_equal':raw_equal,'commit_source_eol_only':eol_only,
 'excluded_preexisting_staged_source_paths':excluded,'qualification_scope':p['validation_scope'],
 'no_push_or_deployment':True}
(r/'audit-commit-receipt.json').write_text(json.dumps(out,indent=2)+'\n')
print(json.dumps({k:out[k] for k in ('passed','commit','commit_paths','qualified_current_source_paths_unchanged','commit_source_raw_equal','commit_source_eol_only','excluded_preexisting_staged_source_paths')}))
