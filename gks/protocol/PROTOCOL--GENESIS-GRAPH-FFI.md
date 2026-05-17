---
id: PROTOCOL--GENESIS-GRAPH-FFI
phase: 2
type: protocol
status: draft
vault_id: default
tier: process
source_type: axiomatic
title: PROTOCOL — Genesis Block FFI contract (Node ↔ Rust)
tags: &a1
  - msp
  - gks
  - genesis-block
  - ffi
  - napi
  - rust
  - protocol
  - contract
crosslinks: &a2
  references:
    - ADR--GENESIS-GRAPH-AS-GKS-BACKEND
    - CONCEPT--GENESIS-GRAPH-BACKEND
    - BLUEPRINT--GENESIS-GRAPH-INTEGRATION
created_at: 2026-05-12T12:30:00.000+07:00
aliases: &a3
  - PROTOCOL
  - agent_governance
  - Interaction contract
cluster: agent_governance
role: Interaction contract
attributes:
  id: PROTOCOL--GENESIS-GRAPH-FFI
  phase: 2
  type: protocol
  status: draft
  vault_id: default
  tier: process
  source_type: axiomatic
  title: PROTOCOL — Genesis Block FFI contract (Node ↔ Rust)
  tags: *a1
  crosslinks: *a2
  created_at: 2026-05-12T12:30:00.000+07:00
  aliases: *a3
  cluster: agent_governance
  role: Interaction contract
  attributes:
    id: PROTOCOL--GENESIS-GRAPH-FFI
    phase: 2
    type: protocol
    status: draft
    vault_id: default
    tier: process
    source_type: axiomatic
    title: PROTOCOL — Genesis Block FFI contract (Node ↔ Rust)
    tags: *a1
    crosslinks: *a2
    created_at: 2026-05-12T12:30:00.000+07:00
    aliases: *a3
    cluster: agent_governance
    role: Interaction contract
    attributes:
      domain: protocol
    domain: protocol
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: true
    secret_type: high_entropy_string
    leak_risk: high
    encryption_level: none
  domain: protocol
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: true
  secret_type: high_entropy_string
  leak_risk: high
  encryption_level: none
---

# PROTOCOL — Genesis Block FFI

## Intent

Pin the **binary interface** between `packages/gks/native/genesis-block/`
(Rust crate compiled via `napi-rs`) and
`packages/gks/src/memory/graph/genesis-graph.ts` (TypeScript adapter
implementing the `GraphBackend` interface).

ABI mismatch across this boundary is a **runtime crash, not a compile
error** — Node loads the `.node` addon dynamically and discovers
signature mismatches by segfaulting. This PROTOCOL exists so the
contract is authoritative in markdown before any code is written;
both `lib.rs` and `genesis-block.ts` MUST match this spec.

## 1. Threading & async model

All public methods on `GenesisDatabase` (the napi-exported class) MUST
return a JS `Promise<T>`, with two exceptions named explicitly in §5.
Implementation in Rust follows this pattern:

```rust
#[napi]
pub async fn add_node(&self, args: NodeInput) -> Result<NodeOutput> {
    let inner = Arc::clone(&self.inner);
    tokio::task::spawn_blocking(move || {
        inner.blocking_write().add_node(args)
    })
    .await
    .map_err(|e| Error::from_reason(format!("join: {e}")))?
    .map_err(|e| Error::from_reason(e.to_string()))
}
```

Why: Node.js's main thread MUST NOT block on graph scans (the engine
is designed for 50k-node / 500k-edge workloads per CONCEPT success
criteria; a synchronous scan could take seconds). `spawn_blocking`
moves heavy work to Tokio's dedicated blocking pool, freeing both the
JS event loop and Tokio's IO worker threads.

## 2. Lifecycle

```
JS                                Rust
──                                ────
GenesisDatabase.open(opts)   ─▶   #[napi(factory)] open(opts: OpenOptions)
                                    │
                                    └─▶ Arc<RwLock<Storage>>
                                          │
                                          ├─ mmap .db
                                          ├─ replay WAL (if any)
                                          └─ allocate page cache
db.flush()                  ─▶    fsync to disk
db.close()                  ─▶    drop locks, fsync, unmap
```

