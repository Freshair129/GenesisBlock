# GenesisBlockDB — Positioning

## One line

**Verifiable, local-first agent memory** — an embedded engine that unifies a
vector index and a property graph behind one durable, bitemporal, cryptographically
signed store, so an AI agent's knowledge is fast to retrieve *and* auditable.

## The problem

Agent "memory" today is usually two systems bolted together: a vector database
for semantic recall and a separate graph/SQL store for relationships — plus glue
to keep them consistent. Neither was built to be **embedded** (they want a server),
**bitemporal** (what did the agent believe last Tuesday?), or **verifiable** (who
wrote this fact, and can it be forged?). For autonomous agents that act on their
memory, those last three are not nice-to-haves.

## What GenesisBlockDB is

A single in-process Rust core — WAL-durable storage, per-collection HNSW vector
indexes, an index-backed property graph, a bitemporal/event-sourced model,
governance tiers, and ed25519-signed CRDT sync — compiled to a **Node.js native
addon** and a **standalone REST server** from the same code. No server required
to start; no second database to reconcile.

## Why it's different

- **One store, two retrieval modes.** Vector k-NN and graph traversal over the
  *same* nodes — `HYBRID` blends semantic similarity with graph-derived K-Impact
  in one query, no cross-system join.
- **Bitemporal by construction.** Every node/edge carries `valid_from`/`valid_to`;
  evolution is supersession, not destructive overwrite. `retract_edge` is a soft
  delete. You can query the graph *as of* any past instant — time-travel for
  "what did the agent know when it made that decision."
- **Verifiable.** Events are ed25519-signed; governance tiers prevent external
  agents from forging `MASTER`-tier facts; consensus votes are signature-checked.
  Memory you can audit, not just trust.
- **Local-first & embeddable.** Runs in-process like SQLite. Optional CRDT gossip
  syncs a swarm of peers without a coordinator.

## Where it sits

The category is **embedded analytical/graph engines**, not hosted databases:

| | GenesisBlockDB | Kuzu | DuckDB+graph | Chroma | Qdrant | Neo4j |
|---|---|---|---|---|---|---|
| Embedded (in-process) | ✅ | ✅ | ✅ | ✅ | ❌ server | ❌ server |
| Vector **and** graph in one store | ✅ | partial | partial | vector only | vector only | graph only |
| Bitemporal / time-travel | ✅ | ❌ | ❌ | ❌ | ❌ | partial |
| Signed events / governance | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Built-in CRDT swarm sync | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ (enterprise) |

Kuzu, DuckDB+graph, and RocksDB+graph are the honest **performance** comparators;
Neo4j and Qdrant are references for the graph and vector axes respectively.

## Measured, not narrated

From the [consolidated performance report](REPORT--2026-06-21-PERFORMANCE-AND-COMPETITIVE.md)
and head-to-head audits (P22–P31), on the same corpus/queries:

- **Graph traversal** is `O(neighborhood)`, not `O(N)`: 1-hop p50 ~22 µs across
  10k→1M nodes — **~189× faster than Kuzu**, **~177× vs LadybugDB**, **~52× vs
  DuckDB**, **7–185× vs server Neo4j** on k-hop (tied-class with RocksDB on µs
  latency — no structural claim there).
- **Vector k-NN**: recall@10 0.984 @ ~1.1 ms p50 — at parity with Chroma on the
  same recall↔latency frontier.
- **Memory**: edge-id interning cut RSS **−35%** (1,057 → 686 MB at 100k/800k);
  optional **SQ8 (4×)** and **binary (32×)** quantization shrink resident vectors
  further for 500k–2M scale.
- **Incremental K-Impact**: `O(V_affected)`, ~1.7 µs — up to ~398,000× faster than
  a full recompute; the governance guard is <0.1% of a write.

## Honest caveats

- Bulk insert throughput trails in-memory-only stores (~1.5–2×) — the cost of WAL
  durability and HNSW build.
- Quantization recall on real embeddings is still being measured before any
  default-on; lossless f32 remains the default.
- Exact Merkle convergence in gossip needs ordered/version-vector digests; today
  pull-based anti-entropy converges graph *state*.

## Who it's for

Builders of **autonomous agents and agent platforms** who need memory that is
local-first, fast on both semantic and relational queries, replayable through
time, and auditable — without standing up and synchronizing two databases.
