---
proposed_id: ADR--GENESISDB-EDGE-NUMERIC-KEYS
type: adr
status: accepted
aliases:
  - ADR
phase: 32
tier: process
cluster: implementation_flow
role: "Architecture decision record"
enforcement_state: inactive
proposed_at: 2026-06-21T00:00:00.000Z
proposed_by: agent
---

# ADR--GENESISDB-EDGE-NUMERIC-KEYS

## Context

[[ADR--GENESISDB-EDGE-ID-INTERNING]] (Layer A, shipped `b78f800`, −37.8% edge RAM)
deferred the remaining edge-RAM lever to "its own ADR": the **8M edge UUID strings**
still stored in `id_to_u32` (`src/lib.rs`). After Layer A an edge consumes:

- one `String` UUID key in `id_to_u32` (forward map, ~36 bytes + map overhead), and
- one `EdgeOutput { id: String, from: String, to: String, .. }` value in `edges`.

The `id_to_u32` forward entry for edges exists only to (1) derive the edge's u32 key and
(2) back CRDT idempotency / delete-by-id. Both can be served **without storing the
string** if the key is *derived deterministically* from the edge id rather than
allocated from a counter. Layer B removes the 8M edge strings from `id_to_u32` entirely.

**Key insight:** an edge's identity is its UUID string (client-knowable, WAL-stable). A
deterministic hash `u64 = trunc64(SHA256(id))` gives a stable internal key with **no
stored string** — re-deriving it on replay/reload reproduces the same key, so adjacency
(`out_idx`/`in_idx`) and the `edges` map stay consistent without a counter or a reverse
map. This is exactly the property Layer A's snapshot fix needed (saved-key stability);
a deterministic hash provides it for free.

Constraint (unchanged): edge `id`/`from`/`to` are **String** at the API and WAL
boundary. The u64 key is internal only and must not be re-exposed.

## Decision

Key edges by `u64 = edge_key(id)` and **drop edge entries from `id_to_u32`**.

1. **`edge_key(id: &str) -> u64`** — `SHA256(id)`, first 8 bytes as big-endian u64.
   Reuses `sha2` (already imported, `src/lib.rs:14`). Deterministic, allocation-free
   key derivation; no counter, no stored string.
2. **Widen edge key u32 → u64**: `edges: DashMap<u64, EdgeOutput>`,
   `out_idx`/`in_idx: DashMap<u32, HashSet<u64>>` (node key stays u32 — nodes are
   unchanged; only the *edge values* in the adjacency sets widen).
3. **Remove `get_or_intern_edge_id`** — replaced by the pure `edge_key`. No
   `id_to_u32.insert` for edges; idempotency becomes "is this u64 already in `edges`?"
   (re-applying the same UUID hashes to the same key → `insert` overwrites in place).
4. **Delete-by-id** (`retract_node` cascade): edges are removed by u64 key from `edges`
   + `out_idx`/`in_idx`; drop the now-meaningless `id_to_u32.remove(&edge.id)` sweep.
5. **CRDT reconcile** LWW conflict check: look up `edges.get(&edge_key(&remote.id))`
   instead of `get_u32(&remote.id)` (edges no longer in `id_to_u32`).
6. **Snapshot reload**: recompute `k = edge_key(&v.id)` from the persisted `EdgeOutput`
   (ignore the saved key) and register `edges`/`out_idx`/`in_idx` under it. This is
   robust to both new (u64) and legacy (u32) `edges.bin` — the key is always
   re-derived from the edge's string id, so adjacency is internally consistent
   regardless of what width the snapshot was written with. Edge keys no longer share
   the node `next_u32` id-space, so the post-load `next_u32` bump from edge keys is
   removed.

Node interning (`id_to_u32`/`u32_to_id`/`trigram_index`/`next_u32`) is **untouched** —
Layer B is edges-only.

## Consequences

### Positive
- Drops 8M edge UUID `String`s from `id_to_u32` (the last remaining ~6% of edge RAM
  identified by [[RCA--EDGE-ID-INTERNING-RAM]]); pulls the 1M/8M ceiling further below
  ~8.8 GB.
- Deterministic key = simpler, faster reload (no counter coordination between node and
  edge id-spaces; the Layer-A "register under saved key" dance becomes "re-derive key").
- Edge insert is one hash, no map write to `id_to_u32`.

