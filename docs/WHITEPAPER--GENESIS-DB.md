# GenesisBlockDB: An Embedded Semantic-Graph Engine for AI Agent Memory
**Whitepaper — evidence-backed revision (2026-06-21)**
**Status:** Advanced prototype with measured benchmarks (see §4, audits P14–P25)

## Abstract
GenesisBlockDB is an **embedded, local-first hybrid graph + vector engine** for AI
agent memory and analytics. It unifies HNSW vector search, an index-backed
property graph, a bitemporal/event-sourced model, governance tiers, and
optional CRDT synchronization in a single in-process Rust core (compiling to a
Node.js NAPI addon and an Axum REST server). This revision replaces earlier
aspirational performance figures with **measured, reproducible benchmarks**.

## 1. Positioning
GenesisBlockDB is an **embedded analytics / agent-memory engine**, not a distributed
enterprise graph DB. Its nearest comparators are **Kuzu, DuckDB (graph
extension), and RocksDB + graph layer**; Neo4j and Qdrant are well-known
*references* rather than the category. The design trades horizontal scale-out
for **in-process, microsecond-class local queries** — exactly what an agent's
working memory needs.

Vector DBs give similarity but not relations; graph DBs give relations but not
fuzzy/cross-lingual semantics. GenesisBlockDB unifies both in one embedded engine.

## 2. Core innovations

### 2.1 Thai-English Neural Bridge
Language-specific centroids bridge Thai↔English embedding spaces; Thai-aware
tokenization filters combining marks (vowels/tones) for high-recall fuzzy
lookups under linguistic noise.

### 2.2 Bitemporal event sourcing & causality
State changes are non-destructive: `supersede_node` versions a node and links it
via `caused_by`; edges carry `valid_from`/`valid_to`. The full causality chain is
auditable.

### 2.3 Incremental K-Impact
Node authority `R(n) = 0.5·DD + 0.3·AS + 0.2·SC` is recomputed **incrementally**
on mutation (localized), not by a full O(V) pass — proven O(V_affected) in §4.

### 2.4 Governance tiers & consensus
MASTER/SPEC/ADR/USER tiers enforced in the engine (not the transport). MASTER
axioms require multi-signature (ed25519) quorum. Guard cost is <0.1% of a write.

### 2.5 Optional distributed layer
Lamport clocks, signed `SignedEvent`s, LWW reconciliation, Merkle root, and P2P
gossip exist for masterless sync — an *option*, not the headline. (Anti-entropy
pull is still a stub; see §5.)

## 3. Autonomic substrate
A background loop performs LPA community detection, semantic-drift tracking, and
structural-gap detection over the meta-graph.

## 4. Measured performance (2026-06-21, audits P14–P25)

All numbers measured on this host (queries on SSD; the project disk is a 7200 RPM
HDD so fsync-bound writes are disk-limited — see P14). Reproduction harnesses in
`benches/`. **Earlier "<30 µs / 120 TPS" figures were measurement artifacts and
are retracted** (see AUDIT--P12).

### 4.1 Vector k-NN (HNSW, bge-m3 1024-dim, L2, 100k vectors)
- **Recall–latency frontier** (ef_search swept, ef_construction=200): recall
  0.984 @ ~1.1 ms (p50) at ef_search=128; 0.964 @ 0.81 ms at ef_search=64.
- vs **Chroma** (hnswlib): 0.981 @ 0.99 ms — GenesisBlockDB's curve passes through
  Chroma's point (same recall↔latency frontier). vs **Qdrant** (server):
  0.999 @ 3.3 ms (carries gRPC network cost).
- **Durable bulk insert:** ~2,000 vec/s (×7.8 over the naïve path) via batched
  WAL + rayon `parallel_insert`. **Concurrent ingest:** 839 TPS (12 writers,
  10k × 1536) after removing the per-op global HNSW lock (×6.1).
- **Memory:** −44% per node after dropping a redundant in-memory f64 copy
  (147→82 MB at 5k×1536; ~1.5 GB at 100k×1536).

### 4.2 Graph traversal (LDBC-lite, fanout 8)
| Nodes | hop1 p50 | hop3 p50 | hop6 p50 | hop1 throughput | RSS |
|---|---|---|---|---|---|
| 10k | 23 µs | 1.97 ms | 4.21 ms | 41,525/s | 146 MB |
| 100k | 22 µs | 2.33 ms | 4.40 ms | 42,783/s | 1.06 GB |
| 1M | 35 µs | 4.58 ms | 9.29 ms | 27,898/s | 12.6 GB |

1-hop latency stays in the tens of µs across a 100× node increase — traversal is
**O(neighborhood), not O(N)**. RAM (edge-id interning) is the scaling ceiling
(~12.6 GB at 1M/8M).

### 4.3 vs Neo4j (embedded GenesisBlockDB vs server Neo4j, same topology)
GenesisBlockDB is **7–185× faster** on k-hop traversal (hop1 @100k: 22 µs vs
2,590 µs); ingest and memory are ~par at 100k. The gap is largely the
embedded-vs-server tax (bolt + Cypher planning + JVM).

### 4.4 Engine costs
- **Governance guard:** ~0.5 µs/op, <0.1% of a durable write.
- **K-Impact:** incremental update ~1.7 µs (flat across 50× scale) vs full
  recompute O(V) (664 ms @500k) — **up to 398,000× faster**, confirming the
  O(V_affected + E_affected) claim.

### 4.5 Durability
Every write is WAL-persisted (group-commit fsync); snapshot instant-load + WAL
replay verified end-to-end (P14/P16).

## 5. Honest limitations
- Insert throughput trails in-memory Chroma (~1.5–2×) — durability + hnsw_rs
  build cost; deferred indexing is the next lever.
- Anti-entropy gossip pull is a stub; `retract_edge` is a stub; `neighbors`
  honors `limit` but not yet `direction`/`rels`.
- RAM bounds graph scale (~1M nodes / 8M edges on 32 GB); edge-id interning is
  the top optimization target.

## 6. Evidence & reproduction
Consolidated results: `REPORT--2026-06-21-PERFORMANCE-AND-COMPETITIVE.md`.
Per-area audits: P14 (post-refactor), P15/P20/P21 (vector + frontier + Qdrant),
P16–P19 (concurrency / WAL / parallel-build / ef), P22 (graph), P23 (Neo4j),
P24/P25 (governance / K-Impact). Harnesses: `benches/vbench*.{rs,py}`,
`graph_bench.rs`, `gov_kimpact_bench.rs`, `neo4j_bench.py`.

## 7. Conclusion
GenesisBlockDB is a credible **embedded AI-native graph+vector engine**: vector recall
at parity with Chroma on the same frontier, graph traversal that stays µs-class
as the graph grows, durable writes, and proven incremental-update and governance
costs — claims now backed by reproducible measurement rather than narrative.
