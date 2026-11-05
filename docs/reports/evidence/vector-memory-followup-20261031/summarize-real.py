from pathlib import Path
import json,hashlib,datetime
r=Path(__file__).parent
run=r/'followuproot/vector-real-release-controller/runs/real-bge-release-pair-20261031T073308Z'
summary=json.loads((run/'run-summary.json').read_text(encoding='utf-8-sig'))
assert summary['status']=='complete',summary['status']
jobs={j['stage']:j for j in summary['jobs']}
assert set(jobs)=={'baseline','candidate'}
def sha(path):
 h=hashlib.sha256()
 with path.open('rb') as f:
  while block:=f.read(1024*1024):h.update(block)
 return h.hexdigest()
vectors={}
for stage,j in jobs.items():
 path=run/j['label']/'gdb/vec_bench.bin'
 actual=sha(path)
 item=next(c for c in j['output_manifest']['components'] if c['path']=='vec_bench.bin')
 assert actual==item['sha256']
 vectors[stage]={'bytes':path.stat().st_size,'sha256':actual}
before=jobs['baseline']['metrics'];after=jobs['candidate']['metrics']
fields=['ingest_s','rebuild_s','compaction_s','snapshot_s','reopen_s','fresh_read_only_reopen_s',
 'ingest_peak_rss_bytes','rebuild_peak_rss_bytes','compaction_peak_rss_bytes','snapshot_peak_rss_bytes','reopen_peak_rss_bytes',
 'search10_p50_us','search10_p95_us','search10_p99_us','search100_p50_us','search100_p95_us','search100_p99_us',
 'search10_recall','search100_recall','reopen_search10_recall','disk_bytes']
out={'recorded_at':datetime.datetime.now().astimezone().isoformat(),'run':str(run),'status':summary['status'],
 'scope':'one serial real-embedding F32 pair; 24 fixed queries; not statistical tail or semantic-label qualification',
 'vector_files':vectors,'vector_bytes_equal':vectors['baseline']==vectors['candidate'],
 'comparison':{f:{'before':before[f],'after':after[f],'delta_percent':(after[f]/before[f]-1)*100 if before[f] else None} for f in fields},
 'jobs':{stage:{k:j[k] for k in ('status','elapsed_seconds','admission','peak_process_tree_working_set_bytes','peak_process_tree_private_bytes')} for stage,j in jobs.items()}}
for stage,j in jobs.items():
 rows=[json.loads(line) for line in (run/j['label']/'process-timeline.jsonl').read_text(encoding='utf-8-sig').splitlines()]
 root_pid=rows[0]['process_tree'][0]['pid']
 out['jobs'][stage]['root_pid']=root_pid
 out['jobs'][stage]['os_root_peak_working_set_bytes']=max(p['peak_working_set_bytes'] for row in rows for p in row['process_tree'] if p['pid']==root_pid)
(r/'isolated-real-comparison.json').write_text(json.dumps(out,indent=2)+'\n')
print(json.dumps(out,indent=2))
assert out['vector_bytes_equal']