### Negative / Trade-offs
- **u64 truncated-hash collision** is theoretically possible. Birthday bound at 8M
  edges: ≈ n²/2⁶⁵ ≈ (8·10⁶)²/3.7·10¹⁹ ≈ **1.7·10⁻⁶** over a database's full lifetime
  at max design scale. On collision, two distinct edges would share a `edges` key and
  silently overwrite/merge adjacency. Deemed **acceptable** for a local-first,
  single-writer agent-memory store at this scale; `EdgeOutput.id` still holds the true
  string, so a collision is *detectable* at read time if ever needed. Future hardening
  (if scale or guarantees demand): widen to u128 (full key fits 16 bytes) or verify
  `edges[k].id == id` on insert and relocate. Not done now.
- WAL/snapshot edge **key width changes** (u32→u64). Legacy `edges.bin` is handled by
  re-deriving the key on load (see Decision §6), so existing local DBs migrate
  transparently; the WAL itself stores `EdgeOutput` (string id), not the numeric key, so
  WAL format is unaffected.

### Neutral
- WAL wire format and CRDT semantics (LWW on `clock`, idempotent re-apply) are
  preserved — only the *internal key derivation* changes.

## Alternatives Considered
| Alternative | Reason Rejected |
|---|---|
| Keep `id_to_u32` edge strings (status quo) | Leaves the last ~6% edge RAM on the table — the whole point of Layer B. |
| u128 full-hash key now | 16 B/edge vs 8 B for ~zero practical collision benefit at this scale; revisit only if guarantees demand. Adds key width churn everywhere. |
| Counter-allocated u64 + keep reverse map | Reintroduces a stored map and the node/edge id-space coupling that complicated Layer A's reload; deterministic hash avoids both. |
| Verify-and-relocate on every insert | Cost on the hot path for a 1.7·10⁻⁶ event; the stored `EdgeOutput.id` already makes collisions detectable lazily. |

## Verification
- Full `cargo test`. Extend `tests/edge_interning_tests.rs`: (a) `edge_key`
  determinism; (b) idempotent re-apply (same UUID → same key, no dup edge); (c) WAL
  replay reproduces identical adjacency; (d) snapshot round-trip keeps
  `out_idx`/`in_idx`/`edges` consistent (incl. legacy-width robustness via re-derivation);
  (e) delete-by-id removes the edge + both adjacency sides.
- `edge-interning-audit` before/after RSS at 100k/800k and 200k/1.6M; record the
  `id_to_u32` size drop and edge-RAM delta vs the Layer-A baseline (600.3 MB) in
  [[RCA--EDGE-ID-INTERNING-RAM]] and SELF-NOTE.

### Outcome (measured 2026-06-21, C: SSD)
Shipped. `edge-interning-audit` (`id: None` → real UUID edges, bulk path):

| Scale | Layer A edge RAM | Layer B edge RAM | Δ | B/edge | `id_to_u32` edge strings |
|---|---|---|---|---|---|
| 100k / 800k | 600.3 MB | **540.6 MB** | **−9.9%** | 787 → 708.6 | 800k → **0** |
| 200k / 1.6M | 1201.4 MB | **1098.4 MB** | **−8.6%** | 787 → 719.9 | 1.6M → **0** |

`edge UUIDs interned = 0` at both scales; `id_to_u32` now holds **nodes only**
(100k / 200k entries; key bytes 589 KB / 1.29 MB — the short `g{i}` node ids, not
~30/60 MB of edge UUIDs). B/edge is linear (708.6 → 719.9). **Combined Layer A+B vs
the original pre-Layer-A baseline (965.4 MB / 1265 B/edge @100k/800k): −44% edge RAM.**
All 47 integration tests green (10 in `edge_interning_tests.rs`, incl. 5 new Layer-B
cases). No WAL/CRDT semantic change.

---
### Related Links
- **Layer A:** [[ADR--GENESISDB-EDGE-ID-INTERNING]]
- **Root Cause:** [[RCA--EDGE-ID-INTERNING-RAM]]
- **Probe:** `benches/edge_interning_audit.rs`

## Changelog
| Version | Date | Summary |
|---|---|---|
| 0.1.0 | 2026-06-21 | Proposed & accepted: numeric u64 edge keys (deterministic SHA256-trunc), drop edge UUID strings from `id_to_u32`; u64 collision risk documented & accepted. |
| 0.2.0 | 2026-06-21 | Shipped & measured: −9.9% (100k) / −8.6% (200k) further edge RAM; edge UUID strings → 0; combined A+B −44% vs original baseline. |
