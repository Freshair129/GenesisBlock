---
proposed_id: ADR--GENESISDB-NODE-ID-INTERNING
type: adr
status: accepted
aliases:
  - ADR
phase: 32
tier: process
cluster: implementation_flow
role: "Architecture decision record"
enforcement_state: inactive
proposed_at: 2026-06-22T00:00:00.000Z
proposed_by: agent
---

# ADR--GENESISDB-NODE-ID-INTERNING

**Status:** Accepted (A1 shipped 2026-06-23; A2/A3 deferred)
**Date:** 2026-06-22
**Deciders:** Engine owner (Boss)
**Roadmap:** MARK XIV Priority 1 — "Node RAM ceiling investigation … `id_to_u32`
interning next." Sibling lever to [[ADR--GENESISDB-VECTOR-QUANTIZATION]].

## Context

Edge RAM was the prior ceiling and is now exhausted as a lever: edge interning
Layer A ([[ADR--GENESISDB-EDGE-ID-INTERNING]]) + numeric u128 keys Layer B
([[ADR--GENESISDB-EDGE-NUMERIC-KEYS]]) cut edge RAM −44% and removed edge id
strings entirely. The roadmap names **node bookkeeping** as the next lever toward
>1M nodes (measured ~12.6 GB @ 1M nodes / 8M edges).

The cause is that **each node id string is held 3–4× in RAM**, plus trigram
overhead (`src/lib.rs` line refs from survey):

| Sink | Type | Cost / 36-char UUID node | Reader(s) |
|---|---|---|---|
| `id_to_u32` (forward) | `DashMap<String, u32>` (`:508`) | id string (key) + bucket | `get_u32`, CRDT idempotency |
| `u32_to_id` (reverse) | `DashMap<u32, String>` (`:509`) | **2nd** copy of id | `find_fuzzy_id`, reload only |
| `nodes[u32].id` | `NodeOutput.id: String` (`:70-87`) | **3rd** copy of id | canonical record |
| `NodeMetadata.node_id` | `String`, **per vector per collection** (`:315-324`) | **4th**+ copy of id | search post-filter |
| `trigram_index` | `DashMap<String, HashSet<u32>>` (`:512`) | ~48 trigrams × `HashSet<u32>` ≈ **432 B/node** | `find_fuzzy_id` only |

Measured: ~1.4–2.1 KB/node of interning+trigram overhead → **~1.6 GB at 1M nodes**
before the `NodeOutput` payload (`labels`, `props`, timestamps, `clock.peer_id`).

Two facts bound the design:

- **Bitemporal versioning is not a RAM sink.** `supersede_node` (`:1275-1305`)
  keeps only the current version (`valid_to: None`) in `nodes`; all history lives
  in the WAL and is reconstructed on temporal queries. No per-node version pile-up.
- **Embeddings already left the node record.** `insert_node_lean` (`:1218-1221`)
  strips `embedding` before insert; vectors live only in the `VectorCollection`
  arena. So node RAM is strings + JSON `props`, not vectors.

**Crucial difference from edges:** edges went fully numeric (key = `hash(id)`)
because an edge key is *re-derivable* from the stored `EdgeOutput.id`. Node ids are
**client-supplied, client-knowable, and must round-trip exactly** through the API
and WAL — they cannot be replaced by a hash. Moreover the dense `u32` node key is
load-bearing for `out_idx`/`in_idx`/`node_to_arena`/`trigram_index` (all `u32`-keyed),
so we **keep dense `u32` interning** and optimize the *string storage and the
trigram*, not the key scheme.

## Decision

Adopt a **two-layer rollout** mirroring the edge-interning playbook: ship the
low-risk, shape-only changes first, then the higher-leverage `Arc<str>` dedup +
optional trigram.

### Layer A — Drop redundant string copies (DECIDED, do first)

Layer A is **three independent sub-changes** that the initial draft treated as
uniformly "free". Implementation (2026-06-23) showed they have **different blast
radii**, so they are split and sequenced:

