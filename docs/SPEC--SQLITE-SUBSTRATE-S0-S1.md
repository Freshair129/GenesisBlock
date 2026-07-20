---
proposed_id: SPEC--SQLITE-SUBSTRATE-S0-S1
type: spec
status: current
tier: process
cluster: implementation_flow
role: "SQLite substrate S0/S1 target specification - normative contract for foundation and props migration before SQL-backed HQL phases"
date: 2026-07-05
related:
  - adr/ADR--GENESISDB-EMBEDDED-SQLITE-SUBSTRATE
  - PLAN--HQL-REFINEMENT
  - SPEC--HQL-V2
  - docs/MASTER-SPEC--GENESIS-DB.md
---

# SPEC - SQLite Substrate S0/S1

**This spec defines the implementation contract for the first two substrate phases in [ADR--GENESISDB-EMBEDDED-SQLITE-SUBSTRATE](adr/ADR--GENESISDB-EMBEDDED-SQLITE-SUBSTRATE.md).** It exists to keep the substrate work doc-first, bounded, and explicitly separate from the native HQL P0 correctness/exposure track.

## 1. Scope and separation

### 1.1 What this spec covers

- **S0 - Foundation:** embed SQLite as an in-process derived projection under WAL authority, add schema v1, and prove crash/replay/rebuild invariants.
- **S1 - Props migration:** move node `props` out of the resident in-memory node map into the SQLite projection while preserving current engine behavior and recovery semantics.

### 1.2 What this spec does not cover

- No SQL-backed HQL execution (`S2`) and no FTS/BM25/hybrid text work (`S3`).
- No HQL grammar growth, no new REST/NAPI/MCP query syntax, and no public SQL surface.
- No MSP domain tables beyond what is required to keep substrate schema versioning extensible.

### 1.3 Track boundary

The work is intentionally split into two concurrent but independent tracks:

| Track | Purpose | Depends on SQLite S0/S1? |
|---|---|---|
| **HQL P0 correctness/exposure** | Fix native HQL wrong-answer/hidden-capability issues (`SEARCH` target semantics, hybrid `K`, `EF`, `OVERSAMPLE`, `TRAVERSE` exposure, strict numeric errors, executor correctness/perf hygiene) | **No** |
| **SQLite substrate S0/S1** | Establish the projection substrate and migrate `props` off the resident node map | **Yes, self-contained** |

Rule: **HQL P0 must remain shippable without waiting for SQLite S0/S1.** Any substrate design choice that would force a P0 rebase is out of scope for these phases.

## 2. Complexity, success criteria, and risk

### 2.1 Complexity classification

- **S0:** `C-3` - architecture/dataflow change across WAL, recovery, snapshotting, and storage layering.
- **S1:** `C-3` - architecture/data-layout change with performance and recovery consequences.

### 2.2 Success criteria

1. The engine still has **one durability authority**: the signed WAL.
2. SQLite can be deleted or corrupted and the engine can rebuild the projection from authoritative state.
3. S1 reduces resident memory pressure by moving `props` off the hop path without changing graph correctness.
4. HQL P0 remains independently implementable on the native executor.

### 2.3 Risk assessment

- **Risk level:** `HIGH`
- Reasons:
  - Cross-cutting persistence and recovery behavior.
  - Crash-window correctness across two embedded stores.
  - Potential hidden coupling to query execution and snapshot lifecycle.

## 3. Normative architecture contract

### 3.1 Storage authority

1. The signed WAL remains the **only source of truth** for mutations.
2. SQLite is a **derived projection** owned by the engine, never an authoritative write store.
3. No public or internal caller may write directly to SQLite except the engine's WAL-apply/rebuild path.

### 3.2 Division of responsibility

| Concern | Authoritative owner in S0/S1 | Notes |
|---|---|---|
| Mutation intent, signatures, clocks, replay order | native WAL path | unchanged |
| Graph adjacency and bitemporal edge traversal | native structures | unchanged |
| Vector collections, HNSW, quantization, rerank sidecar | native structures | unchanged |
| Node `props` payload persistence | SQLite projection | S1 makes SQLite the runtime source for props reads |
| Node labels for future SQL/HQL work | SQLite projection | schema exists in S0, execution use starts later |
| Projection progress / replay watermark | SQLite projection | `projection_state` |

