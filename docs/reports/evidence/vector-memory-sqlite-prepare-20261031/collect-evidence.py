from pathlib import Path
import json,hashlib,gzip,shutil,os,datetime
r=Path(__file__).parent
out=Path(r'D:\GenesisBlock_Dev\GenesisBlock\docs\reports\evidence\vector-memory-sqlite-prepare-20261031')
out.mkdir(parents=True,exist_ok=True)
prune={'reproducer-target','release-target','release-src','final-test-bin','100k-cases','scratch','gdb','db','database','napi-smoke-scratch','output','restore-backups'}
allow={'.rs','.toml','.lock','.py','.ps1','.cjs','.json','.jsonl','.log','.status','.err','.patch','.txt','.md'}
def sha(p):
 h=hashlib.sha256()
 with p.open('rb') as f:
  while b:=f.read(1048576):h.update(b)
 return h.hexdigest()
old_path=out/'artifact-manifest.json'
old={x['source']:x for x in json.loads(old_path.read_text()).get('files',[])} if old_path.exists() else {}
items=[]
for directory,dirs,files in os.walk(r):
 dirs[:]=[x for x in dirs if x not in prune and not x.startswith(('napi-smoke-','release-builds-restore-'))]
 for name in files:
  p=Path(directory)/name
  if p.suffix.lower() not in allow or any(x in name.lower() for x in ('keypair','identity','state.json')):continue
  if p.name in ('proposed-publication-lib.rs','unrun-projection_rebuild_tests.rs','pre-audit-commit-staged.patch'):continue
  rel=p.relative_to(r); target=out/rel
  if p.suffix=='.md':target=target.with_suffix('.txt')
  size=p.stat().st_size
  source_sha=sha(p)
  if size>8*1024*1024:
   if p.suffix not in {'.log','.err','.jsonl'}:continue
   target=Path(str(target)+'.gz');target.parent.mkdir(parents=True,exist_ok=True)
   previous=old.get(str(rel).replace('\\','/'),{})
   if not (previous.get('sha256')==source_sha and target.is_file() and sha(target)==previous.get('export_sha256')):
    with p.open('rb') as src,gzip.open(target,'wb',compresslevel=6) as dst:shutil.copyfileobj(src,dst,1048576)
  else:
   target.parent.mkdir(parents=True,exist_ok=True)
   if not (target.is_file() and sha(target)==source_sha):shutil.copyfile(p,target)
  items.append({'source':str(rel).replace('\\','/'),'export':str(target.relative_to(out)).replace('\\','/'),'bytes':size,'sha256':source_sha,'export_sha256':sha(target)})
(out/'artifact-manifest.json').write_text(json.dumps({'recorded_at':datetime.datetime.now().astimezone().isoformat(),'files':items,'note':'Databases, keys and executable/target artifacts excluded. Large raw logs are exact gzip copies; source SHA checks decompressed bytes.'},indent=2)+'\n',newline='\n')
print(json.dumps({'files':len(items),'export_bytes':sum(p.stat().st_size for p in out.rglob('*') if p.is_file())}))
