import pathlib,subprocess,shutil,hashlib,json,time,sys
r=pathlib.Path(__file__).parent
source=pathlib.Path(r'C:\Users\suanr\genesis-vector-memory-followup-20261031\projection-props-split-20261031T151315\output\gdb\projection.sqlite')
exe=r/'reproducer-target/release/sqlite-prepare-reproducer.exe'
out=r/(sys.argv[1] if len(sys.argv)>1 else 'matrix-real')
out=out.resolve();out.relative_to(r.resolve())
out.mkdir(exist_ok=False)
def sha(p):
 h=hashlib.sha256()
 with p.open('rb') as f:
  while b:=f.read(1048576): h.update(b)
 return h.hexdigest()
def save(): (out/'results.json').write_text(json.dumps(results,indent=2)+'\n')
results={'source':str(source),'source_sha256_before':sha(source),'binary_sha256':sha(exe),'cases':[],'no_genesis_replay':True}
save()
for mode in ['direct','backup','backup-noauto','backup-checkpoint']:
 dest=out/(mode+'.sqlite')
 if mode!='direct':
  if dest.exists(): raise RuntimeError('refusing existing destination')
  t=time.monotonic();shutil.copyfile(source,dest);copy_s=time.monotonic()-t
 else:copy_s=0
 args=[str(exe),mode,str(source)]+([str(dest)] if mode!='direct' else [])+['5932']
 t=time.monotonic()
 with (out/(mode+'.stdout.log')).open('w') as f,(out/(mode+'.stderr.log')).open('w') as e:
  p=subprocess.run(args,stdout=f,stderr=e,creationflags=subprocess.CREATE_NO_WINDOW)
 results['cases'].append({'mode':mode,'copy_seconds':copy_s,'seconds':time.monotonic()-t,'exit':p.returncode,'args':args});save()
 if p.returncode:sys.exit(p.returncode)
results['source_sha256_after']=sha(source)
results['source_unchanged']=results['source_sha256_before']==results['source_sha256_after'];save()
assert results['source_unchanged']

