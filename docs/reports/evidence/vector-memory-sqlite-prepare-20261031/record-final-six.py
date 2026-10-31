from pathlib import Path
import json
r=Path(__file__).parent
names=('concurrency-','consensus_checkpoint_race_tests-','hql_cypher_tests-','journal_format_tests-','rebuild_from_truth_tests-','wal_compaction_tests-')
rows=[x for x in json.loads((r/'final-runtime.json').read_text()) if x['target'].startswith(names)]
assert len(rows)==6 and all(x['exit_code']==0 and x['reason'] is None and x['seconds']<300 and x['peak_rss_bytes']<4*1024**3 for x in rows)
(r/'final-six-results.json').write_text(json.dumps({'source':'5b8bb9bed5dde5fa6e7d4d255da3acce6c1bfe892092456133adb65a78998fdd','tests':'e69dc7dd5cbc5043aa5ed9c717e449e50246a8fbd8ad56ac95ed135718c99be2','per_target_cap_s':300,'per_target_rss_cap_bytes':4*1024**3,'targets':rows},indent=2)+'\n')
p=Path(r'D:\GenesisBlock_Dev\GenesisBlock\docs\reports\GENESISDB-VECTOR-MEMORY-AUDIT.md')
s=p.read_text();key='Raw fresh logs are under `final-runtime-fresh-20261031T213000/`;'
table='Final-source rerun of the six previously incomplete groups (functional timings,\nnot an idle-host throughput comparison):\n\n| Complete target | Seconds | Result |\n|---|---:|---|\n'
for x in rows:table+=f"| {x['target'].split('-')[0]} | {x['seconds']:.3f} | PASS |\n"
s=s.replace(key,table+'\n'+key)
p.write_text(s,encoding='utf-8',newline='\n')
print('Six final-source results bound and documented.')
