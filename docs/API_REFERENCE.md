---
doc_id: API_REFERENCE
status: current
version: generated
owner: GenesisBlockDB Engineering
---

# GenesisBlockDB REST API Reference

**Generated from `src/router.rs` (Axum server) — 2026-07-22.** This replaces the
prior corrupted file. The server is the SSOT; update this when routes change.

- **Base URL:** `http://localhost:3000` (port via `GENESIS_PORT`, bind `0.0.0.0`)
- **Data dir:** `.brain/gks/storage` (via `GENESIS_DATA_DIR`)
- **Bodies:** JSON. Errors use the route-appropriate HTTP status with a plain-text
  message. CORS defaults to localhost origins; `GENESIS_CORS_ORIGIN` selects one
  origin, while `*` explicitly enables permissive mode.
- **Authentication:** when `GENESIS_API_KEY` is set, all `/v1/*` routes require
  `Authorization: Bearer <key>`. This is a bootstrap shared secret, not scoped
  OIDC/JWT authorization. `/metrics` remains unguarded.
- **Run:** `cargo run --features bins --bin genesis-db-server`

> ⚠️ Two contract gotchas that have bitten SDKs:
> 1. `POST /v1/query/hql` accepts both a **raw JSON string** body (e.g. `"SEARCH …"`)
>    and `{"query":"…"}`. `POST /v1/studio/query/read` accepts the same two shapes
>    but enforces the read-only HQL command family.
> 2. Edge `from`/`to` are **node string ids** (e.g. `"N-…"`), not integers.

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
| GET | `/v1/frontier` | _none_ | stable frontier `u64` |
| GET | `/v1/studio/capabilities` | _none_ | negotiated Studio protocol/features/limits |
| GET | `/v1/studio/graph` | query `seed?`, `limit?`, `offset?`, `direction?`, `as_of?` | bounded `StudioGraphScene` without embeddings |
| GET | `/v1/studio/entity/:entity_id` | path id | `StudioEntityInspection` without embeddings |
| POST | `/v1/studio/query/read` | raw JSON string or `{ query }` | read-only HQL result; 256 KiB body ceiling |
| GET | `/v1/studio/relational/schemas` | _none_ | logical `RelationalSchemaPackage[]` |
| POST | `/v1/query/hql` | raw JSON string or `{ query }` | `JSON` (shape depends on command) |
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

## HQL (`/v1/query/hql`, raw string body)
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
{ "name", "model", "dim": u32, "metric": "L2|Cosine", "count": u32 }
```
_(Create with `POST /v1/collection/create` `{ name, model, dim, metric? }`;
`metric` defaults to `L2`. A `default` collection always exists.)_
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
