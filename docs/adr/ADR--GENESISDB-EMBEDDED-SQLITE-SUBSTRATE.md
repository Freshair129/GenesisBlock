---
proposed_id: ADR--GENESISDB-EMBEDDED-SQLITE-SUBSTRATE
type: adr
status: accepted
tier: strategy
cluster: implementation_flow
role: "ADR — embed SQLite inside the engine as the relational/FTS projection under WAL authority; fold HQL P2/P3 and Wave 2.5 into one substrate program"
date: 2026-07-03
deciders: Boss
related:
  - ADR--GENESISDB-COMPETITIVE-SUPERIORITY
  - PLAN--HQL-REFINEMENT
  - SPEC--HQL-V2
  - adr/ADR--GENESISDB-HQL-FILTER-PROJECTION
  - adr/ADR--GENESISDB-KIMPACT-AS-SIGNAL
---

# ADR: Embedded SQLite Substrate (one engine, two storages, one WAL authority)

**Status:** Accepted · **Date:** 2026-07-03 · **Accepted:** 2026-07-20 · **Deciders:** Boss

## 1. Context

Strategic direction (2026-07-03 session): the product is the **MSP memory layer** ("กุ้งเผา"/"2child" vision — human-like belief revision, contextual multi-anchor recall) running **on** GenesisBlockDB; the engine is demoted from "general-purpose competitive DB" to **substrate with one customer**. That reframing exposed what the engine lacks and what it over-builds:

