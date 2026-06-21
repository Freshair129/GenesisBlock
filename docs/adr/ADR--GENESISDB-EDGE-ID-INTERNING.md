---
proposed_id: ADR--GENESISDB-EDGE-ID-INTERNING
type: adr
status: accepted
aliases:
  - ADR
phase: 31
tier: process
cluster: implementation_flow
role: "Architecture decision record"
enforcement_state: inactive
proposed_at: 2026-06-21T00:00:00.000Z
proposed_by: agent
---

# ADR--GENESISDB-EDGE-ID-INTERNING

## Context

The graph RAM ceiling (~12.6 GB @ 1M nodes / 8M edges) is the #1 lever blocking
scale beyond 1M nodes; on-niche embedded comparators (Kuzu, LadybugDB) use ~11×
less, RocksDB+graph ~32× less. [[RCA--EDGE-ID-INTERNING-RAM]] (confirmed,
2026-06-21) measured the cause with the `edge-interning-audit` probe:

- **~1260 bytes/edge** of interning+storage RAM, linear (800k→965 MB, 1.6M→1915 MB).
- The dominant *eliminable* cost is **not** edge storage but two side-effects of
  routing edge UUIDs through `get_or_intern_id()` (`src/lib.rs`):
  1. **Trigram pollution** — every 36-char edge UUID is tokenized into **48.0
     `trigram_index` members** (measured exactly). `trigram_index`'s only reader
     is `find_fuzzy_id()`, which resolves **node** ids; edge UUIDs are never
     fuzzy-searched. ~36% of edge RAM, pure waste. Latent correctness smell:
     edge ids are eligible fuzzy-match candidates for node lookups.
  2. **Redundant reverse map** — `u32_to_id` stores a second copy of every edge
     UUID, but `EdgeOutput.id` (in the `edges` map) already holds the canonical
     string. ~6% of edge RAM.

Implementation note: the live graph index is `out_idx`/`in_idx`
(`DashMap<u32, HashSet<u32>>`) plus `edges: DashMap<u32, EdgeOutput>` — **not**
the EdgeArena/CSR described in [[ADR--GENESISDB-CSR-MUTATION-STRATEGY]] (that
layout is not yet in the code). This ADR is scoped to the id-interning layer and
does not assume CSR.

Constraint (from SELF-NOTE / temporal model): edge `from`/`to` and `id` are
**String** at the API and WAL boundary — client-knowable and WAL-replay-stable.
u32 keys are internal only and **must not** be re-exposed.

## Decision

Split id interning into **node** (searchable) and **edge** (internal-key) paths,
and adopt a **two-layer rollout** so the low-risk majority of the win lands first.

### Layer A — Stop indexing edge ids (DECIDED, do now)

1. **Edges skip `trigram_index`.** Introduce an edge-only interning path that
   assigns a u32 key and the forward `id_to_u32` entry but does **not** call
   `tokenize_id()`. `find_fuzzy_id` is unaffected (node-only by design).
2. **Edges skip the reverse `u32_to_id` map.** Any edge `u32 -> String`
   resolution reads `edges[u32].id`. (Verified: `u32_to_id` readers —
   `find_fuzzy_id`, traversal node-id resolution, delete-by-id — never require
   an edge entry; delete simply no-ops on the missing reverse key.)
3. **Remove the double-intern.** `execute_batch` interns `e.id` at the event
   loop *and* again inside `index_edge_internal`; collapse to a single edge
   intern.

Forward `id_to_u32` for edges is **retained** — it backs CRDT idempotency
(re-applied edge detected as already-present) and delete-by-id. Layer A keeps
the id-space and WAL/CRDT semantics byte-identical; only index side-effects change.

Expected: ~42% of edge RAM eliminated (~345 MB trigram + ~60 MB reverse @
100k/800k); ~4.0 GB at 8M edges, no functional change.

### Layer B — Numeric edge keys (DEFERRED, separate analysis)

Dropping the 8M edge UUID **strings** in `id_to_u32` requires keying edges by a
numeric id (e.g. `u64 = hash(uuid)`), which ripples into `edges`, `out_idx`,
`in_idx` (all currently `u32`-keyed) and the idempotency/collision model
(u32 is too small for hashed ids — birthday collisions ≪1M edges; needs u64).
This is a wider migration for the remaining ~6%; it gets its own ADR after
Layer A is measured. Not started here.

## Consequences

### Positive
- ~42% edge-RAM cut from a surgical, side-effect-only change — pulls the 12.6 GB
  ceiling toward ~8.5 GB, making 1M/8M feasible on 16 GB.
- Removes the latent fuzzy-match correctness smell (edge ids leaving `trigram_index`).
- Faster bulk edge ingest: ~48 fewer HashSet inserts per edge.

### Negative
- Two interning paths to maintain; a future "searchable edge id" feature would
  need to opt back in explicitly.

### Neutral / Trade-offs
- Id-space, WAL format, and CRDT wire semantics are unchanged in Layer A — the
  win is purely from not building unused index structures.
- The larger remaining lever (string-free edge keys) is deferred, not abandoned.

## Alternatives Considered
| Alternative | Reason Rejected |
|---|---|
| Numeric edge keys now (Layer B first) | Ripples into `edges`/`out_idx`/`in_idx` u32→u64 + collision model; high risk for the last ~6%. Sequence after Layer A. |
| Lazy/rebuild-only trigram index | Doesn't stop edge ids entering the index; node fuzzy-search still needs it live. |
| Drop `find_fuzzy_id` entirely | Removes a shipped Thai-aware feature for nodes; out of scope. |
| Keep a separate `edge_id_to_u32` String map | Still stores 8M strings — no RAM win; only avoids trigram (which Layer A already does without a new map). |

## Verification
- Full `cargo test` green; add tests asserting (a) `trigram_index` member count
  is bounded by node-id tokens after an edge-heavy build, (b) edge-id CRDT
  re-apply is idempotent, (c) WAL replay reproduces identical graph state.
- `edge-interning-audit` before/after RSS at 100k/800k and 200k/1.6M; record the
  delta in [[RCA--EDGE-ID-INTERNING-RAM]] and SELF-NOTE.

### Outcome (measured 2026-06-21, C: SSD)
Layer A shipped. Edge RAM **−37.8%** (965.4 → 600.3 MB @100k/800k; 787 vs 1265
B/edge); trigram members from edges **38.4M → 0**; `u32_to_id` 900k → 100k
(nodes only). Projected ceiling **12.6 → ~8.8 GB** at 1M/8M. Edge ingest ~3×
faster at 100k. 38 prior tests + 6 new `tests/edge_interning_tests.rs` green.
Two adjacent snapshot-reload bugs fixed in passing: (1) edge adjacency corrupted
by re-allocating edge u32s decoupled from saved keys; (2) node trigram not
rebuilt on instant-load (dead `find_fuzzy_id` after reload).

---
### Related Links
- **Root Cause:** [[RCA--EDGE-ID-INTERNING-RAM]]
- **Scalability Proof:** [[ADR--GENESISDB-SCALABILITY-VALIDATION]]
- **Graph Mutation:** [[ADR--GENESISDB-CSR-MUTATION-STRATEGY]]
- **Probe:** `benches/edge_interning_audit.rs`

## Changelog
| Version | Date | Summary |
|---|---|---|
| 0.1.0 | 2026-06-21 | Proposed: split node/edge interning; Layer A (edges skip trigram + reverse map, de-dup double-intern) decided; Layer B (numeric edge keys) deferred. |
| 0.2.0 | 2026-06-21 | Accepted: Layer A implemented & measured (−37.8% edge RAM). |
