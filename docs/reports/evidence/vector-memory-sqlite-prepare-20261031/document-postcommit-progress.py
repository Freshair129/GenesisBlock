from pathlib import Path
import json
r=Path(__file__).parent;p=Path(r'D:\GenesisBlock_Dev\GenesisBlock\docs\reports\GENESISDB-VECTOR-MEMORY-AUDIT.md')
s=p.read_text();marker='<!-- SQLITE_POSTCOMMIT_START -->'
assert marker not in s
text='''
<!-- SQLITE_POSTCOMMIT_START -->
### Authorized checkpoint commit and subsequent workload qualification

Audit checkpoint: `f98a324c6ac9c16bbd98679622012519107628a3` on the existing
branch. It commits 1,742 audit/runtime/test/report/evidence paths and excludes all
seven pre-existing staged H2a paths. Their cached patch is byte-identical before
and after commit. All 287 qualified working-tree source hashes remain unchanged.
The recorded 1,328-test qualification includes the additional uncommitted H2a
tests; the original committed tests are unchanged as their prefix. This is a
source-identity bridge, not a claim of a fresh whole-suite CI run at the new HEAD.
The scoped evidence `.gitattributes` preserves verbatim bytes under Windows
`core.autocrlf`; all 1,730 evidence blobs matched their source bytes before commit.

The first actual same-process mutation/two-open 100k workload hit its original
1,200-second cap (controller receipt 1,200.774 s including bounded cleanup),
with sampled tree RSS peak 750,157,824 B. It reached a genuine second-open
`marker_miss`, `force_rebuild=false`, but did not reach backup-complete/query 0.
First open took 420.429 s; the legitimate non-vector insert took 0.0023 s;
normal Drop/snapshot took 76.201 s. Later projection replay took 103.545 s,
edge-history rebuild 51.385 s and backfill 1 us. This is **TIMEOUT / query deadline
NOT_EXERCISED**, not a pass or evidence that the maintenance fix failed.
The owned process tree was terminated with complete cleanup; no unrelated
process was stopped. Raw artifacts are retained under the relocated
`100k-two-open-attempt1/`, mapped by `100k-archive-relocation.json`.

The diagnostic observer emitted 100,001 per-property messages during replay,
making the raw trace 37,144,276 B. The next paired diagnostic variants use the
same extra observer gate in both binaries: per-property trace is enabled only
around query 0. SQL, data, query budgets and retrieval code remain unchanged.
This removes observer allocations/I/O; it is not an engine RAM/performance win.
Both variants build with Rust 1.96 and the same pinned dependencies/release
profile in a private clone. Production source remains the committed `5b8bb9...`.

The requested separated-workload measurement now distinguishes:

- **Fixture preparation:** one RW open, one non-vector `Storage.add_node`, normal
  Drop, zero search queries. It creates a valid closed snapshot with 100,001
  nodes / 100,000 vectors through the real API; no SQLite/P6 edits.
- **Fresh-process RW reopen:** byte-identical copies of that fixture; all snapshot
  loading, marker-miss replay, backup, maintenance, query 0, 24 k10/k100 queries,
  coverage/count/marker checks and normal Drop remain inside its own unchanged
  1,200 s / 2 GiB parent job. Query 0 remains first, with the 5,000 ms deadline.

This does not qualify or rename the failed same-process two-open lifecycle.
Preparation time and fresh-reopen time are reported separately; their sum is not
presented as a lifecycle pass. No warm-up query or OS-cache-cold claim is made.
New fixture/fresh-reopen and real-BGE results remain pending until their actual
guarded executions complete. Recurrent unrelated Rust 1.99.1 compiler activity
has blocked additional admissions; those processes were preserved.
<!-- SQLITE_POSTCOMMIT_END -->

'''
s=s.replace('<!-- SQLITE_PREPARE_FOLLOWUP_END -->',text+'<!-- SQLITE_PREPARE_FOLLOWUP_END -->')
p.write_text(s,encoding='utf-8',newline='\n')
(r/'postcommit-scope.json').write_text(json.dumps({'commit':'f98a324c6ac9c16bbd98679622012519107628a3','same_process_two_open':'TIMEOUT, query not exercised','fresh_workloads':'pending; distinct scope, unchanged caps','production_source_mutation':'none'},indent=2)+'\n')
print('Checkpoint commit, actual timeout and distinct measurement scope documented.')
