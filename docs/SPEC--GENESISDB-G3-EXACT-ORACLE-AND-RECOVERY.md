---
doc_id: SPEC--GENESISDB-G3-EXACT-ORACLE-AND-RECOVERY
owner: GenesisBlockDB Engineering
version: 0.1.1b
created_at: "2026-10-04T00:00:00+07:00,Codex,working-tree"
last_update: "2026-10-04T00:00:00+07:00,Codex"
status: draft
superseded_by: null
attributes:
  doc_type: spec
  domain: query-correctness-and-recovery
  scope: g3-exact-oracle-differential-crash-reopen
  parent_doc: IMPLEMENTATION-PLAN--UEE-HQL2-ORCHESTRATION-2026-09-22
  peer_docs:
    - SPEC--GENESISDB-UNIFIED-OPERATIONAL-BOUNDARY-V1
    - SPEC--GENESISDB-P6-GENERATIONS-LEASES-ACL
  risk: HIGH
  complexity: C-3
---

# G3 — Exact oracle, differential execution and recovery proof

## Purpose and boundary

G3 turns the current G1/G2 contracts into executable, reproducible evidence. It
does not enable HQL2, a planner, EXPLAIN, new storage formats, or new public
surfaces. The current `query-ir.v1`, Relational U2, signed WAL, stable frontier,
snapshot, and P6 lease/temporal/ACL contracts remain authoritative.

The oracle is a pure reference model. It may read a fixture, but it must not
import `Storage`, SQLite projection files, WAL files, filesystem snapshots, or
engine-private helpers. The engine side of the differential test uses only the
public Rust `Storage` API.

## Fixture contract

`tests/fixtures/g3_oracle_cases.json` is versioned as `g3.oracle.v1` and carries
operations, named transaction markers, query cases, and expected canonical
results. Canonicalization removes nondeterministic timestamps and UUIDs while
retaining node identity, properties, valid-window state, row multiplicity,
NULLs, and column names.

The first fixture set covers:

1. valid-time × transaction-time traversal across a supersession;
2. relational left join with unmatched NULL projection and duplicate matching
   rows (bag semantics).

The fixture is intentionally small. It is a correctness anchor, not a workload
benchmark.

## Normative semantics under test

- Valid time is start-inclusive/end-exclusive:
  `valid_from <= as_of < valid_to`; a missing `valid_to` is open-ended.
- Transaction-time reads include only events at or before the named marker.
  A query after supersession may resolve the historical version, while a query
  at the earlier marker must not observe the later closing frame.
- Missing relational matches in a left join project JSON `null`.
- Relational results preserve bag multiplicity; matching two rows produces two
  rows, not a set-deduplicated result.
- Canonical output ordering is deterministic by the query's declared primary-key
  offset ordering and then source row order. No unordered result is declared
  equal merely because it has the same members.

## Differential and recovery gates

The Rust differential target runs every fixture through `Storage`, compares its
canonical result with the fixture oracle, saves a snapshot, removes the
materialized snapshot/projection files while retaining the authoritative WAL,
reopens, and compares the same results again. The test records the stable
frontier before and after recovery and requires the recovered frontier to cover
the fixture's durable mutations.

The existing crash-simulation matrix remains a separate fault-injection gate.
G3 adds only the oracle-backed semantic comparison; it does not weaken existing
`RECOVERY_REQUIRED`, torn-tail, or uncertain-commit behavior.

## Machine-checkable gates

```text
python -m unittest discover -s tests -p 'test_reference*.py'
cargo test --no-default-features --test g3_oracle_differential_tests --test crash_simulation_tests
cargo test --no-default-features --test bitemporal_matrix_wp31_tests --test relational_u2_tests --test unified_transaction_u3_tests
```

The Python runner uses only the standard library; `tools/requirements.txt` is
kept empty so the gate is reproducible without a network install.

## Non-goals and evidence boundary

This spec does not prove hosted CI, power-loss hardware behavior, mobile/device
behavior, release packaging, planner correctness, or production readiness.
Those remain NOT_RUN until their named external gates execute.

## Local execution evidence — 2026-10-04

The pure Python oracle gate passes 1/1 cases under both the direct
standard-library runner and the prescribed `uv run --with-requirements`
runner. The Rust differential/reopen target passes 2/2 cases. The fixture
exercises temporal graph version selection and relational NULL/bag left-join
semantics before and after materialized snapshot/projection removal with WAL
recovery. Existing G1/G2/crash regression targets pass in the bounded matrix;
the full `cargo test --no-default-features` sweep completed with exit code 0,
including the 60-build `probe_vs_recall` target and crate doc-tests.

This is local working-tree evidence only. Hosted CI, power-loss hardware,
mobile/device, release packaging, deployment, independent review, and
production acceptance are NOT_RUN. G4+ remains deferred.

## CHANGELOG

| Version | Date | Status | Summary | Commit | Agent |
|---|---|---|---|---|---|
| 0.1.1b | 2026-10-04 | candidate | Added local Python and Rust differential/reopen evidence for the bounded g3.oracle.v1 fixture; external gates remain NOT_RUN. | working-tree | Codex |
| 0.1.0b | 2026-10-04 | draft | Define a pure G3 oracle, canonical fixtures, differential comparison, and WAL-backed recovery rerun. | working-tree | Codex |
