---
version: "0.1.2b"
created_at: "2026-09-22T00:00:00+07:00,ATHER,working-tree"
last_update: "2026-09-22T00:00:00+07:00,ATHER"
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

สถานะเอกสารนี้คือ `candidate` และเป็น workflow/แผนงานเท่านั้น ยังไม่อนุญาตให้แก้โค้ด,
เปลี่ยน storage format, เปลี่ยน public API, migrate, merge หรือ deploy จนกว่า owner จะอนุมัติ
เอกสารและ ADR ที่ระบุใน P2/P3

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

This document proposes planning only. After owner approval, orchestration may begin at P1. Before
approval, no worker may implement P4-P16 and no branch may be merged into the caller checkout.

**Please review and approve this documentation. I will generate the code once approved.**

## CHANGELOG

| Version | Date | Status | Summary | Commit Hash | Agent |
|---|---|---|---|---|---|
| 0.1.0b | 2026-09-22 | candidate | Initial staged UEE-HQL2 dependency DAG, conflict domains, merge order and gate workflow | working-tree | ATHER |
| 0.1.1b | 2026-09-22 | candidate | Added explicit path ownership, exact verification commands, merge barriers, and corrected topology evidence scope after Verify Gate FAIL | working-tree | ATHER |
| 0.1.2b | 2026-09-22 | candidate | Recorded dirty-checkout preservation and task-owned plan boundary after Review Gate returned unverified | working-tree | ATHER |
