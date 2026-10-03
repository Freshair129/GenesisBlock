---
title: "GenesisBlockDB Document Registry"
doc_id: "DOC-REGISTRY-GENESISBLOCKDB"
status: draft
version: "0.5.60+draft"
updated: "2026-10-03"
owner: "GenesisBlockDB Architecture"
source_of_truth: true
related_issue: 84
related_docs:
  - "docs/README.md"
  - "docs/DOC-STATUS.md"
  - "docs/ADR--GENESISRAG17-SEPARATE-WORKER-PUBLICATION.md"
  - "docs/FLOW--GENESISRAG17-PIPELINE.md"
  - "docs/GENESISRAG17-EXTENSION-MAP.md"
  - "docs/SPEC--GENESISDB-P6-GENERATIONS-LEASES-ACL.md"
---

# GenesisBlockDB Document Registry

## 1. Purpose

This registry identifies the active product, requirements, architecture, contract, evidence, and historical entrypoints for GenesisBlockDB.

It does not replace code and test evidence. Product documents define intent and requirements; implementation status must remain traceable to code, tests, benchmarks, audits, and release evidence.

## 2. Registry rules

- One canonical product definition per document role.
- Product requirements must remain independent of GoVibe, NotiKeeper, or another single client.
- Client-specific schemas live in client repositories or adapters.
- `source_of_truth: false` and superseded documents do not compete with active canonical documents.
- Performance claims must point to reproducible evidence.
- A path in this registry must exist at the registered revision.

The registry is an index of canonical entrypoints, not an exhaustive catalog
of every historical, exploratory, benchmark, interview, or ADR document under
`docs/`. Unregistered documents remain reviewable references, but they do not
claim product-definition or implementation-status authority unless a current
registry row explicitly names them.

## 3. Product and requirements

| Role | Doc ID | Version | Status | Owner | Path |
|---|---|---|---|---|---|
| BRD | `BRD-GENESISBLOCKDB` | `0.1.0+draft` | draft | Freshair129 / Product Authority | `docs/BRD--GENESISBLOCKDB.md` |
| PRD | `PRD-GENESISBLOCKDB-PLATFORM` | `0.1.1+draft` | draft | Freshair129 / Product Authority | `docs/PRD--GENESISBLOCKDB-PLATFORM.md` |
| SRS | `SRS-GENESISBLOCKDB` | `0.1.1+draft` | draft | GenesisBlockDB Engineering | `docs/SRS--GENESISBLOCKDB.md` |

## 4. Architecture and contracts

