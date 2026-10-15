---
doc_id: API_REFERENCE
status: current
version: generated
owner: GenesisBlockDB Engineering
updated: "2026-10-15"
---

# GenesisBlockDB REST API Reference

**Generated from `src/router.rs` (Axum server) — 2026-10-15.** The server is the
SSOT; update this when routes or response contracts change.

- **Base URL:** `http://localhost:3000` (port via `GENESIS_PORT`, bind `0.0.0.0`)
- **Data dir:** `.brain/gks/storage` (via `GENESIS_DATA_DIR`)
- **Bodies:** JSON. REST handler, extractor, and API-key failures use the
  machine-readable envelope below and retain the route's HTTP status. CORS defaults to localhost origins;
  `GENESIS_CORS_ORIGIN` selects one origin, while `*` explicitly enables
  permissive mode.
- **Authentication:** when `GENESIS_API_KEY` is set, all `/v1/*` routes require
  `Authorization: Bearer <key>`. This is a bootstrap shared secret, not scoped
  OIDC/JWT authorization. `/metrics` remains unguarded.
- **Run:** `cargo run --features bins --bin genesis-db-server`

> ⚠️ Two contract gotchas that have bitten SDKs:
> 1. `POST /v1/query/hql` accepts both a **raw JSON string** body (e.g. `"SEARCH …"`)
>    and `{"query":"…"}`. `POST /v1/studio/query/read` accepts the same two shapes
>    but enforces the read-only HQL command family.
> 2. Edge `from`/`to` are **node string ids** (e.g. `"N-…"`), not integers.

## Error response envelope (API-004)

Handler errors, API-key rejections, and JSON/query/path extraction rejections
return `application/json` with a common `error` object:

```json
{
  "contract_version": "genesis.api.errors.v1",
  "error": {
    "category": "validation",
    "code": "GENESIS_VALIDATION_ERROR",
    "message": "COLLECTION_DIM_INVALID: expected 1..65535",
    "status": 400
  },
  "message": "COLLECTION_DIM_INVALID: expected 1..65535"
}
```

`error.category` and `error.code` are stable machine-readable values. The
`message` retains the original engine or extractor detail, including existing
engine code prefixes. `error.status` mirrors the HTTP response status; most
routes retain their previous status, while a write failure that leaves the
Storage instance in recovery-required state returns `503` and category
`durability`.

The shared Rust type is `ApiErrorEnvelopeV1`. Its `details` member is optional;
REST sets `status`, while embedded callers omit it because there is no HTTP
status. Successful NAPI contract results use the same top-level contract
version and an `{ "ok": true, "data": ... }` payload.
For `queryV2Contract`, `data` is the direct `QueryResultV2` object for row
results or direct `ExplainResultV2` object for plan mode; it does not wrap the
Rust `QueryOutcomeV2` enum in another JSON object.

Router-level unmatched paths (`404`), unsupported methods (`405`), and requests
rejected by the global body-size layer can fail before a handler or custom
extractor runs. Axum/Tower may return their transport-level response for those
conditions; they are outside this envelope contract.

| Category | Stable `error.code` | Meaning |
|---|---|---|
| `validation` | `GENESIS_VALIDATION_ERROR` | Request shape or route input rejected |
| `capability` | `GENESIS_CAPABILITY_UNSUPPORTED` | Requested Query IR operation is declared unsupported by the capability manifest |
| `query` | `GENESIS_QUERY_ERROR` | Query evaluation or query execution failed |
| `conflict` | `GENESIS_CONFLICT` | Transaction or relational mutation conflicts with current state |
| `durability` | `GENESIS_DURABILITY_ERROR` | A write requires recovery/reopen before safe continuation |
| `integrity` | `GENESIS_INTEGRITY_ERROR` | Integrity, projection, consensus verification, or rebuild operation failed |
| `internal` | `GENESIS_INTERNAL_ERROR` | Other server-side operation failed |
| `not_found` | `GENESIS_NOT_FOUND` | Requested resource is absent |
| `authorization` | `GENESIS_UNAUTHORIZED` | API key is missing/invalid or an authorization policy denies the operation |
| `rate_limit` | `GENESIS_RATE_LIMITED` | Query admission capacity is exhausted |
| `unavailable` | `GENESIS_UNAVAILABLE` | Engine is rebuilding or temporarily unavailable |

### Six-category conformance matrix

