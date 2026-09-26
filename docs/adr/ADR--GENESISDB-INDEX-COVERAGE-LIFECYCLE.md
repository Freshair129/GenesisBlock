---
proposed_id: ADR--GENESISDB-INDEX-COVERAGE-LIFECYCLE
type: adr
status: accepted
aliases:
  - ADR
phase: 35
tier: process
cluster: implementation_flow
role: "Architecture decision record"
enforcement_state: inactive
proposed_at: 2026-09-22T00:00:00+07:00
proposed_by: agent
---

# ADR--GENESISDB-INDEX-COVERAGE-LIFECYCLE

## Context

The asynchronous HNSW queue currently exposes `index_lag` and
`CollectionInfo.indexed`, but queue drain and point count do not prove that
every durable arena row is present in the navigable graph. The engine already
documents a failure mode where concurrent insertion can leave a point
unreachable. G7 therefore needs a factual structural-coverage signal before
query planning can rely on an index lifecycle state.

This ADR is deliberately narrower than the complete UEE-HQL2 G7 target. It
does not implement lexical indexing, ANN recall qualification, exact/approx
query policy, leases, immutable generations, or the full catalog state machine.

## Decision

Add an explicit, read-only structural validation operation on `Storage`:

1. Drain the existing asynchronous indexing queue.
2. Compare the source arena metadata IDs with all HNSW graph origin IDs.
3. Record source/indexed/missing/extra counts and the source/built commit
   frontiers.
4. Surface the last validation result as `CollectionInfo.coverage`.
5. Invalidate the last successful validation whenever a new vector is staged.

The reported states are:

- `UNVERIFIED`: no successful validation covers the current source set.
- `CATCHING_UP`: vectors are still pending in the asynchronous queue.
- `READY`: the most recent validation found exact source/graph membership
  equality for the current source set.
- `FAILED`: validation found missing or extra graph members.

`READY` means structural membership only. It does not mean exact search,
perfect ANN recall, graph connectivity, lexical readiness, or release
qualification. Query behavior remains unchanged in this slice.

## Verification contract

- Validation is explicit because a full graph membership scan is O(n) and must
  not be added silently to every `flush_index()` or query.
- `source_frontier` is the maximum `created_seq` in the collection metadata.
- `built_frontier` is the maximum `created_seq` represented by validated graph
  members.
- A validation result is stale when source count, indexed count, or source
  frontier changes, or when the global indexing queue is non-empty.
- The full UEE-HQL2 catalog lifecycle (`DECLARED` through `DROPPED`), leases,
  delta generations, and digest validation remain follow-on G7 work.

## Consequences

Existing REST/NAPI collection and status payloads gain factual coverage data
without changing search semantics. Consumers must not infer exactness or ANN
recall from `coverage.state == READY`.

The explicit validation method is initially a core Rust capability. Transport
maintenance endpoints and SDK methods belong to the later G9 parity lane.
