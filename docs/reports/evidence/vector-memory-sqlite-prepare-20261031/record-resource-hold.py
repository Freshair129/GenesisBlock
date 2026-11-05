from pathlib import Path
import json,datetime
r=Path(__file__).parent
out={'recorded_at':datetime.datetime.now().astimezone().isoformat(),'status':'resource_hold_not_complete',
 'completed':['final immutable159-target correctness1328/0/7','Node88/0/0 and all8static/native gates','three private release variants','isolated pinnedSQLite causal matrix'],
 'pending':['full100kcopy hash verification','actual100kbefore/after first-query deadline and correctness','new realBGEuntraced performance/recall/RSS pair'],
 'latest_host_sample':{'at':'2026-10-31T22:48:46.2189598+07:00','active_compiler_processes':6,'cpu_percent_instantaneous':43,'available_ram_bytes':11527102464},
 'admission_failures':[{'task':'matrix isolated attempt1','cpu_average_percent':22.0909090909091},
 {'task':'100kcopy verification attempt1','cpu_average_percent':35.1818181818182},
 {'task':'100kcopy verification attempt2','reason':'competing compiler preflight'}],
 'actual_arrival_hold':{'status':'100k-copy-preparation-final/status.json','seconds':80.1478792,'reason':'resource_competition','owned_cleanup_complete':True},
 'case_copies':{'each_file_count':109,'each_bytes':12689795140,'size_lists_equal':True,'full_hash_qualified':False,'database_opened':False},
 'limits_unchanged':{'query_deadline_ms':5000,'timed_job_seconds':1200,'tree_rss_cap_bytes':2147483648,'admission_cpu_average_below_percent':20,'admission_window_s':10,'min_available_ram_bytes':6442450944},
 'new_ram_savings_claimed':{'MB':0,'percent':0,'reason':'Checkpoint I/O moved to publication, not eliminated'},
 'resume':'RESUME.txt; complete-100k-copy-verification.py then frozen case configs; no warmup, no unsafe dedup, no production DB or deployment'}
(r/'latest-resource-hold.json').write_text(json.dumps(out,indent=2)+'\n')
s=json.loads((r/'qualification-summary.json').read_text());s['reopen_100k']='NOT_RUN: copies exist but full hash verification and engine run await resource admission';s['real_bge_current_pair']='NOT_RUN: source-bound configs/binaries ready, resource admission pending';s['latest_resource_hold']='latest-resource-hold.json'
(r/'qualification-summary.json').write_text(json.dumps(s,indent=2)+'\n')
p=Path(r'D:\GenesisBlock_Dev\GenesisBlock\docs\reports\GENESISDB-VECTOR-MEMORY-AUDIT.md');text=p.read_text()
key='Remaining timing gates: the matched'
note='''Latest resource hold (22:48 local time): six external compiler processes remained,
instantaneous host CPU was 43%, and available RAM was 11,527,102,464 bytes.
Earlier fresh 10-second admissions failed at 22.09% and 35.18% average CPU; a later
attempt was rejected before sampling because compilers were active. The first
100k copy/hash job was interrupted at 80.148 s when an unrelated Cargo arrived;
the current monitor stopped only its owned Python and recorded complete cleanup.
No SQLite/Storage open occurred. Both copies contain 109 files / 12,689,795,140 B
with identical file-size lists, but **this is not full hash qualification**.
`complete-100k-copy-verification.py` must finish byte hashes against the unchanged
closed source before either engine case can start. The source-bound run configs,
frozen binaries and strict result parsers are ready. The optional resource-window
question remains pending; no permission or elapsed time substitutes for admission.
No benchmark deadline, RSS limit, correctness gate or CPU threshold was widened.
The 100k deadline and current real-BGE performance remain explicitly open.

'''
assert key in text
text=text.replace(key,note+key)
p.write_text(text,encoding='utf-8',newline='\n')
print('Recorded actual resource hold; pending gates remain open.')