1. **A1 — Eliminate the node `u32_to_id` reverse map. (SHIPPED 2026-06-23.)** Its
   readers resolve a u32 to an id string that `nodes[u32].id` already holds. Two
   readers existed: `find_fuzzy_id` (now resolves candidates via `nodes[u32].id`)
   and the `TRAVERSE` loop (now picks the far endpoint by **u32 identity** —
   `get_u32(edge.from) == curr_u32` — which needs no reverse map *and* drops a
   per-edge string clone). Reload/`delete`/`get_or_intern_id` stop touching the
   map; it was never persisted (rebuilt from `nodes.bin`). Removes the **2nd** full
   copy of every node id. **Truly free** — no on-disk format change.
   - *Behavior delta (intentional):* a trigram candidate that interns only as an
     **edge endpoint** (no node record) no longer fuzzy-resolves; exact resolution
     is unchanged (still via `id_to_u32`). `find_fuzzy_id` now targets real nodes,
     which is the more-correct contract. `thai_fuzzy_tests` + `neighbors_direction_rels_tests` stay green.
2. **A2 — `NodeMetadata.node_id: String` → `node_u32: u32`. (DEFERRED.)** Removes
   the per-vector id copy (worst multiplier under multi-collection), **but**
   `NodeMetadata` is bincoded into `meta_<name>.bin`, so the change is an **on-disk
   format migration** (old snapshots + test fixtures fail to deserialize without a
   versioned reader). Not "shape only" — needs its own migration plan.
3. **A3 — `trigram_index` value `HashSet<u32>` → denser posting list. (DEFERRED.)**
   In-memory only (no format change), but a naïve sorted `Vec<u32>` makes the build
   **O(n²)** on hot trigrams (a common trigram spans ~all nodes; `get_or_intern_id`
   inserts one at a time). The correct dense structure is a **roaring bitmap**,
   which adds the `roaring` crate — a dependency decision (C-2) deferred to a
   focused pass with its own before/after RAM + ingest-throughput measurement.

Layer A changes **no API, no WAL, no CRDT semantics** — only which in-RAM structures
hold the string. Expected: remove ~2 of the 3–4 id copies + shrink the trigram
posting lists. Conservative target ≈ −0.4–0.6 GB at 1M nodes.

### Layer B — `Arc<str>` interning + optional trigram (DESIGNED, decide after A)

1. **Single-allocation id via `Arc<str>`.** Replace the remaining owned copies
   (`id_to_u32` key, `nodes[u32].id`) with a shared `Arc<str>` so each node id is
   heap-allocated **once** and refcounted. `DashMap<Arc<str>, u32>` still supports
   `get(&str)` via `Arc<str>: Borrow<str>`; serde round-trips `Arc<str>` (rc
   feature). Collapses id storage toward 1× + 8 B/refcount.
2. **Gate node trigram behind an OpenOption (`fuzzy_ids`, default policy TBD).**
   `trigram_index` exists solely for `find_fuzzy_id` on node ids. For the common
   agent-memory workload where ids are UUIDs/known keys, fuzzy-matching random hex
   is near-useless yet costs ~432 B/node. Make it opt-in (or auto-skip pure-UUID
   ids). When off, `find_fuzzy_id` degrades to exact match. This is the single
   biggest remaining lever but it changes a default, hence Layer B not A.

Layer B is the deep cut (toward ~1.0 KB/node interning → a few hundred bytes) but
touches serde and the fuzzy-search contract, so it is sequenced after A is measured.

## Options Considered

### Option A — Layered: drop reverse map + metadata u32 + dense trigram, then `Arc<str>`  ★ recommended
| Dimension | Assessment |
|-----------|------------|
| Complexity | Med, staged — Layer A is shape-only; Layer B adds serde `Arc<str>` + an option |
| Memory | A: ~−0.4–0.6 GB @1M; A+B: toward ~−1.0 GB @1M |
| Risk | Low (A) → Med (B); each layer independently measurable |
| API / WAL | **Unchanged** (ids stay `String` at the boundary) |
| Feature impact | A none; B makes node fuzzy-id opt-in |

**Pros:** Mirrors the proven edge rollout; ships value early; reversible per layer.
**Cons:** Two passes; B revisits serde and the fuzzy contract.

### Option B — Full `Arc<str>` everywhere in one pass
| Dimension | Assessment |
|-----------|------------|
| Complexity | High — serde, DashMap key type, all readers at once |
| Memory | Same endpoint as A+B |
| Risk | Higher — no measured intermediate; one big diff |

