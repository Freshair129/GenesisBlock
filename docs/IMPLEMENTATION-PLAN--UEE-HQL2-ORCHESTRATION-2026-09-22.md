---
version: "0.8.56b"
doc_id: "IMPLEMENTATION-PLAN--UEE-HQL2-ORCHESTRATION-2026-09-22"
owner: "Boss (Founder / Product Authority)"
created_at: "2026-09-22T00:00:00+07:00,ATHER,working-tree"
last_update: "2026-10-03T19:06:00+07:00,ATHER"
status: beta
superseded_by: null
attributes:
  doc_type: "spec"
  domain: "database-architecture"
  scope: "UEE-HQL2 staged adoption and conflict-free orchestration"
  complexity: "C-3"
  risk: "HIGH"
  owner: "Boss (Founder / Product Authority)"
---

# UEE-HQL2 Orchestration Plan

สถานะเอกสารนี้คือ `beta` และเป็น workflow/แผนงานที่ใช้กำกับ execution แบบมี gate
เท่านั้น P4/P5 ได้รับ owner approval แล้ว และ P6 contract พร้อม architecture correction ได้รับ
อนุมัติให้เริ่ม implementation แล้ว รายการ approval เก่าด้านล่างเป็นประวัติของแต่ละช่วง
เมื่อ 2026-09-28 owner อนุมัติ [HQL2 execution ADR](adr/ADR--GENESISDB-HQL2-EXECUTION-BOUNDARY.md)
ให้ทำ P7 และ explicit-v2 execution ต่อในขอบเขตที่กำหนด โดยยังรักษา Verify/Review/Final gates
ส่วน merge, deploy และการ migrate ฐานข้อมูลผู้ใช้ต้องได้รับอนุญาตแยก
เมื่อ 2026-09-28 owner อนุมัติ [H2-D11 durable revision/annotation contract](adr/ADR--GENESISDB-HQL2-DURABLE-REVISIONS-ANNOTATIONS.md)
ให้ sync P8/P6/C4 และทำ implementation slice ตามหัวข้อ 10 โดยยังไม่อนุญาตให้รัน migration กับฐานข้อมูลผู้ใช้
เมื่อ 2026-10-02 owner อนุมัติ [P8 completion addendum](adr/ADR--GENESISDB-HQL2-P8-COMPLETION-ADDENDUM.md)
ให้ทำ D1-D6 ต่อใน worktree แยก โดยคง P6 authorization-before-parse, Compact wire
และลำดับ P9-P13; approval นี้ไม่อนุญาต user database migration, merge, release หรือ deploy

## 1. Decision ที่เสนอ

ไม่ควรนำ UEE-HQL2 Blueprint ทั้งชุดมา implement ตอนนี้ ควรรับเป็น **staged target**:

1. ทำ truth/obligation ledger และ architecture/compatibility ADR ก่อน
2. ปิดช่องว่างและพิสูจน์ U1-U3 ของ current WAL + SQLite projection + unified transaction
3. ค่อยตัดสินใจ G1-G3 (canonical durability, snapshots, exact reference engine)
4. ทำ G4-G6 (HQL2/planner/composition) ต่อเมื่อมี ADR ใหม่อนุมัติ เพราะ HQL v2 ปัจจุบัน
   ห้าม planner/EXPLAIN และ current Query IR เป็น `query-ir.v1`
5. ทำ G7-G10 หลัง exactness, lifecycle, surface parity และ migration contract ผ่านแล้ว

Blueprint package ตรวจผ่าน 9/9 package checks แต่มี engine obligations 190 รายการเป็น
`not_run_engine`; จึงยังไม่ใช่ production/readiness evidence

## 2. Evidence and assumptions

- Current repo มี WAL authority, SQLite/graph/vector projections และ Query IR v1 บางส่วน
  แต่ HQL ยังมี direct dispatch; `match_path`/`relational_named_query` ยังไม่ครบ
- `docs/MASTER_PLAN.md`, `docs/SPEC--GENESISDB-TYPED-QUERY-IR-V1.md`,
  `docs/SPEC--GENESISDB-UNIFIED-OPERATIONAL-BOUNDARY-V1.md` เป็น parent/peer constraints
- `33_TASK_BREAKDOWN.md` และ `36_TASK_EXECUTION_ORDER.md` ระบุว่า superseded และไม่ใช่
  dispatch SSOT
- U2/U3 มีความไม่ตรงกันระหว่าง doc status กับ source/test evidence ต้อง reconcile ใน P1
- Protected caller WIP ห้ามแตะ/clean/revert:
  `docs/REVIEW--GRAPH-VECTOR-SYSTEM-2026-09-07.md`,
  `scripts/docs-validate.mjs`, `tests/zz_probe_discriminates.rs`
- Current checkout is dirty by design. The three caller WIP paths above and this
  task-owned candidate plan are excluded from implementation worktrees; no worker may
  normalize, stage, revert or merge over them. Current observed status is:
  `M docs/REVIEW--GRAPH-VECTOR-SYSTEM-2026-09-07.md`,
  `M scripts/docs-validate.mjs`, `?? tests/zz_probe_discriminates.rs`,
  `?? docs/IMPLEMENTATION-PLAN--UEE-HQL2-ORCHESTRATION-2026-09-22.md`.
- `docs/.doc-graph.json` ยังไม่มี; สถานะนี้ต้องถือเป็น planning limitation ไม่สร้าง graph ใหม่
  แบบเดาเองใน workflow นี้

## 3. Execution DAG

```text
P0 BASELINE (done, read-only)
  -> P1 TRUTH-LEDGER
  -> P2 ARCH-ADR [human approval barrier]
  -> P3 CONTRACT-FREEZE [review barrier]
  -> P4 G1-DURABILITY
  -> P5 U2-U3-TXN
  -> P6 G2-SNAPSHOT
  -> P7 G3-ORACLE
  -> P8 G4-QUERY
  -> P9 G5-PLANNER
  -> P10 G6-COMPOSE
  -> P11 G7-INDEX
  -> P12 G8-RUNTIME
       |-> P13 G9-SURFACE --|
       |-> P14 G9-MIGRATION -|-> P15 G10-SECURITY -> P16 G10-QUAL
```

### Task definitions

| ID | Scope / owner boundary | Depends on | Task acceptance |
|---|---|---|---|
| P0 | Capture branch/status/WIP and validate blueprint package | — | Evidence captured; no caller WIP changed; done read-only |
| P1 | Build 190-obligation status/traceability ledger; docs only | P0 | Every obligation has state, owner, dependency and deterministic command |
| P2 | Write ADRs for staged adoption, versions, WAL/migration, snapshot/ACL/exactness and HQL1/HQL2/planner | P1 | Parent and peer docs agree; owner decision is explicit |
| P3 | Freeze closed contracts, fixtures, error/result semantics and compatibility boundary | P2 | Contract review passes; no implementation mixed in |
| P4 | Prove or close canonical journal/blob/projection/receipt/recovery gaps | P3 | Crash-window, idempotent replay, cold reopen and migration tests pass |
| P5 | Stabilize relational U2 and unified U3 transaction/stable frontier | P4 | Row+graph+vector commit/retry/reopen semantics pass |
| P6 | Generation publication, leases, temporal and ACL visibility | P5 | No mixed snapshot; pinned generation and authorization tests pass |
| P7 | Exact reference interpreter/oracle and golden fixtures | P6 | NULL/bag/temporal/annotation semantics are executable and reproducible |
| P8 | HQL1/HQL2/IR lowering to one pipeline plus truthful EXPLAIN/counters and H2-D11-backed source adapters | P7, approved P8 and H2-D11 contracts | Shared binder/runtime; each storage source passes identity, temporal, retention, ACL and exact-oracle gates |
| P9 | Legal B-tree/annotation paths and planner access reporting | P8 | Pushdown/order counterexamples match oracle; plans are truthful |
| P10 | Cross-domain graph/vector/relational/annotation composition | P9 | One snapshot, exact-oracle parity, no internal network/JSON workaround |
| P11 | HNSW/lexical lifecycle, generations, deltas, watermarks and coverage | P10 | Approximate results are explicit; lifecycle and stale-index gates pass |
| P12 | Budgets, cancellation, spill, cleanup and cost model | P11 | Resource limits preserve correctness and leave no spill residue |
| P13 | REST/NAPI/FFI/SDK/MCP/mobile parity and clean consumer proof | P12 | Declared surfaces pass differential and install/lifecycle checks |
| P14 | Backup/restore, shadow parity, new-directory migration and rollback | P12 | Rehearsal is reversible and independently verified |
| P15 | Security/namespace/ACL/archive/diagnostic/authorization review | P13, P14 | No unresolved high-risk security or policy gap |
| P16 | Crash, soak, benchmark, device/hosted/release and traceability qualification | P15 | Epic DoD is met; external claims are evidence-backed |

Preparation-only sidecars may run after P2/P3 in parallel: new oracle fixtures, per-surface
contract fixtures, benchmark harnesses, and mobile/self-host qualification scripts. They cannot
certify or merge over an unmet dependency.

## 4. Conflict and worktree policy

| Domain | Rule |
|---|---|
| H0 protected WIP | Never edit, stage, clean, revert or resolve over the three caller WIP paths |
| H0b task-owned plan | Only the orchestrator may update this candidate after a gate correction; workers review it read-only |
| H1 `src/lib.rs` | One writer only; P4-P12 are serialized even if analysis is parallel |
| H2 `src/query/*` | One owner for P8-P10; no concurrent grammar/AST edits |
| H3 parent/ADR docs | One documentation owner for P1-P3; reconcile before code |
| H4 tests/fixtures | New file per task; no co-edit of an existing test file |
| H5 transport surfaces | Assign exclusive file groups: router/main, NAPI/types, FFI/header, SDKs, MCP, mobile/release |
| H6 evidence/benchmarks | One producer per output path; artifacts become immutable at gate |
| H7 CI/release/mobile | One owner for each workflow/configuration file |

Every implementation worker uses a clean isolated worktree from the latest gate-approved
revision. The dirty caller checkout is never used for implementation. A merge conflict is a
failed gate: stop the lane and return it to its owner; never auto-resolve by choosing one side.

### Explicit path-ownership matrix

| Task | Exclusive write set | Merge barrier |
|---|---|---|
| P1 | `docs/IMPLEMENTATION-PLAN--UEE-HQL2-ORCHESTRATION-2026-09-22.md`, new ledger docs only | Phase 1 |
| P2 | New ADR files under `docs/adr/` plus explicitly assigned parent-doc sections | Phase 1 |
| P3 | New contract/fixture files only; no `src/` | Phase 1 |
| P4 | `src/lib.rs` durability sections plus new journal/recovery tests | Phase 2 |
| P5 | `src/lib.rs` transaction sections plus new U2/U3 tests | Phase 2 |
| P6 | `src/lib.rs` generation/snapshot sections plus new snapshot tests | Phase 2 |
| P7 | New oracle/fixture files; any core adapter is serialized after P6 | Phase 2 |
| P8 | `src/query/` and assigned HQL/IR executor sections of `src/lib.rs` | Phase 3 |
| P9 | Assigned planner/index sections of `src/lib.rs` and new planner tests | Phase 3 |
| P10 | Assigned composition sections of `src/lib.rs` and new composition tests | Phase 3 |
| P11 | Assigned index lifecycle sections of `src/lib.rs` and new index tests | Phase 4 |
| P12 | Assigned runtime/budget sections of `src/lib.rs` and new budget tests | Phase 4 |
| P13 | `src/router.rs`, `src/main.rs`, `index.d.ts`, `src/ffi.rs`, SDK/MCP/mobile files, one owner per file group | Phase 4 |
| P14 | Backup/migration implementation files and new migration tests, one owner per file | Phase 4 |
| P15 | New security evidence/tests and assigned security docs only | Phase 4 |
| P16 | Release/qualification evidence and workflow files, one owner per file | Phase 4 |

No task may claim a path already owned by another active task. P13 and P14 may run in parallel
only because their write sets are disjoint; their merge is still held until both Verify Gates pass.

## 5. Worker and gate workflow