| Role | Doc ID | Version | Status | Owner | Path |
|---|---|---|---|---|---|
| Architecture composition | `MASTER-SPEC-GENESISBLOCKDB` | `2.3.32b` | current | GenesisBlockDB Architecture | `docs/MASTER-SPEC--GENESIS-DB.md` |
| Architecture index | `C4--GENESISDB-ARCHITECTURE` | `0.1.60b` | current | GenesisBlockDB Architecture | `docs/C4--GENESISDB-ARCHITECTURE.md` |
| HQL2 execution decision | `ADR--GENESISDB-HQL2-EXECUTION-BOUNDARY` | `0.1.1b` | accepted | Boss (Founder / Product Authority) | `docs/adr/ADR--GENESISDB-HQL2-EXECUTION-BOUNDARY.md` |
| HQL2 P8 completion addendum | `ADR--GENESISDB-HQL2-P8-COMPLETION-ADDENDUM` | `0.1.6b` | accepted | Boss (Founder / Product Authority) | `docs/adr/ADR--GENESISDB-HQL2-P8-COMPLETION-ADDENDUM.md` |
| HQL2 Sequence pattern constraints | `ADR--GENESISDB-HQL2-PATTERN-CONSTRAINTS` | `0.2.1b` | beta | Boss (Founder / Product Authority) | `docs/adr/ADR--GENESISDB-HQL2-PATTERN-CONSTRAINTS.md` |
| HQL2 durable revisions and annotations | `ADR--GENESISDB-HQL2-DURABLE-REVISIONS-ANNOTATIONS` | `0.8.17b` | beta | Boss (Founder / Product Authority) | `docs/adr/ADR--GENESISDB-HQL2-DURABLE-REVISIONS-ANNOTATIONS.md` |
| UEE-HQL2 orchestration plan | `IMPLEMENTATION-PLAN--UEE-HQL2-ORCHESTRATION-2026-09-22` | `0.8.53b` | beta | Boss (Founder / Product Authority) | `docs/IMPLEMENTATION-PLAN--UEE-HQL2-ORCHESTRATION-2026-09-22.md` |
| HQL2 typed P8 boundary | `SPEC--GENESISDB-HQL2-P8-TYPED-BOUNDARY` | `0.2.50b` | beta | Boss (Founder / Product Authority) | `docs/SPEC--GENESISDB-HQL2-P8-TYPED-BOUNDARY.md` |
| HQL2 P8 core checkpoint | `REPORT--HQL2-P8-CORE-2026-09-28` | `0.1.49b` | beta | GenesisBlockDB Engineering | `docs/REPORT--HQL2-P8-CORE-2026-09-28.md` |
| HQL2 P7 bounded oracle evidence | `REPORT--HQL2-P7-ORACLE-2026-09-28` | `0.1.4b` | beta | GenesisBlockDB Engineering | `docs/REPORT--HQL2-P7-ORACLE-2026-09-28.md` |
| Commit correctness | `SPEC--WAVE-A-COMMIT-CORRECTNESS` | `0.1.0b` | beta | GenesisBlockDB Engineering | `docs/SPEC--WAVE-A-COMMIT-CORRECTNESS.md` |
| Durable collections and edge history | `SPEC--WAVE-B-DURABLE-COLLECTIONS-EDGE-HISTORY` | `0.1.2b` | beta | GenesisBlockDB Engineering | `docs/SPEC--WAVE-B-DURABLE-COLLECTIONS-EDGE-HISTORY.md` |
| Generation, lease, temporal and ACL contract | `SPEC--GENESISDB-P6-GENERATIONS-LEASES-ACL` | `0.5.27b` | beta | GenesisBlockDB Engineering | `docs/SPEC--GENESISDB-P6-GENERATIONS-LEASES-ACL.md` |
| Query correctness | `SPEC--WAVE-C-QUERY-CORRECTNESS` | `0.2.1b` | beta | GenesisBlockDB Engineering | `docs/SPEC--WAVE-C-QUERY-CORRECTNESS.md` |
| Query budgets and quality gates | `SPEC--WAVE-D-BUDGETS-QUALITY-GATES` | `0.1.0b` | beta | GenesisBlockDB Engineering | `docs/SPEC--WAVE-D-BUDGETS-QUALITY-GATES.md` |
| Query context and client capability | `SPEC--WAVE-E-QUERY-CONTEXT-CLIENT-CAPABILITY` | `0.1.2b` | beta | GenesisBlockDB Engineering | `docs/SPEC--WAVE-E-QUERY-CONTEXT-CLIENT-CAPABILITY.md` |
| ADR | `ADR-GENESISBLOCKDB-DOMAIN-NEUTRAL-CORE` | `0.1.0+draft` | proposed | GenesisBlockDB Architecture | `docs/adr/ADR--GENESISBLOCKDB-DOMAIN-NEUTRAL-CORE.md` |
| ADR | `ADR--GENESISDB-TYPED-QUERY-IR-AGENT-BOUNDARY` | `1.0.1` | accepted | Product Authority | `docs/adr/ADR--GENESISDB-TYPED-QUERY-IR-AGENT-BOUNDARY.md` |
| Query contract | `SPEC-GENESISDB-TYPED-QUERY-IR-V1` | `1.0.3` | accepted | GenesisBlockDB Architecture | `docs/SPEC--GENESISDB-TYPED-QUERY-IR-V1.md` |
| Client contract | `CONTRACT-CLIENT-NAMESPACE-AND-SCHEMA` | `0.1.0+draft` | draft | GenesisBlockDB Engineering | `docs/contracts/CONTRACT--CLIENT-NAMESPACE-AND-SCHEMA.md` |
| API reference | `API_REFERENCE` | generated | current | GenesisBlockDB Engineering | `docs/API_REFERENCE.md` |
| GenesisRAG17 integration ADR | `ADR-GENESISRAG17-SEPARATE-WORKER-PUBLICATION` | `1.0.5b` | beta | GenesisBlockDB Architecture | `docs/ADR--GENESISRAG17-SEPARATE-WORKER-PUBLICATION.md` |
| GenesisRAG17 execution flow | `FLOW-GENESISRAG17-PIPELINE` | `1.0.3b` | beta | GenesisBlockDB Architecture | `docs/FLOW--GENESISRAG17-PIPELINE.md` |
| GenesisRAG17 extension map | `MAP-GENESISRAG17-EXTENSIONS` | `1.0.3b` | beta | GenesisBlockDB Architecture | `docs/GENESISRAG17-EXTENSION-MAP.md` |

## 5. Product narrative and evidence

| Role | Doc ID | Version | Status | Owner | Path |
|---|---|---|---|---|---|
| Positioning | `POSITIONING-GENESISBLOCKDB` | n/a | current | GenesisBlockDB Product | `docs/POSITIONING.md` |
| Whitepaper | `WHITEPAPER-GENESISBLOCKDB-SEMANTIC-SUBSTRATE` | `0.1.0+draft` | draft | GenesisBlockDB Architecture | `docs/WHITEPAPER--GENESISBLOCKDB-SEMANTIC-SUBSTRATE.md` |
| Database whitepaper | `WHITEPAPER--GENESIS-DB` | n/a | current | GenesisBlockDB Architecture | `docs/WHITEPAPER--GENESIS-DB.md` |
| Performance report | `REPORT--2026-06-21-PERFORMANCE-AND-COMPETITIVE` | n/a | historical | GenesisBlockDB Engineering | `docs/REPORT--2026-06-21-PERFORMANCE-AND-COMPETITIVE.md` |
| Product version | `VERSION` | current | current | GenesisBlockDB Engineering | `docs/VERSION.md` |

