---
doc_id: SPEC--WAVE-C-QUERY-CORRECTNESS
owner: GenesisBlockDB Engineering
version: "0.2.1b"
created_at: "2026-09-08T11:30:00+07:00,ATHER,44d0252"
last_update: "2026-09-08T18:15:00+07:00,ATHER,d8ef9af"
status: beta
superseded_by: null
attributes:
  domain: query-correctness
  scope: wave-c-r04-r07
  complexity: C-3
  risk: MEDIUM-HIGH
---

# Wave C — Query correctness and cross-surface visibility

## 1. Decision requested

Wave C continues from the verified Wave B delivery commit `44d0252` on
`codex/wave-c-query-correctness`. The user approved implementation of R-04,
R-05, R-06 and R-07. The engine and regression checkpoint is now
`d8ef9af` (`fix: close large filtered ann shortfall`), following the initial
`cbe5a04` implementation checkpoint. This beta document
records local implementation evidence; it does not authorize push, merge,
release, deployment, schema migration or consumer migration.

Complexity is **C-3** because vector candidate selection, temporal visibility,
graph retrieval and multiple public surfaces must share one observable contract.
Risk is **MEDIUM-HIGH**: incorrect filtering can return false knowledge while
being difficult to detect from latency or a single surface.

The exact filtered-oracle churn matrix, rebuilt N-API/MCP runtime campaign and
full cross-feature regression are now recorded locally; the document remains
beta pending Wave D quality interpretation for lossy BQ modes.

## 2. Parent and peer alignment

Parent contracts are the [master architecture](MASTER-SPEC--GENESIS-DB.md), the
[C4 index](C4--GENESISDB-ARCHITECTURE.md), the [system review](REVIEW--GRAPH-VECTOR-SYSTEM-2026-09-07.md)
and the accepted [Typed Query IR v1](SPEC--GENESISDB-TYPED-QUERY-IR-V1.md).

Peer contracts are [Wave A commit correctness](SPEC--WAVE-A-COMMIT-CORRECTNESS.md),
[Wave B durable collections and edge history](SPEC--WAVE-B-DURABLE-COLLECTIONS-EDGE-HISTORY.md),
epoch/HNSW behavior, temporal model, GRL, REST/N-API parity and the existing FFI
JSON boundary. Wave B's disk schema 4, journal-authoritative collection
definitions, replica-local edge history and asynchronous HNSW indexing remain
unchanged unless a focused compatibility defect is proven.

The implementation must preserve the product boundary:

```text
N-API / REST / FFI / HQL / Query IR
             |
      one core eligibility contract
             |
 vector candidate + graph traversal + GRL
             |
        signed WAL and projections
```

No client-specific ontology, external graph database, external vector database,
new Cypher surface or new FFI endpoint is part of this wave.

## 3. Confirmed findings and assumptions

The review reproduced four defects against the pre-Wave-C baseline:

| ID | Confirmed behavior | Existing cause to verify before fixing |
|---|---|---|
| R-04 | Retracting vectors near a query can make `k=3` return zero while live vectors remain | ANN shortlist is filtered after selection; shortfall detection counts raw hits |
| R-05 | Future vectors can enter current search; equivalent timezone instants can disagree; GRL can include a retracted edge | string timestamps and no shared current-visibility predicate across search/GRL |
| R-06 | Invalid dimension/metric/quantization input can be accepted or silently normalized | unchecked narrowing and fallback enum parsing at public collection boundaries |
| R-07 | `execute_batch` can replace an edge's supplied `valid_from` with the current time | batch edge construction uses `Utc::now()` instead of the input field |

Before each implementation packet, record a focused RCA under `.brain/rca/` and
write a RED test that fails on the Wave B engine. The RCA must cite the actual
call path after source reinspection; the review's line references are diagnostic
starting points, not permanent proof.

Assumptions for this implementation:

1. Current view means a single UTC instant captured at query start. A supplied
   valid-time selector uses the same instant semantics; equivalent RFC 3339
   offsets compare equal after normalization.
2. Existing transaction-time selectors and Wave B history horizons remain the
   authority for retained history. Wave C does not invent missing history or a
   new node system-time model.
3. A vector is eligible only when it belongs to the selected collection, has the
   query-compatible dimension, has a live node version at the selected time, is
   not expired/retracted for the selected view, and passes the existing index
   consistency rules.
4. Invalid input is rejected before WAL append, frontier advance, memory
   publication, vector staging or index enqueue. Existing valid aliases remain
   valid only when their canonical meaning is explicit.