```text
Luna Max Worker
  -> RED evidence (before edit)
  -> minimal change in exclusive worktree
  -> GREEN focused regression
  -> Luna Max Verify Gate
  -> Luna Max Review Gate
  -> topological merge
  -> Luna Max Final Gate at P16
```

Each handoff must include task ID, requirement IDs, base revision, changed files, conflict domain,
commands, raw result summary, known failures and external-unverified claims.

Gate checks and exact commands:

- topology (already run): PowerShell Kahn check over `P0..P16`; result `DAG_ACYCLIC=True`.
  This proves topology only; P1-P16 execution has not run.
- every worker: `git status --short --branch`; `git diff --check`
- P1-P3 docs/contracts: `npm run docs:validate`; `npm run agents:validate`
- P4: `cargo test --no-default-features --test journal_format_tests --test journal_migration_tests --test sqlite_substrate_s0_tests --test durability_slice0_tests --test durability_slice1_tests`
- P5: `cargo test --no-default-features --test relational_u2_tests --test relational_u2_contract_tests --test unified_transaction_u3_tests --test wave_a_commit_tests`
- P6: `cargo test --no-default-features --test wave_a_commit_tests --test temporal_queries_tests --test tx_as_of_wp22_tests --test governance_tests`
- P7: `cargo test --locked --no-default-features --test hql2_oracle_tests --test hql2_graph_oracle_tests --test hql2_rank_oracle_tests --test hql2_pipeline_oracle_tests --test hql2_vector_oracle_differential_tests`;
  package reference Python helpers remain supporting evidence only. P7 closes only after
  composed interpreter coverage, golden expected results and independent review pass.
- P8-P10: `cargo test --no-default-features --test query_ir_tests --test hql_p0_tests --test hql_filter_tests --test hql_cypher_tests --test wave_c_query_correctness_tests --test napi_rest_parity_tests --test rest_api_tests`
- P11: `cargo test --no-default-features --test async_indexing_tests --test hnsw_capacity_tests --test hnsw_recall_floor_tests --test multi_collection_tests --test wave_d_quality_artifact_tests`
- P12: `cargo test --no-default-features --test wave_d_budget_tests --test wave_d_rest_tests`
- P13: `npm run build:debug`; `npm test`; `cargo test --no-default-features --test napi_rest_parity_tests --test rest_api_tests`
- P14: `cargo test --no-default-features --test backup_restore_u9_tests --test persistence_tests --test rebuild_from_truth_tests --test meta_format_migration_tests`
- P15: `cargo test --no-default-features --test governance_tests --test hardening_tests`
- P16: `cargo test --no-default-features`; `npm run build:debug`; `npm test`; `cargo build --no-default-features --features mobile`; `cargo bench --bench ldbc_lite`; `cargo run --release --features bins --bin industrial-audit`; `npm run docs:validate`; `npm run agents:validate`
- blueprint package evidence: from the extracted package root, set `PYTHONUTF8=1` and run
  `uv run --with-requirements tools/requirements.txt tools/verify_blueprint.py`

Hosted CI, registry publication, physical-device acceptance, clean external SDK install,
power-loss/crash campaigns, soak, security certification and release assets remain external
evidence; no local command above upgrades them to verified.

## 6. Acceptance levels

### Epic DoD

Approved ADR/spec set, all accepted task ACs have machine-checkable evidence, all 190 obligations
are honestly classified, full regression and required external gates are recorded, no protected
WIP was changed, and no known high-risk contradiction remains.

### Four phase gates

1. **Truth/contracts:** P1-P3 complete and owner-approved.
2. **Durability/semantics:** P4-P7 complete with recovery, snapshot and exact-oracle evidence.
3. **Query/composition:** P8-P10 complete with cross-frontend and exactness evidence.
4. **Qualification:** P11-P16 complete with lifecycle, surface, security and release evidence.

### Task ACs

Each task table row is the minimum task AC. A task is not mergeable without RED/GREEN evidence,
its deterministic verify command, independent review, and an explicit disposition for every failure.

## 7. Approval boundary

Historical phase decisions below remain an audit trail. The 2026-09-28 owner-approved
HQL2 execution ADR now authorizes P7 and explicit-v2 continuation within its stated
gates. The owner subsequently approved P8 typed-boundary version 0.1.1b and
isolated upstream integration with `approve`, then approved H2-D11 on 2026-09-28.
H2-D11 authorizes P8/P6/plan truth-sync and additive schema-v6 code plus fixture tests;
it does not authorize running migration on a user database. The initial scalar/core checkpoint
is recorded in `REPORT--HQL2-P8-CORE-2026-09-28.md`; P8 remains partial and P9-P16
are not qualified by that checkpoint. The user explicitly authorized scoped
branch commit/push and the PR #194 merge. PR #194 is now merged at `4f02d6b`
from head `4f4e298`; its final hosted snapshot reports 48 checks passed, five
failed and five skipped, and no review decision. Three worker OS checks and
the rebuilt Linux addon/worker failed at markerless schema-v6 bootstrap; the
Android GitHub Packages job failed during checkout after the merged PR ref
disappeared. The PR was merged despite those failures; no admin bypass was
used. This does not qualify P8 or authorize deployment/user-database
migration. P9-P16 retain their existing dependency and approval gates.
The subsequently approved D7 addendum now has local implementation evidence for
actor-scoped zero-hop and bounded one-hop HQL1 ID projections through
`Storage::query_v2`, including one endpoint-ID string equality filter:
9/9 focused tests, 382/0/1 across 33 root HQL2 targets and
194/0/0 across 11 separate P6/schema-v6/compatibility targets. The adapter
preflights and reserves parser resources before legacy AST construction; other
HQL1 forms and legacy transports remain unchanged. Independent D7 review and
full shared-runtime/P8/P13 qualification remain open.

The approved HistoryScan slice is implemented and locally verified for all
five supported kinds: Node, Edge, Row, Vector and Annotation. HQL/typed-IR
result bags match independent P7 catalogs assembled from WAL revisions and
captured frontiers/valid-time windows, including endpoint, vector-owner and
annotation-reference ACL dependencies. Storage-backed P7 ChangeScan
differentials now cover all five supported revision kinds with exact sequence,
operation and subject identity; the HQL and typed-IR bags agree.
History/Change passes 16/16,
Annotation source passes 7/7, P7 passes 130/130 and all 33 HQL2 targets pass
382/0/1. The storage-backed `Values`/`UnionAll` P7 differential covers 169
nullable bag pairs and 338 Storage executions while preserving duplicate/NULL
multiplicity. Artifact HistoryScan remains capability-unsupported; broader
ChangeScan/source-oracle semantics, P8/P13 qualification, hosted CI and
independent review remain open.

The approved HQL2/IR transaction-time contract is now implemented in
`Storage::query_v2`: explicit `tx_as_of` selects one frontier S for revision
scans, graph/vector/annotation operators, hydration and `Snapshot.tx`; the
P6 generation/catalog/current policy remain pinned and source floors fail
closed without current-state fallback. Five focused targets pass 56/56, the
33-target HQL2 sweep passes 382/0/1, and the earlier separate 11-target P6/schema-v6/
compatibility sweep passes 194/0/0. These are local regression results only;
broad P8/P13, transport, hosted CI and independent review remain open.

The latest local selected sweeps include storage-backed HQL/typed-IR scalar
and aggregate differentials against independent P7: the scalar target covers
81 four-value nullable bags (162 executions), and the aggregate target covers
121 nullable bags of lengths 0-4 across seven functions (242 executions).
Both pass 1/1. The storage-backed HQL/typed-IR Join differential matches
independent P7 for Inner/Left/Semi/Anti (5/7/3/2 rows), including duplicate,
missing-property NULL and JSON-null keys; bare HQL JOIN defaults to Inner.
The storage-backed HQL/typed-IR `Values`/`UnionAll` differential matches P7
for all 169 pairs from 13 nullable bags, with 338 Storage executions; explicit
null-last ordering preserves NULL and duplicate multiplicity. The focused
target passes 1/1. The explicit 33-target HQL2 sweep passes 382/0/1. These
bounded fixtures do not close broad exact-oracle or P8
acceptance. PR #196 at head `8ac07f6` is merged at `fb7085a`. Its hosted
snapshot has 10 displayed checks passing, five failing and one skipped. Worker
tests fail on Linux/macOS/Windows and the rebuilt Linux addon/worker fails with
`RECOVERY_REQUIRED: markerless database identity is missing`. The Windows
`cargo test` check failed at 15m16; its detailed log is unavailable, so the
cause is not confirmed. No review decision is recorded. Worker startup remains
approval-gated; Windows job-time disposition, broad P8/P13 and release
qualification remain open.

H2-D11 R4/P6 ACL conformance is implemented: the namespace-wide grant authorizes
the HQL2 query boundary and same-namespace target/evidence references, while
annotation source reads, hydration, and Annotation ChangeScan subjects require
explicit Annotation(Read) in the same lease. The approved narrow budget addendum
preserves namespace-only access to other readable ChangeScan subjects and
excludes unauthorized Annotation revisions before candidate counting, byte
reservation and materialization. The regression verifies three hidden
Annotation revisions do not exhaust a one-node budget while one readable Node
is returned. The ACL target passes 11/11; the expanded History/Change target
passes 16/16, Annotation source/ACL passes 7/7, the final 33-target HQL2 sweep
passes 382/0/1, and the separate 11-target P6/schema-v6/compatibility group
passes 194/0/0. Previous hosted worker failures and independent review remain
open; hosted validation of the current branch tip is still required.
No schema/migration change; broader P8/P13, transport and security gates remain
open.

## 8. P5 execution evidence

วันที่ `2026-09-22` ดำเนินการ P5 ในขอบเขต U2/U3 unified transaction, recovery และ
stable-frontier semantics โดยจัดเป็นงานความเสี่ยงสูงด้าน durability/recovery และ signed-WAL
compatibility การเปลี่ยนแปลงที่ตรวจแล้วมีดังนี้:

- รักษา `local_frame_seq` ของ derived transaction checkpoint เพื่อให้ retry หลัง fold และ cold
  reopen ใช้ local receipt sequence เดิม โดยไม่ปะปนกับ `origin_commit_seq`.
- ป้องกันไม่ให้ vector materialization failure ถูกประกาศเป็น `stable: true` และให้ recovery/retry
  ทำงานต่อได้.
- normalize remote fold receipt ตอน replay ที่ปลายทาง และใช้ canonical signature bytes ที่ไม่รวม
  derived local metadata เพื่อคง compatibility กับ WAL/signature เดิม.
- เพิ่ม regression สำหรับ local retry, vector failure/recovery และ replicated cold-reopen receipt;
  ไม่ขยาย scope เข้า P6 snapshot หรือ P8 planner.

หลักฐานเครื่องที่ทำซ้ำได้:

```text
cargo test --no-default-features --test relational_u2_tests --test relational_u2_contract_tests --test unified_transaction_u3_tests --test unified_transaction_p5_tests --test wave_a_commit_tests
=> 29 passed, 0 failed
cargo fmt --all -- --check                         => pass
git diff --check                                  => pass
```

RED evidence ของ compacted-retry regression คือ `left: 3, right: 2`; หลังแก้ไขแล้ว focused
test และ full P5 gate เป็น GREEN Verify Gate รายงาน `VERIFY_PASS` และ Review Gate v2 รายงาน
`REVIEW_PASS` โดยยังคง `tests/zz_probe_discriminates.rs` เป็น protected untracked WIP.

สถานะปัจจุบันคือ **P5 Final Gate ผ่านและ owner approve แล้ว**; P6 ยังอยู่ที่ contract gate.

## 9. P6 pre-implementation contract gate

### 9.1 Architecture finding

P5 approval authorizes entering P6 analysis, but the current parent/peer documents are not yet
specific enough to implement P6 safely. Existing evidence shows:

- `src/lib.rs` already has coherent snapshot/recovery frontiers and atomic snapshot-file handling
  (`save_state`, `compact`, journal replay), but no explicit published `generation_id`, pinning rule,
  stale-generation error, or generation retention contract.
