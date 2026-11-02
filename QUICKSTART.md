# GenesisBlockDB — 5-minute Quickstart (Node.js)

GenesisBlockDB is an **embedded** hybrid graph + vector engine — no server to run,
no container, no network. You `require` the native addon and call it in-process,
the same way you'd use SQLite.

## 1. Install

```bash
npm install @freshair129/gks-genesis-block-native
```

Registry installations use prebuilt platform binaries. For this source checkout,
run `npm ci` followed by `npm run build:debug` with a Rust toolchain on PATH.
The K-Impact retirement described here requires a newly built addon; older
published binaries retain their previous contract. See the
[retirement and migration decision](docs/adr/ADR--GENESISDB-RETIRE-KIMPACT.md).

## 2. Open a database, add knowledge, search it

```js
// index.js is CommonJS — default-import the module, then destructure.
import binding from '@freshair129/gks-genesis-block-native'
const { GenesisDatabase } = binding

// Embedded: this opens (or creates) an on-disk database at ./agent-memory.
// A `default` vector collection always exists.
const db = GenesisDatabase.open({
  path: './agent-memory',
  vectorDim: 4,        // your embedding dimension (e.g. 1024 for bge-m3)
})

// Add two nodes, each carrying an embedding. The vector is staged durably and
// indexed asynchronously (HNSW).
await db.addNode({ id: 'doc:cats', labels: ['Doc'], embedding: [1, 0, 0, 0] })
await db.addNode({ id: 'doc:dogs', labels: ['Doc'], embedding: [0, 1, 0, 0] })

// Connect them in the graph.
await db.addEdge({ from: 'doc:cats', to: 'doc:dogs', rel: 'RELATED_TO' })

// HNSW indexing is async; flush for read-your-write (or poll indexLag()).
await db.flushIndex()

// Vector k-NN search, ranked by vector relevance.
const hits = await db.hybridSearch({ queryVector: [0.9, 0.1, 0, 0], k: 2 })
console.log(hits.map(h => h.node.id))   // → [ 'doc:cats', 'doc:dogs' ]

// Or query with HQL (search / traverse / match / context).
const traversed = await db.executeHql('TRAVERSE FROM "doc:cats" DEPTH 1 REL ANY')
console.log(traversed)

// Persist an instant-load snapshot (also happens on clean shutdown).
await db.saveState()
```

## 3. What you just used

| Capability | Call |
|---|---|
| Embedded open / snapshot | `GenesisDatabase.open(opts)`, `saveState()` |
| Durable node/edge ingest | `addNode()`, `addEdge()` |
| Vector nearest-neighbor search | `hybridSearch({ queryVector, k })` |
| Query language (HQL) | `executeHql('SEARCH … | TRAVERSE … | MATCH … | CONTEXT …')` |
| Async index control | `flushIndex()`, `indexLag()` |

## Next steps

- **Per-collection vector spaces** — route different embedding models to
  isolated collections with `createCollection(name, model, dim, metric)`.
- **Bitemporal queries** — every node/edge has `valid_from`/`valid_to`; pass
  `asOf` to query the graph as it was at a past instant.
- **REST instead of in-process** — `cargo run --features bins --bin genesis-db-server` exposes
  the same engine under `/v1/*` on port 3000.
- **Performance** — see the [benchmark page](docs/index.html) and the
  [competitive report](docs/REPORT--2026-06-21-PERFORMANCE-AND-COMPETITIVE.md).
- **Positioning** — why an embedded graph+vector engine for agent memory:
  [POSITIONING](docs/POSITIONING.md).
