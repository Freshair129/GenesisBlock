import hashlib,json,pathlib
r=pathlib.Path(r'C:\Users\suanr\genesis-vector-memory-20261031')
p=pathlib.Path(r'D:\GenesisBlock_Dev\GenesisBlock')
manifest=json.loads((r/'verify-source-manifest.json').read_text())
errors=[]
for f,h in manifest.items():
 if f.startswith(('src/','tests/','benches/')) or f in ('Cargo.lock','Cargo.toml','build.rs','index.d.ts'):
  actual=hashlib.sha256((p/f).read_bytes()).hexdigest()
  verify=hashlib.sha256((r/'verify-src'/f).read_bytes()).hexdigest()
  if actual!=h or verify!=h: errors.append(f)
original=json.loads((r/'baseline-source.json').read_text())
for f,h in original['preserved_staged'].items():
 if hashlib.sha256((p/f).read_bytes()).hexdigest()!=h: errors.append('preserved:'+f)
result={'checked_source_files':sum(1 for f in manifest if f.startswith(('src/','tests/','benches/')) or f in ('Cargo.lock','Cargo.toml','build.rs','index.d.ts')),'preserved_staged_files':len(original['preserved_staged']),'mismatches':errors}
(r/'source-verification.json').write_text(json.dumps(result,indent=2))
print(result)
