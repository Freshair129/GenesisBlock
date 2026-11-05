from pathlib import Path
import hashlib,json,subprocess
r=Path(__file__).parent;d=Path(r'D:\GenesisBlock_Dev\GenesisBlock')
def git(*args):return subprocess.check_output(['git','-c','safe.directory=D:/GenesisBlock_Dev/GenesisBlock',*args],cwd=d)
x=json.loads((r/'audit-commit-preflight.json').read_text());protected=list(x['protected_staged'])
assert git('rev-parse','HEAD').decode().strip()==x['parent']
assert git('rev-parse','--show-object-format').decode().strip()=='sha1'
assert hashlib.sha256(git('diff','--cached','--binary','--',*protected)).hexdigest()==x['protected_patch_sha256']
expected=[]
for path in x['owned_paths']:
 p=d/path
 expected.extend(q.relative_to(d).as_posix() for q in (p.rglob('*') if p.is_dir() else [p]) if q.is_file())
staged=git('diff','--cached','--name-only','-z').decode().rstrip('\0').split('\0')
assert set(staged)==set(expected)|set(protected),(set(staged)-set(expected)-set(protected),set(expected)-set(staged))
entries={}
for raw in git('ls-files','--stage','-z','--','docs/reports/evidence').split(b'\0'):
 if not raw:continue
 meta,path=raw.split(b'\t',1);entries[path.decode()]=meta.split()[1].decode()
for path,oid in entries.items():
 b=(d/path).read_bytes();actual=hashlib.sha1(b'blob '+str(len(b)).encode()+b'\0'+b).hexdigest()
 assert actual==oid,('Evidence bytes changed by Git',path)
q=json.loads((r/'qualification-receipt.json').read_text())
for path,h in q['full_source_manifest'].items():assert hashlib.file_digest((d/path).open('rb'),'sha256').hexdigest()==h,path
out={'passed':True,'parent':x['parent'],'audit_changed_files':len(expected),'protected_staged_paths':len(protected),'evidence_blobs_byte_exact':len(entries),'qualified_source_paths_unchanged':len(q['full_source_manifest']),'protected_cached_patch_unchanged':True}
(r/'audit-index-verification.json').write_text(json.dumps(out,indent=2)+'\n')
print(json.dumps(out))
