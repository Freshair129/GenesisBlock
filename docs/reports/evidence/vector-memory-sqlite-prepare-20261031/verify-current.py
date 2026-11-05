from pathlib import Path
import hashlib,json,subprocess,datetime
r=Path(__file__).parent
old=r.parent/'genesis-vector-memory-20261031'
repo=Path(r'D:\GenesisBlock_Dev\GenesisBlock')
verify=old/'verify-src'
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
all_paths=json.loads((old/'verify-source-manifest.json').read_text()).keys()
# Compilation provenance covers code/config/declarations. Reports and RCAs evolve
# independently; keep their differences visible and verify original staged docs
# against their protected hashes separately below.
paths=[p for p in all_paths if not p.startswith(('docs/','.brain/'))]
document_paths=[p for p in all_paths if p not in paths]
manifest={p:sha(repo/p) for p in paths}
mismatches=[p for p,h in manifest.items() if not (verify/p).is_file() or sha(verify/p)!=h]
preserved=json.loads((old/'baseline-source.json').read_text())['preserved_staged']
protected=[p for p,h in preserved.items() if sha(repo/p)!=h]
staged=subprocess.run(['git','-c','safe.directory=D:/GenesisBlock_Dev/GenesisBlock','diff','--cached','--name-only'],cwd=repo,capture_output=True,text=True,check=True).stdout.splitlines()
if sorted(staged)!=sorted(preserved):protected.append('STAGED_PATH_SET_CHANGED')
actual_head=subprocess.run(['git','-c','safe.directory=D:/GenesisBlock_Dev/GenesisBlock','rev-parse','HEAD'],cwd=repo,capture_output=True,text=True,check=True).stdout.strip()
actual_branch=subprocess.run(['git','-c','safe.directory=D:/GenesisBlock_Dev/GenesisBlock','branch','--show-current'],cwd=repo,capture_output=True,text=True,check=True).stdout.strip()
if actual_head!=json.loads((old/'baseline-source.json').read_text())['head']:protected.append('BASE_HEAD_CHANGED')
document_differences=[p for p in document_paths if sha(repo/p)!=sha(verify/p)]
receipt={'recorded_at':datetime.datetime.now().astimezone().isoformat(),'source_files':len(manifest),'preserved_staged_files':len(preserved),'source_mismatches':mismatches,'protected_mismatches':protected,'noncompiled_document_paths':len(document_paths),'noncompiled_document_differences':document_differences,'scope_note':'Updated report/RCA are not compiled source; all seven original staged paths are checked independently. Attempt 1 incorrectly included evolving documents in code comparison.','passed':not mismatches and not protected}
(r/'verify-source-manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
receipt.update({'head':actual_head,'branch':actual_branch,'staged_paths':staged})
(r/'verify-source-receipt.json').write_text(json.dumps(receipt,indent=2)+'\n')
print(json.dumps(receipt))
raise SystemExit(0 if receipt['passed'] else 1)