5. `execute_batch`, single-edge mutation, transaction mutation, signed reconcile
   and replay must share the same edge temporal mapping; no new mutation mode is
   introduced.

## 3.1 Implementation evidence

Checkpoint `d8ef9af` completes the approved core contract:

- filtered ANN refills by eligible distinct node IDs, raises HNSW exploration
  with each refill and falls back to the exact oracle after exhausting slots,
  including large collections and quantized rerank candidates;
- RFC3339 instants are compared as UTC values, current/future/expiry checks use
  one query timestamp, and the same node/edge predicate is applied to vector,
  graph, HQL/MATCH and GRL reads;
- non-finite vectors, zero/invalid query controls and malformed `as_of` values
  fail before WAL/index publication; existing collection dimension and enum
  checks remain strict;
- batch edges preserve caller `valid_from`, with a focused REST regression and
  core coverage. Existing `EdgeInput.supersede` behavior remains unchanged and
  is intentionally outside this checkpoint.

RED reproduced five baseline failures plus the large-collection fixed-
`ef_search` shortfall. GREEN passes seven Wave C core tests, the REST capability
regression and the 28-cell oracle matrix documented in
[`AUDIT--WAVE-C-FILTERED-ORACLE-2026-09-08.md`](AUDIT--WAVE-C-FILTERED-ORACLE-2026-09-08.md).
Rebuilt N-API/MCP passes 26/26; host mobile and mobile+FFI checks pass. The
final `cargo test --no-default-features` sweep passes all suites with the three
pre-existing ignored soak tests unchanged. BQ recall remains a separate
lossy-quality axis for Wave D.

## 4. Scope and acceptance contract

### C0 — Shared query eligibility contract

Create one internal, allocation-light eligibility path that can be used by vector
search, current graph traversal, historical traversal, MATCH and GRL metadata
expansion. It must distinguish current-view time from an explicit valid-time or
transaction-time selector without changing the existing public request shapes.

The contract must make these decisions explicit:

- normalize and validate RFC 3339 instants before comparison;
- use half-open validity `[valid_from, valid_to)`;
- apply expiry and retraction rules consistently on current and historical paths;
- preserve Wave B's `tx_as_of`, horizon and beyond-horizon behavior;
- capture `now` once per query, not once per row or edge;
- return a typed/diagnosable error for an unsupported temporal selector rather
  than silently falling back to current view.

The shared helper must not become a public database API in this wave. Existing
N-API, REST and FFI response shapes remain compatible.

### C1 — R-04 filtered ANN top-k correctness

For current and retained transaction-time vector search, candidate selection must
continue until it has `k` eligible distinct node IDs or has exhausted a bounded
candidate source. If the ANN shortlist is insufficient after visibility,
collection, deduplication and retraction checks, use the existing exact/fallback
mechanism where its cost contract permits. Return fewer than `k` only when fewer
than `k` eligible vectors exist.

Acceptance matrix:

| Case | Required result |
|---|---|
| 0%, 10%, 50%, 90% nearby vector retractions | `k=1,3,10` returns eligible IDs when enough remain |
| Re-embed the same node repeatedly | no duplicate IDs and no retired vector leaks |
| None, F16, SQ8, calibrated SQ8 and BQ | same eligibility contract; ranking checked against an exact filtered oracle where supported |
| Current and `tx_as_of` search | current and historical candidate sets respect their selected view |
| Fewer than `k` eligible vectors | return the exact eligible count, not a fabricated placeholder |
| Invalid collection/dimension/non-finite query | reject before search side effects |

The RED/GREEN test must record candidate counts, eligible counts, result IDs,
recall against the exact filtered oracle and p50/p95 latency. A single smoke run
does not support a no-regression claim.

### C2 — R-05 temporal and GRL parity

Apply C0 to vector search, `neighbors`, Query IR traversal, HQL `TRAVERSE`/
`MATCH`, `retrieve_context` and the REST/N-API/FFI calls that expose them.

Acceptance matrix:

| Case | Required result |
|---|---|
| Future `valid_from` in current view | absent from vectors, graph and GRL |
| Expired or retracted node/edge in current view | absent everywhere unless the existing explicit invalid/history option applies |
| Same instant with `Z` and a `+07:00` offset | identical results and ordering |
| Historical valid-time inside/outside `[from,to)` | visible only inside the half-open window |
| Wave B `tx_as_of` before/after edge replacement or retract | retained history and current view agree with edge-version intervals |
| GRL H1/H2 around a retracted edge | no stale edge or far node in context metadata |
| HQL, Query IR, REST, N-API and existing FFI | equivalent IDs, visibility and typed error behavior |

