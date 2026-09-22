---
version: "0.1.0b"
created_at: "2026-09-22T00:00:00+07:00,Codex"
last_update: "2026-09-22T00:00:00+07:00,Codex"
status: "candidate"
superseded_by: null
attributes:
  domain: "genesisdb-contracts"
  doc_type: "complexity-rule"
  scope: "G0 only"
---

# GenesisBlock UEE/HQL2 G0 Contract Boundary

## Status and scope

This document defines the first implementation slice for the UEE/HQL2 blueprint:
an isolated, fail-closed structural contract boundary. It is a v2 contract
registry and validation layer only.

The existing `query-ir.v1`, `/v1/*` routes, `GWA1/GSG1` journal, SQLite
projection, vector files, and migration behavior remain unchanged. This slice
does not authorize GBF2, GBO2, snapshot leases, ACL enforcement, HQL2
execution, or v1-to-v2 migration.

## Contract identities

The boundary recognizes these exact discriminators:

| Contract | Value | Meaning |
|---|---|---|
| API request | `genesis.api.v2` | v2 query envelope |
| Query IR | `query-ir.v2` | unbound logical DAG |
| Transaction | `genesis.tx.v2` | typed atomic mutation envelope |

The boundary must never reinterpret a v1 payload as v2.

## Structural guarantees

1. Query requests require exactly one source: HQL plus language version, or
   query IR. Unknown top-level fields fail closed.
2. Query IR requires a non-empty node list, unique identifiers, known inputs,
   an existing root, and an acyclic dependency graph.
3. Transaction envelopes require a UUID transaction ID, a valid namespace, at
   least one mutation, and a recognized mutation discriminator.
4. Temporal intervals reject `to < from`; decimal frontiers reject empty,
   signed, or leading-zero encodings.
5. Vector values must be finite and bounded by the wire contract.
6. Actor context is untrusted metadata. It is not authentication or
   authorization.
7. Receipt frontiers are optional at this stage because no v2 commit
   coordinator exists yet; their decimal encoding and namespace are still
   validated when present.

Operator-specific config validation, binder authorization, schema/collection
lookup, canonical JCS encoding, and execution semantics are intentionally
deferred to G1-G3.

## Parent and peer alignment

- Parent architecture: `docs/C4--GENESISDB-ARCHITECTURE.md` and
  `docs/MASTER-SPEC--GENESIS-DB.md` retain v1 as the current shipped boundary.
- Current peer contract: `docs/SPEC--GENESISDB-TYPED-QUERY-IR-V1.md` remains
  authoritative for current consumers.
- Blueprint source: `specs/01-system-architecture.md`,
  `specs/02-data-model-annotations.md`, `specs/05-query-ir-type-system.md`,
  `specs/08-transactions-temporal-security.md`, and the local JSON schemas.

## Verification

The G0 tests cover version separation, unknown-field rejection, source
exclusivity, DAG validation, cycle rejection, transaction mutation validation,
temporal/frontier boundaries, and receipt validation. They do not claim v2
storage or engine conformance.

## Changelog

| Version | Date | Status | Summary | Commit Hash | Agent |
|---|---|---|---|---|---|
| 0.1.0b | 2026-09-22 | candidate | Introduced isolated G0 v2 contract boundary; no v1 integration. | working-tree | Codex |
