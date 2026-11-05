from pathlib import Path
import json,hashlib
r=Path(__file__).parent
old=json.loads((r/'qualification-attempt1/final-test-artifacts.json').read_text())
prev={x['binary']:x for x in old['artifacts']}
new=[]
for s in (r/'final-cached-core-build/stdout.log').read_text(errors='replace').splitlines():
 try:d=json.loads(s)
 except:continue
 if d.get('reason')=='compiler-artifact' and d.get('profile',{}).get('test') and d.get('executable'):
  p=Path(d['executable']);new.append({'target':d['target']['name'],'binary':p.name,'path':str(p),'sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'fresh':d['fresh'],'profile':d['profile']})
assert len(new)==159
assert set(prev)=={x['binary'] for x in new}
changed=[x['binary'] for x in new if prev[x['binary']]['sha256']!=x['sha256']]
receipt={'count':len(new),'changed_binaries':changed,'unchanged_binaries':159-len(changed),'artifacts':new}
(r/'final-compiled-binary-comparison.json').write_text(json.dumps(receipt,indent=2)+'\n')
print(json.dumps({'count':159,'changed_count':len(changed),'unchanged':159-len(changed),'changed_first5':changed[:5]}))
