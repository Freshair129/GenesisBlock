---
title: "Retire K-Impact Importance Policy"
doc_id: "ADR--GENESISDB-RETIRE-KIMPACT"
type: adr
status: accepted
version: "1.0.0"
owner: "GenesisBlockDB Architecture"
decision_date: "2026-11-02"
supersedes:
  - "ADR--GENESISDB-KIMPACT-ALGORITHM"
  - "ADR--GENESISDB-KIMPACT-AS-SIGNAL"
  - "ADR--GENESISDB-EVA-KIMPACT-BRIDGE"
---

# ADR: Retire K-Impact importance policy

## Context

GenesisBlockDB previously computed an importance scalar, stored it as a
top-level `impact` field, blended it into vector search with `alpha`, and used
it in governance, community summaries, and structural-gap decisions. These
engine-owned ranking and authority rules couple generic graph storage to
application policy and produce unsupported authority claims.

## Decision

Remove engine-owned importance computation and every policy consumer. The core
will not calculate, refresh, rank, veto, aggregate, or threshold nodes by
impact. Vector search scores remain vector relevance; graph traversal and
community discovery remain independent of importance.

Current APIs reject a supplied legacy `impact` or `alpha` value with
`KIMPACT_REMOVED`, with guidance to store domain-owned signals under generic
`props` and perform application-specific ranking above the database. The HQL
`ALPHA` clause is removed. It is not accepted as an ignored compatibility
parameter.

## Historical data and signed events

Existing persisted or replicated top-level impact values are historical data,
not active policy. Where decoded into the current graph projection, a legacy
value is preserved idempotently under `props["legacy.genesisdb.impact"]`.
Migration fails on a conflicting existing property rather than overwriting
either value. New public/runtime DTOs do not expose a top-level impact field.

Signed journal/event envelopes retain their original legacy wire shape until
signature and hash verification completes. Normalization occurs only at the
runtime projection boundary. This preserves canonical event bytes, signatures,
and historical replay behavior while removing impact from current APIs.

Old SQLite projection columns may remain readable as migration inputs; they
are not part of the current schema/API contract and are not recomputed.

## Consequences

- Vector search is ordered by vector relevance and its existing retrieval
  parameters; no hidden or default alpha blend exists.
- Similarity does not imply authority. Governance policy belongs to the
  consuming application.
- Existing impact history remains inspectable as generic legacy metadata after
  projection migration.
- Historical proposals to retain or revive K-Impact remain in the repository
  as superseded design history, not current implementation requirements.

## Release compatibility boundary

Backward decoding of historical signed events is distinct from forward or mixed-
version compatibility. New event envelopes omit the retired field. The base
serializer emitted `impact:null` for an empty optional value, so an older peer
can reconstruct different canonical bytes when it decodes a new envelope.
Mixed-version replication/consensus and reopening a newly written store with
an older binary are not qualified by the backward-replay regressions.

This branch retains its base package version for local development; it is not
a drop-in published release of that version. Release work must select and
validate explicit engine/wire capability/version boundaries and an upgrade
strategy before distributing these breaking contracts.

## Required verification

- Verify an old signed legacy node/edge event before runtime normalization and
  prove replay succeeds with its signature intact.
- Verify restore/snapshot and SQLite edge-version migration preserve historical
  values without exposing active output fields.
- Verify explicit legacy `impact`/`alpha` inputs fail with `KIMPACT_REMOVED`.
- Verify vector search and HQL have no hidden alpha value or impact blend.