| Category | Core typed source | NAPI contract method | REST representative | Python / Go reader |
|---|---|---|---|---|
| `validation` | Request decoding, relational transaction preflight, `BIND_ERROR` | `queryV2Contract` / `commitTransactionContract` | JSON extractor / Query IR request | Nested stable fields and legacy fallback |
| `capability` | Structured `CAPABILITY_UNSUPPORTED` | `queryV2Contract` | Unsupported Query IR capability | Nested stable fields and legacy fallback |
| `query` | Structured `QUERY_BUDGET_EXCEEDED` | `queryV2Contract` | HQL query budget (reason-only execution error) | Nested stable fields and legacy fallback |
| `conflict` | Explicit transaction identity/frontier checks and SQLite extended primary-key/unique constraint codes from rollback-only relational preflight | `commitTransactionContract` | `/v1/transaction/commit` frontier mismatch, relational collision, or delete/write collision | Nested stable fields and legacy fallback |
| `durability` | Explicit recovery-required / post-WAL state | `commitTransactionContract` | Recovery-required transaction write | Nested stable fields and legacy fallback |
| `integrity` | Structured `DATA_CORRUPTION` | `queryV2Contract` | Consensus verification failure | Nested stable fields and legacy fallback |

The Rust envelope serializer exercises every row with the exact NAPI result
shape; the REST integration suite exercises all six route categories. NAPI
runtime tests cover request validation, query subcode preservation beside the
stable code, transaction conflict, legacy Promise-rejection compatibility, and
success. Python and Go fixtures read every required category while keeping
stable and legacy codes separate.

### Client migration

- New clients should read `error.category`, `error.code`, `error.message`, and
  `error.status`.
- Query IR responses that already returned top-level `{ "code", "message" }` keep
  those fields as deprecated compatibility aliases. Their prior specific codes
  and full diagnostic messages remain unchanged; the new stable category code
  is in `error.code`.
- HQL budget input validation is checked before query execution and retains
  the `QUERY_BUDGET_INVALID` alias with HTTP `400`. Runtime HQL execution
  failures still cross the legacy reason-only Storage boundary, so their full
  diagnostic remains in `message`, while HTTP status and legacy `code` use the
  safe route mapping `500` / `QUERY_EXECUTION_FAILED`. Clients must use
  `error.category = "query"` and `error.code = "GENESIS_QUERY_ERROR"`; the
  reason text is not parsed to recover `QUERY_BUDGET_EXCEEDED`.
- Other routes previously returned plain text. Their exact detail now appears in
  `error.message` and the top-level `message` alias. Clients that display raw
  response text should move to the JSON envelope. The Python SDK reads the new
  envelope and legacy fields; the Go SDK reads envelope code/category/message
  and falls back to legacy fields; Studio's remote transport reads envelope
  messages and still accepts legacy plain-text servers.
- Capability failures are selected from the request shape and the published
  Query IR capability manifest; REST does not infer categories by searching
  diagnostic text.

### SDK error fields

- Python `QueryError.code` prefers nested `error.code`, falling back to the
  deprecated top-level `code`. `stable_code` contains only nested `error.code`,
  `legacy_code` contains only top-level `code`, and `category` contains nested
  `error.category` when present. `message` prefers nested `error.message`, then
  the legacy top-level `message`, then raw response text.
- Go `APIError.Code` uses the same stable-then-legacy preference. `StableCode`,
  `LegacyCode`, and `Category` expose the individual fields; `Message` retains
  the nested diagnostic first, then the legacy top-level message.
- Studio's remote transport throws the envelope's nested or top-level
  `message`, with response-text fallback for older plain-text servers.
- Embedded callers can opt into the additive `Storage::query_v2_contract` /
  `queryV2Contract(accessJson, requestJson)` and
  `Storage::commit_transaction_contract` /
  `commitTransactionContract(transactionJson)` methods. The NAPI methods return
  `{ "contract_version", "ok", "data" | "error" }` objects;
  existing `query_v2`, `commitTransaction`, and other NAPI promise-rejection
  methods retain their old signatures and behavior.
- The query contract maps structured `QueryErrorV2.code` values to category
  and stable code, retaining the former Query IR code at top-level `code` and
  structured safe details at top-level `details`. Transaction contract
  conflict and durability categories come from explicit frontier/payload
  checks and typed recovery state. Query subcodes remain in top-level `code`
  and safe query details remain in top-level `details` during migration.
  Unknown reason-only Storage errors remain `internal`; error messages are
  never parsed to choose a category.