- Temporal visibility already exists for valid-time `as_of` and transaction-time `tx_as_of`, with
  focused tests. The contract does not yet define how those selectors bind to a pinned snapshot
  generation during a cross-domain read.
- `src/router.rs` has process-level `GENESIS_API_KEY` middleware. This is not entity/namespace ACL;
  there is no approved principal, resource, action, policy storage, default-deny, or revocation
  contract for P6.
- Existing governance tiers protect `MASTER` writes, but governance is not an authorization model
  for read visibility and must not be silently reused as ACL.

### 9.2 Proposed bounded contract for owner decision

Before source changes, approve or amend these decisions:

1. **Generation:** one monotonically increasing `generation_id` identifies a coherent publication
   of WAL frontier, SQLite projection, graph state, and vector snapshot metadata. A read may pin one
   generation; mixed-generation results are rejected with a named error.
2. **Lease:** a core-only read lease contains generation identity, owner token, expiry and fencing
   epoch. Expiry/revocation must prevent a stale reader from publishing results. No REST/NAPI/SDK
   surface is added in P6; those belong to the later surface task.
3. **Temporal binding:** `as_of` and `tx_as_of` are evaluated against the pinned generation and
   return a named beyond-horizon/invalid-generation error rather than silently falling back to
   current state.
4. **ACL boundary:** introduce an explicit `AccessContext`/policy contract with principal,
   namespace, action and resource. Decide whether ACL-enabled databases are default-deny and how
   policy metadata is WAL-authoritative, replayable, revocable and snapshot-visible. API-key
   middleware remains transport authentication, not the policy engine.
5. **Compatibility:** no new REST/NAPI/FFI field, disk-format migration, GBF2/GBO2 change, P7
   oracle, P8 planner, P11 index lifecycle, P13 surface parity or P14 backup/restore work is part
   of this P6 slice without a separate approved contract.

### 9.3 Proposed execution DAG after contract approval

```text
P6-DOC  owner decision on generation/lease/ACL contract
  -> P6-A  generation publication + atomic snapshot pinning
  -> P6-B  lease expiry/fencing + stale-generation rejection
  -> P6-C  temporal binding + ACL visibility/replay
  -> P6 VERIFY -> REVIEW -> FINAL -> owner approve P6
```

The `src/lib.rs` write domain remains serialized. New test files are disjoint:
`tests/p6_generation_tests.rs`, `tests/p6_lease_tests.rs`, and `tests/p6_visibility_tests.rs`.
The P6 gate command must be expanded to include those files plus the existing Wave A, temporal,
transaction-as-of and governance suites. No source worker may start before P6-DOC is approved.

### 9.4 Astra architecture-gate correction — owner approved

The P6 contract was owner-approved, but the independent Astra architecture gate returned
`P6_ARCH_NEEDS_CHANGE`. The owner approved the corrected architecture using “approve P6 architecture correction”; implementation is now authorized only within this P6 contract.

1. **Publication ordering:** hold the publication boundary through WAL durability, SQLite/graph/
   vector application, the index flush barrier and component-manifest validation; only then write
   and expose the durable generation publication record. A frontier counter alone is not a
   generation proof.
2. **Fail-closed snapshots:** every component write/rename failure must fail the candidate
   generation. Recovery must validate component identity/frontier/integrity as one unit and fall
   back to WAL replay instead of accepting a partial snapshot.
3. **Lease API:** use an engine-issued opaque `ReadLease` carrying generation, owner token,
   monotonic expiry, fencing epoch, access context and temporal selectors. The core API must expose
   `publish_generation`, `pin_generation`, `validate_lease` and `revoke_lease`; leases expire across
   restart and stale generations are rejected without fallback to live state.
4. **ACL durability:** add a dedicated versioned, signed and revision-checked
   `AccessPolicyChanged` WAL event. Policy state must be included in folds/snapshots and replayed
   before an ACL-enabled read. A minimum-reader/schema guard must prevent older engines from
   silently skipping the event and disabling enforcement.
5. **API shape:** the implementation design must settle `GenerationInfo`, `AccessContext`,
   `TemporalRead`, `AccessAction`, `AccessResource` and the policy administration actor plus
   expected-revision CAS before tests or source are authored. Existing process-level API-key
   middleware and governance tiers remain separate concerns.

The RED test files remain disjoint (`tests/p6_generation_tests.rs`, `tests/p6_lease_tests.rs`,
`tests/p6_visibility_tests.rs`). Astra confirmed risk `HIGH / C-3`; the P6 source write domain
stays serialized.

### 9.5 Approved P6 implementation contract

Durable parent-linked design: SPEC--GENESISDB-P6-GENERATIONS-LEASES-ACL.md.

The following API and compatibility choices make the approved correction executable. They do not
expand P6 into transport/SDK work:

1. **Generation record:** GenerationInfo contains generation_id, covered wal_frontier,
   publication_seq, txn_frontier, history_horizon, acl_revision, and the SHA-256 digest of the
   validated component manifest. The generation snapshot covers state at wal_frontier; the signed
   GenerationPublished frame is appended at publication_seq = wal_frontier + 1. Reuse publication
   only while the stable frontier still equals publication_seq.
2. **Publication API:** publish_generation() -> Result<GenerationInfo> flushes HNSW, writes a
   no-fold snapshot under the commit boundary, validates every component and the manifest, then
   appends the signed publication event. The event changes no graph, vector or relational data.
   Failure before the event leaves the previous generation authoritative. Publishing is lazy: it
   occurs on explicit publish or when pin_generation sees a new frontier, not on every write.
3. **Lease API:** pin_generation(access: AccessContext, temporal: TemporalRead, ttl: Duration) ->
   Result<ReadLease>; validate_lease(&ReadLease) -> Result<()>; revoke_lease(&ReadLease) ->
   Result<()>; and with_read_lease(&ReadLease, FnOnce(&ReadView) -> Result<T>) -> Result<T>.
   ReadLease fields are private (owner token, generation, monotonic expiry, fencing epoch, access
   context, selectors and lease id). Restart changes the owner token. Revocation advances the
   storage fencing epoch and invalidates every outstanding lease for that handle. The commit/read
   boundary stays held through the callback and final lease validation; there is no live fallback.
4. **Temporal binding:** TemporalRead is { as_of: Option<String>, tx_as_of: Option<u64> }.
   as_of must parse as RFC3339; tx_as_of must be at or above the generation history_horizon,
   otherwise return TEMPORAL_BEYOND_HORIZON. Query IR through ReadView inherits pinned selectors;
   conflicting request selectors fail. ReadView operations without temporal support reject a
   temporal lease instead of returning current state.
5. **ACL contract:** AccessPolicy has a revision and Disabled/Enforced mode; grants are exact
   (principal, action, resource) triples. AccessContext is (principal, namespace). AccessAction
   is Read, Write or ManagePolicy; P6 enforces reads and policy administration only. Graph, edge
   and vector records have no namespace field today and therefore belong to the logical default
   namespace. Relational reads authorize their base table and every joined table in the declared
   namespace. A Namespace grant authorizes all resources there; exact Node/Edge/Collection/Table
   grants authorize only that resource. Unsupported namespace/resource combinations fail closed.
6. **Policy administration:** PolicyAdminActor wraps AccessContext.
   replace_access_policy(actor, expected_revision, policy) is compare-and-swap and appends only
   expected_revision + 1. While disabled, bootstrap requires principal local-owner; while
   enforced, the actor needs ManagePolicy on its namespace. The local Rust caller is trusted to
   assert the actor; authentication/identity binding for REST/NAPI/FFI is out of scope. Each
   AccessPolicyChanged event is engine-signed, versioned, replayable, folded and snapshot-visible.
   Applying a newer policy revokes leases pinned to an older ACL revision.
7. **Snapshot integrity and compatibility:** snapshots carry a versioned manifest with a snapshot
   frontier and sorted { path, bytes, sha256 } entries for every required component, including
   projection.sqlite. state.json is the final commit marker and is not self-hashed. Validate safe
   names, required-file set, frontier agreement, byte counts and hashes before mutating memory.
   An invalid P6 snapshot falls back to the complete WAL, never partial instant-load state.
   Propagate component write/rename errors and rename state.json last. SCHEMA_VERSION advances
   from 4 to 5; v4 snapshots without a P6 manifest remain readable via legacy load/replay. Older
   readers fail closed on schema 5, and journal-only reads fail on the unknown signed event. This
   is a reader-compatibility bump, not a data migration.
8. **Boundaries:** no REST/NAPI/FFI/SDK fields or endpoints, write authorization, entity namespace
   migration, retention-policy change, P7 oracle/planner, backup/restore redesign, or merge is part
   of P6. Existing API-key middleware and governance tiers remain separate.

#### P6 acceptance and execution DAG

1. P6-DOC is approved.
2. P6-RED-GENERATION, P6-RED-LEASE and P6-RED-VISIBILITY run in parallel in exclusive test files.
3. P6-SOURCE is one serialized owner of src/lib.rs.
4. P6-VERIFY runs the new tests, Wave A/temporal/transaction-as-of/governance regressions,
   cargo fmt --check, cargo check --no-default-features, documentation validation and
   git diff --check.
5. P6-REVIEW is an independent Luna review. Any finding returns to source, then Verify repeats.
6. P6-FINAL checks exact evidence, scope, WIP preservation and outstanding external gates.
7. Stop for owner approval of P6; do not begin P7 or merge without separate approval.

Each test worker must demonstrate RED before source integration. The three test files have
exclusive ownership; src/lib.rs remains single-writer. The historical RED workers are complete;
remaining Verify, Review and Final gates must use the requested gpt-5.6-luna at Max reasoning and
record the exact model and gate evidence. Protected tests/zz_probe_discriminates.rs
is user WIP and must not be read, modified, staged or deleted.

Final Gate must separate local evidence from unrun CI, performance, device, release and deployment
evidence. Merge is forbidden in this slice.

#### P6 Review Gate return — 2026-09-23

The requested gpt-5.6-luna Max read-only Review returned FAIL and the lane returned to its
serialized source owner. It found: (1) a P1 snapshot ACL materialization that can bypass signature
verification when state.json is edited; (2) a P2 raw budgeted-HQL entrypoint that parses before the
Enforced ACL guard; and (3) missing test coverage for invalid P6 signatures in the legacy JSONL
replay path. The earlier nested-lock P1 is withdrawn: commit_lock is a ReentrantMutex and its
captured-Storage regression passes.

The approved P6 correction authorizes the in-scope fixes. Verify must rerun after them, followed by
an independent Review and Final gate. Peer P6 event authority remains a separate owner decision;
no replication policy is inferred here, and P7/merge/deploy remain out of scope.

Correction and gates completed 2026-09-23: non-default revision-0 ACL snapshots without verified
policy-event provenance now fail closed with `RECOVERY_REQUIRED`; its regression was observed RED
before the source correction and GREEN afterward. The seven scoped Rust integration suites passed
45/45, `cargo check --no-default-features` passed, scoped rustfmt and `git diff --check` passed,
and documentation validation reported 0 violations in 229 files. The requested
`gpt-5.6-luna` at Max reasoning final read-only Review passed with no remaining P1/P2 findings in
the reviewed P6 paths. A fresh release
build with `bins` passed.

Local audit results: `industrial-audit` ingested 10,000 nodes at 16.37 nodes/s;
`hql-query-stress` completed 300 queries in 30.84 ms at 102.81 us/query. Criterion's isolated
rerun measured 1/2/3-hop traversal means of 53.98 us, 384.06 us and 1.364 ms. A first Criterion
comparison attempt used the checkout's existing target/criterion output despite CLI target-dir;
it replaced the `new` samples/reports and promoted the 3-hop sample to `base`, while the 1/2-hop
base files remained unchanged. The exact artifact effect was reported to the owner; no complete
artifact backup was created by this run, and restoration was not attempted because reconstructing
from partial artifacts would be unsafe. The initial comparison printed no significant change for
1/2-hop traversal (p=0.65 for each) and a 47.3% 3-hop improvement (p=0.00 as rendered by Criterion)
against the 2026-09-07 baseline. Subsequent Criterion output was isolated with `CRITERION_HOME`
under the P6 run-root. These local audits are not hosted CI, release, or UAT evidence.

