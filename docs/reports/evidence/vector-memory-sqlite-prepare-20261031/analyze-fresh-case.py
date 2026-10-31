from pathlib import Path
import json,sys
r=Path(__file__).parent;mode=sys.argv[1];assert mode in ('before','after')
case=r/'100k-fresh-cases'/mode
result=json.loads((case/'result.json').read_text());receipt=json.loads((case/'run-receipt.json').read_text())
guard=json.loads((r/f'fresh-{mode}-guard/status.json').read_text(encoding='utf-8-sig'))
assert guard['exit']==0 and guard['reason'] is None and guard['cleanup_complete'] and not guard['remaining']
assert receipt['input_unchanged'] and receipt['engine_exit']==0
events=[]
for n,l in enumerate((case/'stderr.log').read_text().splitlines()):
 for prefix in ('GB_MEMORY_MUTATE_REOPEN ','GB_VECTOR_QUERY_DIAG '):
  if l.startswith(prefix):events.append((n,json.loads(l[len(prefix):])));break
def stage(name):
 xs=[n for n,e in events if e.get('stage')==name];assert len(xs)==1,(name,xs);return xs[0]
assert not any(e.get('stage')=='first_open_start' for _,e in events),'Unexpected fixture setup in measured reopen'
start=stage('second_open_start');end=stage('second_open_complete');q=stage('query0_start');qe=stage('query0_complete')
assert start<end<q<qe and result['same_process'] is False and result['repeat_count']==1
opened=[e for n,e in events if start<n<end]
sync=[e for e in opened if e.get('event')=='projection_sync_on_open'];assert len(sync)==1 and sync[0]['branch']=='marker_miss' and sync[0]['force_rebuild'] is False
backups=[e for e in opened if e.get('event')=='projection_publish_backup_done'];assert len(backups)==1
backup=backups[0];assert backup['shm_mx_frame_le_u32_at_16']>=1000
maintenance=[e for e in opened if e.get('event')=='projection_publish_maintenance_done']
assert not any(e.get('event')=='query_plan' for n,e in events if n<q),'Search warmup'
assert not any(e.get('event')=='projection_props_stage' for n,e in events if n<q),'Observer scoping failed'
plans=[e for n,e in events if q<n<qe and e.get('event')=='query_plan'];assert len(plans)==1 and plans[0]['slots']==100000 and plans[0]['k']==10 and plans[0]['query_dim']==1024
props=[e for n,e in events if q<n<qe and e.get('event')=='projection_props_stage'];assert props
assert result['metadata_node_count']==100001 and result['metadata_count']==100000 and result['marker_recovered'] and result['coverage_bench_ok']
assert result['coverage_bench']['source_count']==100000 and result['coverage_bench']['indexed_count']==100000
for field,k in [('query_samples_k10',10),('query_samples_k100',100)]:
 x=result[field];assert 'error' not in x and x['result_count_min']==k and x['result_count_max']==k
assert result['query_sample_count']==24 and result['query_budget_default_elapsed_ms']==5000
if mode=='after':
 assert result['state']=='ok' and 'error' not in result['query0'] and result['query0_memory']['seconds']<5
 assert result['result_count_contract']['passed']
 assert len(maintenance)==1 and maintenance[0]['maintenance_status']=='ok'
 assert maintenance[0]['shm_n_backfill_le_u32_at_96']==maintenance[0]['shm_mx_frame_le_u32_at_16']
else:assert not maintenance
out={'mode':mode,'scope':'Fresh-process RW reopen of independently prepared, byte-identical100001node/100000vector fixture; not two-open lifecycle qualification',
 'parent_guard':{k:guard[k] for k in ('exit','reason','seconds','peak_rss','admission_cpu','admission_free','cleanup_complete')},
 'query0':result['query0'],'query0_memory':result['query0_memory'],'first_props':props[0],'reopen_memory':result['second_open_memory'],
 'final_drop':result['second_drop_memory'],'backup':backup,'maintenance':maintenance,'projection_stages':[e for e in opened if e.get('event')=='projection_sync_candidate_stage'],
 'search10':result['query_samples_k10'],'search100':result['query_samples_k100'],'coverage':result['coverage_bench'],
 'counts':[100001,100000],'marker_recovered':True,'source_inputs_unchanged':True,'after_deadline_gate_passed':mode=='after',
 'recall_scope':'Reported fixed24queries; compare actual before/after values and realBGE contracts, not stateok alone'}
(r/f'fresh-{mode}-analysis.json').write_text(json.dumps(out,indent=2)+'\n');print(json.dumps(out,indent=2))
