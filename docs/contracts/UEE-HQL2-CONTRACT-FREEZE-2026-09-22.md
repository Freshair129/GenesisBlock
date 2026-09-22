---
version: "0.1.0b"
created_at: "2026-09-22T00:00:00+07:00,ATHER,working-tree"
last_update: "2026-09-22T00:00:00+07:00,ATHER"
status: candidate
superseded_by: null
attributes:
  doc_type: "contract"
  domain: "database-architecture"
  scope: "UEE-HQL2 P3 contract and compatibility freeze"
  complexity: "C-3"
  risk: "HIGH"
  owner: "Boss (Founder / Product Authority)"
---

# UEE-HQL2 Contract Freeze

## Status and evidence boundary

This is a P3 candidate contract freeze. It is a reviewable documentation and fixture
artifact; it is not an implementation acceptance report and it authorizes no source,
storage, migration, API, merge or deployment change.

`engine_evidence_status: not_run_engine` is the default status for every fixture in
`UEE-HQL2-CONTRACT-FIXTURES.json`. A schema or fixture can describe a desired boundary
without proving that the current engine accepts, rejects or emits it.

## Frozen compatibility boundary

| Boundary | P3 decision | Evidence posture |
|---|---|---|
| Typed query contract | `query-ir.v1` remains the current compatibility boundary | Current repository specification; implementation is partial by design |
| HQL language | HQL v1 remains the current shipped language contract | Current repository specification; code remains the runtime authority |
| REST HQL route | `/v1/query/hql` remains the current route | Existing route compatibility is preserved; P3 adds no route code |
| Blueprint API envelope | `genesis.api.v2` is a candidate only | Blueprint proposal; not enabled by this freeze |
| Blueprint typed IR | `query-ir.v2` is a candidate only | Blueprint proposal; not enabled by this freeze |
| HQL2/planner/EXPLAIN | Deferred to P8 and a separate approved ADR | Unsupported or unverified until that gate |
| Disk/WAL authority | Existing WAL and current storage formats remain authoritative | P2 ADR decision; no migration begins in P3 |

There is no silent version promotion. A caller declaring a candidate version must not be
reinterpreted as a current version, and a current request must not be rewritten to a
candidate contract without an explicit versioned compatibility decision.

The candidate identifiers are exact and remain disabled by this freeze:
`genesis.api.v2`, `query-ir.v2` and `hql.v2`.

## Request compatibility rules

1. The existing REST HQL compatibility envelope has two accepted input representations:
   a raw JSON string containing HQL and an object containing `{ "query": "..." }`.
   P3 freezes these as one differential fixture pair; it does not change the route.
2. The two REST representations must normalize to the same logical HQL request before
   semantic comparison. The fixture is not a claim that the comparison has passed.
3. The current typed boundary keeps `contract_version: "query-ir.v1"` and the closed
   operation union documented by `SPEC--GENESISDB-TYPED-QUERY-IR-V1`.
4. The Blueprint candidate request is closed: its version, namespace, request id and
   exactly one of `hql` plus `language_version` or `ir` are explicit. Unknown fields,
   mixed HQL/IR representations and unsupported enum values fail closed in the future
   implementation gate.
5. Candidate HQL v2 requests do not make HQL v2 the current parser. They are retained as
   fixtures for the later HQL/IR boundary work.

## Result and error semantics

The candidate result fixture uses an explicit snapshot, column, row, semantics,
completeness, index-frontier and cursor envelope. Completeness is not inferred from row
count. A truncated or failed result must disclose its status and reason; an approximate
or lagging index must be disclosed rather than presented as exact.

Candidate errors use stable machine-readable fields: `code`, `stage`, `message` and
`retryable`, with optional `transaction_id`, `outcome`, source `span` and structured
`detail`. Durable mutation uncertainty must not be reported as a successful durable
publication. Unsupported planner/EXPLAIN behavior must fail closed until its own ADR,
executor and evidence gates are approved.

## Fixture manifest

The machine-readable P3 fixture set is:

[`UEE-HQL2-CONTRACT-FIXTURES.json`](UEE-HQL2-CONTRACT-FIXTURES.json)

It contains current-boundary, compatibility, candidate and negative cases. Every case
records its source references and keeps its expected result at `candidate` until an
implementation worker supplies RED/GREEN and independent verification evidence.

## Ownership and change control

- P3 owns only the two new files named above.
- Existing source, schemas, route handlers, SDKs, tests, WAL, snapshots and migration
  code are outside this phase.
- Any change to a current public contract, result/error field, version discriminator or
  planner boundary requires a new versioned ADR or an explicit amendment approved by the
  owner.
- P4 may begin only after this P3 freeze passes Verify Gate, Review Gate and Final Gate
  and receives explicit owner approval.

## Provenance

- `docs/adr/ADR--GENESISDB-UEE-HQL2-STAGED-ADOPTION.md` — approved P2 staged-adoption and
  compatibility decision.
- `docs/SPEC--GENESISDB-TYPED-QUERY-IR-V1.md` — current typed IR v1 boundary and error
  taxonomy.
- `docs/SPEC--HQL-V1.md` — current HQL v1 behavior and REST input compatibility.
- `docs/IMPLEMENTATION-PLAN--UEE-HQL2-ORCHESTRATION-2026-09-22.md` — P3 ownership and DAG
  gate.
- Extracted Blueprint `contracts/query-request.schema.json`, `query-result.schema.json`,
  `error.schema.json` and `examples/query-request.json` — candidate source material only.

## Non-goals

- Implementing HQL2, a planner, EXPLAIN, new indexes or a new executor.
- Changing `/v1`, N-API, FFI, SDK, MCP or mobile behavior.
- Changing WAL/storage formats, snapshots, ACLs, migrations or rollback code.
- Claiming Blueprint obligations are implemented or production-ready.

## CHANGELOG

| Version | Date | Status | Summary | Commit Hash | Agent |
|---|---|---|---|---|---|
| 0.1.0b | 2026-09-22 | candidate | Added P3 closed compatibility boundary and fixture manifest contract | working-tree | ATHER |