P6 remains pending the owner decision on peer replication authority for generation/ACL events and
explicit owner acceptance of the final P6 result. H2-D11 does not alter that decision. Do not merge.

#### API details required by RED workers

- GenerationInfo fields: generation_id: u64, wal_frontier: u64, publication_seq: u64,
  txn_frontier: u64, history_horizon: u64, acl_revision: u64,
  component_manifest_sha256: String.
- AccessContext fields: principal: String, namespace: String. PolicyAdminActor has access:
  AccessContext. AccessPolicy has revision: u64, mode: AccessPolicyMode (Disabled/Enforced),
  grants: Vec<AccessGrant>. AccessGrant fields are principal, action and resource.
- AccessAction variants: Read, Write, ManagePolicy. AccessResource variants: Namespace(String),
  Node(String), Edge(String), Collection(String), Table { namespace: String, table: String }.
- ReadView exposes node_view(id), node_versions(id, at_seq), neighbors(seed, args, is_inferred),
  hybrid_search(args), execute_query_ir(request), execute_hql(query), and
  query_relational(query). Each returns Result; no Storage reference is exposed to the callback.
  Direct supported Storage reads remain compatible when policy is Disabled and return
  ACCESS_CONTEXT_REQUIRED when Enforced unless entered through a ReadView operation.
- A point-node read requires an exact Node grant or the default Namespace grant. A relational
  query requires a matching Namespace grant or grants for its base Table and every joined Table.
  Composed graph, HQL, Query IR and retrieval reads require the default Namespace grant because
  current graph records have no tenant field. Hybrid search also requires a default-namespace
  grant; a Collection grant alone never authorizes returning associated node records.
- Query IR inherits TemporalRead.as_of as valid_at and tx_as_of as-is. P8 HQL2/typed IR through
  `Storage::query_v2` binds one selected `S` across revision-backed reads and result metadata;
  Neighbors, hybrid search, legacy `execute_hql`, relational reads, node_view and node_versions
  retain their existing selector-rejection boundaries unless separately contracted.
- Stable error prefixes for tests: GENERATION_STALE, LEASE_OWNER_MISMATCH, LEASE_EXPIRED,
  LEASE_REVOKED, TEMPORAL_BEYOND_HORIZON, ACCESS_CONTEXT_REQUIRED, ACCESS_DENIED,
  ACCESS_POLICY_REVISION_CONFLICT, SNAPSHOT_MANIFEST_INVALID and
  SNAPSHOT_COMPONENT_INVALID.

## 10. Owner-approved P8 H2-D11 implementation slice

Owner approval on 2026-09-28 authorizes base H2-D11 and ADR 0.3.0b. Review found
that version insufficiently specified fold retention, old-reader compatibility,
reopen proof and peer isolation. The owner approved amended ADR 0.4.0b on
2026-09-29; this 0.5.0b plan authorizes its implementation using temporary
fixture databases only. No migration against an existing user database,
P9-P16, merge or deployment is authorized.

### Ordered tasks and gates

1. **Revision event foundation:** add typed `RecordRevisionTransactionV1` WAL
   events, engine-issued UUIDv4 identities, local transaction coordinates,
   expected-revision CAS and replay/fold identity preservation. Tests prove
   atomic append, conflict rejection, duplicate/retry behavior and no projection-
   generated identity.
2. **Schema-v6 recovery gate:** add additive projections and a fail-closed
   `upgrade_state=in_progress/ready` reader gate. Build an explicit offline,
   idempotent migration routine with dry-run manifest, chunk resume, final
   signed P6 generation receipt and schema-5 compatibility tests. Use temporary
   fixture directories only; do not invoke it on an existing user database.
   The candidate WAL refinement uses signed local-only
   Schema6MigrationChunkV1/Schema6MigrationCommitV1 events, binds the verified
   backup to the dry-run manifest, and preserves source tx_from/tx_to separately
   from new journal frame sequences. Reopen re-verifies the full chunk/commit
   authority and receipt; compaction preserves/re-emits it; recursive peer
   ingress rejects it and peer export excludes it. The receipt covers the final
   migration-commit frame and post-migration manifest digest. A migration-only
   open resumes only the exact migration ID and manifest digest. Unknown signed
   control events fail closed. The v5-to-v6 migration now advances the disk
   schema version; its cutover and R6b activation/recovery implementation have
   been verified with temporary fixtures only. No user database migration is
   authorized.
   Owner-approved ADR R6b adds a signed local `Schema6ActivationV1` for fresh
   and migrated v6 stores. If `state.json` is absent, preflight every WAL
   source before replay; require complete activation/migration/receipt proof for
   v6, replay only v5-compatible WAL as v5, and return `RECOVERY_REQUIRED` for
   unproven v6-only events. Fold retains activation. Temporary-fixture tests
   cover v5-only, fresh-v6, complete/torn-tail WAL-only, invalid activation,
   migration proof and fold/cold-reopen parity. Crash recovery passes 17/17,
   migration passes 19/19, and the selected 40-target HQL2/durability/authority
   aggregate passes; no user database was opened for migration.
3. **Row/vector revisions:** add registry row IDs and exact KeyCodec v1 while
   preserving nullable SQLite primary-key behavior; version vector values with
   collection-space fingerprints and report unavailable original bytes rather
   than treating quantized data as exact.
4. **Annotation persistence and authorization:** immutable annotation
   revisions, frozen/live targets, selectors and normalized target/evidence
   indexes are implemented. `AccessResource::Annotation` is persisted through
   policy-event v2. AnnotationScan and AnnotationLookup check normalized
   target/evidence references under the P6 lease and hide the whole annotation
   if a reference is missing or unauthorized. The current HQL2 surface still requires the
   namespace-wide query grant; exact-grant-only query authorization is not open.
5. **P8 source adapters:** `RecordRefV2.database_id` and Node/Edge/Row/Annotation
   revision scans now have fixture-backed implementation, bounded keyset pages,
   source filtering, catalog-bound row tables, nullable identity fields and
   property hydration. `FieldIdV2`/`ExecBatchV2` now batches selected fields
   under the lease, validates stable row keys and supports row-dependent names.
   HQL/IR AnnotationLookup now distinguishes exact frozen targets from live
   logical-identity matches at S,V, excludes evidence from lookup matches, and
   implements optional-null versus required-inner behavior with multiplicity.
   Compact and ordered multi-segment Sequence Expand execute in HQL and typed IR
   using exact node/edge revisions in one P6 lease. Each step supports
   direction/relation filters and bounded 0..=32 hops; Walk/Trail/Simple applies
   globally across the sequence. Per-step aliases, optional null extension, one
   typed path, deterministic ordering, fail-closed graph budgets and measured
   expansion counters are verified. Structural root Match now supports compact
   and ordered Sequence patterns through HQL/IR, including one deterministic
   minimum-hop path per endpoint pair for SHORTEST. Exact KNN and Original
   Rerank now execute through one HQL/IR kernel under the same P6 lease.
   Original schema-v6 vector reads validate owner revision, namespace/node ACL,
   H2-D11 collection fingerprint and dimension at S,V; no HNSW, quantized or
   sidecar data is used. KNN skips missing originals, Original Rerank fails
   closed on any missing candidate, and candidate/distance budgets do not
   return partial rows. HQL contextual vector inference maps `space_id` to the
   collection fingerprint; Approx remains fail-closed. HistoryScan/ChangeScan
   now execute over exact retained revisions with source floors and recursive
   current ACL. Sequence node ID/labels and D4 node/edge property constraints
   now execute through HQL and typed IR under the authorized P6 snapshot;
   Compact constraints remain fail-closed. D2/D3 registered lexical and context
   profiles and D5 contextual literals are implemented. The focused P8
   completion target passes 10/10, including edge-property filtering before
   SHORTEST; the explicit 26-root-HQL2 sweep passes 349/0/1. A separate
   11-target P6/schema-v6/compatibility sweep passes 194/0/0. A read-only
   review found and prompted correction of JSON-literal size undercounting;
   confirmation review marked it fixed, and independent static implementation
   review found no concrete defect. Full HQL1/HQL2/IR differential, broad
   oracle/resource and P8/P13 gates remain open. D1 uses exact-record-only
   denial before parsing rather than an ACL-hidden-ID result fixture.
   The focused History/Change target
   passes 9/9; vector-operator target passes 12/12.
   The Blueprint AnnotationPut example and oracle
   also represent and validate `evidence` separately from `targets`.
   The historical post-RowScan sweep of all 21 named HQL2
   integration targets reported 296 passed, 3 failed and 1 ignored. The three
   failures were the two exact-KNN and one original-Rerank runtime tests
   rejected at binding; vector-revision (2) and wire (18) targets passed in a
   separate invocation after Cargo stopped at the failing vector target. The
   earlier 294/3/1 sweep is a historical checkpoint. These counts are local
   regression evidence only, not engine acceptance or P8 closure; the protected
   probe was not compiled; this is local regression evidence, not P8 acceptance.
6. **Compatibility/recovery review:** run schema-5/v6 cold-open, replay, fold,
   backup/restore fixture and crash-resume checks; independent read-only review;
   rerun Verify after every finding. A raw SQLite copy is never accepted as
   migration backup evidence.

### Ownership and acceptance

The serialized owner is the only writer to `src/lib.rs`. Each new integration
test uses a new dedicated file; never inspect, format, compile by explicit test
target, or otherwise touch protected `tests/zz_probe_discriminates.rs`. Candidate
focused commands after their test files exist:

```text
cargo test --no-default-features --test hql2_durable_revision_tests
cargo test --no-default-features --test schema6_migration_tests
cargo test --no-default-features --test hql2_annotation_acl_tests
cargo test --no-default-features --test query_ir_tests
```

Then run the applicable P6/HQL2 regression suites, `cargo check
--no-default-features`, formatting on owned Rust files only, `npm run docs:validate`,
`npm run agents:validate`, and `git diff --check`. P8 closure still requires the
approved boundary's full acceptance matrix and independent Review/Final; local
core results never imply P13 or external release readiness.

### HQL2 HistoryScan/ChangeScan acceptance task (TDD; high risk)

The owner delegated the final HistoryScan/ChangeScan semantic choice on
2026-09-29; the P8/P6/H2-D11 documents now freeze the result shape, temporal
window, source-floor and recursive current-ACL rules. The single-writer task is
not implementation evidence until this RED command passes after the runtime is
added. Keep `src/lib.rs` serialized and preserve the protected probe.

```yaml
tasks:
  - id: HQL2-HISTORY-CHANGE
    description: Implement lease-bound HistoryScan and ChangeScan on exact retained revisions with fail-closed source floors and recursive ACL, including H2-D11 Vector HistoryScan by its compact JSON owner/collection key.
    domain: Rust storage security
    tier_hint: T3
    acceptance:
      verify_command: cargo test --no-default-features --test hql2_history_change_tests
  - id: HQL2-TX-AS-OF
    description: Apply one validated P8 transaction-time frontier through every HQL2/typed-IR source read, hydration, operator and response snapshot without changing the pinned lease or current ACL.
    domain: Rust storage security
    tier_hint: T3
    acceptance:
      verify_command: cargo test --locked --offline --no-default-features --jobs 1 --test hql2_history_change_tests --test hql2_storage_source_tests --test hql2_expand_tests --test hql2_annotation_lookup_tests --test hql2_vector_operator_tests
```

The target compares HQL and typed Query IR with expected typed results and
covers exact revision hydration, tombstones, deterministic order, valid-time
selection, source floors, unsupported families, invalid bounds, recursive
Annotation/Edge authorization, vector-owner events, and no partial results on
budget failure. The runtime is implemented in the isolated worktree; its
focused gate now passes 12/12. After the focused gate, rerun all root HQL2
targets, `cargo check --no-default-features`, docs/agent validation, and
`git diff --check`.

### HQL2 root Match anchors acceptance task (TDD; high risk)

