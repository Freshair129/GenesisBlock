from pathlib import Path
import os,json
r=Path(__file__).parent
prune={'release-target','release-src','reproducer-target','final-test-bin','100k-cases','scratch','gdb','db','database','output','restore-backups'}
found={}
def visit(x):
 if isinstance(x,dict):
  if isinstance(x.get('identities'),list):
   for i in x['identities']:
    if all(k in i for k in ('ProcessId','CreationDate','Name')):found[(i['ProcessId'],i['CreationDate'],i['Name'])]=i
  for y in x.values():visit(y)
 elif isinstance(x,list):
  for y in x:visit(y)
for directory,dirs,files in os.walk(r):
 dirs[:]=[x for x in dirs if x not in prune]
 for name in files:
  if name.endswith(('.json','.status')) and 'identity' not in name.lower() and 'key' not in name.lower():
   try:visit(json.loads((Path(directory)/name).read_text(encoding='utf-8-sig')))
   except (ValueError,UnicodeError):pass
(r/'owned-identities-for-final-proof.json').write_text(json.dumps(list(found.values()),indent=2)+'\n')
print('Recorded identities',len(found))
