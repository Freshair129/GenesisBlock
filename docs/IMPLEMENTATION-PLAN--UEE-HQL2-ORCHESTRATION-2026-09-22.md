---
version: "0.2.2b"
created_at: "2026-09-22T00:00:00+07:00,ATHER,working-tree"
last_update: "2026-10-04T00:00:00+07:00,Codex"
status: candidate
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

สถานะเอกสารนี้คือ `candidate` และเป็น workflow/แผนงานที่ใช้กำกับ execution แบบมี gate
เท่านั้น P4/P5 ได้รับ owner approval แล้ว และ P6 contract พร้อม architecture correction ได้รับ
อนุมัติให้เริ่ม implementation แล้ว; P6 Final Gate, P7-P16, merge และ deploy ยังไม่ผ่าน approval

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
| P8 | HQL1/HQL2/IR lowering to one pipeline plus truthful EXPLAIN/counters | P7 | All frontends share binder/runtime; only after new ADR approves this boundary |
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
- P7: `uv run --with-requirements tools/requirements.txt python -m unittest discover -s tests -p 'test_reference*.py'`
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

The initial plan required owner approval before orchestration. P4 and P5 final-gate acceptance are
now recorded. P6 source implementation remains blocked until the contract decisions in the next
section are explicitly approved; P7-P16 and any merge/deploy action remain blocked as well.

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
explicit owner acceptance of the final P6 result. Do not start P7 or merge.

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
- Query IR inherits TemporalRead.as_of as valid_at and tx_as_of as-is. Neighbors and hybrid search
  may inherit as_of but reject tx_as_of; node_view, node_versions, HQL and relational reads reject
  non-empty TemporalRead until their existing contracts can bind both selectors without fallback.
- Stable error prefixes for tests: GENERATION_STALE, LEASE_OWNER_MISMATCH, LEASE_EXPIRED,
  LEASE_REVOKED, TEMPORAL_BEYOND_HORIZON, ACCESS_CONTEXT_REQUIRED, ACCESS_DENIED,
  ACCESS_POLICY_REVISION_CONFLICT, SNAPSHOT_MANIFEST_INVALID and
  SNAPSHOT_COMPONENT_INVALID.

## 10. P7/G3 execution evidence — 2026-10-04

The bounded G3 slice is implemented in the working tree without enabling G4+
(HQL2/planner/EXPLAIN) or changing public transport surfaces. The deliverables
are the pure standard-library oracle in `tools/reference_oracle.py`, the
versioned fixture `tests/fixtures/g3_oracle_cases.json`, the Python golden test
`tests/test_reference_g3.py`, the public-Storage Rust differential/reopen test
`tests/g3_oracle_differential_tests.rs`, and the empty reproducibility manifest
`tools/requirements.txt`.

The fixture covers bitemporal valid-window and transaction-marker semantics,
relational left-join NULL projection, bag multiplicity, deterministic ordering,
snapshot/projection removal, and WAL reopen recovery. The existing crash,
frontier, projection, relational, temporal, lease, ACL, REST and NAPI suites
remain separate regression gates.

Local evidence recorded for this slice:

- Python reference gate: 1/1 passed with both the direct standard-library
  runner and the prescribed `uv run --with-requirements` runner.
- G3 Rust differential/reopen gate: 2/2 passed.
- Existing targeted G1/G2/crash matrix: 62/62 passed before the G3 additions.
- Full `cargo test --no-default-features` completed with exit code 0; the
  60-build `probe_vs_recall` target passed 1/1 and the crate doc-test target
  passed 0/0 with no failures.
- `cargo fmt --check`, `cargo check --no-default-features`, `npm run
  docs:validate`, `npm run agents:validate`, and `git diff --check` are local
  gates; hosted CI, power-loss hardware, mobile/device, release packaging,
  deployment and production acceptance remain NOT_RUN.

The schema-manifest correction from v4 to v5 is synchronized in
`modules.json` and the affected NAPI tests. Three pre-P6/legacy test fixtures
that intentionally rewrite or remove snapshot metadata were made internally
consistent with the P6 integrity boundary; the RCA is recorded at
`.brain/rca/RCA--EPOCH-E2-LEGACY-METADATA-FIXTURE-P6-MANIFEST-2026-10-04.md`.
G4+ remains deferred until the G3 evidence and owner review boundary are
accepted.

## CHANGELOG

| Version | Date | Status | Summary | Commit Hash | Agent |
|---|---|---|---|---|---|
| 0.2.2b | 2026-10-04 | candidate | Added bounded G3 exact-oracle, differential/reopen and legacy-fixture evidence; G4+ remains deferred and external gates remain NOT_RUN | working-tree | Codex |
| 0.2.1b | 2026-09-23 | candidate | Recorded P6 revision-0 fail-closed correction, 45/45 Verify, gpt-5.6-luna Max Review, local audit evidence and remaining peer-authority owner decision | working-tree | ATHER |
| 0.2.0b | 2026-09-23 | candidate | Returned P6 to source after Luna Review; clarified signed snapshot authority, pre-parse ACL and legacy JSONL coverage gates | working-tree | ATHER |
| 0.1.0b | 2026-09-22 | candidate | Initial staged UEE-HQL2 dependency DAG, conflict domains, merge order and gate workflow | working-tree | ATHER |
| 0.1.1b | 2026-09-22 | candidate | Added explicit path ownership, exact verification commands, merge barriers, and corrected topology evidence scope after Verify Gate FAIL | working-tree | ATHER |
| 0.1.2b | 2026-09-22 | candidate | Recorded dirty-checkout preservation and task-owned plan boundary after Review Gate returned unverified | working-tree | ATHER |
| 0.1.3b | 2026-09-22 | candidate | Recorded P5 implementation evidence, RED/GREEN result, and Verify/Review gate outcomes; Final Gate and owner acceptance remain pending | working-tree | ATHER |
| 0.1.4b | 2026-09-23 | candidate | Recorded P5 owner approval and blocked P6 source work pending explicit generation, lease, temporal-binding and ACL contract decisions | working-tree | ATHER |
| 0.1.5b | 2026-09-23 | candidate | Recorded Astra P6 architecture correction: fail-closed publication, opaque leases, signed ACL event and minimum-reader guard | working-tree | ATHER |
| 0.1.6b | 2026-09-22 | candidate | Recorded owner approval and pinned P6 implementation API, snapshot, ACL and gate contract | working-tree | ATHER |
| 0.1.7b | 2026-09-22 | candidate | Added the parent-linked P6 specification and routed remaining gates to gpt-5.6-luna Max | working-tree | ATHER |