The approved typed boundary now defines anchors as exact, non-null `Entity`
references keyed only by node aliases in a root Match pattern. Literals and
declared parameters are accepted; field references are not, because Match has
no input scope. Matching compares the full H2-D11 `RecordRefV2` identity in the
same pinned P6 graph snapshot. The operator performs no direct lookup, so an
absent, stale, unauthorized or foreign-database reference yields no rows without
revealing whether the record exists. Anchors filter paths before `SHORTEST`
deduplication.

```yaml
tasks:
  - id: HQL2-MATCH-ANCHORS
    description: Execute typed root Match anchors as exact node revision predicates in the pinned P6 snapshot.
    domain: Rust query authorization
    tier_hint: T3
    acceptance:
      verify_command: cargo test --no-default-features --test hql2_expand_tests
```

Required cases: exact full-identity match; stale revision and foreign database
produce no rows; unknown/edge/path aliases and non-Entity or nullable values
fail at bind; and `SHORTEST` selects the shortest path among paths satisfying
the anchor. Entity-parameter decoding must validate the H2-D11 reference shape
before it enters binding.

### HQL2 structural unsigned parameters acceptance task (TDD; medium risk)

Implement the P8 contract for parameterized `TAKE`, `SKIP`, `CHANGE SCAN
AFTER`, `KNN k`, and `RERANK k`. Resolve only exact, non-null typed
`DecimalU64` parameters after `BoundParametersV2` validation; do not coerce
`I64`, nullable, or other values. Preserve u64 for TAKE/SKIP/change floors and
reject KNN/RERANK values that do not fit u32. Missing and wrong-type values
must return deterministic bind errors. Literal behavior remains unchanged.

```yaml
tasks:
  - id: HQL2-UNSIGNED-PARAMETERS
    description: Lower structural unsigned HQL parameters from validated DecimalU64 values without casts.
    domain: Rust query binding
    tier_hint: T2
    acceptance:
      verify_command: cargo test --locked --offline --no-default-features --test hql2_lower_tests --test hql2_scalar_execution_tests --test hql2_history_change_tests --test hql2_vector_operator_tests
```

Required cases: parameterized structural positions lower/execute with exact
DecimalU64; literals stay compatible; undeclared and wrong-type parameters
fail closed; canonical-u64 validation remains in the existing decoder; and
KNN/RERANK overflow is rejected before execution.

### HQL2 implicit null-order acceptance task (TDD; medium risk)

When HQL2 `ORDER BY` omits `NULLS FIRST/LAST`, lower to `NULLS LAST` for both
ASC and DESC. Preserve an explicit placement unchanged; do not change typed-IR
requirements or legacy HQL behavior.

```yaml
tasks:
  - id: HQL2-IMPLICIT-NULL-ORDER
    description: Apply the frozen HQL2 NULLS LAST default when an ORDER BY key omits null placement.
    domain: Rust query semantics
    tier_hint: T2
    acceptance:
      verify_command: cargo test --locked --offline --no-default-features --test hql2_lower_tests --test hql2_execution_tests --test hql2_scalar_execution_tests
```

Required cases: omitted null order lowers to NullsLast and executes with NULL
last for both ASC and DESC; explicit NULLS FIRST/LAST remain unchanged; typed
IR remains closed and unchanged; existing non-null ordering and stable ties
remain compatible.

Outcome: implemented in HQL lowering; `hql2_lower_tests`,
`hql2_execution_tests`, and `hql2_scalar_execution_tests` pass 58/58 together.
The explicit 22-target HQL2 sweep passes 325/0/1.

### HQL2 checked remainder acceptance task (TDD; medium risk)

Map HQL `%` to the new closed Query IR v2 `rem` binary discriminator and
implement it in the shared scalar binder/runtime. Accept only matching I64 or
F64Finite operand types; propagate NULL; reject zero divisors with
`division_by_zero`; reject I64 `MIN % -1` with `integer_overflow`; use
truncating-quotient float remainder with the dividend's sign and reject any
non-finite result. DecimalU64 remains non-arithmetic and no casts are added.

```yaml
tasks:
  - id: HQL2-CHECKED-REMAINDER
    description: Lower and execute HQL remainder through a typed checked scalar operator and the closed IR schema.
    domain: Rust query semantics and typed IR
    tier_hint: T2
    acceptance:
      verify_command: cargo test --locked --offline --no-default-features --test hql2_lower_tests --test hql2_wire_tests --test hql2_scalar_execution_tests --test hql2_execution_tests
```

Required cases: HQL lowering, typed-IR decoding, positive/negative I64 and
F64 values, nullable propagation, zero-divisor errors, I64 overflow, wrong
types/no coercion, and schema/checksum synchronization.

Outcome: implemented through wire decoding, binding, HQL lowering, and shared
scalar execution. Focused lower/wire/scalar/storage targets pass 79/79, and the
nullable I64 bag matches the independent test-only reference. The explicit
22-target HQL2 sweep passes 328/0/1. The fixture schema checksum is synchronized.
Remaining HQL2/P8 acceptance gates stay open.

### Implementation status after H2-D11 approval

| Work item | Status | Local evidence / remaining gate |
|---|---|---|
| HQL2 HistoryScan/ChangeScan | Implemented; local verification passed | `HistoryRevision` and `ChangeEvent` use exact retained schema-v6 revisions under one P6 lease. Focused History/Change target: 16 passed; HQL/IR parity, exact historical hydration, transaction-time selection, retract tombstones, source floors, recursive Annotation/Edge authorization, Vector owner ACL and budget exhaustion are covered. HQL/typed-IR Node, Edge, Row, Vector and Annotation HistoryScan and ChangeScan bags match independent P7 for exact sequence, operation and subject identity. Artifact remains capability-unsupported; broader ChangeScan error/semantic and P8 oracle coverage remain open. |
| HQL2 Vector HistoryScan | Implemented; local verification passed | HQL and typed IR read exact vector revisions using H2-D11's compact JSON `(owner_id, collection_id)` key under the P6 lease; the vector source floor is enforced before access and owner-node ACL retained. The WAL-derived P7 HistoryScan differential is part of five-kind parity; P7 passes 130/130, Annotation source 7/7, History/Change 16/16 and latest HQL2 sweep 382/0/1 across 33 targets. Broader P8 and independent review remain open. |
| HQL2 transaction-time snapshot (`tx_as_of`) | Implemented; local verification passed | HQL2 and typed IR `query_v2` select `S = tx_as_of` (or pinned frontier L when omitted) across revision-backed scans, graph/vector/annotation operators, hydration and `Snapshot.tx`; enforce `history_horizon <= S <= L` and each used source floor before access. Keep the validated P6 generation/catalog/current policy pinned and never fall back to current rows. Five focused targets pass 56/56; HQL2 regression passes 382/0/1 across 33 targets; the separate P6/schema-v6/compatibility sweep passed 194/0/0 across 11. Broad P8/P13, transport, hosted CI and independent review remain open. |
| HQL2 root Match anchors | Implemented; local verification passed | Exact non-null Entity parameter/literal by full `RecordRefV2`; node aliases only; no direct lookup; anchor filtering precedes shortest deduplication. `hql2_expand_tests`: 12/12; `hql2_value_tests`: 6/6; all 22 root HQL2 targets: 319/0/1. |
| HQL2 structural unsigned parameters | Implemented; local verification passed | `TAKE`, `SKIP`, `CHANGE SCAN AFTER`, `KNN k`, and `RERANK k` resolve exact typed `DecimalU64` through `BoundParametersV2`; no casts/nullable widening; ranking bounds are checked as u32. Four focused targets pass 50/50; all 22 root HQL2 targets pass 323/0/1. |
| HQL2 implicit null ordering | Implemented; local verification passed | Omitted HQL `NULLS` clause defaults to `NULLS LAST` for ASC and DESC; explicit placement and the typed-IR required field remain unchanged. Three focused targets pass 58/58; all 22 root HQL2 targets pass 325/0/1. |
| HQL2 checked remainder | Implemented; local verification passed | HQL `%` lowers to the closed `rem` discriminator; exact same-type I64/F64 only, NULL propagation, checked zero-divisor/overflow errors, no coercion. Four focused targets pass 79/79; all 22 root HQL2 targets pass 328/0/1. |
| HQL2 vector ranking oracle differential | Implemented; local verification passed | HQL and typed IR match the independent P7 exact-L2 rank oracle for KNN and Original Rerank; two test-only fixtures pass 2/2. The all-root-HQL2 sweep passes 331/0/1 across 24 targets. Broad exact-oracle coverage and P8 acceptance remain open. |
| Typed database-bound identity | Partial | `RecordRefV2` validates lineage/revision syntax; graph, row, vector and annotation writers produce CAS-bound UUID revisions. Node, edge, row and annotation scans return lease-bound refs; HistoryScan/ChangeScan cover supported retained revisions, including row history and events with exact HQL/IR parity and row-property hydration (focused target 1/1). Artifact history remains capability-unsupported under the approved contract; broader shared-runtime, exact-oracle and review gates remain open. |
| Schema-v6 open gate and additive tables | Implemented; fixture-only | Explicit offline v5-to-v6 migration, verified engine backup/manifest binding, resumable signed chunks/commit, ready-marker proof, signed local activation, markerless WAL-only reopen, fold/rebuild and recursive peer isolation pass 19/19 migration plus 17/17 crash tests. Selected aggregate: 40 targets pass. No user database has been migrated. |
| KeyCodec v1 and row registry | Partial | Exact typed key bytes and UUID row IDs are persisted; upsert/update preserve row identity, delete/reinsert allocates a new identity, and nullable-key behavior remains non-unique. RowScan binds the authorized table catalog and returns the row revision ref. HQL/typed-IR HistoryScan and ChangeScan read retained row revisions from the source floor; a dedicated insert/update parity and exact-property-hydration target passes 1/1. Pre-cutover history remains unavailable as specified; no user database was migrated. |
| WAL revision event and projections | Partial | Local graph, standalone relational batch, unified transaction, vector writes, annotation CAS writes and supported consensus graph proposals carry revision envelopes; consensus signs the final envelope and stale predecessors fail before append. Graph/row/vector replay and annotation compact/reopen are covered. Folded graph and relational materializations are not yet revision-bound peer-ingress checkpoints and remain fail-closed. |
| Annotation persistence and ACL | Partial | Annotation payloads and separate target/evidence roles are normalized; frozen refs/cycles are preflighted before WAL; policy event v2 stores `Annotation(namespace)` through compact/reopen. AnnotationScan, hydration and Annotation ChangeScan subjects require explicit Annotation(Read), separately from the Namespace(Read) query grant; unauthorized Annotation revisions are excluded from ChangeScan count/byte budget and row materialization before query-budget charging. Same-namespace refs are checked recursively at the P6 snapshot. Namespace-only denial, combined-grant scan/change, and hidden-revision threshold tests pass. Exact-grant-only query authorization remains unsupported. |
| HQL2 storage-backed source adapters and text operators | Partial | Node/Edge/Row/Annotation scans return revision-bound refs with bounded pagination and current ACL under the P6 lease; `FieldIdV2`/`ExecBatchV2` provides aligned selective hydration. LexicalMatch and ContextPack now execute with registered `unicode-whitespace-bm25-v1` and `unicode-scalar-v1` profiles under that source boundary. Transport parity, broad exact-oracle coverage and full P8/P13 qualification remain open. |
| HQL1 actor-scoped adapter | Partial | `Storage::query_v2` supports only differential-tested zero-hop and bounded one-hop unlabeled/unconstrained node-ID projections after P6 authorization; one hop may include one endpoint-ID exact string equality filter. The legacy parser is preflighted and budgeted before AST construction. Focused target passes 9/9; the latest HQL2 regression sweep passes 382/0/1 across 33 targets. Other HQL1 commands/forms, independent review and full shared-runtime/P8/P13 acceptance remain open. |
| HQL2 structural root Match | Partial | Compact/Sequence graph patterns execute through HQL and typed IR under one P6 graph snapshot; deterministic shortest-per-endpoint results, stable tie order, P6 budget errors and HQL/IR parity are tested. Typed-IR anchors compare full validated RecordRefs with no direct lookup and filter before shortest deduplication. Node/edge properties filter before SHORTEST; the 10-test P8 completion target passes. |
| HQL2 Sequence node ID, labels and properties | Implemented; local verification passed | Exact UTF-8 IDs, conjunctive labels and node/edge exact-JSON properties execute for HQL/typed-IR Match and Expand under one P6 snapshot; no direct lookup, optional semantics and pre-SHORTEST filtering preserved. Latest regression sweep 382/0/1 across 33 targets; the separate 11-target P6/schema-v6/compatibility group passed 194/0/0. Compact remains unsupported; broad P8/P13 acceptance remains open despite no concrete static review finding. |
| HQL/typed-IR relational Join | Implemented; local differential passed | HQL `JOIN TABLE` lowers to the existing two-input Join operator for Inner/Left/Semi/Anti; bare JOIN defaults to Inner. HQL and typed IR match independent P7 for duplicate, SQL-NULL and JSON-null values (5/7/3/2); Semi/Anti keep left-only scope. Broad exact-oracle, review and P8/P13 gates remain open. |
| HQL/typed-IR `Values`/`UnionAll` differential | Test-only differential passed | All 169 pairs from 13 empty/length-one/two nullable bags over NULL/-1/2 match independent P7 through 338 Storage executions; explicit null-last ordering retains duplicate and NULL multiplicity. HQL2 sweep is 382/0/1 across 33 targets. No runtime/contract/schema/transport change; broad exact-oracle and P8/P13 gates remain open. |
| Vector query parameter boundary | Partial | Typed IR declarations and HQL contextual parameters validate collection fingerprint/space and dimension with finite values. Exact KNN and Original Rerank execute original vectors with deterministic distance ties; Approx remains fail-closed. The vector target passes 12/12; broader P8 qualification remains open. |

