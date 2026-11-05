import json,re,pathlib
r=pathlib.Path(r'C:\Users\suanr\genesis-vector-memory-20261031');a=json.loads((r/'final-bounded-tests.json').read_text())
counts={'passed':0,'failed':0,'ignored':0};limited=[];failures=[];completed=[]
for d in a:
 log=(r/'final-bounded-tests'/d['log']).read_text(errors='replace')
 if d['reason']:limited.append(d);continue
 matches=list(re.finditer(r'test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;',log));assert matches,d
 m=matches[-1]
 for k,v in zip(counts,m.groups()[1:]):counts[k]+=int(v)
 completed.append(d['target'])
 if d['exit_code']!=0:failures.append(d)
s={'source_version':'candidate-final-proof-sharing','runner_complete':(r/'final-bounded-tests.status').exists(),'targets_attempted':len(a),'completed_targets':len(completed),'counts_in_completed_targets':counts,'limited_targets':limited,'failed_targets':failures,'node_full':{'passed':88,'failed':0,'status':(r/'final-node-tests.status').read_text().strip()},'qualification':json.loads((r/'final-qualification.status').read_text()),'dts_status':(r/'final-dts-check.status').read_text().strip(),'native_smoke_status':(r/'final-napi-smoke.status').read_text().strip()}
s['focused_concurrency']={'complete':(r/'final-concurrent-focused.status').exists(),'cases':json.loads((r/'final-concurrent-focused.json').read_text()) if (r/'final-concurrent-focused.json').exists() else []}
recovery=json.loads((r/'final-recovery-focused.json').read_text()) if (r/'final-recovery-focused.json').exists() else []
rc={'passed':0,'failed':0,'ignored':0}
for d in recovery:
 if d['reason'] or d['exit_code']!=0:continue
 m=list(re.finditer(r'test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;',(r/'final-recovery-focused'/d['log']).read_text(errors='replace')))[-1]
 for k,v in zip(rc,m.groups()[1:]):rc[k]+=int(v)
s['focused_recovery']={'complete':(r/'final-recovery-focused.status').exists(),'counts_in_completed_subsets':rc,'cases':recovery}
(r/'final-test-summary.json').write_text(json.dumps(s,indent=2)+'\n')
print({k:v for k,v in s.items() if k not in ('limited_targets','failed_targets')})
for d in limited:print('LIMITED',d['target'],d['reason'])
for d in failures:print('FAIL',d['target'])