**Rejected:** Loses the de-risking staged measurement that made edge interning safe.

### Option C — Hash node ids to a numeric key (edge-style)
| Dimension | Assessment |
|-----------|------------|
| Memory | Largest (no id strings in maps) |
| Correctness | **Breaks** exact id round-trip; ids are client-knowable, not re-derivable |

**Rejected:** Node ids must return verbatim through API/WAL; a hash key cannot
reproduce the original string the way an edge key is re-derived from `EdgeOutput.id`.

### Option D — FST / trie symbol table replacing `DashMap<String,u32>`
**Rejected (for now):** Large rewrite of the hot intern/lookup path for a marginal
gain over `Arc<str>`; revisit only if A+B prove insufficient.

## Trade-off Analysis

The win is bounded by how many of the 3–4 id copies we can collapse and how much of
the 432 B/node trigram we can shed. **Layer A is nearly free** — dropping the reverse
map and storing a `u32` in `NodeMetadata` are the exact moves that worked for edges,
with no API/WAL/feature change, and the trigram posting-list densification is pure
shape. **Layer B holds the larger lever but spends two budgets**: serde churn for
`Arc<str>`, and a *product decision* — is node-id fuzzy search worth ~432 B/node by
default? For agent-memory-with-UUIDs it is not; for human-authored ids it can be.
Making it an OpenOption hands that trade to the operator rather than guessing.

Hashing (C) looks tempting because it worked for edges, but the edge precedent does
not transfer: edges re-derive their key from the stored canonical string, whereas a
node has no second authoritative copy to reconstruct a hashed-away id. Keeping dense
`u32` interning and shrinking the *strings around it* is the correct read of the edge
lesson.

This lever stacks with quantization, not overlaps it: node interning frees
bookkeeping RAM, quantization frees vector RAM. Both are needed to clear 16/32 GB at
1M+ nodes.

## Consequences

### Positive
- Removes 2 of 3–4 id copies (A) and trends toward 1× (B); shrinks the trigram
  posting lists — together a meaningful fraction of the ~1.6 GB/1M interning cost.
- No API/WAL/CRDT change in Layer A; existing snapshots load unchanged
  (`id_to_u32`/maps are already rebuilt from `nodes.bin` on load, `:2133-2159`).
- Optional trigram (B) lets UUID-keyed workloads pay zero fuzzy-search tax.

### Negative
- Layer B introduces `Arc<str>` into serde-derived structs (rc feature, custom care
  on (de)serialize) and an OpenOption that changes default fuzzy behavior.
- `find_fuzzy_id` must switch from `u32_to_id` to `nodes[u32].id` (A) and degrade to
  exact when trigram is off (B) — both need explicit tests.

