from pathlib import Path
import json
r=Path(__file__).parent
x=json.loads((r/'matrix-isolated-analysis.json').read_text());assert x['mechanism_observed']
g=json.loads((r/'matrix-isolated-guard-attempt2/status.json').read_text(encoding='utf-8-sig'))
assert g['exit']==0 and g['reason'] is None and g['cleanup_complete'] and not g['remaining']
p=Path(r'D:\GenesisBlock_Dev\GenesisBlock\docs\reports\GENESISDB-VECTOR-MEMORY-AUDIT.md');s=p.read_text()
s=s.replace('**The deferral mechanism is fixed and\nregression-tested. The 1 GB timed causal\nreproducer confirmed the WAL mechanism; the isolated timing rerun and 100k reopen\ndeadline remain unqualified.** They must','**The deferral mechanism is fixed, regression-tested, and reproduced with pinned\nSQLite on the saved 1 GB projection. The 100k reopen deadline remains unqualified.** It must')
s=s.replace('The isolated timing rerun and 100k reopen\ndeadline remain unqualified.','The 100k reopen deadline remains unqualified.')
s=s.replace('| Pinned-SQLite matrix | Mechanism observed on the saved 1.017 GB real projection; concurrent external compilers started during its tail, so timings are diagnostic and an isolated rerun is pending |','| Pinned-SQLite matrix | Fresh isolated four-mode run passed: SQLite 3.46.0, exact payloads, source unchanged, deferred backfill observed; original interfered attempt retained separately |')
text='\nFresh isolated rerun (`matrix-isolated/`, `matrix-isolated-analysis.json`):\n\n| Mode | First prepare ms | First row us | Publication maintenance ms |\n|---|---:|---:|---:|\n'
for c in x['cases']:
 m=f"{c['publication_maintenance_us']/1000:.3f}" if c['publication_maintenance_us'] is not None else 'n/a'
 text+=f"| {c['mode']} | {c['first']['prepare_us']/1000:.3f} | {c['first']['row_us']} | {m} |\n"
text+='\nAll payloads and source hashes passed. Normal backup again left 248,368 frames\nunbackfilled, and first prepare backfilled all of them. The no-autocheckpoint\ncontrol left them unbackfilled through prepare; the production maintenance\ncompleted backfill before the first SELECT. The runtime reported SQLite 3.46.0\nin every case. No Genesis replay or SQL/search warm-up ran in this harness.\nAdmission CPU averaged '+f"{g['admission_cpu']:.2f}"+'%; free RAM minimum '+f"{g['admission_free']/1024**3:.2f}"+' GiB;\nthe complete four-mode guarded job took '+f"{g['seconds']:.3f}"+' s and sampled tree RSS peaked at\n'+f"{g['peak_rss']/1e6:.2f}"+' MB. Continuous process checks found no competing compiler/benchmark.\nThis fixed-order single observation does not establish total-I/O or statistical\nperformance improvement, and the OS file cache was not reset. Maintenance\nincludes schema loading and real checkpoint work; it is not a dummy cache prime.\n'
s=s.replace('<!-- SQLITE_MATRIX_RESULTS_END -->',text+'<!-- SQLITE_MATRIX_RESULTS_END -->')
s=s.replace('Remaining timing gates: pinned timed causal matrix; the matched','Remaining timing gates: the matched')
p.write_text(s,encoding='utf-8',newline='\n')
summary=json.loads((r/'qualification-summary.json').read_text());summary['timed_sqlite_matrix']={'status':'PASS','source':'matrix-isolated-analysis.json','guard':'matrix-isolated-guard-attempt2/status.json','historical_interfered_attempt':'matrix-real-interference.json'}
(r/'qualification-summary.json').write_text(json.dumps(summary,indent=2)+'\n')
print('Isolated causal matrix documented; 100k and real pair remain pending.')