`open` is the only factory entry. The handle is opaque on the JS
side — JS cannot construct a `GenesisDatabase` directly.

## 3. Type marshalling

All structs marked `#[napi(object)]` on the Rust side marshal to
plain JS objects. Field name conversion is napi-rs default
(`snake_case` ↔ `camelCase`); the TS adapter normalises back to
`snake_case` keys before returning to GKS callers (because GKS's
existing `GraphEdge` interface uses `snake_case`).

### 3.1 `OpenOptions`

```ts
interface OpenOptions {
  path: string                  // .db file path, created if absent
  pageCacheMB?: number          // default 64
  readOnly?: boolean            // default false
}
```

### 3.2 `NodeInput` / `NodeOutput`

```ts
interface NodeInput {
  id?: string                   // omitted ⇒ Rust generates stable id
  labels: string[]
  props?: Record<string, unknown>
}

interface NodeOutput {
  id: string                    // always present in output
  labels: string[]
  props: Record<string, unknown>
}
```

Stable-ID generation lives **in Rust** (per the ADR), ensuring the
same input produces the same ID whether called from TS, an MCP tool,
or a future CLI binding directly to the Rust crate.

### 3.3 `EdgeInput` / `EdgeOutput`

```ts
interface EdgeInput {
  id?: string
  from: string
  to: string
  rel: string
  props?: Record<string, unknown>
  validFrom?: string            // ISO 8601; default = engine's now()
  supersede?: boolean           // default false
}

interface EdgeOutput {
  id: string
  from: string
  to: string
  rel: string
  props: Record<string, unknown>
  validFrom: string             // always present
  validTo: string | null        // null ⇒ still valid
  recordedAt: string            // ingestion timestamp
  supersededBy?: string         // present iff this edge was superseded
}
```

Bi-temporal fields (`validFrom`, `validTo`, `recordedAt`) are
**mandatory in the output** and align with GKS's existing `GraphEdge`
shape in `packages/gks/src/memory/graph.ts:39`.

### 3.4 Query inputs

```ts
interface QueryInput {
  from?: string
  to?: string
  rel?: string
  asOf?: string                 // ISO 8601 — temporal projection
  includeInvalid?: boolean      // default false (skip retracted)
  limit?: number
}

interface NeighborInput {
  depth?: number                // default 1
  rel?: string
  direction?: 'out' | 'in' | 'both'   // default 'out'
  asOf?: string
  includeInvalid?: boolean
  limit?: number
}

interface NeighborOutput {
  node: NodeOutput
  path: EdgeOutput[]            // edge sequence from seed
  depth: number
}
```

### 3.5 Cypher (P3.4 — stub in v0)

```ts
interface CypherResultRow {
  [columnName: string]: CypherValue
}

type CypherValue =
  | null | boolean | number | string
  | NodeOutput | EdgeOutput
  | CypherValue[]
  | { [k: string]: CypherValue }
```

In v0 (P3.1 scaffold), `cypher(query, params)` MUST be present in the
FFI surface but MAY return a rejected Promise with `Error('cypher: v0
stub — implementation lands in P3.4')`. This keeps the ABI stable so
P3.4 does not require recompiling JS consumers.

## 4. Async surface (all return `Promise<T>`)

| JS method | Rust `#[napi]` | Behaviour |
|---|---|---|
| `addNode(input)` | `async fn add_node` | Insert node; ID generated if absent |
| `addEdge(input)` | `async fn add_edge` | Insert edge; respect `supersede` flag |
| `retractEdge(id, at?)` | `async fn retract_edge` | Set `valid_to`; return retracted edge or null |
| `query(q?)` | `async fn query` | Filter edges by `(from, to, rel, asOf, …)` |
| `neighbors(seed, q?)` | `async fn neighbors` | BFS/DFS from seed up to `depth` |
| `cypher(q, params?)` | `async fn cypher` | Execute Cypher; v0 stub per §3.5 |
| `flush()` | `async fn flush` | `fsync` outstanding pages |
| `close()` | `async fn close` | Final fsync + drop |

All errors propagate as JS `Error` (`Error::from_reason` on the Rust
side). The error message MUST begin with a stable prefix identifying
the failure domain:

