from pathlib import Path
import hashlib,json,difflib
r=Path(__file__).parent
bench=(r/'reopen100k-vector_memory.rs').read_text()
old_assert='            assert_eq!(repeat, 2, "GB_MEMORY_OPEN_MUTATE_BEFORE_REOPEN=1 requires exactly two opens");'
assert bench.count(old_assert)==1
bench=bench.replace(old_assert,'''            let prepared = std::env::var_os("GB_MEMORY_PREPARED_MARKER_ID").is_some();
            let prepare_only = std::env::var_os("GB_MEMORY_PREPARE_MARKER_ONLY").is_some();
            assert!(!(prepared && prepare_only), "fixture preparation and reopen are separate workloads");
            assert_eq!(repeat, if prepared || prepare_only { 1 } else { 2 }, "repeat must match the actual open count");''')
start=bench.index('    stage("first_open_start"',bench.index('fn run_mutate_before_reopen'))
end=bench.index('    stage("second_open_start"',start)
original=bench[start:end]
new='''    // Prepared fixtures were created in a separate process through add_node +
    // normal Drop, with no queries. Fresh reopen has no retained Storage/cache.
    let prepared_marker = std::env::var("GB_MEMORY_PREPARED_MARKER_ID").ok();
    let fresh_process = prepared_marker.is_some();
    let (marker_id, first_open_memory, marker_memory, first_drop_memory) =
        if let Some(marker_id) = prepared_marker {
            assert!(!marker_id.is_empty(), "prepared fixture requires a marker id");
            stage("prepared_fixture", json!({"marker_id":marker_id,"no_warmup":true}));
            (marker_id, Value::Null, Value::Null, Value::Null)
        } else {
'''+''.join('        '+l+'\n' for l in original.rstrip().splitlines())+'''            (marker.id, first_open_memory, marker_memory, first_drop_memory)
        };
    if std::env::var_os("GB_MEMORY_PREPARE_MARKER_ONLY").is_some() {
        return json!({"state":"fixture_prepared","method":"one RW open, legitimate non-vector add_node, normal Drop; no queries",
            "marker_id":marker_id,"input_n":input_n,"input_dim":dim,"expected_nodes_after_marker":100_001,
            "expected_vector_nodes":100_000,"first_open_memory":first_open_memory,"marker_memory":marker_memory,
            "first_drop_memory":first_drop_memory,"repeat_count":1,"queries_executed":0});
    }

'''
bench=bench[:start]+new+bench[end:]
tail=bench.index('    stage("second_open_start"',start)
prefix=bench[:tail];body=bench[tail:]
body=body.replace('storage.node_view(&marker.id)','storage.node_view(&marker_id)').replace('"marker_id":marker.id','"marker_id":marker_id')
body=body.replace('"repeat_count":2,','"repeat_count":if fresh_process {1} else {2},').replace('"same_process":true,','"same_process":!fresh_process,')
body=body.replace('"mutation_mode":"GB_MEMORY_OPEN_MUTATE_BEFORE_REOPEN=1",','"mutation_mode":if fresh_process {"prepared fixture, fresh RW process"} else {"GB_MEMORY_OPEN_MUTATE_BEFORE_REOPEN=1"},')
qstart='    stage("query0_start",'
assert body.count(qstart)==1
body=body.replace(qstart,'    std::env::set_var("GB_VECTOR_PROPS_QUERY_DIAGNOSTICS", "1");\n'+qstart,1)
qdone='    stage("query0_complete", json!({"query":query0,"memory":query0_memory}));'
assert body.count(qdone)==1
body=body.replace(qdone,qdone+'\n    std::env::remove_var("GB_VECTOR_PROPS_QUERY_DIAGNOSTICS");',1)
bench=prefix+body
bench_path=r/'fresh-reopen-vector_memory.rs';bench_path.write_text(bench,newline='\n')
sources={}
for variant in ('before','after'):
 src=(r/f'{variant}-traced-lib.rs').read_text()
 pos=src.index('    fn projection_props_unlocked(')
 anchor='        let diagnostics = std::env::var_os("GB_VECTOR_QUERY_DIAGNOSTICS").is_some();'
 at=src.index(anchor,pos)
 changed=src[:at]+src[at:].replace(anchor,anchor[:-1]+'\n            && std::env::var_os("GB_VECTOR_PROPS_QUERY_DIAGNOSTICS").is_some();',1)
 assert changed.replace(anchor[:-1]+'\n            && std::env::var_os("GB_VECTOR_PROPS_QUERY_DIAGNOSTICS").is_some();',anchor,1)==src
 p=r/f'fresh-{variant}-traced-lib.rs';p.write_text(changed,newline='\n')
 sources[variant]={'path':str(p),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()}
 (r/f'fresh-{variant}-observer-only.patch').write_text(''.join(difflib.unified_diff(src.splitlines(True),changed.splitlines(True),fromfile='prior-diagnostic-lib',tofile='query-only-props-diagnostic-lib')))
out={'bench':{'path':str(bench_path),'sha256':hashlib.sha256(bench_path.read_bytes()).hexdigest()},'sources':sources,
 'scope':'External diagnostic/harness only; production D/C engine unchanged. Identical new observer gate in both variants. No query/data/budget correctness gate removed.',
 'old_workload':'Original same-process two-open1200s timeout retained, not qualified by these new workloads.',
 'new_workloads':['fixture preparation: one open + add_node + normal Drop, zero queries','fresh-process RW reopen includes ALL snapshot/replay/backup/maintenance work, q0first,24k10/k100,coverage/count/marker checks,normal Drop'],
 'caps':'Each workload1200s/2GiB; engine query5000ms; no warmup; no parent timer reset or full-two-open pass claim'}
(r/'fresh-reopen-variant-metadata.json').write_text(json.dumps(out,indent=2)+'\n')
print(json.dumps(out,indent=2))
