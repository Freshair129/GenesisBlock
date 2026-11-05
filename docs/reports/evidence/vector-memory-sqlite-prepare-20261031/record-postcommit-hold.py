from pathlib import Path
import json,datetime
r=Path(__file__).parent
wait=json.loads((r/'fixture-preparation-wait-status.json').read_text(encoding='utf-8-sig'))
assert wait['state']=='resource_wait_expired_no_launch'
case=r/'100k-fixture-preparation'
assert not any((case/n).exists() for n in ('run-receipt.json','result.json','stdout.log','stderr.log'))
samples=json.loads((r/'fixture-preparation-guard/admission.json').read_text(encoding='utf-8-sig'))
avg=sum(s['cpu'] for s in samples)/len(samples);free=min(s['free'] for s in samples)
out={'recorded_at':datetime.datetime.now().astimezone().isoformat(),'status':'RESOURCE_HELD_NOT_COMPLETE','checkpoint_commit':'f98a324c6ac9c16bbd98679622012519107628a3',
 'same_process_two_open':{'status':'TIMEOUT','query0_exercised':False,'parent_seconds':1200.7737842,'peak_tree_rss_bytes':750157824,'cleanup_complete':True},
 'fresh_variants':'BUILT: both pinnedRust1.96, observer scoped identically toq0','fixture_setup':'NOT_LAUNCHED: bounded resource wait expired',
 'last_admission':{'cpu_average_percent':avg,'minimum_available_ram_bytes':free,'required_cpu_below_percent':20,'required_minimum_ram_bytes':6*1024**3},
 'fresh_rw_reopen_100k':'NOT_RUN','current_real_bge_pair':'NOT_RUN','new_memory_savings_MB_or_percent':'NOT_MEASURED',
 'current_production_engine':'unchanged5b8bb9...; verified287source paths and253Cmirror paths; sevenH2a staged paths preserved',
 'no_deployment_or_production_database_change':True,'full_original_two_open_qualification':'OPEN; no split-workload substitute claim'}
(r/'postcommit-resource-hold.json').write_text(json.dumps(out,indent=2)+'\n')
summary=json.loads((r/'qualification-summary.json').read_text());summary['reopen_100k']='NOT_QUALIFIED: two-open TIMEOUT beforeq0; fresh-process fixture preparation not launched due resources';summary['postcommit_hold']='postcommit-resource-hold.json'
(r/'qualification-summary.json').write_text(json.dumps(summary,indent=2)+'\n')
p=Path(r'D:\GenesisBlock_Dev\GenesisBlock\docs\reports\GENESISDB-VECTOR-MEMORY-AUDIT.md');s=p.read_text()
text=f'''\nLatest post-commit resource disposition (2026-11-01): the bounded preparation\nwait ended after nine admission attempts without opening the relocated pristine\nfixture. Last 10-second admission averaged {avg:.2f}% CPU with\n{free:,} B ({free/1024**3:.2f} GiB) available RAM, against unchanged <20% /\n>=6 GiB thresholds. No preparation result/query exists. Initial StrictMode config\nrejections were infrastructure errors with no launch; explicit original defaults\n(`kind=benchmark`, `cap_seconds=1200`) fixed the schema, and future waiters fail\nfast on non-resource exceptions. These are not benchmark passes. Both fresh\ndiagnostic binaries built successfully under pinned Rust 1.96; fresh 100k\nquery deadline/quality and the new real-BGE pair remain NOT_RUN. No new RAM\nsavings percentage or MB delta is claimed. See `postcommit-resource-hold.json`.\n'''
s=s.replace('<!-- SQLITE_POSTCOMMIT_END -->',text+'<!-- SQLITE_POSTCOMMIT_END -->')
p.write_text(s,encoding='utf-8',newline='\n')
resume=(r/'RESUME.txt').read_text();(r/'RESUME.txt').write_text('LATEST: fixture resource waiter EXPIRED with NO LAUNCH; no owned child remains. See postcommit-resource-hold.json. Both fresh variants BUILT. To resume, waitfor original CPU/RAM/compiler admission, then run fixture-preparation-guard-config.json (no run artifacts exist). Never relabel original two-open TIMEOUT.\n\n'+resume,encoding='utf-8',newline='\n')
print(json.dumps(out,indent=2))
