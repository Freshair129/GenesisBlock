from pathlib import Path
import json,sys
r=Path(__file__).parent;case=sys.argv[1]
p=r/'100k-cases'/case
result=json.loads((p/'result.json').read_text())
receipt=json.loads((p/'run-receipt.json').read_text())
guard=json.loads((r/f'100k-{case}-guard/status.json').read_text(encoding='utf-8-sig'))
lines=(p/'stderr.log').read_text(errors='replace').splitlines()
events=[]
for n,line in enumerate(lines):
 for prefix in ('GB_MEMORY_MUTATE_REOPEN ','GB_VECTOR_QUERY_DIAG '):
  if line.startswith(prefix):events.append((n,json.loads(line[len(prefix):])));break
def stage(name):
 matches=[i for i,e in events if e.get('stage')==name]
 assert len(matches)==1,(name,matches)
 return matches[0]
start=stage('second_open_start');end=stage('second_open_complete')
q0=stage('query0_start');qend=stage('query0_complete')
assert start<end<q0<qend
opened=[e for n,e in events if start<n<end]
sync=[e for e in opened if e.get('event')=='projection_sync_on_open']
backup=[e for e in opened if e.get('event')=='projection_publish_backup_done']
maintenance=[e for e in opened if e.get('event')=='projection_publish_maintenance_done']
assert len(sync)==1 and sync[0]['branch']=='marker_miss' and sync[0]['force_rebuild'] is False,sync
assert len(backup)==1 and backup[0]['shm_mx_frame_le_u32_at_16']>=1000,backup
assert not any(e.get('event')=='query_plan' for n,e in events if end<n<q0),'query before q0'
queries=[e for n,e in events if q0<n<qend and e.get('event')=='query_plan']
assert len(queries)==1 and queries[0]['slots']==100000 and queries[0]['k']==10 and queries[0]['query_dim']==1024,queries
props=[e for n,e in events if q0<n<qend and e.get('event')=='projection_props_stage']
assert props,'missing timed props diagnostic'
valid_guard=guard['reason'] is None and guard['exit']==0 and guard['cleanup_complete'] and not guard['remaining']
after_pass=False
if case=='after':
 assert len(maintenance)==1 and maintenance[0]['maintenance_status']=='ok'
 assert maintenance[0]['shm_n_backfill_le_u32_at_96']==maintenance[0]['shm_mx_frame_le_u32_at_16']
 assert valid_guard and receipt['input_unchanged'] and receipt['process_exit_code']==0
 assert result['state']=='ok' and result['result_count_contract']['passed']
 assert result['metadata_node_count']==100001 and result['metadata_count']==100000 and result['marker_recovered']
 assert result['coverage_bench_ok'] and result['coverage_bench']['state']=='READY'
 assert result['query0_memory']['seconds']<5
 assert 'error' not in result['query_samples_k10'] and 'error' not in result['query_samples_k100']
 after_pass=True
else:assert not maintenance
out={'case':case,'guard_valid':valid_guard,'engine_state':result['state'],'after_deadline_and_content_pass':after_pass,
     'second_open_branch':sync[0],'backup':backup[0],'maintenance':maintenance,
     'projection_stages':[e for e in opened if e.get('event')=='projection_sync_candidate_stage'],
     'first_open':result['first_open_memory'],'first_drop':result['first_drop_memory'],'second_open':result['second_open_memory'],
     'query0':result['query0'],'query0_memory':result['query0_memory'],'first_props':props[0],
     'query_samples_k10':result['query_samples_k10'],'query_samples_k100':result['query_samples_k100'],
     'coverage_bench':result['coverage_bench'],'metadata_counts':[result['metadata_node_count'],result['metadata_count']],
     'marker_recovered':result['marker_recovered'],'input_unchanged':receipt['input_unchanged'],
     'method':'First search immediately after actual marker-miss publication; unchanged 5000ms query and 1200s/2GiB parent caps; no SQL/search warmup or OS cache reset.'}
(r/f'100k-{case}-analysis.json').write_text(json.dumps(out,indent=2)+'\n')
print(json.dumps(out,indent=2))
