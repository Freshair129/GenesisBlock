from pathlib import Path
import json, runpy, shutil, sys

r = Path(__file__).parent
api = runpy.run_path(str(r / 'prepare-100k-cases.py'))
source = api['SOURCE_DB']
case = api['require_under'](r / '100k-fixture-preparation-nov01', r)
assert not case.exists(), 'Refusing existing fixture'
assert shutil.disk_usage(r).free >= api['MIN_FREE_BYTES']
expected = json.loads((r / '100k-two-open-attempt1/copy-manifest.json').read_text())['source']
wal_before = api['source_wal_state']()
before = api['tree_manifest'](source)
assert before == expected, 'Source differs from previously verified pristine manifest'
case.mkdir()
destination = api['require_under'](case / 'gdb', r)
shutil.copytree(source, destination, copy_function=shutil.copy2)
copied = api['tree_manifest'](destination)
after = api['tree_manifest'](source)
wal_after = api['source_wal_state']()
assert before == copied == after == expected, 'Copy or source hash mismatch'
api['write_json'](case / 'copy-manifest.json', {'source': before, 'copy': copied, 'source_after': after,
    'wal_before': wal_before, 'wal_after': wal_after, 'sqlite_opened': False})
cfg = json.loads((r / 'fixture-preparation-config.json').read_text())
cfg['case_path'] = str(case)
config = r / 'fixture-preparation-nov01-config.json'
assert not config.exists()
api['write_json'](config, cfg)
guard = json.loads((r / 'fixture-preparation-guard-config.json').read_text())
guard['args'][-1] = str(config)
guard['output'] = str(r / 'fixture-preparation-nov01-guard')
api['write_json'](r / 'fixture-preparation-nov01-guard-config.json', guard)
print(json.dumps({'copy_verified': True, 'tree_sha256': before['tree_sha256'], 'bytes': before['byte_count'], 'files': before['file_count']}))