### 3.3 Public-surface freeze

S0 and S1 must not change:

- HQL grammar or semantics
- REST request/response shapes
- NAPI public method signatures
- MCP tool signatures

Any unavoidable public contract change is a spec violation for these phases and must be deferred or split into a separate approved doc.

## 4. S0 - Foundation contract

### 4.1 Deliverables

S0 introduces:

1. Embedded `rusqlite` substrate under the existing feature model.
2. Schema v1 for the projection.
3. Projection replay/watermark logic.
4. Rebuild path from authoritative state.
5. Crash/recovery/snapshot tests.

### 4.2 Minimum schema v1

Schema v1 must include at least:

| Object | Purpose | Minimum fields |
|---|---|---|
| `props` | Node payload projection keyed by engine node identity | `node_u32`, serialized payload, projection metadata needed for idempotent upsert |
| `node_labels` | Normalized label rows for future indexed filtering/anchors | `node_u32`, `label` |
| `projection_state` | Replay watermark and schema versioning | `key`, `value` or equivalent minimal authoritative metadata |

This spec does **not** require the final SQL DDL shape, only the behavioral contract above.

### 4.3 Replay and rebuild invariants

S0 is complete only if these invariants hold:

1. **WAL-first:** a mutation is durable only when the signed WAL says it is durable.
2. **Idempotent apply:** replaying the same WAL slice into SQLite more than once converges to the same projection state.
3. **Gap healing:** if SQLite lags behind the WAL, open/recovery replays authoritative WAL events idempotently. S0/S1 may scan the full WAL; durable missing-suffix cursoring is owned by unified transaction phase U3 because Lamport time is not a WAL sequence.
4. **Full rebuild:** if SQLite is missing or invalid, the engine can recreate it from authoritative state without manual intervention.
5. **Snapshot coherence:** the SQLite file joins the same snapshot unit as the native snapshot outputs.

### 4.4 Crash windows to prove

S0 verification must explicitly cover:

1. Crash after WAL commit but before SQLite apply.
2. Crash mid-SQLite transaction during projection apply.
3. Startup with SQLite missing.
4. Startup with SQLite present but behind the recorded WAL position.

Expected outcome in all cases: recovery converges without trusting SQLite over the WAL.

### 4.5 S0 non-goals

- No runtime reads served from SQLite yet unless needed for internal parity checks.
- No SQL planner or public diagnostics surface.
- No FTS tables or BM25 semantics.

## 5. S1 - Props migration contract

### 5.1 Goal

S1 moves node `props` from the resident in-memory node representation into the SQLite projection so that payload storage becomes paged/on-disk while graph traversal and vector execution stay native.

### 5.2 Read/write model

After S1:

1. Writes still enter the signed WAL first.
2. SQLite becomes the runtime source for node `props` reads.
3. Native graph traversal must not require resident `props` to walk adjacency.
4. Any node view returned to existing public surfaces must remain behaviorally compatible with current responses.

### 5.3 Performance gates

S1 is not complete without both gates:

1. **RSS gate:** re-measure the resident-memory benchmark on the existing P31/PQ memory harness and show a meaningful reduction attributable to props migration.
2. **Traversal gate:** graph traversal/hop-path benchmarks show no material regression caused by props leaving memory, because props are not on the traversal hot path.

This spec intentionally does not hardcode numeric thresholds; the approval gate is comparison against the current repo baselines and benchmark methodology.

### 5.4 Compatibility rules

- Node identity, labels, temporal fields, and graph reachability semantics must not change.
- Existing tests that assert logical node content must still pass, even if the retrieval path for `props` is different.
- WAL replay, snapshot restore, and rebuild behavior from S0 remain valid after S1.

### 5.5 S1 non-goals

- No SQL pushdown for HQL predicates yet.
- No label-index execution changes yet.
- No change to hybrid/vector ranking behavior.

## 6. Verification matrix