- Transaction relational identifier, row, and primary-key encoding failures
  use `validation`; a missing registered schema or vector collection uses
  `not_found`; embedding dimension mismatch uses `validation`. Primary-key and
  declared unique-index violations are classified from SQLite extended
  constraint codes in a rollback-only projection transaction, including
  ordered delete/write batches. A governance policy rejection uses
  `authorization` and HTTP `403`.

### Taxonomy implementation limit

The REST adapter assigns categories from the typed request shape, route
contract, or typed Storage recovery state. The additive Core/NAPI contract
methods return `ApiErrorEnvelopeV1`; legacy Storage and NAPI methods still
return their original reason-only errors for compatibility. REST handlers that
call those legacy methods therefore retain an explicit route fallback category
for unknown engine failures. Generic mutation failures with HTTP `500` use
`internal`; validation is selected only by explicit request/extractor checks or
route validation. Diagnostics and legacy subcodes remain available, but are
never parsed to select `error.category`. The shared typed contract now covers
the HQL2/Query IR v2 and transaction commit boundaries; upgrading every legacy
NAPI operation to typed errors is a separate compatibility migration.

## Write outcomes and recovery (Wave A)

The core distinguishes rejection from a durable or uncertain write. REST wraps
the unchanged diagnostic in the error envelope; NAPI exposes the same message
through promise rejection. Inspect `error.category` and the message detail rather
than assuming that every non-success status means no write occurred.

| Error prefix / outcome | Client action |
|---|---|
| Unified input/PK/unique/FK rejection | No WAL frame or graph/vector/row publication; correct the payload and retry the same transaction ID |
| `COMMIT_OUTCOME_UNKNOWN` | The request was sent but the WAL acknowledgement failed; reopen and inspect durable state; retry the same identity only for APIs with an idempotency contract |
| `DURABLE_COMMIT_APPLY_FAILED` | The message includes a durable frame sequence; address the underlying error and reopen to replay, rather than issuing a new mutation identity |
| `RECOVERY_REQUIRED` | Query/write/checkpoint access is stopped on this Storage instance; reopen is required |

Superseding a node retains its two-frame history. If its second write fails, the
closing frame is already durable; this is partial durable completion, not full
rejection. Supported query calls see a complete publication boundary, and
concurrent reads serialize. ANN remains eventual until its existing flush or
read-your-write barrier. Diagnostic getters are not a readiness signal during
recovery-required state. See [Wave A](SPEC--WAVE-A-COMMIT-CORRECTNESS.md) for the
exact boundary, tests and performance limitations.

## Routes

