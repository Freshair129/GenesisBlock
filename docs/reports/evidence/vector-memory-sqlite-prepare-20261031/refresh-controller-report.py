from pathlib import Path
import json
r=Path(__file__).parent
x=json.loads((r/'controller-validation.json').read_text());assert x['passed']
p=Path(r'D:\GenesisBlock_Dev\GenesisBlock\docs\reports\GENESISDB-VECTOR-MEMORY-AUDIT.md');s=p.read_text()
s=s.replace('for subsequent edits inside the private clone. This controller preparation is not itself a\nbenchmark pass and still needs its execution checks in a resource window.','for subsequent edits inside the private clone. Normal-completion and owned-child\ntimeout fixtures both passed on the current controller: cleanup completed and no\ntracked owner remained. The deliberate three-second timeout is a controller test,\nnot a workload failure or a benchmark pass. The validation receipt binds its SHA256.')
s=s.replace('Remaining gates: fresh final 159-target suite; strict static/native checks;\ncontroller completion/timeout checks; pinned timed causal matrix; the matched','Remaining timing gates: pinned timed causal matrix; the matched')
p.write_text(s,encoding='utf-8',newline='\n')
print('Controller fixture and completed-gate status refreshed.')