| Prefix | Meaning |
|---|---|
| `genesis-block: parse:` | Cypher parse failure |
| `genesis-block: read-only:` | Write attempted on read-only handle |
| `genesis-block: io:` | Disk / file system error |
| `genesis-block: schema:` | Schema version mismatch (migration needed) |
| `genesis-block: internal:` | Bug — file an issue |

## 5. Sync helpers (the only blocking calls)

Two RAM-only helpers are allowed to be synchronous because their
worst-case runtime is bounded at <1ms:

| JS method | Rust `#[napi]` | Returns |
|---|---|---|
| `statusSync()` | `fn status_sync` | `{ open, readOnly, pageCacheMB }` |
| `schemaVersionSync()` | `fn schema_version_sync` | `number` |

NO other method may be synchronous. Adding a third sync method
requires amending this PROTOCOL.

## 6. Versioning & migration

The first 4 bytes of the `.db` file are the schema version (u32
little-endian). On `open`, Rust reads the version and:

- If `version == CURRENT_VERSION`: proceed.
- If `version < CURRENT_VERSION`: run migrations in
  `migrations/<from>_to_<to>.rs` sequentially, fsync after each.
- If `version > CURRENT_VERSION`: refuse to open with
  `genesis-block: schema: db version N exceeds engine version M`.

`schemaVersionSync()` exposes the current opened-file version so MSP
can warn the user before silent upgrades.

## 7. Concurrency invariants

1. **Single writer per `.db` file** — enforced by `RwLock` on the
   Rust side and by an OS-level file lock at `open` time.
   Concurrent `open` of the same path with `readOnly: false` MUST
   return `genesis-block: io: file locked by pid <N>`.
2. **Multi-reader OK** — multiple `readOnly: true` handles on the
   same `.db` file are permitted and share a process-local page
   cache only within the same Node process.
3. **No tearing across calls** — every `#[napi]` async method takes
   either a read or a write lock for its full duration. A partially
   applied transaction MUST roll back on error before releasing the
   lock.

## 8. Test obligations (referenced from BLUEPRINT P3.2)

The Rust crate MUST ship the following tests at the `cargo test`
level, in addition to the parametrised TS suite:

- `tests/abi_match.rs` — loads the produced `.node` and asserts every
  exported symbol's signature matches this PROTOCOL byte-for-byte.
- `tests/bi_temporal.rs` — round-trips edges across `valid_from /
  valid_to` boundaries; asserts `asOf` projection is correct.
- `tests/single_writer.rs` — opens the same `.db` twice in write
  mode from two processes; asserts the second errors with the
  documented prefix.
- `tests/schema_migration.rs` — opens a fixture `.db` at version N-1
  and asserts the migration to N succeeds and is idempotent.

## 9. What this PROTOCOL does NOT specify

- The internal Rust module layout beyond the public napi surface
  (storage page format, B-tree fanout, etc.). Those are
  implementation details that may change without amending this doc.
- The exact Cypher subset (deferred to BLUEPRINT §"Cypher v0 scope").
- The on-disk file format byte layout (lives in
  `[[BLUEPRINT--GENESIS-GRAPH-INTEGRATION]]` and any future
  `[[ENTITY--GENESIS-BLOCK-FILE-FORMAT]]` atom).

## 10. Amendment policy

Any change to §1–§7 that breaks JS ↔ Rust ABI compatibility requires:

1. A new ADR superseding the relevant section of
   `[[ADR--GENESIS-GRAPH-AS-GKS-BACKEND]]`.
2. A schema version bump per §6.
3. A migration routine in `migrations/`.
4. Reciprocal `supersedes` / `superseded_by` crosslinks per the MSP
   contradiction policy.

Adding a new async method (e.g. `compact()` for vacuum/GC in a later
phase) is **not** breaking and only requires a minor version note in
the BLUEPRINT.

## Source

`[[ADR--GENESIS-GRAPH-AS-GKS-BACKEND]]`, `[[BLUEPRINT--GENESIS-GRAPH-INTEGRATION]]`,
`[[CONCEPT--GENESIS-GRAPH-BACKEND]]`.
