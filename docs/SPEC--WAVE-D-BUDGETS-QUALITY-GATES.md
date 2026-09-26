---
doc_id: SPEC--WAVE-D-BUDGETS-QUALITY-GATES
owner: GenesisBlockDB Engineering
version: "0.1.0b"
created_at: "2026-09-08T18:30:00+07:00,ATHER,5463296"
last_update: "2026-09-08T20:35:00+07:00,ATHER"
status: beta
superseded_by: null
attributes:
  domain: query-budgets-quality
  scope: wave-d-r09-r11
  complexity: C-3
  risk: MEDIUM-HIGH
---

# Wave D — Query budgets and per-index quality gates

## 1. Decision requested

Wave D follows the verified Wave C checkpoint `5463296` on
`codex/wave-c-query-correctness`. It proposes scoped work for R-09 (query
budgets and REST execution control) and R-11 (per-index retrieval quality
evidence). This document records the approved Wave D scope. Local
implementation and evidence are complete on the isolated worktree; it
authorizes no production release, push, merge or deployment claim.

Complexity is **C-3** because the work crosses core query execution, HQL,
Query IR, GRL, REST scheduling and benchmark evidence. Risk is
**MEDIUM-HIGH**: a missing bound can turn a valid graph query into unbounded
CPU, memory or response work, while a weak quality gate can publish a misleading
recall claim.

## 2. Parent and peer alignment

- Wave C establishes shared temporal visibility, finite input validation and
  filtered-ANN eligibility. Its BQ results show why eligibility and recall must
  be reported separately.
- Wave B establishes durable collection definitions and per-collection index
  settings; Wave D must measure those settings without changing journal schema.
- Query IR remains the machine boundary. HQL and REST are compatibility fronts;
  they must inherit the same budget and error semantics.
- Wave E product-specific hybrid retrieval and SDK/distribution work remains
  out of scope.

## 3. Confirmed findings

### R-09 — Query work is not bounded consistently

- `limit=0` can still produce one neighbor because validation occurs after a
  result is pushed.
- Query IR bounds `k` and traversal depth but does not bound expanded nodes,
  edges, candidate rows, serialized bytes or elapsed time.
- HQL `MATCH` can build a dense intermediate frontier before applying `LIMIT`.
- REST handlers perform synchronous storage work inside async handlers, so an
  expensive query can starve unrelated status requests.

### R-11 — Existing quality guards do not describe one deployed index

- The current recall guard permits an individual build below the target when a
  five-build median passes.
- Scientific ingestion checks do not cover filtered recall, tombstone churn,
  quantizer quality, tail latency, memory or reopen cost.
- Wave C's 28-cell audit proves the eligibility contract, while BQ recall on an
  adversarial synthetic corpus demonstrates that recall needs a per-config
  envelope rather than one global threshold.

## 4. Proposed contract

### D1 — Shared query budget

Introduce one internal budget resolved from Query IR/HQL/REST input and safe
defaults when callers omit it. The budget covers:

- maximum expanded nodes and edges;
- maximum vector candidates and deduplicated result rows;
- maximum serialized response bytes;
- absolute deadline/elapsed time;
- typed exhaustion reason (`nodes`, `edges`, `candidates`, `bytes` or
  `deadline`).

Zero, negative-after-conversion and overflowing values fail before work starts.
The budget is internal and additive to existing public response shapes; no new
query-language syntax is required in this wave.

### D2 — REST execution control

Move blocking storage calls behind a bounded blocking executor/admission guard.
Status and health routes must remain responsive while an admitted expensive
query consumes its budget. Rejected admission and budget exhaustion must return
stable typed errors that NAPI, REST and MCP can surface without changing the
core result model.

### D3 — Per-index quality envelope

Add an audit harness and versioned result artifact keyed by:

- dataset, dimension, metric and query distribution;
- quantizer, calibration, rerank and collection `ef_search`;
- filter selectivity and 0/10/50/90% update/retraction churn;
- `k`, exact filtered-oracle recall, eligibility shortfall and result equality;
- p50/p95/p99 latency, flush lag, checkpoint/reopen time and memory/sidecar
  bytes.

The harness must publish measured pass/fail rows per index configuration. BQ
and other lossy configurations may have a different declared recall envelope;
no aggregate median may hide a failing configuration.

## 5. Acceptance and exit gates

1. Supernode, dense-cycle and high-cardinality queries stop at each budget with
   a typed, deterministic error and no partial mutation.
2. `limit=0` and numeric overflow are rejected before traversal/search work.
3. REST status remains responsive under concurrent admitted expensive queries;
   p95/p99 admission and status latency are recorded.
4. Query IR, HQL and REST expose the same effective budget and exhaustion reason;
   NAPI/MCP/FFI parity tests cover the boundary.
5. The quality harness produces per-index rows for the configured dataset,
   metric, quantizer, filter selectivity and churn matrix, with exact-oracle
   recall, shortfall and p50/p95/p99 evidence.
6. `cargo test --no-default-features`, rebuilt NAPI/MCP tests, mobile/FFI host
   checks, clippy, fmt and documentation validation pass.
7. No schema version bump, HNSW replacement, lexical/BM25 fusion, product
   ranking semantics, SDK redesign or release claim is introduced.

## 6. Implementation sequence after approval

1. RED: add bounded-query and REST starvation reproductions, plus quality-harness
   schema assertions.
2. RCA: record the concrete unbounded call paths and baseline latency/memory.
3. GREEN D1/D2: implement the shared budget and bounded REST execution with
   focused core, REST and NAPI parity tests.
4. GREEN D3: run the per-index matrix against an exact filtered oracle and
   record artifacts; tune no HNSW defaults until the envelope is visible.
5. Final review: compare parent/peer contracts, update capability disclosure,
   run the full gates, then decide whether any configuration is ready for a
   consumer-facing claim.

## 7. Out of scope

Wave D does not add full-text fusion, new HQL syntax, a new storage backend,
sharding, product-specific GraphRAG ranking, SDK distribution changes, mobile
artifacts, schema migration or production deployment.

## 8. Version diff

| Artifact | Before | Proposed after approval |
|---|---|---|
| Wave D spec | candidate | 0.1.0b, beta |
| Wave C | 0.2.1b, beta | unchanged |
| Engine/schema | 0.2.5 / disk schema 4 | 0.2.5 / disk schema 4, unchanged |

## CHANGELOG

| Version | Date | Status | Summary | Commit Hash | Agent |
|---|---|---|---|---|---|
| 0.1.0b | 2026-09-08 | beta | Approved scope; bounded query budgets, REST admission control, and per-index quality evidence implemented locally | 3751881, ce41d5e | ATHER |