## 6. Historical and superseded documents

| Role | Doc ID | Version | Status | Replaced by | Path |
|---|---|---|---|---|---|
| Historical terminology whitepaper | `WHITEPAPER-GENESIS-KNOWLEDGE-SYSTEM-HISTORICAL` | `2.1.0-superseded` | superseded | `WHITEPAPER-GENESISBLOCKDB-SEMANTIC-SUBSTRATE` | `docs/WHITEPAPER--GENESIS-KNOWLEDGE-SYSTEM.md` |
| Historical implementation status snapshot | `DOC-STATUS-GENESISBLOCKDB-HISTORICAL` | `2026.06.21+archived` | superseded | `DOC-REGISTRY-GENESISBLOCKDB` plus current code/test evidence | `docs/DOC-STATUS.md` |

## 7. Client boundary

```text
GoVibe domain       NotiKeeper domain       Future client domain
      |                     |                         |
      +-------- client adapters / SDK contracts -----+
                            |
                GenesisBlockDB generic core
```

The registry records GenesisBlockDB product documents only. GoVibe and NotiKeeper application schemas are external client contracts and must not be registered as native GenesisBlockDB ontology.

## 8. Known follow-up documents

The following documents should be created only when implementation work requires them:

- generic node/edge mutation contract;
- WAL durability acknowledgment contract;
- backup/restore and migration contract;
- SDK/interface conformance matrix;
- NotiKeeper adapter conformance report;
- third-client namespace conformance report.

## 9. Changelog

Version diff 0.5.59+draft -> 0.5.60+draft: register the Node-revision
ChangeScan differential against P7, synchronize P7/P8/plan versions, and
record PR #194 merged at `4f02d6b` with 48 checks passing, five failing and
five skipped. History/Change 15/15 and the 32-target HQL2 sweep 379/0/1;
other ChangeScan source kinds, worker correction and broad acceptance remain
open.

Version diff 0.5.58+draft -> 0.5.59+draft: register the WAL-derived P7
HistoryScan differentials for all five supported kinds and synchronize the
P7/P8/P6/plan/report/C4/master version chain. Record History/Change 15/15,
Annotation source 7/7, P7 130/130, HQL2 379/0/1 and P6/schema-v6/compatibility
194/0/0. Artifact HistoryScan, broad ChangeScan/P8/P13, hosted checks and review
remain open.

Version diff 0.5.55+draft -> 0.5.56+draft: synchronize H2-D11, P6, P8,
plan, C4 and master references to the verified annotation ACL conformance
change. The regression passes; HQL2 is 375/0/1 across 32 targets and the
selected P6/schema-v6 suite is 43/0/0. At the prior PR head, worker checks fail
and the Windows Rust job was cancelled; this source patch awaits hosted CI and
review. No contract/schema/migration change; broad P8/P13 gates remain open.

Version diff 0.5.54+draft -> 0.5.55+draft: synchronize P6, C4, master,
H2-D11, orchestration plan and P8 report versions with PR #194 run
37083654705 at docs-only head 43cc6e8. Record four worker bootstrap failures
and a Windows Rust Join budget failure; local Join target passes 5/5, exact
budget dimension remains unconfirmed, and PR remains OPEN/UNSTABLE. Keep
worker correction approval-gated and broad acceptance open.

Version diff 0.5.53+draft -> 0.5.54+draft: synchronize P6, C4, master,
H2-D11, orchestration plan and P8 report versions with the latest HQL2
regression checkpoint (374/0/1 across 32 targets); record PR #194 head
31168524 as OPEN/UNSTABLE with four worker checks failing at fresh schema-v6
bootstrap. Keep worker correction approval-gated and broad acceptance open.

Version diff 0.5.52+draft -> 0.5.53+draft: register HQL/typed-IR Row
HistoryScan/ChangeScan parity and exact row-property hydration (1/1), update
the synchronized H2-D11/P8/plan/report versions, and record the 374/0/1
32-target HQL2 regression sweep. Broad P8/P13, hosted CI and independent
review gates remain open.

Version diff 0.5.51+draft -> 0.5.52+draft: register the implemented HQL2/IR
`tx_as_of` path with one no-fallback frontier across sources, graph/vector/
annotation operators, hydration and `Snapshot.tx`; record focused 56/56,
History/Change 14/14, HQL2 373/0/1 and P6/compatibility 194/0/0. Local
regression is not broad P8/P13 acceptance; hosted CI and independent review
remain open.

Version diff 0.5.50+draft -> 0.5.51+draft: synchronize the accepted P8
transaction-time snapshot contract across P6, H2-D11, P8, report and plan;
record that runtime currently rejects explicit `tx_as_of` and uses the current
frontier, with implementation/verification and hosted CI/review/broad P8/P13
gates still open.

