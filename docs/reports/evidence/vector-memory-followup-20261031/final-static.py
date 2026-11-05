from pathlib import Path
import subprocess,json,datetime,sys
r=Path(__file__).parent
repo=Path(r'D:\GenesisBlock_Dev\GenesisBlock')
checks=[('fmt',['cargo','fmt','--all','--','--check']),
 ('docs',['node','scripts/docs-validate.mjs']),
 ('diff',['git','-c','safe.directory=D:/GenesisBlock_Dev/GenesisBlock','diff','--check']),
 ('source',[sys.executable,str(r/'verify-followup.py')])]
results=[]
for name,command in checks:
 p=subprocess.run(command,cwd=repo,capture_output=True,text=True,encoding='utf-8',errors='replace',timeout=60)
 output=p.stdout+p.stderr
 (r/f'final-{name}.log').write_text(output,encoding='utf-8')
 results.append({'name':name,'command':command,'exit_code':p.returncode,'output':output})
 print(name,p.returncode,flush=True)
receipt={'recorded_at':datetime.datetime.now().astimezone().isoformat(),'cwd':str(repo),'checks':results,'passed':all(v['exit_code']==0 for v in results)}
(r/'final-static-validation.json').write_text(json.dumps(receipt,indent=2)+'\n')
raise SystemExit(0 if receipt['passed'] else 1)