| Method | Path | Request body | Response |
|---|---|---|---|
| POST | `/v1/node/add` | `NodeInput` | `NodeOutput` |
| POST | `/v1/node/supersede` | `{ id, new_props?, caused_by? }` | `NodeOutput` |
| POST | `/v1/edge/add` | `EdgeInput` | `EdgeOutput` |
| POST | `/v1/edge/retract` | `{ id, at? }` | `EdgeOutput` (retracted) |
| POST | `/v1/collection/create` | `{ name, model, dim, metric? }` | `{ ok: true }` |
| GET | `/v1/collections` | _none_ | `CollectionInfo[]` |
| POST | `/v1/vector/add` | `{ node_id, collection, embedding }` | `{ ok: true }` |
| POST | `/v1/bulk/nodes` | `NodeInput[]` | `200` (empty) |
| POST | `/v1/bulk/edges` | `EdgeInput[]` | `200` (empty) |
| POST | `/v1/bulk/rebuild` | _none_ | `200` (empty) |
| POST | `/v1/relational/schema/register` | `RelationalSchemaPackage` | current `schema_version` |
| GET | `/v1/relational/schema/:namespace` | _none_ | normalized `RelationalSchemaPackage` or `404` |
| POST | `/v1/relational/mutate` | `RelationalMutationBatch` | `RelationalMutationResult` |
| POST | `/v1/relational/query` | `NamedQueryRequest` | JSON row array |
| POST | `/v1/transaction/commit` | `GenesisTransaction` | `TransactionCommitResult` |
| GET | `/v1/frontier` | _none_ | `{ frame: u64, txn: u64 }` — durable frame and transaction lineage frontiers |
| GET | `/v1/studio/capabilities` | _none_ | negotiated Studio protocol/features/limits |
| GET | `/v1/studio/graph` | query `seed?`, `limit?`, `offset?`, `direction?`, `as_of?` | bounded `StudioGraphScene` without embeddings |
| GET | `/v1/studio/entity/:entity_id` | path id | `StudioEntityInspection` without embeddings |
| POST | `/v1/studio/query/read` | raw JSON string or `{ query }` | read-only HQL result; 256 KiB body ceiling |
| GET | `/v1/studio/relational/schemas` | _none_ | logical `RelationalSchemaPackage[]` |
| POST | `/v1/query/hql` | raw JSON string or `{ query }` | `JSON` (shape depends on command) |
| POST | `/v1/query/ir` | `QueryIrRequestV1` | versioned Query IR response envelope |
| GET | `/v1/query/ir/capabilities` | _none_ | operation-specific Query IR capability manifest |
| POST | `/v1/query` | `QueryInput` | `EdgeOutput[]` |
| POST | `/v1/search/hybrid` | `HybridSearchInput` | `NeighborOutput[]` |
| POST | `/v1/reason/context` | `HybridSearchInput` | `NeighborOutput[]` (alpha forced 0.4) |
| GET | `/v1/insight/drift/:cluster_id` | path `u32` | `SuperNode[]` |
| GET | `/v1/insight/communities` | _none_ | community detection results |
| GET | `/v1/insight/gaps` | _none_ | structural gap analysis |
| POST | `/v1/insight/rebuild` | _none_ | trigger community detection rebuild |
| GET | `/v1/status` | _none_ | `ExtendedStatus` |
| GET | `/v1/version` | _none_ | `{ version }` |
| GET | `/v1/swarm/status` | _none_ | `SwarmStatus` |
| POST | `/v1/consensus/propose` | `{ event: Event, signature: u8[] }` | `String` (proposal id) |
| POST | `/v1/consensus/sign-vote` | `{ proposal_id, approve }` | `u8[]` (ed25519 signature) |
| POST | `/v1/consensus/vote` | `{ proposal_id, peer_id, approve, signature: u8[] }` | `bool` (quorum reached) |
| POST | `/v1/consensus/verify` | `Event` | `bool` |

**Engine capabilities NOT exposed over REST** (NAPI/embedded only) include
`compact`, `set_language_centroid`, `set_index_params`, `reconcile_state`, and
`flush_index`. REST exposes batch/transaction commit, tiered context, index lag,
and bounded graph traversal through their guarded logical routes; it never
exposes a raw SQLite connection or projection file.

## Relational U2 contract

Applications never receive a SQLite handle and cannot submit raw SQL. Schema
registration and mutation events enter the signed Genesis WAL before the staged
SQLite transaction commits. Network reads execute only named queries registered
in the current schema package; the ad hoc `query_relational` method remains an
embedded compatibility API and is not a REST route.

```jsonc
// RelationalMutationBatch
{
  "mutation_id": "UUID",
  "namespace": "app_namespace",
  "schema_version": 1,
  "operations": [{ "table": "notes", "kind": "Upsert", "values": {}, "key": null }]
}

// NamedQueryRequest
{
  "namespace": "app_namespace",
  "schema_version": 1,
  "query_name": "note_by_id",
  "parameters": { "note_id": "note-1" },
  "limit": 10
}
```

`mutation_id` retries are idempotent only when the complete payload is identical;
reuse with another payload returns `REL_MUTATION_CONFLICT`. Schema versions must
advance exactly by one and additive migrations cannot remove or incompatibly
change existing tables, columns, or primary keys.

> **Async vector indexing.** HNSW insertion runs off the write path on a
> dedicated indexing thread — a vector is durable (WAL) and in its collection's
> arena immediately, but **eventually searchable** (the index lags). NAPI exposes
> `flushIndex()` (drain the queue — read-your-write) and `indexLag()` (staged but
> not-yet-indexed count). See `ADR--GENESISDB-ASYNC-INDEXING`.

## Typed Query IR V1

