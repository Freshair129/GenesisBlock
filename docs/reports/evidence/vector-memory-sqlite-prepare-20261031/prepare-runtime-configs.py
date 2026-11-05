from pathlib import Path
import json,hashlib,shutil,sys
r=Path(__file__).parent
old=Path(r'C:\Users\suanr\genesis-vector-memory-followup-20261031')
py=sys.executable
def sha(p):return hashlib.file_digest(p.open('rb'),'sha256').hexdigest()
def save(p,x):
 if p.exists():raise RuntimeError(f'refusing existing config {p}')
 p.parent.mkdir(parents=True,exist_ok=True);p.write_text(json.dumps(x,indent=2)+'\n')
for case in ('before','after'):
 exe=r/f'diagnostic-{case}.exe'
 cfg={'binary_path':str(exe),'binary_sha256':sha(exe),'case_path':str(r/'100k-cases'/case),
      'input_path':r'C:\Users\suanr\genesis-vector-memory-20261031\inputs\synthetic-100000-1024',
      'expected_engine_state':'error' if case=='before' else 'ok',
      'expected_query0_state':'error' if case=='before' else 'ok','args':[]}
 p=r/f'100k-{case}-config.json';save(p,cfg)
 save(r/f'100k-{case}-guard-config.json',{'exe':py,'args':['-u',str(r/'run-100k-case.py'),'--config',str(p)],
      'cwd':str(r),'output':str(r/f'100k-{case}-guard'),'env':{}})
base=json.loads((old/'projection-props-split-20261031T151315/run-config.json').read_text(encoding='utf-8-sig'))
driver=r/'run-real-case.py'
shutil.copyfile(old/'projection-props-split-20261031T151315/spawn-benchmark.py',driver)
for case,exe in [('before',old/'candidate-release.exe'),('after',r/'candidate-untraced-release.exe')]:
 root=r/'real-pair'/case;root.mkdir(parents=True,exist_ok=False)
 cfg={k:base[k] for k in ('input_dir','expected_input_hashes','env_whitelist','extra_env','clear_env','limits')}
 cfg.update(run_root=str(root),repo=str(root),binary=str(exe),binary_sha256=sha(exe))
 cfg['env_whitelist']['GB_VECTOR_QUERY_DIAGNOSTICS']=None
 cfg['env_whitelist']['GB_MEMORY_OPEN_REPEAT']=None
 cfg['extra_env']['GB_MEMORY_OUT']=str(root/'output')
 p=root/'run-config.json';save(p,cfg)
 save(r/f'real-{case}-guard-config.json',{'exe':py,'args':['-u',str(driver),str(p)],'cwd':str(root),
      'output':str(r/f'real-{case}-guard'),'env':{}})
print('Prepared frozen before/after configs; no benchmark launched.')
