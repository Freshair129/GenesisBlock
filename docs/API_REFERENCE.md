# GenesisBlockDB REST API Reference

**Generated from `src/main.rs` (Axum server) — 2026-06-22.** This replaces the
prior corrupted file. The server is the SSOT; update this when routes change.

- **Base URL:** `http://localhost:3000` (port via `GENESIS_PORT`, bind `0.0.0.0`)
- **Data dir:** `.brain/gks/storage` (via `GENESIS_DATA_DIR`)
- **Bodies:** JSON. **Errors:** `500 Internal Server Error` with a plain-text
  message. CORS: permissive.
- **Run:** `cargo run --bin genesis-db-server`

> ⚠️ Two contract gotchas that have bitten SDKs:
> 1. `POST /v1/query/hql` takes a **raw JSON string** body (e.g. `"SEARCH …"`),
>    **not** `{"query":"…"}`.
> 2. Edge `from`/`to` are **node string ids** (e.g. `"N-…"`), not integers.

## Routes

| Method | Path | Request body | Response |
|---|---|---|---|
| POST | `/v1/node/add` | `NodeInput` | `NodeOutput` |
| POST | `/v1/node/supersede` | `{ id, new_props?, caused_by? }` | `NodeOutput` |
| POST | `/v1/edge/add` | `EdgeInput` | `EdgeOutput` |
| POST | `/v1/collection/create` | `{ name, model, dim, metric? }` | `{ ok: true }` |
| GET | `/v1/collections` | _none_ | `CollectionInfo[]` |
| POST | `/v1/bulk/nodes` | `NodeInput[]` | `200` (empty) |
| POST | `/v1/bulk/edges` | `EdgeInput[]` | `200` (empty) |
| POST | `/v1/bulk/rebuild` | _none_ | `200` (empty) |
| POST | `/v1/query/hql` | **raw JSON string** (HQL) | `JSON` (shape depends on command) |
| POST | `/v1/query` | `QueryInput` | `EdgeOutput[]` |
| POST | `/v1/search/hybrid` | `HybridSearchInput` | `NeighborOutput[]` |
| POST | `/v1/reason/context` | `HybridSearchInput` | `NeighborOutput[]` (alpha forced 0.4) |
| GET | `/v1/insight/drift/:cluster_id` | path `u32` | `SuperNode[]` |
| GET | `/v1/status` | _none_ | `ExtendedStatus` |
| GET | `/v1/swarm/status` | _none_ | `SwarmStatus` |
| POST | `/v1/consensus/propose` | `{ event: Event, signature: u8[] }` | `String` (proposal id) |
| POST | `/v1/consensus/vote` | `{ proposal_id, peer_id, approve }` | `bool` (quorum reached) |
| POST | `/v1/consensus/verify` | `Event` | `bool` |

**Engine capabilities NOT exposed over REST** (NAPI/embedded only): `execute_batch`,
tiered `retrieve_context` / HQL `CONTEXT` end-to-end, `neighbors` (graph
traversal), `retract_edge`, `compact`, `detect_communities`,
`generate_meta_graph`, `calculate_structural_gaps`, `set_language_centroid`,
`set_index_params`, `reconcile_state`, `flush_index`, `index_lag`.

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
CONTEXT FOR <target> TIER <H0..H5> [BUDGET <n>]
```
`~` prefix on target/seed enables fuzzy id resolution. `SEARCH` runs pure vector
k-NN (alpha=0); `MATCH` is hybrid (vector + K-Impact, k=10). `IN <collection>`
scopes the query to a named vector collection (quoted `"code"` or bare `code`);
omitted → the `default` collection. The query dim is validated against the
collection dim.

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