TTL semantics must be stated in the capability/spec output: query-time expiry
uses the captured query instant; maintenance may reclaim storage later but cannot
change the query result before expiry. No maintenance worker redesign is included.

### C3 — R-06 strict collection and vector input boundaries

Keep Wave B's durable-definition validation and add the remaining public-boundary
contract:

- reject dimension `0`, `65536` and `65537` before mutation;
- reject unknown supplied metric and quantization values; preserve documented
  aliases only when they map to one canonical value;
- reject non-finite vector values and invalid finite parameter ranges;
- validate query `k`, `ef_search`, oversample and alpha where those fields exist;
- preserve the frontier, collection catalog, WAL, memory maps and index backlog
  byte/semantic state after every rejected request.

Test the core, REST and N-API surfaces plus the existing FFI JSON methods that
expose the same input. Do not add a new FFI collection endpoint solely for this
wave. Error prefixes/codes must be stable enough for existing clients to
distinguish invalid input from recovery-required state.

### C4 — R-07 batch edge temporal parity

Use the same temporal constructor for `add_edge`, `execute_batch`, transaction,
signed reconcile and replay. `EdgeInput.valid_from` must survive the batch frame,
projection, snapshot and reopen exactly after RFC 3339 normalization. Test
`valid_to`/supersede/caused-by fields that already exist on the input and report
any field still intentionally unsupported instead of silently dropping it.

Acceptance requires a single edge and a batch edge with identical inputs to
produce equivalent current and historical visibility, including before/at/after
the supplied valid-time boundary.

## 5. RED → GREEN work packets

1. **C0/C2 temporal predicate:** reproduce future, offset, expiry and GRL stale
   visibility; implement the shared predicate; run temporal/GRL parity tests.
2. **C1 filtered ANN:** reproduce zero-result tombstone shortlist; implement
   eligibility-aware candidate refill/fallback; run quantizer and exact-oracle
   tests.
3. **C3 input boundaries:** reproduce invalid dimensions/enums/non-finite values
   and unchanged-frontier requirements across surfaces; implement strict parsing.
4. **C4 batch mapping:** reproduce lost `valid_from`; implement shared mapping;
   run single/batch/transaction/replay parity tests.

Each packet requires its own RCA evidence and focused test run. Keep the engine
source changes surgical; do not refactor `src/lib.rs` merely because it is large.

## 6. Verification and exit gates

Before declaring Wave C complete:

- focused RED/GREEN suites cover all acceptance rows and preserve Wave A/B tests;
- full `cargo test --no-default-features` and relevant REST tests pass;
- rebuilt N-API/MCP tests pass, including cross-surface temporal and validation
  cases;
- host mobile/FFI check, clippy with warnings denied, fmt and doc validation pass;
- exact filtered-vector oracle reports recall, eligible counts and latency for
  0/10/50/90% churn and every supported quantizer;
- `query_ir_capabilities` documents temporal normalization, eligibility and any
  unsupported selector; no design-only capability is advertised as implemented;
- version diff and evidence paths are recorded; no release, merge, push,
  deployment or consumer migration is implied.

Out of scope: Wave D query budgets/REST worker admission, R-10 product-specific
hybrid retrieval, R-11 quality gates, HNSW algorithm replacement, external graph
or vector database integration, new query-language syntax, schema version bump,
large-scale soak, power-loss certification and network-partition certification.

## 7. Version diff

| Artifact | Before | After |
|---|---|---|
| Wave C spec | 0.2.0b, beta | 0.2.1b, beta |
| Document registry | 0.3.6+draft | 0.3.7+draft |
| Engine/schema | 0.2.5 / disk schema 4 | engine checkpoint `d8ef9af`; schema unchanged |
| Wave B | 0.1.2b, beta | unchanged |

## CHANGELOG

| Version | Date | Status | Summary | Commit Hash | Agent |
|---|---|---|---|---|---|
| 0.1.0b | 2026-09-08 | candidate | Proposed R-04/R-05/R-06/R-07 query correctness packet after Wave B | 44d0252 | ATHER |
| 0.2.0b | 2026-09-08 | beta | Approved implementation checkpoint with RED/GREEN evidence; exact filtered-oracle and rebuilt N-API/MCP campaigns remain | cbe5a04 | ATHER |
| 0.2.1b | 2026-09-08 | beta | Closed large-collection refill shortfall; recorded 28-cell oracle and 26/26 N-API/MCP runtime evidence | d8ef9af | ATHER |