- **Lacks:** relational tables with real schemas (MSP's User_Block / episodic / session records), SQL filtering + aggregation, lexical/BM25 search. HQL's refinement plan (P2: OR/parens, `count(*)`, label index; P3: text query) was about to grow a bespoke mini-database inside HQL to compensate.
- **Over-builds (for this mission):** benchmark arms-race features and the enterprise ops floor (frozen per the strategy session).
- **Measured weakness this design attacks:** RSS 686 MB @100k/800k (7.1× vs LadybugDB) — all node `props` live resident in DashMap; there is no paged/on-disk payload store.
- **Proven precedent, external:** Qdrant embeds RocksDB in-process for payload storage beside its own HNSW — "own index engine + embedded commodity store" is a shipped, industry-standard composition.
- **Proven precedent, internal:** NotiKeeper (our first real consumer) already runs GenesisBlockDB + SQLite FTS5 (trigram BM25) + RRF **side by side at the app layer**. This ADR pulls that exact composition inside the engine.

## 2. Decision

**Embed SQLite (via `rusqlite`, `bundled` feature — compiled into the same binary, no system dependency, no server, no IPC) as a second in-process storage beside the native graph/vector core, under a single durability authority: the existing ed25519-signed WAL.**

### 2.1 Division of labor

| Concern | Storage | Note |
|---|---|---|
| Graph: adjacency (`out_idx`/`in_idx`), bitemporal edges, `supersede`/`caused_by` | **native** | unchanged |
| Vectors: per-collection HNSW, quantization, rerank sidecar | **native** | unchanged |
| Governance, ed25519 signing, WAL, CRDT sync | **native** | the moat; unchanged |
| Node `props` / payload (today: resident DashMap JSON) | **SQLite** | paged, indexed, spills to disk — the RSS lever |
| Labels | **SQLite** (`node_labels(node_u32, label)` indexed) | replaces the planned bespoke `label_idx` (HQL P2-T3) |
| MSP tables: User_Block, episodic (incl. emotion fields), session log | **SQLite** | real schemas + migrations |
| Lexical search: BM25 | **SQLite FTS5** (trigram tokenizer) | replaces the planned hand-written Rust BM25 (Wave 2.5) — same design NotiKeeper validated |
| Filtering (WHERE incl. OR/parens), aggregation (`count`, future group-by) | **SQLite** | HQL compiles these down instead of growing an evaluator |
| Recall pipeline (SQL filter → graph expand → vector rerank → RRF) | **native orchestration** | in-process function calls end-to-end |

### 2.2 Durability contract — WAL is the only authority

The one hard problem is two durability domains in one process. Rules (each is a testable invariant):

1. **Every write enters the signed WAL first** (existing path, unchanged). SQLite is a **derived projection**, never a source of truth.
2. SQLite apply happens **after** WAL ack, through the same single-writer application path (compatible with SQLite's single-writer model).
3. During S0/S1, startup scans the authoritative WAL and applies node events idempotently using the full LWW clock `(time, peer_id)`. A Lamport scalar is metadata, not a WAL cursor. A durable commit sequence and missing-suffix optimization move to unified transaction phase U3; until then recovery favors correctness over replay speed. If SQLite is corrupt/missing, rebuild it from the WAL. Recovery never trusts SQLite over the WAL.
4. Snapshot = existing snapshot set + the SQLite file, captured through the same atomic temp-dir swap. One backup unit.
5. **No direct external writes to SQLite.** Read-only SQL access may be exposed later (a diagnostics/query surface); any write bypassing the WAL is a corruption bug by definition.
6. Crash tests must cover the torn window: WAL committed + SQLite not yet applied (→ replay heals), and mid-SQLite-transaction crash (→ SQLite's own journal rolls back, replay reapplies).

### 2.3 Feature gating

`rusqlite` sits behind the existing feature architecture: on by default, present in `mobile` (SQLite is native on iOS/Android; bundled build ≈ +~1 MB), and adds no `napi_*`/server coupling — the core/napi split is unaffected.

## 3. Options considered (the "why not X" register)

| Option | Verdict | Reason |
|---|---|---|
| **SQLite (rusqlite bundled)** | **CHOSEN** | Only candidate with relational + SQL + FTS5(BM25) + aggregation + ~1 MB + public-domain license + on-device ubiquity + single-writer model matching our WAL pipeline. NotiKeeper already proved the exact FTS5+RRF composition against this engine. |
| RocksDB | rejected | KV only — provides "fast bytes", which our WAL/snapshot already are. None of what we lack (tables/SQL/FTS/aggregation); heavy C++ dep, tens of MB, harder mobile builds. Qdrant embeds it because Qdrant needs only payload KV; we need relational+FTS. (Its `SstFileWriter` bulk-load *technique* remains copy-worthy for Wave 2.1 — technique, not engine.) |
| DuckDB | deferred option | Embedded, MIT, but OLAP/columnar — wrong shape for row-level point lookups of an agent-memory workload; its vector extension had documented WAL-recovery gaps (competitive ADR §2.1). Revisit only if an analytics surface is ever needed. |
| SurrealDB (embedded mode) | rejected | (1) It is a whole graph+vector+document engine — embedding it **replaces** our core rather than complementing it; (2) BSL-class source-available license — commercial-use constraints, legal risk inside our product (verify current terms before ever revisiting); (3) it is a tracked **competitor** (competitive ADR §1.1) whose missing capabilities (bitemporal, signed provenance) are exactly our moat. Building on a competitor's restricted-license engine is strategically incoherent. |
| CockroachDB | rejected (category error) | A distributed SQL **server cluster** — not embeddable at all. Solves multi-node scale-out OLTP, which is not this product's model. |
| CouchDB | rejected (category error) | An HTTP document **server** with multi-master replication — not embeddable (its embeddable cousins are different products). Our multi-node/replication path is the native CRDT+ed25519 sync (Wave 3.2), which is a differentiator, not a gap to outsource. |

### 3.1 Self-hosting clarification (recorded because the question keeps recurring)

**Self-host requires embedding nothing new.** `genesis-db-server` (Axum, `--features bins`) *is* the self-host deployment: one binary on a server, storage embedded in-process, REST under `/v1/*`. "Self-host" changes where the process runs, not what is inside it. Multi-node HA, when needed, is the CRDT sync path — not a distributed SQL dependency.

## 4. Program: SUBSTRATE phases (folds HQL P2/P3 in — per operator directive 2026-07-03)

**HQL P0 (defect fixes) and P1 (pattern power) are unaffected and proceed as planned** — they are native graph/grammar work with no SQLite dependency. The former P2/P3 are superseded/reshaped as follows:

- **S0 — Foundation (own PR):** rusqlite dep + schema v1 (`props`, `node_labels`, `projection_state`) + the §2.2 durability contract implemented + crash/replay/rebuild tests. No behavior change visible to callers.
- **S1 — Props migration (own PR):** node `props` move from resident DashMap to the SQLite projection (read-through, WAL-replay-rebuildable). **Gate: RSS re-measured on the P31 harness** — this is the structural attack on the 7.1× RAM gap; graph-bench must show no traversal regression (props are not on the hop path).
- **S2 — HQL over SQL (own PR; supersedes old P2-T1/T2/T3):**
  - WHERE (now incl. **OR/parentheses**) on prop/label fields compiles to SQL against the projection → **indexed, pushed-down filtering** — upgrading the filter-projection ADR's documented "post-retrieval WHERE" limitation, not just adding syntax.
  - `RETURN count(*)` → SQL `COUNT`.
  - `(:Label)` anchors and label predicates → `node_labels` index (old P2-T3's bespoke `label_idx` is cancelled; consistency comes free from the WAL→projection pipeline instead of a hand-maintained DashMap).
  - `score`/`depth` predicates (engine-computed fields) stay in the small in-memory evaluator; the grammar is unchanged from SPEC--HQL-V2 §4.1–4.2 — only the execution engine changes. P2-T4 (CONTEXT clauses) stays ship-or-drop as planned.
- **S3 — Text & hybrid (own PR; resolves old P3 AND Wave 2.5 together):** FTS5 (trigram, BM25) over designated text props + `SEARCH TEXT "…"` + RRF fusion with vector results when both signals present. The P3-T0 design fork is hereby **decided by substrate choice**: option (b) in-engine lexical, on proven FTS5 rather than hand-rolled BM25; the P3 ADR shrinks to documenting ranking/fusion semantics. One lexical engine serves both HQL and the REST/NAPI hybrid surface — the Wave 2.5 duplication risk is closed.
- **S4 — MSP schemas (with the MSP product, own ADR):** episodic/semantic/sensory tables, hypothesis-hold state convention, consolidation hooks. Out of scope here; this ADR only guarantees the substrate supports it.

Ordering after approval of `SPEC--GENESISDB-UNIFIED-OPERATIONAL-BOUNDARY-V1`: S0 → S1 → U2 relational contract → U3 unified transaction → U4/U5 artifact proof. S2/S3 are renamed to demand-gated U6 and no longer block the database product path. HQL P0 remains independent.

## 5. Consequences

**Easier:** engine identity sharpens to the un-copyable combo (relational+graph+vector+bitemporal+signed, embedded, one log); RSS weakness attacked structurally instead of micro-optimized; HQL stops growing into a database language (stays thin graph/vector verbs per its three ADRs); Wave 2.5 and HQL-P3 collapse from "build a lexical engine" to "wire FTS5"; MSP gets real schemas and migrations; maintenance is one repo/one release/one backup unit — the operator's stated goal.

**Harder:** a second storage engine in the binary (+~1 MB, new dep to track); the §2.2 contract must be enforced forever (new class of crash tests); WAL replay now also rebuilds a projection (replay time grows with data — mitigated by snapshotting the SQLite file); HQL execution becomes two-engine (SQL + native) even though the language stays planner-free — fixed pipelines only, no cross-store optimizer (reaffirming the no-planner invariant).

**Supersedes / amends:** PLAN--HQL-REFINEMENT P2-T1/T2/T3 execution strategy + P3-T0 option set (notes added in that plan); SPEC--HQL-V2 §4/§5 execution notes (amend when this ADR is accepted); competitive ADR Wave 2.5 (implementation = FTS5, not hand-rolled); the VQ/RSS roadmap gains S1 as its node-payload lever (node-id interning remains separate and still valid).

**Revisit:** DuckDB analytics surface (only on demonstrated need); read-only SQL as a public diagnostics surface; whether GRL/association-recall wants SQL-side candidate pre-filtering (S4 territory).

## 6. Action items

1. [x] S0: rusqlite + projection schema + WAL-authority replay/rebuild tests
2. [x] S1: props → SQLite projection + RSS/traversal evidence
3. [ ] U2: app-defined relational schema and named-query contract
4. [ ] U3: durable commit sequence and unified cross-domain transaction protocol
5. [ ] U6 (demand-gated): SQL-backed HQL and FTS5+BM25+RRF
6. [x] Amend SPEC--HQL-V2 positioning; keep grammar contract unchanged
7. [x] HQL P0 remains separate and independently shippable
