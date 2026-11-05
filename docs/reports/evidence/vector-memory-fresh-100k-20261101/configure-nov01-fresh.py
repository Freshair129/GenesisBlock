from pathlib import Path
import json, sys
r = Path(__file__).parent
assert not (r / '100k-fresh-cases').exists()
for source, dest in [('run-fresh-workload.py', 'run-fresh-workload-nov01.py'),
                     ('prepare-fresh-case-copies.py', 'prepare-fresh-case-copies-nov01.py')]:
    text = (r / source).read_text(encoding='utf-8')
    text = text.replace('100k-fixture-preparation/', '100k-fixture-preparation-nov01/')
    if source.startswith('prepare-'):
        text = text.replace("'fixture-preparation-guard/status.json'", "'fixture-preparation-nov01-guard/status.json'")
        text = text.replace("'run-fresh-workload.py'", "'run-fresh-workload-nov01.py'")
    target = r / dest
    assert not target.exists()
    target.write_text(text, encoding='utf-8')
cfg = {'kind':'benchmark', 'cap_seconds':1200, 'exe':sys.executable,
       'args':['-u',str(r/'prepare-fresh-case-copies-nov01.py')], 'cwd':str(r),
       'output':str(r/'fresh-copy-nov01-guard'), 'env':{}}
out = r / 'fresh-copy-nov01-guard-config.json'
assert not out.exists()
out.write_text(json.dumps(cfg, indent=2)+'\n', encoding='utf-8')
