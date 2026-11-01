from pathlib import Path
import hashlib,json,shutil
r=Path(__file__).parent; repo=Path(r'D:\GenesisBlock_Dev\GenesisBlock')
old=r/'release-src/20261031T150004Z-b5e55f51'; clone=r/'release-src/query-evidence-20261101'
assert not clone.exists(); shutil.copytree(old,clone)
def digest(p):
    with p.open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
before=r/'entry-lib.rs'; after=r/'proposed-publication-lib.rs'
assert digest(before)=='812afdbe74e96aee4f0c08b93488dda4ac5a6c0e4784b72bb9f615d30c7877a9'
assert digest(after)=='5b8bb9bed5dde5fa6e7d4d255da3acce6c1bfe892092456133adb65a78998fdd'
for f in ('vector_memory.rs','query_evidence.rs'):shutil.copyfile(repo/'benches'/f,clone/'benches'/f)
shutil.copyfile(before,clone/'src/lib.rs')
manifest={'clone':str(clone),'before_lib':digest(before),'after_lib':digest(after),
  'bench':digest(clone/'benches/vector_memory.rs'),'helper':digest(clone/'benches/query_evidence.rs'),
  'cargo_lock':digest(clone/'Cargo.lock'),'cargo_toml':digest(clone/'Cargo.toml')}
(r/'query-evidence-build-manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
print(json.dumps(manifest))
