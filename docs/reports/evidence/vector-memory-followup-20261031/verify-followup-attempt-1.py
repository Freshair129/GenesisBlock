from pathlib import Path
import hashlib,json,subprocess,datetime
r=Path(__file__).parent
old=r.parent/'genesis-vector-memory-20261031'
repo=Path(r'D:\GenesisBlock_Dev\GenesisBlock')
verify=old/'verify-src'
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
paths=json.loads((old/'verify-source-manifest.json').read_text()).keys()
manifest={p:sha(repo/p) for p in paths}
mismatches=[p for p,h in manifest.items() if not (verify/p).is_file() or sha(verify/p)!=h]
preserved=json.loads((old/'baseline-source.json').read_text())['preserved_staged']
protected=[p for p,h in preserved.items() if sha(repo/p)!=h]
staged=subprocess.run(['git','-c','safe.directory=D:/GenesisBlock_Dev/GenesisBlock','diff','--cached','--name-only'],cwd=repo,capture_output=True,text=True,check=True).stdout.splitlines()
if sorted(staged)!=sorted(preserved):protected.append('STAGED_PATH_SET_CHANGED')
receipt={'recorded_at':datetime.datetime.now().astimezone().isoformat(),'source_files':len(manifest),'preserved_staged_files':len(preserved),'source_mismatches':mismatches,'protected_mismatches':protected,'passed':not mismatches and not protected}
(r/'verify-source-manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
(r/'verify-source-receipt.json').write_text(json.dumps(receipt,indent=2)+'\n')
print(json.dumps(receipt))
raise SystemExit(0 if receipt['passed'] else 1)