Version diff 0.5.49+draft -> 0.5.50+draft: register the verified H2-D11 Vector
HistoryScan implementation and synchronize H2-D11, P6, P8, checkpoint and plan
versions; record the still-open P8 `tx_as_of`, hosted CI, broader acceptance and
independent-review gates.

Version diff 0.5.47+draft -> 0.5.48+draft: register HQL `JOIN TABLE` lowering
through the approved RowScan/Join contract for Inner/Left/Semi/Anti and bare
JOIN default; HQL/typed IR match independent P7, including duplicate,
missing-property NULL and JSON-null result bags. Record 365/0/1 across 31
HQL2 targets; synchronize P8, checkpoint, plan, Master and C4. Hosted CI for
this local revision is pending; broader P8/P13/review gates remain open.

Version diff 0.5.46+draft -> 0.5.47+draft: register the storage-backed typed-IR
Join P7 differential for Inner/Left/Semi/Anti and 364/0/1 across 30 HQL2
targets; synchronize P8, checkpoint, plan and C4 versions. At parent PR head
484916b core checks pass but worker checks fail on all OSes with
`RECOVERY_REQUIRED: markerless database identity is missing`; no reviews exist,
and the local test is not in CI yet, so PR #194 stays open and unmerged.

Version diff 0.5.45+draft -> 0.5.46+draft: register the storage-backed
HQL/typed-IR aggregate differential against independent P7 over 121 nullable
bags; synchronize P8, checkpoint, plan and C4 versions; record 363/0/1 across
29 HQL2 targets. Hosted Rust/core checks pass on all three OSes, but worker
checks fail across OSes and review remains pending, so merge and broader P8/
P13/review gates remain open.

Version diff 0.5.44+draft -> 0.5.45+draft: register the storage-backed
HQL/typed-IR scalar differential against independent P7 over 81 nullable bags;
synchronize P8, checkpoint, plan and C4 versions; record 362/0/1 across 28
HQL2 targets and 190/0/0 across 11 compatibility targets. Hosted worker checks
still fail; retain merge and broader P8/P13/review gates.

Version diff 0.5.43+draft -> 0.5.44+draft: synchronize final full Rust and
strict Clippy verification across H2-D11, P6, P8, plan and parent documents;
retain `probe_vs_recall` as NOT_RUN and all broader P8/P13/review/release gates.

Version diff 0.5.42+draft -> 0.5.43+draft: record verified H2-D11 R6b
schema-v6 recovery, sync Query IR V1 1.0.3/match_path from upstream, and
update the parent/P6/HQL2 verification records; full P8/P13 gates remain open.

Version diff 0.5.41+draft -> 0.5.42+draft: register owner-approved H2-D11
R6b/P6 schema-v6 activation authority for WAL-only recovery and synchronize
ADR, P6, Master, C4 and HQL2 plan versions. Runtime recovery remains pending.

Version diff 0.5.40+draft -> 0.5.41+draft: register H2-D11 ADR 0.8.6b's
approved AnnotationPut evidence-field clarification; evidence remains distinct
from targets and follows the existing reference ACL path. No schema or
migration change is authorized by this documentation update.

Version diff 0.5.39+draft -> 0.5.40+draft: register D7's differential-proven
one-hop endpoint-ID exact string filter, 9/9 focused adapter tests, 361/0/1
across 27 root HQL2 targets and 190/0/0 across 11 separate compatibility
targets; synchronize canonical HQL2 documents and retain other HQL1,
independent-review, shared-runtime/P8/P13 gates.

Version diff 0.5.35+draft -> 0.5.36+draft: synchronize canonical versions and
register approved P8 D1-D5 implementation, the 9/9 focused target, three D5
literal-size unit tests and 528/0/1 across 37 explicit regression targets.
Retain independent review and broad P8/P13 acceptance gates.

Version diff 0.5.34+draft -> 0.5.35+draft: register owner-approved P8
completion addendum D1-D6 and synchronize the P8, P6, pattern ADR, plan,
master, architecture index and core checkpoint; implementation evidence was
pending at that checkpoint.

Version diff 0.5.33+draft -> 0.5.34+draft: register the implemented
lease-bound HQL/typed-IR Sequence node-ID/label slice, 7 focused passes and
338/0/1 across 25 HQL2 targets; retain ACL-hidden fixture and P8 gates.

Version diff 0.5.32+draft -> 0.5.33+draft: register the delegated HQL2
Sequence node-ID/conjunctive-label contract across P6/P8 and the implementation
plan; runtime implementation and verification remain pending.

Version diff 0.5.31+draft -> 0.5.32+draft: register a passing HQL root-Match
differential against the independent P7 graph bag and 331/0/1 across 24 root
targets; broad exact-oracle and P8 acceptance remain open.

Version diff 0.5.30+draft -> 0.5.31+draft: register HQL/IR exact-KNN and
Original Rerank differentials against the independent P7 oracle and 330/0/1
across 23 root targets; broad exact-oracle and P8 acceptance remain open.

Version diff 0.5.28+draft -> 0.5.29+draft: register the candidate HQL2
Sequence node ID/label contract and P6 dataflow diagram; no source changes are
authorized before owner approval.