This status supersedes earlier wording that said no migration runner or resume
path exists. H2-D11/P8 remain partial: explicit historical vector/record scans,
remaining operators and P8 review/acceptance remain open; no user database has
been migrated.
Focused durable revision/key-codec/vector/Query IR tests passed 26/26; the schema-v6 migration suite
passed 19/19; storage/P6/journal/backup/rebuild regression targets passed after
updating one stale peer-event-shape assertion. The latest selected HQL2/P6
matrix passed 194 tests across 19 targets, with zero failures and one ignored
parser child entrypoint exercised by its parent. It includes typed hydration,
dynamic property names, annotation source/lookup ACL, nullable entity
projection, source-payload budgeting and HQL/IR lookup parity.
Verification is fixture-only; no user database has been opened for migration.
The final locked/offline no-default-features Rust integration suite passed with
no failures (`--jobs 4 --no-fail-fast -- --skip probe_vs_recall`). That one
discriminating probe is NOT_RUN; pre-existing ignored soak cases remain ignored.
Default and no-default strict all-target Clippy both pass, as do formatting,
diff checks and `docs:validate` (239 files, 0 violations). Full P8/P13,
shared-runtime, independent-review, release and consumer-qualification gates
remain open; no user database was migrated.
Schema-6 peer ingress still rejects unversioned folded graph and relational
materializations with `UPGRADE_REQUIRED`. A separate H2-D11 offline sweep now
passes 62/62 across eight explicit targets: durable revisions 15, annotation
ACL 8, annotation lookup 4, annotation source 3, key codec 3, storage sources
8, vector revisions 2, and schema-v6 migration 19. This supplements, but is
not added to, the separately counted 154-test P8 core matrix.
The Blueprint Python semantic-oracle suite separately passed 26/26 tests after
the annotation fixture adopted its distinct `evidence` array; this validates
the fixture contract only, not Rust HQL2 execution.
An explicit rerun of all 21 named `hql2_*` integration targets then recorded
294 passed, 3 failed and 1 ignored. The only failures are the exact-KNN and
   original-Rerank runtime tests, unavailable at bind at that checkpoint; 2 vector parameter
validation tests pass. The vector-revision (2) and wire (18) targets ran
separately after Cargo stopped at the failing vector target. Oracle suites are
included, so this sweep does not replace the 154-test P8 core matrix or qualify
P8. The protected probe was not selected or compiled.
After that historical sweep, the focused HQL/IR source differential isolated
the RowScan `after_image` omission; the row's UUIDv4 revision identity was
unchanged. The targeted correction now passes the differential, the storage-
source target passes 9/9, and the annotation-source target passes 4/4,
including separate evidence-reference ACL behavior. That prior 21-target sweep
reported 296 passed, 3 failed and 1 ignored; its remaining failures were the
exact-KNN/Rerank bind gates. The later explicit offline sweep of all 21 root
HQL2 targets passed 307, with zero failures and one ignored parser child
entrypoint exercised by its parent; it was superseded by the final explicit
offline sweep of all 22 root HQL2 targets: 318 passed, zero failed and one
ignored parser child entrypoint exercised by its parent. The dedicated vector
operator target passes 12/12, including HQL/IR parity and lease-bound original-vector behavior.
The correction is recorded in the
local RCA `.brain/rca/RCA--HQL2-ROW-SOURCE-PROPERTY-HYDRATION.md`. These local
results remain regression evidence, not P8 acceptance.

## CHANGELOG

Version diff `0.8.55b -> 0.8.56b`: record the test-only HQL/typed-IR
`Values`/`UnionAll` differential against P7 for 169 nullable bag pairs and 338
Storage executions, with explicit null-last ordering preserving NULL and
duplicate multiplicity. HQL2 passes 382/0/1 across 33 targets; P6/schema-v6/
compatibility remains 194/0/0 across 11. No runtime, contract, schema or
transport change. Keep broad exact-oracle, ChangeScan semantics, shared-runtime,
review and P8/P13 qualification open.

Version diff `0.8.54b -> 0.8.55b`: synchronize PR #196 hosted results at head
`8ac07f6` / merge `fb7085a`; 10 displayed checks pass, five fail and one is
skipped. Record worker bootstrap failures across Linux/macOS/Windows and the
rebuilt Linux addon, plus the Windows Rust failure at 15m16 with cause
unverified. Local History/Change 16/16, HQL2 380/0/1 and P6/schema-v6/
compatibility 194/0/0 remain unchanged. Keep worker correction, Windows
job-time disposition, independent review and broad P8/P13 gates open.

Version diff `0.8.53b -> 0.8.54b`: extend the test-only P7 ChangeScan
differential to Edge/Row/Vector/Annotation alongside Node; seven events match on
sequence, operation and exact subject identity for HQL and typed IR. Record
History/Change 16/16 and 380/0/1 across 32 HQL2 targets. Keep broader semantic,
worker, review and P8/P13 gates open.

Version diff `0.8.51b -> 0.8.52b`: add WAL-derived independent P7 HistoryScan
differentials for Node, Edge, Row and Annotation alongside Vector; record
History/Change 15/15, Annotation source 7/7, P7 130/130, HQL2 379/0/1 and
P6/schema-v6/compatibility 194/0/0. Artifact HistoryScan, broad ChangeScan/P8/P13,
hosted CI and independent review remain open.

Version diff `0.8.50b -> 0.8.51b`: add the test-only P7 Vector HistoryScan
oracle profile and WAL-derived HQL/typed-IR result-bag differential; record
P7 130/130, History/Change 14/14, HQL2 377/0/1 and P6/schema-v6/
compatibility 194/0/0. Hosted CI and review remain open; broad P8/P13 and
transport acceptance are not claimed.

Version diff 0.8.49b -> 0.8.50b: implement the owner-approved narrow
ChangeScan budget rule. Namespace(Read) continues to cover same-namespace
references and other readable ChangeScan subjects; unauthorized Annotation
revisions are excluded before caller-budget accounting/materialization. The
threshold regression passes; ACL is 11/11, History/Change 14/14, the HQL2
sweep 376/0/1 and selected six-target P6/schema-v6 43/0/0. No schema/migration
change. This source revision is not yet pushed; hosted CI, independent review,
and broad P8/P13 and transport gates remain open.

Version diff 0.8.47b -> 0.8.48b: refresh completed PR #194 run
37083654705 at docs-only head 43cc6e8 (engine source remains at 31168524).
Linux/macOS Rust, standard Node, docs, fmt/clippy and version checks pass;
Windows Rust fails the HQL/IR Join differential with `query_limit`; local
reproduction passes 5/5 and the exact budget dimension is unconfirmed. Worker
tests fail on Linux/macOS/Windows and rebuilt Linux addon with markerless
database identity missing. PR remains OPEN/UNSTABLE; no bypass or merge.

Version diff 0.8.46b -> 0.8.47b: refresh hosted PR #194 evidence at head
31168524. The 32-target local HQL2 sweep remains 374/0/1; GitHub reports the
core Rust/standard Node/docs/fmt checks passing, with three worker OS jobs and
the rebuilt Linux addon/worker job failing at markerless schema-v6 bootstrap.
The worker correction remains approval-gated; PR is OPEN/UNSTABLE and not
merged. Preserve broad P8/P13, independent review, security and release gates.

Version diff `0.8.18b -> 0.8.19b`: implement HQL2 implicit null ordering as NULLS LAST for either direction; preserve explicit placement and typed-IR requirements; record 58 focused passes and 325/0/1 across 22 HQL2 targets.
Version diff `0.8.19b -> 0.8.20b`: freeze checked HQL `%`/typed Query IR `rem` semantics.
Version diff `0.8.20b -> 0.8.21b`: implement the frozen HQL `%`/typed-IR `rem` contract through checked scalar execution; record 79/79 focused passes and 328/0/1 across 22 HQL2 targets; retain remaining P8 gates.
Version diff `0.8.22b -> 0.8.23b`: add and pass Original Rerank HQL/typed-IR differential against the independent P7 rank oracle; record 2/2 vector ranking tests and 330/0/1 across all 23 root HQL2 targets; keep broad exact-oracle and P8 acceptance open.
Version diff `0.8.23b -> 0.8.24b`: add and pass a test-only HQL root-Match differential against the independent P7 graph bag; record 1/1 focused and 331/0/1 across all 24 root HQL2 targets; keep broad exact-oracle and P8 acceptance open.
Version diff `0.8.24b -> 0.8.25b`: record the delegated C-3 decision and exact P6/HQL/typed-IR contract for Sequence node ID and conjunctive labels; implementation and verification remain pending, with properties and Compact constraints fail-closed.
Version diff `0.8.25b -> 0.8.26b`: implement Sequence node ID/labels through HQL and typed IR under P6; record 7 focused passes and 338/0/1 across 25 root HQL2 targets; retain the ACL-hidden fixture, independent review and full P8 gates.
Version diff `0.8.26b -> 0.8.27b`: record owner approval of P8 addendum D1-D6 and authorize test-first implementation of lexical/context profiles, Sequence properties, contextual literals and the corrected pre-parse ACL fixture; runtime evidence remains pending.
Version diff `0.8.28b -> 0.8.29b`: add and pass the edge-property-before-SHORTEST regression; record 10/10 P8 completion tests and 349/0/1 across 26 root HQL2 targets plus a separate 190/0/0 across 11 P6/schema-v6/compatibility targets; confirmation review marked the D5 fix complete and independent static review found no concrete defect; retain broad P8/P13 gates.
Version diff `0.8.44b -> 0.8.45b`: implement the approved C3 HQL2-TX-AS-OF contract with one no-fallback selected frontier across source scans, graph/vector/annotation operators, hydration and result metadata while retaining the pinned P6 generation/current policy. Record 56/56 across five focused targets, History/Change 14/14, HQL2 373/0/1 and P6/compatibility 194/0/0; broader P8/P13, transport, hosted CI and independent review remain open.