`POST /v1/query/ir` accepts a closed versioned envelope. The current partial implementation supports
`operation.kind = "search"`, `operation.kind = "traverse"` and target-id
`operation.kind = "context"`. Request validation and declared capability failures return HTTP `400`;
query execution failures return HTTP `500`, based on the typed request category. For compatibility,
these query routes retain their historical top-level `code` and `message` fields alongside the
common `error` object. Legacy route-specific codes include `QUERY_IR_VERSION_UNSUPPORTED`,
`QUERY_IR_VALIDATION_FAILED`, `QUERY_CAPABILITY_UNSUPPORTED`, `QUERY_EXECUTION_FAILED`, and
`QUERY_BUDGET_EXCEEDED`; their full diagnostic messages remain unchanged. The top-level Query IR
`code` remains a compatibility-only alias extracted from the legacy diagnostic. It does not select
`error.category`, `error.code`, or HTTP status. New clients should use the stable nested fields and
may continue reading the top-level aliases during migration.

```json
{
  "contract_version": "query-ir.v1",
  "request_id": "client-correlation-id",
  "operation": {
    "kind": "traverse",
    "seed_id": "entity:42",
    "depth": 2,
    "relations": ["depends_on"],
    "direction": "out"
  }
}
```

`search` requires exactly one of `target_id` or `query_vector`, plus `mode` and `k`. Supported modes
are `vector` and `hybrid`; lexical mode remains planned and typed metadata filters are unsupported.
Supplying a `filters` object is recognized and rejected with `QUERY_CAPABILITY_UNSUPPORTED` until a
typed metadata contract exists. This remains the top-level legacy code; the envelope reports
`error.category = "capability"` and `error.code = "GENESIS_CAPABILITY_UNSUPPORTED"`.
`context` is implemented for a target id with `tier` `H0`–`H6`, optional token `budget` and optional
`fuzzy`; query-vector and temporal context fail closed with `QUERY_CAPABILITY_UNSUPPORTED`. Its
`data` is the existing context packet, including `coverage`, and compressed packets carry the
`context_truncated` warning. Unsupported query-vector and temporal context requests preserve the
legacy code and details while the envelope uses the stable capability category. `GET /v1/query/ir/capabilities` is the runtime authority for implemented
operation kinds and current bounds.

## HQL (`/v1/query/hql`, raw string body)

HQL remains a compatibility frontend. New machine integrations should prefer Typed Query IR for
implemented operations.
```
SEARCH <target> SIMILAR TO [v1, v2, …] K <k> [IN <collection>] [LANGUAGE "th"] [AS OF "<rfc3339>"]
TRAVERSE FROM <seed> DEPTH <n> REL <rel|INFER(rel)|ANY> [AS OF "…"]
MATCH <target> SIMILAR TO [v…] ALPHA <a> [IN <collection>] [LANGUAGE "…"] [AS OF "…"]
MATCH (<node>) (<edge> (<node>))* [AS OF "…"] [<clauses>]        # Cypher graph patterns
CONTEXT FOR <target> TIER <H0..H5> [BUDGET <n>]

# Optional trailing <clauses> on SEARCH / TRAVERSE / MATCH (both forms):
  [ WHERE <field> <op> <value> (AND …)* ] [ ORDER BY <field> (ASC|DESC)? ] [ LIMIT <n> ] [ RETURN <field> ("," …)* ]
```
`~` prefix on target/seed enables fuzzy id resolution. `SEARCH` runs pure vector
k-NN (alpha=0); `MATCH <t> SIMILAR` is hybrid (vector + K-Impact, k=10). `IN
<collection>` scopes the query to a named vector collection (quoted `"code"` or
bare `code`); omitted → the `default` collection. The query dim is validated
against the collection dim.

**Cypher graph patterns** (`MATCH (` routes here, not to hybrid): a linear path
`(a:Label {k:v})-[r:REL]->(b) …`. Nodes are `(var? :Label? {props}?)`; edges are
`-[var? :Type?]->` / `<-[…]-` / `-[…]-` (out / in / either). `{id:"…"}` anchors on
a node id. Clause fields are variable-qualified — `a`, `a.id`, `a.label`,
`a.prop.<key>`; `RETURN` omitted ⇒ one object per row keyed by variable. Linear
paths only in v1 (no variable-length `*`, branching, or `OR`). See
`ADR--GENESISDB-HQL-CYPHER-PATTERNS`.

## Data model (from `src/lib.rs`)