### Neutral / Trade-offs
- Node key stays a dense internal `u32` (unlike edges' u128 hash) — deliberate, to
  keep `out_idx`/`in_idx`/`node_to_arena`/trigram keys compact.
- Id strings remain `String` at the API/WAL boundary; only in-RAM representation
  changes.

## Action Items
1. [x] **A1: remove node `u32_to_id`** — repointed `find_fuzzy_id` to `nodes[u32].id`
       and `TRAVERSE` to u32-identity endpoint selection; dropped reverse-map
       writes in `get_or_intern_id`/reload/`delete`. Added `tests/node_interning_tests.rs`
       (exact resolve, fuzzy resolve, traversal both directions, snapshot reload).
       *(shipped 2026-06-23)*
2. [ ] A2: change `NodeMetadata.node_id: String` → `node_u32: u32` **with a versioned
       `meta_<name>.bin` reader** (old bins deserialize via a compat path). Update the
       reload `node_to_arena` rebuild (`:2264`) to use the stored u32 directly.
3. [x] **A3: `trigram_index` value → roaring bitmap** (`roaring = "0.10"`).
       `get_or_intern_id`/reload insert into `RoaringBitmap`; `find_fuzzy_id`
       accumulates candidates via `bitmap.iter()`; measurement helpers cast
       `len() as usize`. Suite green (member counts + fuzzy results unchanged).
       *(shipped 2026-06-23)*
4. [ ] Measure RSS @ 100k/500k/1M with the node-RAM probe; record in a new
       `RCA--NODE-ID-INTERNING-RAM` (A1 structural delta verified by tests; RSS
       quantification pending a probe run).
5. [ ] Layer B: introduce `Arc<str>` for the id; verify serde round-trip + DashMap
       `&str` lookups; re-measure.
6. [ ] Layer B: add `fuzzy_ids` OpenOption (or pure-UUID auto-skip); document the
       recall/RAM trade and default.
7. [ ] Update [[METRICS-REVIEW--2026-06-22-WEEKLY]] and SELF-NOTE with the curve.

## Verification
- New `tests/node_interning_tests.rs`: (a) exact id resolution unchanged; (b)
  `find_fuzzy_id` resolves the nearest real node after reverse-map removal; (c)
  `TRAVERSE` out/in directions resolve via u32-identity endpoint selection; (d)
  snapshot save + reopen reproduce fuzzy + traversal.
- A2/A3 add: versioned `meta_<name>.bin` round-trip (old→new); `trigram_index`
  member counts unchanged by the roaring switch; ingest-throughput non-regression.
- `industrial-audit` / `scientific-audit` RSS at 100k → 1M nodes before vs after,
  per sub-change; confirm no ingest/lookup regression ([[feedback_bench_windows]]).

### Outcome (A1, measured by suite 2026-06-23)
A1 shipped. Removed the `u32_to_id: DashMap<u32, String>` field — one full id-string
copy per interned id eliminated (nodes *and* edge endpoints; the field held ~all
interned ids). Two readers repointed: `find_fuzzy_id` → `nodes[u32].id`; `TRAVERSE`
→ `get_u32(edge.from) == curr_u32` (also drops a per-edge string clone). Full
`cargo test` green — **edge_interning 10/10** (assertion updated to read `nodes[au].id`),
**node_interning 4/4** (new), **thai_fuzzy 1/1** + **neighbors_direction 9/9** (the
changed paths); `cargo check --all-targets` clean (server bin + `edge_interning_audit`
probe updated to report the reverse map as removed). RSS delta quantification pending
a probe run; structural removal is verified.

### Outcome (A3, shipped 2026-06-23)
A3 shipped. `trigram_index` value type `HashSet<u32>` → `RoaringBitmap` (`roaring =
"0.10"`) — denser posting lists at scale with O(1)-amortized inserts (no O(n²)
sorted-`Vec` build). Insert sites use `or_insert_with(RoaringBitmap::new).insert(id)`;
`find_fuzzy_id` accumulates via `bitmap.iter()`; `edge_interning_audit` + the test
helper cast `len() as usize`. Full suite green (trigram member counts and
`find_fuzzy_id` results unchanged; `thai_fuzzy` + `node_interning` + `edge_interning`
pass). **A2 remains deferred** (meta-bin format migration — see Decision).

---
### Related Links
- **Sibling RAM lever:** [[ADR--GENESISDB-VECTOR-QUANTIZATION]]
- **Edge precedent (Layer A/B playbook):** [[ADR--GENESISDB-EDGE-ID-INTERNING]],
  [[ADR--GENESISDB-EDGE-NUMERIC-KEYS]]
- **Root cause (edges, method to reuse):** [[RCA--EDGE-ID-INTERNING-RAM]]
- **Scale proof:** [[ADR--GENESISDB-SCALABILITY-VALIDATION]]
- **Roadmap:** [[ROADMAP]] (MARK XIV P1)

## Changelog
| Version | Date | Summary |
|---|---|---|
| 0.1.0 | 2026-06-22 | Proposed: keep dense `u32` node key; Layer A (drop reverse map, `NodeMetadata` u32 back-link, dense trigram posting lists) decided first; Layer B (`Arc<str>` id dedup + optional trigram) designed; node-id hashing rejected (ids must round-trip exactly). |
| 0.2.0 | 2026-06-23 | Accepted (partial): A1 (drop `u32_to_id` reverse map) implemented + tested (suite green). A2 (`NodeMetadata` u32) reclassified as on-disk format migration; A3 (dense trigram) reclassified as roaring-dep C-2 — both deferred with reasons. |
| 0.3.0 | 2026-06-23 | A3 shipped: `trigram_index` → `RoaringBitmap` (`roaring = "0.10"`); suite green. Only A2 remains. |