Version diff `0.8.43b -> 0.8.44b`: synchronize the accepted P8 transaction-time snapshot contract across P6 and the implementation plan; add the C3 HQL2-TX-AS-OF acceptance task for one no-fallback selected frontier across source scans, hydration, operators and result metadata. Runtime was pending at that checkpoint.
Version diff `0.8.42b -> 0.8.43b`: implement and verify H2-D11 Vector HistoryScan through HQL and typed IR under the P6 vector floor/owner ACL; record 12 focused passes, 367/0/1 across 31 HQL2 targets and 194/0/0 across 11 P6/compatibility targets. Retain transaction-time snapshot, broad acceptance and independent review gates.
Version diff `0.8.40b -> 0.8.41b`: implement HQL `JOIN TABLE` lowering through the approved typed-IR Join operator for Inner/Left/Semi/Anti, default bare JOIN to Inner, and add HQL/typed-IR differential parity against independent P7; record 365/0/1 across 31 HQL2 targets. Hosted CI for this local revision has not run; the prior parent has worker failures and review remains pending, so retain merge, broad exact-oracle, independent-review and P8/P13/release gates.
Version diff `0.8.38b -> 0.8.39b`: add and pass a storage-backed HQL/typed-IR aggregate differential against independent P7 for 121 nullable bags; record 363/0/1 across 29 HQL2 targets. Refresh hosted evidence at `d4bc870`: Rust/core checks pass on Linux/macOS/Windows, worker checks fail across OSes, and review remains pending; retain worker-fix approval, broad exact-oracle, independent-review and P8/P13/release gates.
Version diff `0.8.37b -> 0.8.38b`: add and pass a storage-backed HQL/typed-IR scalar differential against independent P7 for 81 nullable bags; record 362/0/1 across 28 HQL2 targets and 190/0/0 across 11 compatibility targets. Update hosted evidence at `cef747a`: worker tests fail across operating systems and Windows Rust reaches its 15m15s timeout; the new test is not in CI yet. Preserve worker-fix/timeout approval gates and broad exact-oracle, independent-review, P8/P13/release gates.
Version diff `0.8.36b -> 0.8.37b`: refine hosted PR #194 evidence at `cdfb90a`: Windows `cargo test` itself passes, including the protected probe in 225.86s and doc-tests, but the 15-minute job timeout cancels post-cache finalization; retain the probe and propose a Windows-only 20-minute job timeout. Worker CI still fails on all OSes because sidecars make a fresh root nonempty before schema-v6 activation; retain the separate worker docs/code approval gate, independent review, and broad P8/P13/release gates.
Version diff `0.8.35b -> 0.8.36b`: synchronize hosted PR #194 evidence at `9344b71`; Linux/macOS Rust and standard Node/mobile/consumer checks pass, Windows Rust exceeds the 15-minute job limit in the protected probe, and worker tests fail cross-platform because sidecars make a fresh database root nonempty before schema-v6 activation. Preserve protected WIP; retain the worker approval gate, independent review, and broad P8/P13/release gates.
Version diff `0.8.34b -> 0.8.35b`: complete local integration verification for the approved H2-D11 schema-v6/consensus path; full Rust suite and default/no-default strict Clippy pass, with `probe_vs_recall` explicitly NOT_RUN; retain fixture-only scope and broader P8/P13/review/release gates.
Version diff `0.8.33b -> 0.8.34b`: implement approved H2-D11 R6b signed activation preflight and markerless WAL recovery; record 17/17 crash tests, 19/19 migration tests and 40 passing selected HQL2/durability/authority targets; retain fixture-only scope and broader P8/P13 gates.
Version diff `0.8.32b -> 0.8.33b`: incorporate owner-approved H2-D11 R6b schema activation and markerless WAL recovery proof/test gates.
Version diff `0.8.31b -> 0.8.32b`: add the differential-proven one-hop endpoint-ID string equality filter to D7; record 9/9 focused adapter tests, 361/0/1 across 27 root HQL2 targets and 190/0/0 across 11 compatibility targets; retain other HQL1, independent-review and P8/P13 gates.
Version diff `0.8.27b -> 0.8.28b`: implement approved D1-D5, record 9 focused passes, three JSON literal size-preflight unit tests and 528/0/1 across 37 explicit HQL2/P6/schema-v6/compatibility targets; retain reviewer confirmation and broad P8/P13 gates.

