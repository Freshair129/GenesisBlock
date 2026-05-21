---
id: ADR--GENESIS-BLOCK-STORAGE-LAYOUT
phase: 2
type: adr
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: ADR — Genesis Block on-disk storage layout for P3.2 MVP
tags: &a1
  - msp
  - gks
  - graph
  - backend
  - genesis-block
  - rust
  - storage
  - decision
crosslinks: &a2
  references:
    - BLUEPRINT--GENESIS-GRAPH-INTEGRATION
    - ADR--GENESIS-GRAPH-AS-GKS-BACKEND
    - SPEC--GENESIS-GRAPH-BACKEND
    - PROTOCOL--GENESIS-GRAPH-FFI
    - CONCEPT--GENESIS-GRAPH-BACKEND
created_at: 2026-05-18T19:50:00.000+07:00
aliases: &a3
  - ADR
  - implementation_flow
  - Architecture decision record
cluster: implementation_flow
role: Architecture decision record
attributes:
  id: ADR--GENESIS-BLOCK-STORAGE-LAYOUT
  phase: 2
  type: adr
  status: stable
  vault_id: default
  tier: process
  source_type: axiomatic
  title: ADR — Genesis Block on-disk storage layout for P3.2 MVP
  tags: *a1
  crosslinks: *a2
  created_at: 2026-05-18T19:50:00.000+07:00
  aliases: *a3
  cluster: implementation_flow
  role: Architecture decision record
  domain: adr
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: true
  secret_type: high_entropy_string
  leak_risk: high
  encryption_level: none
---

# ADR — Genesis Block on-disk storage layout for P3.2 MVP

## Context

`[[BLUEPRINT--GENESIS-GRAPH-INTEGRATION]]` Phase 3.2 requires a **storage MVP** for the new Rust-backed Genesis Block engine, with the explicit gate:

> **⛔ HALT GATE 1** — Human reviewer signs off the storage layout direction
> before any persistent on-disk format is committed. Required because storage
> format is the single hardest thing to migrate after data lands.
>
> Approved either by `gate-1: approved` label on the P3.1 PR, **OR** a new
> `[[ADR--GENESIS-BLOCK-STORAGE-LAYOUT]]` if the approach diverges from the
> upstream LadybugDB fork.

P3.1 (the napi scaffold at `packages/gks/native/genesis-block/`) landed under PR #148 without that label. We need to commit a layout direction before writing the first node row in P3.2.

The existing TypeScript backend (`packages/gks/src/memory/graph/genesis-graph.ts`) already persists to JSONL — its layout note says:

> Eventual upgrade path (`[[BLUEPRINT--GENESIS-BLOCK-INTEGRATION]]` P3.1-P3.6):
> the Rust crate at `packages/gks/native/genesis-graph/` takes over the same
> directory. The on-disk format is forward-compatible because the Rust binary
> will accept either a `.db` binary file or the existing `.jsonl` event log;
> the TS adapter shape stays unchanged.

So the strict ADR-level question is: **does the P3.2 Rust crate keep the JSONL event log, or jump straight to a single-column page layout (LadybugDB-fork style) per `[[BLUEPRINT--GENESIS-GRAPH-INTEGRATION]]` §"Module breakdown"?**

## Options considered

| Option | Description | Cost | Risk |
|---|---|---|---|
| **A — JSONL event log (chosen)** | Rust crate reads/writes the same `<path>/manifest.json` + `<path>/genesis-graph.jsonl` files the TS impl uses today. Replay-on-open, append-on-mutation. | Low — format already exists; tests already cover it; ~150 LOC of Rust I/O. | Low — read scans linearly so >50k edges may not meet the BLUEPRINT §"Risks" <50ms p50 target. Tracked as P3.5 follow-up. |
| **B — Single-column page layout (LadybugDB-fork)** | Forward-port the upstream LadybugDB columnar page format. Fixed-size pages, free-list, page cache. Schema version byte in header. | High — ~600 LOC of storage code in Rust; migration tooling from JSONL; new format risk if benchmarks don't pan out. | High — burns the "hardest to migrate" budget on a format we haven't benchmarked yet. Locks us in before P3.5 data. |
| **C — In-memory only (no persistence)** | Skip persistence for P3.2; `load(path)` is a no-op. Reject all writes if path provided. | Trivial. | Breaks `[[BLUEPRINT--GENESIS-GRAPH-INTEGRATION]]` "Definition of done" for P3.2 (existing tests assume persistence); not a real MVP. |

## Decision

**Option A — JSONL event log.**

The P3.2 Rust crate persists to the **same on-disk layout** as the existing TypeScript backend:

```
<path>/
  manifest.json        — { "schema_version": "1.0.0" }
  genesis-graph.jsonl  — append-only event log; one JSON object per line:
                         { "kind": "node" | "edge" | "edge_retract", "payload": {...} }
```

This is the chosen MVP format for **P3.2 — P3.4**. The columnar page layout from Option B becomes a candidate for **P3.5 (Benchmarks)** *if and only if* the benchmark numbers from JSONL miss the <50 ms p50 threshold on the 50k-node / 500k-edge fixture.

## Rationale