### NodeInput
```jsonc
{ "id": "N-1"?, "labels": ["USER"], "props": {}?, "embedding": [f64]?,
  "lang": "en"?, "valid_from": "<rfc3339>"?, "caused_by": "…"?, "ttl": 3600?,
  "collection": "default"? }
```
_(`collection` routes `embedding` into a named vector space; defaults to `default`.)_
### NodeOutput
```jsonc
{ "id", "labels": [], "props": {}, "impact": f64?, "embedding": [f64]?,
  "lang": "en"?, "valid_from", "valid_to": null?, "caused_by": null?,
  "expires_at": null?, "clock": { "time": u32, "peer_id": "…" },
  "collection": "default"? }
```
_(Embedding is omitted from node read responses — the vector lives in its
collection's arena/HNSW. `collection` records which space it lives in.)_

### EdgeInput / EdgeOutput
```jsonc
// EdgeInput
{ "id": "…"?, "from": "N-1", "to": "N-2", "rel": "LINK", "props": {}?,
  "valid_from": "…"?, "supersede": false?, "impact": f64?, "caused_by": "…"? }
// EdgeOutput
{ "id", "from": "N-1", "to": "N-2", "rel", "props", "valid_from",
  "valid_to": null?, "recorded_at", "superseded_by": null?, "impact": f64?,
  "caused_by": null?, "clock": { "time", "peer_id" } }
```
**`from`/`to` are node string ids** (interned to `u32` internally only).

### QueryInput (`/v1/query`)
```jsonc
{ "from": "N-1"?, "to": "N-2"?, "rel": "…"?, "as_of": "…"?,
  "include_invalid": false?, "limit": u32? }
```
### HybridSearchInput (`/v1/search/hybrid`, `/v1/reason/context`)
```jsonc
{ "query_vector": [f64], "k": u32, "alpha": f64?, "lang": "…"?, "as_of": "…"?,
  "collection": "default"? }
```
_(Searches the named collection; query length is validated against the
collection dim — a mismatch is a typed error, not garbage neighbors.)_
### CollectionInfo (`/v1/collections`)
```jsonc
{
  "name", "model", "dim": u32, "metric": "L2|Cosine", "quant",
  "count": u32, "indexed": u32, "ef_search": u32?, "rerank": boolean,
  "sidecar_resident_bytes": i64, "sidecar_disk_bytes": i64,
  "arena_resident_bytes": i64, "index_lag": u32,
  "coverage": {
    "collection", "state": "UNVERIFIED|CATCHING_UP|READY|FAILED",
    "source_count": u32, "indexed_count": u32, "missing_count": u32,
    "extra_count": u32, "pending_count": u32,
    "source_frontier": i64, "built_frontier": i64, "validated": boolean
  }
}
```
_(Create with `POST /v1/collection/create` `{ name, model, dim, metric? }`;
`metric` defaults to `L2`. A `default` collection always exists. `coverage`
is structural source-to-HNSW membership only; `READY` is not an exactness or
ANN-recall claim. The core validation operation is explicit and is not yet a
REST maintenance route.)_
### Attach a vector to a node (`POST /v1/vector/add`)
```jsonc
{ "node_id": "N-1", "collection": "code", "embedding": [f64] }
```
_(Attaches an ADDITIONAL vector to an existing node in another collection — one
node, one vector per collection, e.g. a `code` and a `text` embedding. The node
must exist; `embedding` length is validated against the collection dim. Durable
via WAL `Event::Vector`; eventually searchable. NAPI: `addVector`.)_
### NeighborOutput
```jsonc
{ "node": NodeOutput, "path": [EdgeOutput], "depth": u32 }
```
### ExtendedStatus (`/v1/status`)
```jsonc
{ "open", "read_only", "page_cache_mb", "node_count", "edge_count", "memory_usage_mb" }
```
### SwarmStatus (`/v1/swarm/status`)
```jsonc
{ "peer_id", "logical_clock": u32, "peers": [SyncPeer] }
```

## Governance
Tiers `MASTER` (0) / `SPEC` (1) / `ADR` (2) / `USER` (3), derived from node
`labels`. External callers cannot create/modify `MASTER`-tier nodes (→ `403`-class
error); MASTER promotion requires multi-signature consensus. Guard cost is
<0.1% of a write (audit P24).

**Consensus votes are signed.** A voter signs `VOTE|{proposal_id}|{peer_id}|{approve}`
with its ed25519 key (`/v1/consensus/sign-vote`); `submit_vote` verifies the
signature against the voter's registered public key (`SyncPeer.verifying_key`, or
this node's own key for a self-vote) before counting it. Unknown-peer, malformed,
or non-matching signatures are rejected (`400`) and not counted — forged or
replayed votes cannot reach quorum. See `ADR--GENESISDB-CONSENSUS-VOTE-SIGNATURES`.