| Version | Date | Status | Summary | Commit Hash | Agent |
|---|---|---|---|---|---|
| 0.8.56b | 2026-10-03 | beta | Add test-only HQL/typed-IR Values/UnionAll P7 differential for 169 nullable bag pairs and 338 Storage executions; HQL2 382/0/1 across 33 targets, P6/compatibility 194/0/0; no runtime/schema/transport change; retain broad oracle, shared-runtime, review and P8/P13 gates | working-tree | ATHER |
| 0.8.55b | 2026-10-03 | beta | Synchronize merged PR #196 hosted status: 10 displayed checks pass, five fail, one skips; worker bootstrap failures across OSes and rebuilt Linux addon; Windows Rust failure at 15m16 not diagnosed; local History/Change 16/16, HQL2 380/0/1, P6/compatibility 194/0/0; keep approval and broad P8/P13 gates open | working-tree | ATHER |
| 0.8.54b | 2026-10-03 | beta | Extend test-only P7 ChangeScan differential to Edge/Row/Vector/Annotation alongside Node; seven events match exact sequence, operation and subject identity for HQL/typed IR; History/Change 16/16 and HQL2 380/0/1; broad semantics, worker correction, P8/P13 and independent review remain open | working-tree | ATHER |
| 0.8.53b | 2026-10-03 | beta | Add Node-revision ChangeScan differential against P7; History/Change 15/15 and HQL2 379/0/1 across 32 targets; record PR #194 merged at 4f02d6b with 48 pass, five fail and five skipped; other event source kinds, worker correction, P8/P13 and independent review remain open | working-tree | ATHER |
| 0.8.52b | 2026-10-03 | beta | Add WAL-derived P7 HistoryScan differentials for all five supported kinds; verify endpoint/owner/reference ACL and exact temporal identities; History/Change 15/15, Annotation source 7/7, P7 130/130, HQL2 379/0/1 and P6/schema-v6/compatibility 194/0/0; Artifact, broad ChangeScan/P8/P13, hosted review remain open | working-tree | ATHER |
| 0.8.51b | 2026-10-03 | beta | Add P7 Vector HistoryScan differential using WAL-derived revisions and H2-D11 owner ACL; P7 130/130, History/Change 14/14, HQL2 377/0/1 and P6/schema-v6/compatibility 194/0/0; hosted checks/review and broad P8/P13 remain open | working-tree | ATHER |
| 0.8.50b | 2026-10-03 | beta | Implement approved narrow ChangeScan budget policy across H2-D11/P6/P8; ACL 11/11, History/Change 14/14, HQL2 376/0/1 and selected P6/schema-v6 43/0/0; source revision awaits hosted CI/review; broad P8/P13 and transport remain open | working-tree | ATHER |
| 0.8.49b | 2026-10-03 | beta | Implement H2-D11 R4/P6 annotation ACL conformance for scan/hydration and ChangeScan subjects; preserve separate namespace query grant and recursive reference ACL; regression passes, HQL2 375/0/1 and selected P6/schema-v6 43/0/0; possible ChangeScan budget side channel, hosted verification/review and broad P8/P13 remain open | working-tree | ATHER |
| 0.8.48b | 2026-10-03 | beta | Refresh PR #194 completed CI at docs-only head 43cc6e8: Windows Join query-budget failure plus four worker bootstrap failures; Join target passes local 5/5; exact budget dimension unconfirmed; PR unmerged and broad P8/P13/review gates open | working-tree | ATHER |
| 0.8.47b | 2026-10-03 | beta | Refresh PR #194 hosted evidence at head 31168524: 374/0/1 across 32 local HQL2 targets; core checks pass but three worker OS jobs and rebuilt Linux addon/worker fail at schema-v6 bootstrap; worker correction remains approval-gated, PR unmerged, broad P8/P13/review gates open | working-tree | ATHER |
| 0.8.46b | 2026-10-03 | beta | Add HQL/typed-IR Row HistoryScan/ChangeScan parity for retained insert/update revisions and exact property hydration (1/1); refresh explicit HQL2 sweep to 374/0/1 across 32 targets; correct stale row-history status; broad P8/P13, transport, CI and independent review remain open | working-tree | ATHER |
| 0.8.45b | 2026-10-03 | beta | Implement approved HQL2/IR `tx_as_of` with one no-fallback frontier across scans, operators, hydration and snapshot; focused 56/56, History/Change 14/14, HQL2 373/0/1 and P6/compatibility 194/0/0; broad P8/P13, transport, CI and review remain open | working-tree | ATHER |
| 0.8.44b | 2026-10-03 | beta | Synchronize P8/P6 transaction-time snapshot contract and add C3 HQL2-TX-AS-OF tests-first acceptance; implementation was pending at that checkpoint | working-tree | ATHER |
| 0.8.43b | 2026-10-03 | beta | Implement approved Vector HistoryScan in HQL/typed IR with same P6 lease/floor/owner ACL; focused 12/12, HQL2 367/0/1 and P6/compatibility 194/0/0; track contracted but unimplemented `tx_as_of` as next separate C3 slice | working-tree | ATHER |
| 0.8.42b | 2026-10-03 | beta | Synchronize approved H2-D11 Vector HistoryScan identity, P6 floor/owner ACL, and P8 acceptance; implementation/parity verification pending | working-tree | ATHER |
| 0.8.41b | 2026-10-03 | beta | Implement HQL Join lowering through existing RowScan/Join contract for four kinds and bare JOIN default; HQL/typed IR match independent P7; record 365/0/1 across 31 HQL2 targets; hosted CI pending; retain merge/P8/P13/review gates | working-tree | ATHER |
| 0.8.40b | 2026-10-03 | beta | Add and pass storage-backed typed-IR Join P7 differential for Inner/Left/Semi/Anti; record 364/0/1 across 30 HQL2 targets without expanding HQL JOIN; at hosted parent head 484916b core checks pass but worker checks fail across OSes and no PR reviews exist; retain merge/P8/P13/review gates | working-tree | ATHER |
| 0.8.39b | 2026-10-03 | beta | Add and pass storage-backed HQL/typed-IR P7 aggregate differential over 121 nullable bags; record 363/0/1 across 29 HQL2 targets; hosted Rust/core checks pass on Linux/macOS/Windows, worker checks fail across OSes and review remains pending; scoped commit/push/merge requested but CI/review gates remain open | working-tree | ATHER |
| 0.8.38b | 2026-10-03 | beta | Add and pass storage-backed HQL/typed-IR P7 scalar differential over 81 nullable bags; record 362/0/1 across 28 HQL2 targets and 190/0/0 compatibility targets; update hosted worker failures and Windows timeout; scoped commit/push/merge requested but CI/review gates remain open | working-tree | ATHER |
| 0.8.37b | 2026-10-02 | beta | Refine hosted PR #194 evidence: Windows Rust test step passes with protected probe but 15-minute job timeout cancels post-cache; propose 20-minute Windows timeout; worker bootstrap errors persist across OS; keep worker code approval, independent review and P8/P13 gates open | working-tree | ATHER |
| 0.8.36b | 2026-10-02 | beta | Synchronize hosted PR #194 evidence: Linux/macOS Rust and standard Node/mobile checks pass; Windows full suite times out in protected probe; worker CI fails at fresh schema-v6 bootstrap; keep all P8/P13/review/release gates open | working-tree | ATHER |
| 0.8.35b | 2026-10-02 | beta | Full locked/offline no-default-features Rust suite passes with `probe_vs_recall` NOT_RUN; default/no-default strict all-target Clippy passes; consensus signs schema-v6 revision envelopes; broad P8/P13/review/release gates remain open | working-tree | ATHER |
| 0.8.34b | 2026-10-02 | beta | Implement and locally verify R6b markerless WAL recovery; crash 17/17, migration 19/19 and selected 40-target aggregate pass; fixture-only, no user DB migration, broad P8/P13 gates remain open | 0135c29 | ATHER |
| 0.8.33b | 2026-10-02 | beta | Incorporate approved H2-D11 R6b schema activation and markerless WAL recovery gates; implementation and tests remain pending | working-tree | ATHER |
| 0.8.32b | 2026-10-02 | beta | Implement D7's one-hop endpoint-ID exact string filter through the shared HQL2 runtime; record 9/9 focused tests, 361/0/1 across 27 HQL2 targets and 190/0/0 across 11 compatibility targets; retain other HQL1, review, P8/P13 and no-migration gates | working-tree | ATHER |
| 0.8.31b | 2026-10-02 | beta | Extend D7 with differential-proven one-hop HQL1 forms and pre-parse resource reservation; record 8/8 focused tests, 361/0/1 across 27 HQL2 targets and 190/0/0 across 11 compatibility targets; retain other HQL1, independent review, P8/P13 and no-migration gates | working-tree | ATHER |
| 0.8.30b | 2026-10-02 | beta | Implement D7's actor-scoped HQL1 zero-hop ID projection; record 5/5 focused tests and 354/0/1 across 27 root HQL2 targets; keep other HQL1 forms, independent review, P8/P13 and no-migration gates | working-tree | ATHER |
| 0.8.29b | 2026-10-02 | beta | Record D4 edge-property-before-SHORTEST regression, 10/10 P8 completion tests and 349/0/1 across 26 HQL2 targets plus separate 190/0/0 across 11 P6/schema-v6/compatibility targets; keep broad P8/P13, resource and no-migration gates | working-tree | ATHER |
| 0.8.28b | 2026-10-02 | beta | Implement approved P8 D1-D5; record focused, budget, JSON literal-size and 37-target regression evidence; keep P9-P13, reviewer confirmation and no-migration gates | working-tree | ATHER |
| 0.8.27b | 2026-10-02 | beta | Record owner approval of P8 completion addendum D1-D6; authorize bounded implementation while retaining P9-P13 and no-migration gates | working-tree | ATHER |
| 0.8.26b | 2026-09-30 | beta | Implement lease-bound HQL/typed-IR Sequence ID and labels; record 7 focused passes and 338/0/1 across 25 HQL2 targets; retain ACL-hidden fixture and P8 review gates | working-tree | ATHER |
| 0.8.25b | 2026-09-30 | beta | Freeze delegated Sequence node-ID/label contract across HQL, typed IR and P6 snapshot; implementation and verification pending; retain property/Compact fail-closed limits | working-tree | ATHER |
| 0.8.24b | 2026-09-30 | beta | Add 1/1 test-only HQL root-Match differential against independent P7 graph bag; record parallel-edge and relation-filter coverage plus 331/0/1 across 24 root HQL2 targets; retain broad P8 gates | working-tree | ATHER |
| 0.8.23b | 2026-09-30 | beta | Add 1/1 Original Rerank differential to the HQL/typed-IR exact-KNN P7 oracle target; record 2/2 and 330/0/1 across 23 root HQL2 targets; retain broad P8 gates | working-tree | ATHER |
| 0.8.22b | 2026-09-30 | beta | Add a 1/1 HQL/typed-IR exact-KNN differential against the independent P7 oracle for tie order and missing-original exclusion; record 329/0/1 across 23 root HQL2 targets; retain broad P8 gates | working-tree | ATHER |
| 0.8.17b | 2026-09-30 | beta | Implement exact typed-IR root Match anchors with 12/12 Match, 6/6 value tests and 319/0/1 across 22 HQL2 targets; keep constrained patterns, remaining operators, transport and P8 acceptance open | working-tree | ATHER |
| 0.8.18b | 2026-09-30 | beta | Implement structural unsigned HQL parameters for TAKE/SKIP/CHANGE/KNN/RERANK using typed DecimalU64 without coercion; record 50 focused passes and 323/0/1 across 22 targets; keep remaining P8 gates open | working-tree | ATHER |
| 0.8.19b | 2026-09-30 | beta | Implement HQL2 default null ordering as NULLS LAST independent of direction; record 58 focused passes and 325/0/1 across 22 targets; retain remaining P8 gates | working-tree | ATHER |
| 0.8.20b | 2026-09-30 | beta | Freeze checked HQL remainder and typed Query IR `rem` acceptance; runtime implementation pending | working-tree | ATHER |
| 0.8.21b | 2026-09-30 | beta | Implement checked HQL `%` and typed-IR `rem` for same-type I64/F64 with NULL propagation and fail-closed zero/overflow behavior; record 79/79 focused and 328/0/1 across 22 targets; retain remaining P8 gates | working-tree | ATHER |
| 0.8.16b | 2026-09-30 | beta | Freeze root Match anchor semantics as full H2-D11 RecordRefV2 identity predicates under one P6 snapshot; add a focused TDD acceptance task while preserving remaining P8 gates | working-tree | ATHER |
| 0.8.15b | 2026-09-30 | beta | Implement and locally verify HQL/IR HistoryScan/ChangeScan over durable revisions, recursive ACL, exclusive floors and no-partial budget failures; record 9/9 focused and 318/0/1 across 22 root targets; retain lexical/context, constrained-pattern, transport and P8 gates | working-tree | ATHER |
| 0.8.14b | 2026-09-29 | beta | Freeze delegated HistoryScan/ChangeScan contract and add a machine-checkable TDD target; runtime and P8 closure remain open | working-tree | ATHER |
| 0.8.13b | 2026-09-29 | beta | Implement exact KNN and Original Rerank through lease-bound original vectors; record 307/0/1 across 21 root HQL2 targets and retain explicit history, remaining operator and P8 gates | working-tree | ATHER |
| 0.8.12b | 2026-09-29 | beta | Record the RowScan after_image correction and then-current 296/3/1 HQL2 sweep; retain remaining operator and P8 gates | working-tree | ATHER |
| 0.8.11b | 2026-09-29 | beta | Record RowScan property-hydration RED and annotation-source 4/4; preserve HQL2 historical sweep count and approval gate | working-tree | ATHER |
| 0.8.10b | 2026-09-29 | beta | Record the explicit 21-target HQL2 sweep and three KNN/Rerank bind failures; keep oracle evidence separate from P8 acceptance and retain open contract gates | working-tree | ATHER |
| 0.8.9b | 2026-09-29 | beta | Record 26/26 Blueprint semantic-oracle passes for separate annotation evidence alongside the 62/62 H2-D11 sweep; retain HQL2 operator and P8 gates | working-tree | ATHER |
| 0.8.8b | 2026-09-29 | beta | Record separate 62/62 H2-D11 fixture verification across revisions, annotations, sources, vectors and schema-v6 migration; retain HQL2 operator and P8 gates | working-tree | ATHER |
| 0.8.7b | 2026-09-29 | beta | Implement structural HQL/IR root Match and SHORTEST with focused graph/scalar evidence; retain anchor, constraint, vector/history, remaining-operator and P8 gates | working-tree | ATHER |
| 0.8.6b | 2026-09-29 | beta | Record typed IR vector-parameter decode and 2 passing mismatch tests; keep original-vector reads, Knn/Rerank and P8 acceptance open | working-tree | ATHER |
| 0.8.5b | 2026-09-29 | beta | Record structural Sequence Expand under P6, seven focused path/budget tests and 206 passes across 21 targets; retain pattern-constraint and P8 gates | working-tree | ATHER |
| 0.8.4b | 2026-09-29 | beta | Record constrained HQL/IR Expand, separate AnnotationPut evidence fixture/oracle and 199 passing tests across 20 targets; retain extended pattern and P8 gates | working-tree | ATHER |
| 0.8.3b | 2026-09-29 | beta | Record HQL/IR AnnotationLookup frozen/live and optional/required execution; 194 tests pass across 19 P8/P6/H2-D11 targets, with eight operators and P8 gates open | working-tree | ATHER |
| 0.8.2b | 2026-09-29 | beta | Record typed selective HQL2 property hydration and 188 passing tests across 18 HQL2/P6 targets; retain original-vector, operator, transport and P8 closure gates | working-tree | ATHER |
| 0.8.1b | 2026-09-29 | beta | Record lease-bound Node/Edge/Row/Annotation source scans, P6 reference checks, budgeted property reads and 133/133 selected P8/P6 tests; retain selective hydration and P8 closure gates | working-tree | ATHER |
| 0.8.0b | 2026-09-29 | beta | Record annotation CAS writes, separate target/evidence projection, P6 policy-event v2 and 8/8 tests; keep annotation reads and P8 source adapters open | working-tree | ATHER |
| 0.7.0b | 2026-09-29 | beta | Record owner-bound vector revision writes and 2/2 tests; retain annotation/ACL, source-adapter and P8 closure gates | working-tree | ATHER |
| 0.6.0b | 2026-09-29 | beta | Record schema-v6 migration/recovery implementation and 19/19 fixture evidence; retain vector, annotation/ACL, source-adapter and P8 closure gates | working-tree | ATHER |
| 0.5.0b | 2026-09-29 | beta | Record owner approval of amended ADR 0.4.0b and authorize schema-v6 migration implementation on temporary fixtures only | working-tree | ATHER |
| 0.4.0b | 2026-09-28 | candidate | Propose the schema-v6 migration event/source-coordinate implementation gate; owner approved the candidate, but implementation remains gated on amended recovery requirements | working-tree | ATHER |
| 0.3.3b | 2026-09-28 | candidate | Record verified relational row identity/revision writes, replay and KeyCodec persistence; retain vector/annotation/adapter/migration and peer-fold gaps | working-tree | ATHER |
| 0.3.2b | 2026-09-28 | candidate | Record verified local graph revision writes/replay, preserve transaction-frontier semantics, and list row/vector/annotation/adapter/migration plus peer-fold gaps | working-tree | ATHER |
| 0.3.1b | 2026-09-28 | candidate | Record measured H2-D11 implementation status and clarify that the migration and durable revision path remain pending | working-tree | ATHER |
| 0.3.0b | 2026-09-28 | candidate | Record owner-approved H2-D11 contract, synchronize P8/P6/C4, and define the serialized P8 storage-revision implementation and migration gates | working-tree | ATHER |
| 0.2.1b | 2026-09-23 | candidate | Recorded P6 revision-0 fail-closed correction, 45/45 Verify, gpt-5.6-luna Max Review, local audit evidence and remaining peer-authority owner decision | working-tree | ATHER |
| 0.2.2b | 2026-09-28 | candidate | Reconcile approved HQL2 continuation and replace helper-only P7 gate with four composed Rust oracle targets; retain unapproved P8 concrete-boundary gate | working-tree | ATHER |
| 0.2.3b | 2026-09-28 | candidate | Record subsequent P8 contract and isolated integration approval; distinguish partial scalar evidence from P8-P16 closure | working-tree | ATHER |
| 0.2.0b | 2026-09-23 | candidate | Returned P6 to source after Luna Review; clarified signed snapshot authority, pre-parse ACL and legacy JSONL coverage gates | working-tree | ATHER |
| 0.1.0b | 2026-09-22 | candidate | Initial staged UEE-HQL2 dependency DAG, conflict domains, merge order and gate workflow | working-tree | ATHER |
| 0.1.1b | 2026-09-22 | candidate | Added explicit path ownership, exact verification commands, merge barriers, and corrected topology evidence scope after Verify Gate FAIL | working-tree | ATHER |
| 0.1.2b | 2026-09-22 | candidate | Recorded dirty-checkout preservation and task-owned plan boundary after Review Gate returned unverified | working-tree | ATHER |
| 0.1.3b | 2026-09-22 | candidate | Recorded P5 implementation evidence, RED/GREEN result, and Verify/Review gate outcomes; Final Gate and owner acceptance remain pending | working-tree | ATHER |
| 0.1.4b | 2026-09-23 | candidate | Recorded P5 owner approval and blocked P6 source work pending explicit generation, lease, temporal-binding and ACL contract decisions | working-tree | ATHER |
| 0.1.5b | 2026-09-23 | candidate | Recorded Astra P6 architecture correction: fail-closed publication, opaque leases, signed ACL event and minimum-reader guard | working-tree | ATHER |
| 0.1.6b | 2026-09-22 | candidate | Recorded owner approval and pinned P6 implementation API, snapshot, ACL and gate contract | working-tree | ATHER |
| 0.1.7b | 2026-09-22 | candidate | Added the parent-linked P6 specification and routed remaining gates to gpt-5.6-luna Max | working-tree | ATHER |
