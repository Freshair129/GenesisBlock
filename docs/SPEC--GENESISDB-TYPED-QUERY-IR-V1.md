---
title: "GenesisBlockDB Typed Query IR V1"
doc_id: "SPEC-GENESISDB-TYPED-QUERY-IR-V1"
status: accepted
version: "1.0.1"
updated: "2026-08-14"
owner: "GenesisBlockDB Architecture"
implementation_status: partial
source_of_truth: true
related_docs:
  - "docs/adr/ADR--GENESISDB-TYPED-QUERY-IR-AGENT-BOUNDARY.md"
  - "docs/PRD--GENESISBLOCKDB-PLATFORM.md"
  - "docs/SRS--GENESISBLOCKDB.md"
---

# Specification: GenesisBlockDB Typed Query IR V1

## 1. Purpose and status

This specification defines the approved transport-neutral query boundary for new GenesisBlockDB
integrations. Its implementation status is `partial`: `search` and `traverse` are implemented in the
core, N-API and REST vertical slice with compatibility tests. The reserved `match_path`, `context`
and `relational_named_query` operations remain planned.

## 2. Contract principles

| ID | Requirement |
|---|---|
| TQIR-001 | Every request SHALL declare `contract_version: "query-ir.v1"`. |
| TQIR-002 | Every request SHALL contain exactly one discriminated `operation`. |
| TQIR-003 | Unknown fields, operation kinds and enum values SHALL fail validation unless a later contract explicitly defines forward-compatible handling. |
| TQIR-004 | Namespace, temporal, collection and consistency scope SHALL be explicit when used. |
| TQIR-005 | Resource controls such as `k`, depth, limit, `ef_search`, oversample and packet budget SHALL be bounded by engine capability limits. |
| TQIR-006 | Unsupported capabilities SHALL return a typed error and SHALL NOT silently degrade semantics. |
| TQIR-007 | Query results SHALL preserve client IDs, temporal metadata and evidence supported by the selected operation. |
| TQIR-008 | The same request SHALL have equivalent observable semantics across Rust, N-API, REST and supported SDK/MCP surfaces. |
| TQIR-009 | Free-form NL, prompts and model configuration SHALL NOT appear in the engine request. |
| TQIR-010 | HQL compatibility mapping SHALL be tested against the typed executor before HQL direct dispatch can be retired. |

## 3. Request envelope

The canonical JSON representation is:

```json
{
  "contract_version": "query-ir.v1",
  "request_id": "client-generated-id",
  "namespace": "example-client",
  "temporal": {
    "valid_at": "2026-08-14T00:00:00Z"
  },
  "consistency": {
    "index": "eventual"
  },
  "operation": {
    "kind": "traverse",
    "seed_id": "entity:42",
    "depth": 2,
    "relations": ["depends_on"],
    "direction": "out"
  }
}
```

Envelope fields:

| Field | Required | Rule |
|---|---|---|
| `contract_version` | yes | Exact V1 discriminator `query-ir.v1`. |
| `request_id` | yes | Caller-generated correlation/idempotency identifier; not a database record ID. |
| `namespace` | conditional | Required when the deployment or operation is namespace-scoped. |
| `temporal.valid_at` | no | RFC 3339 valid-time selector. Unsupported temporal behavior fails explicitly. |
| `consistency.index` | no | `eventual` or `read_your_write`; default and cost must be capability-reported. |
| `operation` | yes | Exactly one operation object from section 4. |

## 4. V1 operation union

V1 reserves these operation discriminators because they map to existing engine capability families:

| `kind` | Required core fields | Purpose |
|---|---|---|
| `search` | `mode`, one of `target_id` or `query_vector`, `k` | Vector, lexical or hybrid retrieval using declared capability fields. |
| `traverse` | `seed_id`, `depth`, `relations`, `direction` | Bounded forward, reverse or bidirectional graph traversal. |
| `match_path` | `pattern`, `limit` | Typed bounded linear path matching; the final typed pattern schema is frozen during implementation design. |
| `context` | `target_id` or `query_vector`, `tier`, `budget` | Versioned agent-context assembly without adding HQL grammar. |
| `relational_named_query` | `query_name`, `parameters` | Execute a pre-registered bounded named query; arbitrary SQL is forbidden. |

Operation-specific schemas SHALL be closed/discriminated types. Implementers SHALL NOT use a generic
`payload: object` escape hatch. The implementation slice may ship operations incrementally, but the
capability manifest must identify each as `implemented`, `partial`, `proposed` or `unsupported`.