Version diff `0.5.27+draft -> 0.5.28+draft`: synchronize checked HQL
remainder/typed-IR `rem` across the P8 contract, plan, checkpoint, architecture
index and master spec; record 79 focused passes and 328/0/1 across 22 HQL2
target tests. Constrained patterns, remaining operators, transport, independent
review and full P8 gates remain open.

| Version | Date | Owner | Summary |
|---|---|---|---|
| 0.5.60+draft | 2026-10-03 | GenesisBlockDB Architecture | Register the Node-revision ChangeScan P7 differential and updated P7/P8/plan versions; record PR #194 merged with 48 pass, five fail and five skipped; retain other source-kind and broad HQL2 acceptance gates |
| 0.5.59+draft | 2026-10-03 | GenesisBlockDB Architecture | Register WAL-derived independent P7 HistoryScan differentials for Node/Edge/Row/Vector/Annotation and synchronize HQL2/P7/P8/P6/plan/C4/master versions; History/Change 15/15, Annotation source 7/7, P7 130/130, HQL2 379/0/1, P6/schema-v6/compatibility 194/0/0; Artifact and broad acceptance/hosted review remain open |
| 0.5.58+draft | 2026-10-03 | GenesisBlockDB Architecture | Register the WAL-derived independent P7 Vector HistoryScan differential and synchronize P7/P8/P6/plan/report/C4/master versions; P7 130/130, History/Change 14/14, HQL2 377/0/1, P6/schema-v6/compatibility 194/0/0; prior PR head has five failed checks and no reviews, new checks pending; broad P8/P13 open |
| 0.5.56+draft | 2026-10-03 | GenesisBlockDB Architecture | Synchronize HQL2/P6/plan/report/C4/master references for explicit Annotation(Read) on annotation source and ChangeScan subjects; HQL2 375/0/1 and selected P6/schema-v6 43/0/0 pass; possible ChangeScan budget side channel, prior PR worker failures and cancelled Windows Rust remain open |
| 0.5.55+draft | 2026-10-03 | GenesisBlockDB Architecture | Synchronize HQL2/P6/plan/report references and PR #194 run 37083654705: four worker failures plus one Windows Rust Join budget failure; PR OPEN/UNSTABLE, no implementation change, broad P8/P13/review gates remain open |
| 0.5.54+draft | 2026-10-03 | GenesisBlockDB Architecture | Synchronize HQL2/P6/plan/report references to current versions and 374/0/1 across 32 targets; record PR #194 as OPEN/UNSTABLE with four worker-CI failures; worker correction remains approval-gated, broad P8/P13/review gates open |
| 0.5.53+draft | 2026-10-03 | GenesisBlockDB Architecture | Register HQL/typed-IR Row HistoryScan/ChangeScan parity, exact row-property hydration (1/1), and HQL2 regression 374/0/1 across 32 targets; hosted CI, independent review and broad P8/P13 gates remain open |
| 0.5.52+draft | 2026-10-03 | GenesisBlockDB Architecture | Register implemented HQL2/IR `tx_as_of` with one no-fallback frontier; focused 56/56, History/Change 14/14, HQL2 373/0/1 and P6/compatibility 194/0/0; hosted CI, review and broad P8/P13 gates remain open |
| 0.5.51+draft | 2026-10-03 | GenesisBlockDB Architecture | Synchronize accepted HQL2 `tx_as_of` contract and record runtime rejection/current-frontier gap; runtime acceptance, hosted CI/review and broader P8/P13 gates remain open |
| 0.5.50+draft | 2026-10-03 | GenesisBlockDB Architecture | Register and synchronize verified Vector HistoryScan HQL/IR parity, P6 vector floor and owner ACL; HQL2 367/0/1 and P6/compatibility 194/0/0; keep tx_as_of, hosted CI, review and broader P8/P13 gates open |
| 0.5.49+draft | 2026-10-03 | GenesisBlockDB Architecture | Register approved H2-D11 Vector HistoryScan identity, floor and owner ACL; synchronize P6/P8/plan versions, with runtime implementation and parity verification pending |
| 0.5.48+draft | 2026-10-03 | GenesisBlockDB Architecture | Implement HQL JOIN TABLE lowering through the approved RowScan/Join contract for four kinds and bare default; HQL/typed IR match independent P7 for duplicate, missing-property NULL and JSON-null result bags; record 365/0/1 across 31 targets; hosted CI pending, P8/P13/review gates remain open |
| 0.5.47+draft | 2026-10-03 | GenesisBlockDB Architecture | Register storage-backed typed-IR Join P7 differential for all four join kinds and 364/0/1 across 30 HQL2 targets without expanding HQL JOIN; parent head 484916b core checks pass but worker checks fail and no PR review exists; PR remains unmerged |
| 0.5.46+draft | 2026-10-03 | GenesisBlockDB Architecture | Register storage-backed HQL/typed-IR P7 aggregate differential over 121 nullable bags; synchronize canonical HQL2/plan/C4 versions and 363/0/1 across 29 targets; hosted Rust/core checks pass on Linux/macOS/Windows, worker checks fail across OSes and review remains pending. |
| 0.5.45+draft | 2026-10-03 | GenesisBlockDB Architecture | Register storage-backed HQL/typed-IR P7 scalar differential over 81 nullable bags; synchronize canonical HQL2/plan/C4 versions and 362/0/1 plus 190/0/0 regression results; latest worker CI and Windows Rust checks fail, broader merge/P8/P13/review gates remain open. |
| 0.5.44+draft | 2026-10-02 | GenesisBlockDB Architecture | Synchronize full locked/offline Rust suite and both strict Clippy results across current HQL2/H2-D11/P6 documents; `probe_vs_recall` NOT_RUN and broader qualification gates remain open. |
| 0.5.43+draft | 2026-10-02 | GenesisBlockDB Architecture | Record verified R6b fixture recovery, Query IR V1 1.0.3/match_path integration and 40 passing HQL2/durability/authority targets; full P8/P13 qualification remains open. |
| 0.5.42+draft | 2026-10-02 | GenesisBlockDB Architecture | Register owner-approved H2-D11 R6b schema-v6 WAL-only recovery contract and synchronized parent/P6/plan docs; implementation pending. |
| 0.5.41+draft | 2026-10-02 | GenesisBlockDB Architecture | Register owner-approved H2-D11 AnnotationPut evidence-field clarification and ADR 0.8.6b; separate evidence references remain under existing ACL behavior. |
| 0.5.40+draft | 2026-10-02 | GenesisBlockDB Architecture | Register D7's differential-proven one-hop endpoint-ID exact string filter, 9/9 focused tests, 361/0/1 HQL2 and 190/0/0 compatibility regression results; retain shared-runtime, independent review and P8/P13 gates. |
| 0.5.39+draft | 2026-10-02 | GenesisBlockDB Architecture | Register D7's differential-proven one-hop HQL1 extension and parser preflight, 8/8 focused tests, 361/0/1 HQL2 and 190/0/0 compatibility regression results; retain shared-runtime, independent review and P8/P13 gates. |
| 0.5.38+draft | 2026-10-02 | GenesisBlockDB Architecture | Register D7's actor-scoped HQL1 projection, 5/5 adapter tests and 354/0/1 across 27 HQL2 targets; synchronize canonical documents and retain shared-runtime/P8/P13 gates. |
| 0.5.37+draft | 2026-10-02 | GenesisBlockDB Architecture | Record the D4 edge-property-before-SHORTEST regression, 10/10 P8 completion tests and 349/0/1 across 26 HQL2 targets plus separate P6/schema-v6/compatibility evidence; retain broad P8/P13 gates. |
| 0.5.36+draft | 2026-10-02 | GenesisBlockDB Architecture | Synchronize canonical HQL2 doc versions; register D1-D5 implementation, focused/budget/literal-size tests, and 528/0/1 across 37 selected targets; retain independent review and broad P8/P13 gates. |
| 0.5.35+draft | 2026-10-02 | GenesisBlockDB Architecture | Register accepted P8 D1-D6 decisions and synchronized contracts; no implementation or P8-completion evidence inferred. |
| 0.5.34+draft | 2026-09-30 | GenesisBlockDB Architecture | Register implemented lease-bound HQL/typed-IR Sequence node-ID/label slice, 7 focused passes and 338/0/1 across 25 HQL2 targets; retain ACL-hidden fixture and broader P8 gates. |
| 0.5.33+draft | 2026-09-30 | GenesisBlockDB Architecture | Register delegated HQL/typed-IR Sequence node-ID/label contract under P6; runtime implementation pending, with properties/Compact constraints fail-closed. |
| 0.5.32+draft | 2026-09-30 | GenesisBlockDB Architecture | Register passing HQL root-Match differential against independent P7 graph bag and 331/0/1 across 24 root HQL2 targets; retain broad oracle and P8 gates. |
| 0.5.31+draft | 2026-09-30 | GenesisBlockDB Architecture | Register passing HQL/IR exact-KNN and Original Rerank differentials against independent P7 ranking and 330/0/1 across 23 root HQL2 targets; retain broad oracle and P8 gates. |
| 0.5.30+draft | 2026-09-30 | GenesisBlockDB Architecture | Register passing HQL/IR exact-KNN differential against the independent P7 oracle and 329/0/1 across 23 root HQL2 targets; retain broad oracle and P8 gates. |
| 0.5.29+draft | 2026-09-30 | GenesisBlockDB Architecture | Register candidate lease-bound HQL/IR Sequence node ID/label semantics and the approval gate; implementation remains unauthorized pending owner approval. |
| 0.5.28+draft | 2026-09-30 | GenesisBlockDB Architecture | Register checked HQL `%`/typed-IR `rem`, 79 focused passes and 328/0/1 across 22 HQL2 targets; retain remaining HQL2/P8 gates. |
| 0.5.27+draft | 2026-09-30 | GenesisBlockDB Architecture | Register HQL2 NULLS LAST default and exact DecimalU64 structural parameters; record 58 focused passes and 325/0/1 across 22 targets; retain remaining HQL2/P8 gates. |
| 0.5.26+draft | 2026-09-30 | GenesisBlockDB Architecture | Register exact typed DecimalU64/no-cast structural HQL parameters and 323/0/1 across 22 HQL2 targets; retain remaining HQL2/P8 gates. |
| 0.5.25+draft | 2026-09-30 | GenesisBlockDB Architecture | Register exact typed-IR root Match anchors and 319/0/1 across 22 HQL2 targets; retain remaining HQL2 and P8 gates. |
| 0.5.24+draft | 2026-09-30 | GenesisBlockDB Architecture | Synchronize P6 spec and P8 report to HistoryScan/ChangeScan runtime and 9/9 focused tests; retain remaining operator and P8 gates. |
| 0.5.23+draft | 2026-09-30 | GenesisBlockDB Architecture | Register lease-bound HistoryScan/ChangeScan, recursive reference ACL, 9/9 focused tests and 318/0/1 across 22 HQL2 targets; retain remaining operator and P8 gates. |
| 0.5.22+draft | 2026-09-29 | GenesisBlockDB Architecture | Register exact HQL/IR KNN and Original Rerank on lease-bound original vectors; synchronize H2-D11/P6/P8 and parent documents; record 307/0/1 across 21 root HQL2 targets and retain remaining operator/P8 gates. |
| 0.5.21+draft | 2026-09-29 | GenesisBlockDB Architecture | Record the RowScan after_image correction and then-current 296/3/1 HQL2 sweep; synchronize plan/spec/report/C4 and retain remaining operator/P8 gates. |
| 0.5.20+draft | 2026-09-29 | GenesisBlockDB Architecture | Register focused RowScan property-hydration RED and RCA, synchronize HQL2 status docs, and retain the specific runtime approval gate. |
| 0.5.19+draft | 2026-09-29 | GenesisBlockDB Architecture | Register the explicit 21-target HQL2 sweep; retain three KNN/Rerank failures, distinguish oracle evidence from P8 acceptance and synchronize report/plan versions. |
| 0.5.18+draft | 2026-09-29 | GenesisBlockDB Architecture | Register 26/26 Blueprint semantic-oracle passes for separate annotation evidence; synchronize HQL2 plan/report versions and retain operator/P8 gates. |
| 0.5.17+draft | 2026-09-29 | GenesisBlockDB Architecture | Record 62/62 separate H2-D11 fixture tests and synchronize orchestration plan/report versions; retain HQL2 operator and P8 gates. |
| 0.5.16+draft | 2026-09-29 | GenesisBlockDB Architecture | Register structural HQL/IR root Match with deterministic SHORTEST; retain anchors, constrained patterns, six unavailable operators, vector/P8 and release gates. |
| 0.5.15+draft | 2026-09-29 | GenesisBlockDB Architecture | Register typed IR Vector parameter decoding with space/dimension checks and two focused passing tests; retain HQL inference, original-vector reads, Knn/Rerank, P8 and release gates. |
| 0.5.14+draft | 2026-09-29 | GenesisBlockDB Architecture | Register structural HQL/IR Sequence Expand; update HQL2 checkpoint to 206 passing tests across 21 targets, with pattern-constraint/P8/release gates open. |
| 0.5.13+draft | 2026-09-29 | GenesisBlockDB Architecture | Register constrained HQL/IR Expand and separate AnnotationPut evidence fixture/oracle; update HQL2 checkpoint to 199 passing tests across 20 targets, with P8/release gates open. |
| 0.5.12+draft | 2026-09-29 | GenesisBlockDB Architecture | Register HQL/IR AnnotationLookup semantics and 194 passing P8/P6/H2-D11 tests across 19 targets; keep P8 and release gates open. |
| 0.5.11+draft | 2026-09-29 | GenesisBlockDB Architecture | Register typed selective HQL2 hydration and 188 passing P8/P6 tests across 18 targets; keep remaining operators, transport and P8 partial. |
| 0.5.10+draft | 2026-09-29 | GenesisBlockDB Architecture | Register four revision-bound source scans, annotation reference ACL, payload-budget precharge and 133/133 selected P8/P6 tests; keep P8 partial. |
| 0.5.9+draft | 2026-09-29 | GenesisBlockDB Architecture | Register annotation CAS-write, normalized target/evidence projection, P6 policy-event v2 and 8/8 tests; keep annotation read ACL and P8 source adapters open. |
| 0.5.8+draft | 2026-09-29 | GenesisBlockDB Architecture | Register durable vector revision write/projection/reopen and fail-closed ingress evidence; annotation/ACL, source adapters, user DB migration and release gates remain open. |
| 0.5.7+draft | 2026-09-29 | GenesisBlockDB Architecture | Register 19/19 fixture migration/recovery evidence and H2-D11/P6/P8/plan/C4 checkpoint versions; user DB migration and release qualification remain open. |
| 0.5.6+draft | 2026-09-29 | GenesisBlockDB Architecture | Record owner approval of amended H2-D11 ADR 0.4.0b and fixture-only migration implementation authority; synchronize ADR, P6, plan and C4 statuses. |
| 0.5.5+draft | 2026-09-28 | GenesisBlockDB Architecture | Register amended H2-D11 recovery, fold/cold-reopen, reader-compatibility, peer-isolation and receipt-proof gates; ADR 0.4.0b awaits owner approval. |
| 0.5.4+draft | 2026-09-28 | GenesisBlockDB Architecture | Register owner-approved migration envelope/source-coordinate candidate and pending implementation gate across ADR, P6, plan and C4. |
| 0.5.3+draft | 2026-09-28 | GenesisBlockDB Architecture | Register fixture-backed relational row revision and registry persistence evidence; retain vector, annotation, adapter, migration and folded-peer gaps. |
| 0.5.2+draft | 2026-09-28 | GenesisBlockDB Architecture | Register current H2-D11 graph revision implementation evidence and updated plan/ADR versions while keeping remaining scope explicit. |
| 0.5.1+draft | 2026-09-28 | GenesisBlockDB Architecture | Correct H2-D11 implementation status and register updated ADR/plan and focused KeyCodec evidence without implying migration completion. |
| 0.5.0+draft | 2026-09-28 | GenesisBlockDB Architecture | Register owner-approved H2-D11 and synchronize its approved P8/P6 contracts and staged implementation plan. |
| 0.4.8+draft | 2026-09-29 | GenesisBlockDB Architecture | Registered Typed Query IR V1 1.0.3 and C4 0.1.14b after implementing the typed linear match_path slice. |
| 0.4.7+draft | 2026-09-23 | GenesisBlockDB Architecture | Sync P6 spec version after snapshot-authority and HQL ACL review corrections. |
| 0.4.6+draft | 2026-09-23 | GenesisBlockDB Architecture | Classify Enforced-ACL legacy accessors and refresh the P6 specification version. |
| 0.4.5+draft | 2026-09-22 | GenesisBlockDB Architecture | Registered the owner-approved P6 contract and synchronized Master/C4 document versions. |
| 0.4.4+draft | 2026-09-22 | GenesisBlockDB Architecture | Registered the approved Wave E query context and client capability implementation record. |
| 0.4.3+draft | 2026-09-10 | GenesisBlockDB Architecture | Truth-synced the Wave D beta status and Typed Query IR 1.0.2 registry entries after release verification. |
| 0.4.2+draft | 2026-09-08 | RWANG | Reconcile GenesisRAG17 document versions 1.0.3b after zuri's pre-merge ADR-071 to ADR-073 collision repair; preserve historical rows. |
| 0.4.1+draft | 2026-09-08 | GenesisBlockDB Architecture | Reconciled the GenesisRAG17 registry entries with the live zuri ADR-071 reference and retained historical report links. |
| 0.4.0+draft | 2026-09-08 | GenesisBlockDB Architecture | Registered the GenesisRAG17 TEST worker/publication ADR, execution flow and extension map. |
| 0.3.9+draft | 2026-09-08 | GenesisBlockDB Engineering | Registered the Wave D candidate for query budgets, REST execution control and per-index quality evidence. |
| 0.3.8+draft | 2026-09-08 | GenesisBlockDB Engineering | Recorded large-collection filtered-ANN fix `d8ef9af`, 28-cell oracle evidence and rebuilt N-API/MCP runtime pass. |
| 0.3.7+draft | 2026-09-08 | GenesisBlockDB Engineering | Recorded approved Wave C implementation checkpoint `cbe5a04` and beta exit evidence. |
| 0.3.6+draft | 2026-09-08 | GenesisBlockDB Engineering | Registered the Wave C query-correctness candidate packet. |
| 0.3.5+draft | 2026-09-08 | GenesisBlockDB Engineering | Registered the B4 identity-preserving replay benchmark correction. |
| 0.3.4+draft | 2026-09-08 | GenesisBlockDB Engineering | Recorded Wave B approval, implementation and synchronized parent versions. |
| 0.3.3+draft | 2026-09-08 | GenesisBlockDB Engineering | Registered Wave B candidate design; implementation remains unapproved. |
| 0.3.2+draft | 2026-09-08 | GenesisBlockDB Architecture | Registered approved Wave A commit contract and synchronized parent architecture versions. |
| 0.3.0+draft | 2026-08-14 | GenesisBlockDB Architecture | Registered the accepted Typed Query IR ADR/spec and removed it from the follow-up list. |
| 0.3.1+draft | 2026-08-14 | GenesisBlockDB Architecture | Truth-synced Query IR spec, ADR and C4 versions after the partial search/traverse implementation. |
| 0.2.0+draft | 2026-08-13 | GenesisBlockDB Architecture | Reconciled registered frontmatter and defined canonical-entrypoint scope for automated validation. |
| 0.1.0+draft | 2026-08-03 | GenesisBlockDB Architecture | Established the active standalone-product registry and separated historical status snapshots. |
