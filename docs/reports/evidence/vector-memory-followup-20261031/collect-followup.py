from pathlib import Path
import json,hashlib,shutil,difflib,datetime,os
r=Path(r"C:\Users\suanr\genesis-vector-memory-followup-20261031")
repo=Path(r"D:\GenesisBlock_Dev\GenesisBlock")
e=repo/'docs/reports/evidence/vector-memory-followup-20261031';e.mkdir(parents=True,exist_ok=True)
# Only small evidence/controller paths; never copy scratch databases or build trees.
allowed={'.json','.jsonl','.log','.err','.status','.patch','.ps1','.py','.cjs','.txt','.csv','.rs','.toml'}
excluded_parts={'gdb','db','database','scratch','scratch-copy','target','readonly-db','read-only-db','node-smoke-scratch'}
def evidence_files(base):
 for root,dirs,names in os.walk(base):
  dirs[:]=[name for name in dirs if name.lower() not in excluded_parts and not name.lower().endswith('.gdb')]
  for name in names:
   f=Path(root)/name
   if is_evidence(f,base):yield f
def is_evidence(f,base):
 return (f.is_file() and f.suffix in allowed
         and not any(part.lower() in excluded_parts or part.lower().endswith('.gdb') for part in f.relative_to(base).parts)
         and f.name.lower() not in {'state.json','identity.json','keypair.json'}
         and f.stat().st_size<8*1024*1024)
for f in r.iterdir():
 if f.is_file() and f.suffix in allowed and f.stat().st_size<8*1024*1024:
  shutil.copyfile(f,e/f.name)
combined=[]
for folder in ('six-runtime','six2-runtime','final-runtime'):
 for f in sorted((r/folder).glob('*.log')):combined.append('===== '+folder+'/'+f.name+' =====\n'+f.read_text(errors='replace'))
(e/'rust-test-logs.txt').write_text('\n'.join(combined))
for d in r.iterdir():
 if not d.is_dir() or not d.name.startswith(('query-reopen-','projection-props-split-','candidate-repeat-build-','baseline-addon-profile-','journal-2000-phase-diag-')):continue
 for f in evidence_files(d):
  dst=e/d.name/f.relative_to(d);dst.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(f,dst)
# Native controls keep both excluded pilot and complete ABBAAB experiment.
n=r/'followuproot/native-perf-abbaab'
for f in n.iterdir():
 if f.is_file() and f.suffix in allowed:dst=e/'native-controls'/f.name;dst.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(f,dst)
for d in (n/'runs').iterdir():
 if not d.is_dir():continue
 for f in evidence_files(d):
  dst=e/'native-controls/runs'/d.name/f.relative_to(d);dst.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(f,dst)
real=r/'followuproot/vector-real-release-controller'
if real.exists():
 for f in evidence_files(real):
  dst=e/'real-release-controls'/f.relative_to(real);dst.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(f,dst)
shutil.copyfile(repo/'.brain/rca/2026-10-31-vector-memory-timeouts.md',e/'timeout-rca.txt')
files=('src/lib.rs','src/vector_memory_tests.rs','Cargo.toml','Cargo.lock','benches/vector_memory.rs','benches/scripts/vector_memory_matrix.ps1','index.d.ts')
p={'recorded_at':datetime.datetime.now().astimezone().isoformat(),'head':'ad94aa2e46025b7729d117ff376ab281ac15c49d','source':{f:hashlib.sha256((repo/f).read_bytes()).hexdigest() for f in files},'engine_stages':{},'binaries':{}}
for f in ('baseline-lib.rs','index-and-cache-lib.rs','encoding-cache-lib.rs'):
 p['engine_stages'][f]=hashlib.sha256((r/f).read_bytes()).hexdigest()
for f in ('baseline-release.exe','candidate-release.exe','baseline-profiled.node','candidate-profiled.node'):
 p['binaries'][f]=hashlib.sha256((r/f).read_bytes()).hexdigest()
p['diagnostic_binaries']={}
for f in ('query-reopen-rw-20261031T122655/diagnostic-vbench-genesis-repeat.exe','candidate-repeat-build-20261031T131156/candidate-vbench-genesis-repeat.exe','projection-props-split-20261031T151315/projection-props-split-vbench.exe'):
 p['diagnostic_binaries'][f]=hashlib.sha256((r/f).read_bytes()).hexdigest()
(e/'provenance.json').write_text(json.dumps(p,indent=2)+'\n')
for name,old,new in [('followup-engine.patch',(r/'baseline-lib.rs').read_text(),(repo/'src/lib.rs').read_text()),('followup-cargo.patch',(r/'baseline-Cargo.toml').read_text(),(repo/'Cargo.toml').read_text())]:
 (e/name).write_text(''.join(difflib.unified_diff(old.splitlines(keepends=True),new.splitlines(keepends=True),fromfile='a/'+('src/lib.rs' if 'engine' in name else 'Cargo.toml'),tofile='b/'+('src/lib.rs' if 'engine' in name else 'Cargo.toml'))))
print({'exported':str(e),'source':p['source']['src/lib.rs']})