### 4.1 Search controls

Search MAY include `collection`, `language`, `filters`, `k`, `ef_search` and `oversample` where the
capability manifest declares support. `target_id` resolution and vector collection compatibility
must preserve the current HQL P0 correctness rules.

### 4.2 Traverse controls

`depth`, relation count and result limit are mandatory bounded values. `direction` is one of `out`,
`in` or `both`. Relation labels are client data and do not grant domain authority.

### 4.3 Context result contract

The `context` operation SHALL return a versioned packet with selected records, evidence, omissions,
budget accounting and index lag. Hard budget, relevant compression and evidence requirements from
the superseded context proposal remain requirements of the operation-specific implementation design.

## 5. Result envelope

```json
{
  "contract_version": "query-ir.v1",
  "request_id": "client-generated-id",
  "status": "ok",
  "operation_kind": "traverse",
  "data": {},
  "meta": {
    "capability_version": "current",
    "index_lag": 0,
    "warnings": []
  }
}
```

`data` is a closed type selected by `operation_kind`. Partial or silently reinterpreted results are
not successful unless the operation contract explicitly defines that behavior and reports it in
`meta.warnings`.

## 6. Error taxonomy

Failures SHALL use stable machine-readable codes:

- `QUERY_IR_VERSION_UNSUPPORTED`;
- `QUERY_IR_VALIDATION_FAILED`;
- `QUERY_CAPABILITY_UNSUPPORTED`;
- `QUERY_SCOPE_UNAUTHORIZED`;
- `QUERY_RESOURCE_LIMIT_EXCEEDED`;
- `QUERY_TARGET_NOT_FOUND`;
- `QUERY_COLLECTION_MISMATCH`;
- `QUERY_EXECUTION_FAILED`.

Errors SHALL identify the invalid field or unsupported capability without exposing secrets or raw
model prompts.

## 7. HQL compatibility mapping

| HQL family | Query IR target |
|---|---|
| `SEARCH` | `operation.kind = "search"` |
| `TRAVERSE` | `operation.kind = "traverse"` |
| `MATCH (<pattern>)` | `operation.kind = "match_path"` |
| `MATCH ... SIMILAR` / `HYBRID` | `operation.kind = "search", mode = "hybrid"` |
| `CONTEXT` | `operation.kind = "context"` |

The HQL parser remains a compatibility adapter. Migration is complete only when shared fixtures prove
equivalent validation, result ordering where deterministic, temporal behavior, collection behavior
and error semantics. Removing an HQL surface is outside V1 scope.

## 8. NL adapter contract

The NL adapter is a separate package or client component and is not linked into the Rust storage core.
Its minimum pipeline is:

```text
NL input -> intent/model processing -> QueryRequestV1 candidate
         -> JSON Schema validation -> capability validation -> authorization policy
         -> GenesisBlockDB typed query API
```

The adapter SHALL:

- emit only the approved versioned schema;
- treat model output as untrusted;
- reject or clarify ambiguous intent rather than inventing destructive or unbounded operations;
- keep provider credentials and prompts outside database state unless a client explicitly stores an
  auditable record through a separate mutation contract;
- record request correlation and adapter/model version for client-side audit;
- never fall back to arbitrary HQL or SQL.

## 9. Implementation and acceptance gates

The contract is considered shipped only when all gates pass:

1. JSON Schema and native typed structures are frozen for the first implementation slice.
2. Core typed executor validates and executes at least `search` and `traverse`.
3. N-API and REST expose the same request/result semantics.
4. HQL parity fixtures pass through compatibility mapping.
5. Capability/version reporting distinguishes implemented operations.
6. SDK and MCP conformance tests cover supported operations.
7. NL adapter tests prove schema rejection, capability rejection and fail-closed ambiguity behavior.

## 10. Non-goals

- removing HQL;
- embedding an LLM or provider SDK in the database engine;
- arbitrary SQL, unrestricted Cypher or executable JSON;
- defining GoVibe, NotiKeeper or another client ontology;
- claiming every reserved V1 operation is already implemented.

## CHANGELOG

| Version | Date | Status | Summary | Commit Hash | Agent |
|---|---|---|---|---|---|
| 1.0.0 | 2026-08-14 | accepted | Approved Query IR V1 envelope, compatibility posture and external NL adapter boundary. | 84f2553 | ATHER |
| 1.0.1 | 2026-08-14 | current | Recorded partial implementation of search/traverse across core, REST and N-API with HQL parity; remaining V1 operations stay planned. | working-tree | ATHER |
