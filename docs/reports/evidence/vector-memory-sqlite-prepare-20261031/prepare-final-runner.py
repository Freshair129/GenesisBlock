from pathlib import Path
r=Path(__file__).parent
s=(r/'run-full-core.py').read_text(encoding='utf-8-sig')
s=s.replace('all-core-build-receipt/','final-cached-core-build/')
s=s.replace(" p=pathlib.Path(path);shutil.copyfile(p,dep/p.name);binaries.append({'target':d['target']['name'],'binary':p.name,'sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'profile':d['profile']})", """ p=pathlib.Path(path); digest=hashlib.sha256()
 with p.open('rb') as source,(dep/p.name).open('wb') as target:
  while block:=source.read(1024*1024):
   target.write(block);digest.update(block)
 actual=digest.hexdigest()
 expected=next(x['sha256'] for x in json.loads((r/'final-compiled-binary-comparison.json').read_text())['artifacts'] if x['binary']==p.name)
 assert actual==expected, p.name
 binaries.append({'target':d['target']['name'],'binary':p.name,'sha256':actual,'profile':d['profile']})""")
(r/'run-full-core-final.py').write_text(s,newline='\n')