| Area | S0 | S1 |
|---|---|---|
| WAL replay correctness | required | required |
| SQLite-missing rebuild | required | required |
| Crash-window recovery | required | required |
| Snapshot restore | required | required |
| Public API shape parity | required | required |
| RSS benchmark | optional baseline capture | required gate |
| Graph traversal benchmark | optional baseline capture | required gate |
| HQL P0 independence check | required doc review | required doc review |

## 7. Required documentation updates when implementation starts

The implementation PRs for S0 and S1 must update:

1. `ADR--GENESISDB-EMBEDDED-SQLITE-SUBSTRATE` if behavior diverges from this contract.
2. `docs/C4--GENESISDB-ARCHITECTURE.md` component/container notes if runtime ownership changes become externally meaningful.
3. `docs/SPEC--HQL-V2.md` only if a substrate decision unexpectedly changes HQL behavior. Otherwise, do not couple them.

## 8. Definition of done

### 8.1 S0 done when

- Schema v1 exists.
- Replay watermarking exists.
- Gap replay and full rebuild are proven by tests.
- Snapshot path includes SQLite coherently.
- No public API/HQL behavior change is introduced.

### 8.2 S1 done when

- `props` no longer need to be resident in the primary in-memory node map.
- Public behavior remains compatible.
- RSS improvement is demonstrated.
- Graph/traversal correctness and performance gates pass.
- Recovery invariants from S0 still hold.

## 9. Implementation evidence snapshot (2026-07-20)

The current working implementation satisfies the S0/S1 contract shape with the following
repo-verified evidence:

- `rusqlite` is embedded with the bundled build path, and the projection schema includes
  `props`, `node_labels`, and `projection_state`.
- Projection replay/rebuild behavior is covered by `tests/sqlite_substrate_s0_tests.rs`,
  including missing/corrupt SQLite recovery, stale-watermark healing, full-clock LWW, and snapshot round-trip.
- Resident nodes are now lean for `props`: the runtime stores `Value::Null` in the primary
  in-memory node map and hydrates `props` through the SQLite projection for behaviorally
  compatible reads.
- Public HQL behavior remains on the native executor track; the targeted HQL P0 suites still
  pass independently of the substrate work.

### 9.1 Benchmark evidence

Audit harness: `sqlite-props-audit` (`benches/sqlite_props_audit.rs`).

Medium run (`N=5000`, `fanout=4`, `prop_bytes=1024`, `depth=3`, `q=100`):

- RSS: `16 MB -> 25 MB -> 44 MB`
- Node ingest: `1.81 s`
- Edge ingest: `0.67 s`
- Resident nodes: `5000`
- Resident null props: `5000`
- Resident lean ratio: `1.0`
- Expected inline payload bytes: `5,120,000`
- Resident inline prop bytes: `20,000`
- Saved resident payload lower bound: `5,100,000`
- Traversal: `p50 4444.2 us`, `p95 6021.3 us`, `p99 7522.0 us`, `214 trav/s`

Large run (`N=20000`, `fanout=4`, `prop_bytes=2048`, `depth=3`, `q=200`):

- RSS: `15 MB -> 40 MB -> 106 MB`
- Node ingest: `31.3 s`
- Edge ingest: `5.7 s`
- Traversal: `p50 4551.0 us`, `p95 6073.9 us`, `p99 6913.5 us`, `211 trav/s`

Interpretation: the resident graph stays fully lean for `props` while traversal latency stays
in the same hop-3 class as the medium run. This meets the intent of the S1 RSS/traversal gate:
payload bytes moved off the resident node map without introducing a material hop-path regression
in the measured fixture.

## 10. Version diff

| From | To | Change |
|---|---|---|
| none | 0.1.0b | New doc-first target spec for SQLite substrate S0/S1, with explicit separation from the HQL P0 native correctness/exposure track. |
| 0.1.0b | 0.1.1b | Added implementation evidence and benchmark snapshot for the current S0/S1 SQLite substrate rollout. |
| 0.1.1b | 0.1.2b | Corrected recovery contract: Lamport time is not a WAL cursor; S0/S1 uses idempotent full-WAL replay and defers durable sequence cursoring to U3. |