1. **Forward-compatibility is already an architectural commitment.** The existing TS backend's header comment already documents that "the Rust binary will accept either a `.db` binary file or the existing `.jsonl` event log." Choosing JSONL for P3.2 honours that commitment instead of breaking it.
2. **The "definition of done" is test-suite pass, not format choice.** `[[BLUEPRINT--GENESIS-GRAPH-INTEGRATION]]` P3.2 DoD is "existing `test/memory/graph/` suite passes with the new backend parametrised in." JSONL is sufficient for that — most existing tests are correctness, not perf.
3. **HALT GATE 1's spirit is "don't commit a hard-to-migrate format prematurely."** JSONL is the *least* prematurely-committed format we have, because it is the format we already ship and already migrate against (`[[SPEC--GENESIS-BLOCK-MANIFEST]]` §schema_version contract).
4. **Performance is benchmark-gated downstream.** `[[BLUEPRINT--GENESIS-GRAPH-INTEGRATION]]` P3.5 has the explicit benchmark step. Locking in a column-store before that data exists inverts the gate order.
5. **The TS GraphStore at `packages/gks/src/memory/graph.ts` already validates this layout in production.** It is the in-process backend MSP has run against since Phase 0. The Rust crate inherits a proven format.

## Consequences

### Positive

- P3.2 implementation surface drops from ~600 → ~250 LOC of Rust I/O.
- Migration story is trivial: existing `*.jsonl` files in user workspaces work unchanged when the Rust backend is enabled.
- The TS pure-fallback path (when prebuilt `.node` is missing) keeps working without divergence.
- Bi-temporal logic (P3.3) is unaffected — JSONL already carries `valid_from` / `valid_to` columns per `[[BLUEPRINT--GENESIS-GRAPH-INTEGRATION]]` §"Module breakdown".

### Negative

- Linear scan on the JSONL replay path means cold-load time scales O(events). Mitigated by: (a) keep `outIdx` / `inIdx` adjacency maps in memory like the TS impl already does; (b) JSONL parsing is parallelisable in Rust (per-line independence).
- Single-column page benefits (range pushdown, vector instructions, compression) are not available in v0. **Tracked under future `[[ADR--GENESIS-BLOCK-COLUMNAR-STORAGE]]` to be written iff P3.5 benchmarks demand.**
- The on-disk format is not space-optimal — JSONL is ~2-4× larger than a packed binary format. Acceptable for the expected workspace size (10s of thousands of nodes/edges in a typical project).

### Neutral

- Schema version stays `1.0.0`. The schema version byte in the on-disk header (per `[[PROTOCOL--GENESIS-GRAPH-FFI]]` §6) tracks the *log event schema*, not the file container — so a future migration to columnar pages would bump *that* version, not break JSONL readers in older crates.

## Out of scope (deferred)

- **Columnar page layout** — deferred to a future ADR contingent on P3.5 benchmark outcomes. If JSONL meets the <50 ms p50 target on the 50k/500k fixture, this stays deferred indefinitely.
- **WAL / crash recovery semantics** — JSONL is already append-only with O_APPEND atomicity on POSIX. Windows behaviour matches per Node's `fs.appendFile` impl. No further WAL needed in v0.
- **Compression on disk** — out of scope; gzip-on-flush is a candidate post-benchmark optimisation.
- **Encryption at rest** — out of scope; same posture as the TS impl (`[[SPEC--EPISODE-ATOM]]` §9 also defers this).

## Alternatives rejected

**Option B (single-column page layout) was rejected** because:

- It commits the hardest-to-migrate decision before any benchmark evidence exists.
- It triples the implementation surface of P3.2.
- Its main benefit (perf) is exactly what P3.5 is designed to measure — running P3.5 against JSONL gives us the data needed to decide whether B is even worth it.

**Option C (in-memory only) was rejected** because it fails the BLUEPRINT P3.2 definition of done (tests that rely on persistence will fail). It would only be appropriate if we re-scoped P3.2 to "P3.2.a walking skeleton" — which Boss explicitly chose not to do (preferred ADR-first over split-PR).

## Migration path

If a future P3.5+ benchmark demands columnar storage:

1. Write `[[ADR--GENESIS-BLOCK-COLUMNAR-STORAGE]]` documenting the format choice.
2. Bump the schema version major byte in `[[PROTOCOL--GENESIS-GRAPH-FFI]]` §6 (e.g. `1.x` → `2.0`).
3. The Rust crate's `open(path)` detects the schema version, reads-then-migrates JSONL → columnar pages on first open (one-shot migration, idempotent).
4. New writes go to columnar; the JSONL file is retained for one upgrade cycle for safety, then deleted.

No data loss in the migration window.

## Open questions

None for P3.2. P3.5-time questions about columnar format will be tracked in the follow-up ADR if it becomes necessary.

## Connections

- [[BLUEPRINT--GENESIS-GRAPH-INTEGRATION]]
- [[ADR--GENESIS-GRAPH-AS-GKS-BACKEND]]
- [[SPEC--GENESIS-GRAPH-BACKEND]]
- [[PROTOCOL--GENESIS-GRAPH-FFI]]
- [[CONCEPT--GENESIS-GRAPH-BACKEND]]
