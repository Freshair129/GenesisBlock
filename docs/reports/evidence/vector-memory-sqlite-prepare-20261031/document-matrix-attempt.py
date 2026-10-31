from pathlib import Path
import json
r=Path(__file__).parent
x=json.loads((r/'matrix-analysis.json').read_text());assert x['mechanism_observed']
p=Path(r'D:\GenesisBlock_Dev\GenesisBlock\docs\reports\GENESISDB-VECTOR-MEMORY-AUDIT.md');s=p.read_text()
s=s.replace('reproducer and the 100k reopen deadline are still NOT_RUN/unqualified.** They must','reproducer confirmed the WAL mechanism; the isolated timing rerun and 100k reopen\ndeadline remain unqualified.** They must')
s=s.replace('The exact attribution of the historical 2.631-second\nreal-embedding tail still needs the separate timed causal matrix.','The standalone matrix reproduces this class of prepare stall on the saved real\nprojection without Genesis replay. It cannot retroactively observe the callback\nstate of the historical 2.631-second trace.')
s=s.replace('| Timed pinned-SQLite matrix / 100k / new real-embedding pair | NOT_RUN; resource admission blocked |','| Pinned-SQLite matrix | Mechanism observed on the saved 1.017 GB real projection; concurrent external compilers started during its tail, so timings are diagnostic and an isolated rerun is pending |\n| 100k / new real-embedding pair | NOT_RUN; fresh admission required |')
table='\n<!-- SQLITE_MATRIX_RESULTS_START -->\nStandalone pinned-SQLite diagnostic attempt (no Genesis replay; **interfered timing,\nnot isolated performance qualification**):\n\n| Mode | First prepare ms | First row us | Publication maintenance ms | Backfill during first prepare |\n|---|---:|---:|---:|---|\n'
for c in x['cases']:
 a=c['before_first_prepare'];b=c['after_first_prepare']
 transition=f"{a['n_backfill']} -> {b['n_backfill']}" if a and b else 'n/a'
 maintenance=f"{c['publication_maintenance_us']/1000:.3f}" if c['publication_maintenance_us'] is not None else 'n/a'
 table+=f"| {c['mode']} | {c['first']['prepare_us']/1000:.3f} | {c['first']['row_us']} | {maintenance} | {transition} |\n"
table+='\nAll four modes returned the exact `{}` payload; source SHA256 stayed\n`'+x['source_sha256']+'`.\nBackup produced 248,368 WAL frames. Disabling autocheckpoint is a diagnostic control\nonly; the production fix retains it. Maintenance moves work to publication.\nWhole-case wall times include backup, connections and close-time checkpoint work;\nthey are not claimed to improve. Unrelated gs-creator compilers started at\n22:26:04 local time after admission, so the raw timing attempt is preserved with\n`matrix-real-interference.json` and excluded from isolated performance conclusions.\nThe next guard continuously rejects newly arriving competing compiler/benchmark\nprocesses, stops only its owned tree, and retains the original time/RSS limits.\n<!-- SQLITE_MATRIX_RESULTS_END -->\n'
key='The prepared 100k workload makes one legitimate non-vector `Storage.add_node`,'
s=s.replace(key,table+'\n'+key)
p.write_text(s,encoding='utf-8',newline='\n')
print('Documented causal observation and interference boundary.')
