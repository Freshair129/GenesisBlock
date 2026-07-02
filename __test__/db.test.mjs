// Direct NAPI surface tests — exercises GenesisDatabase (the async wrapper over
// Storage) to verify that key invariants hold at the JavaScript boundary.

import test from 'node:test';
import assert from 'node:assert';
import os from 'node:os';
import fs from 'node:fs';
import path from 'node:path';
import { GenesisDatabase } from '../index.js';

function tempDb(suffix) {
  return path.join(os.tmpdir(), `genesis-napi-test-${suffix}-${process.pid}`);
}

// Best-effort temp-dir removal. On Windows the in-process WAL writer + indexing
// threads keep the DB files open for the lifetime of the GenesisDatabase (their
// handles are only released when it is GC'd), so an immediate rmSync raises
// ENOTEMPTY/EBUSY/EPERM. The assertions are what we care about; the OS reclaims
// the temp dir, so we swallow those lock errors and let other errors surface.
function cleanup(dbPath) {
  try {
    fs.rmSync(dbPath, { recursive: true, force: true });
  } catch (err) {
    if (!['ENOTEMPTY', 'EBUSY', 'EPERM', 'EACCES'].includes(err.code)) throw err;
  }
}

// ---------------------------------------------------------------------------
// Read-your-write: add a node with a vector, flush the HNSW index, then
// hybridSearch must return that node as the nearest neighbour.
// ---------------------------------------------------------------------------
test('NAPI: read-your-write — addNode + flushIndex + hybridSearch', async () => {
  const dbPath = tempDb('ryw');
  try {
    const db = GenesisDatabase.open({ path: dbPath, vectorDim: 3 });
    await db.addNode({ id: 'v1', labels: ['Vec'], embedding: [1.0, 0.0, 0.0] });
    await db.flushIndex();
    const results = await db.hybridSearch({ queryVector: [0.9, 0.1, 0.0], k: 1, alpha: 0.0 });
    assert.strictEqual(results.length, 1, 'search must return exactly one result');
    assert.strictEqual(results[0].node.id, 'v1', 'nearest neighbour must be the inserted node');
  } finally {
    cleanup(dbPath);
  }
});

// ---------------------------------------------------------------------------
// P1b: the per-query `oversample` knob must pass through the NAPI hybridSearch
// surface and still return the true nearest on a quantized+rerank collection
// (where it widens the exact rerank pool).
// ---------------------------------------------------------------------------
test('NAPI: hybridSearch accepts oversample on a quantized+rerank collection', async () => {
  const dbPath = tempDb('oversample');
  try {
    const db = GenesisDatabase.open({ path: dbPath, vectorDim: 3 });
    // name, model, dim, metric, quant, efSearch, rerank
    await db.createCollection('ovq', 'm', 4, 'L2', 'sq8', null, true);
    await db.addNode({ id: 'o1', labels: [], embedding: [1.0, 0.0, 0.0, 0.0], collection: 'ovq' });
    await db.addNode({ id: 'o2', labels: [], embedding: [0.0, 1.0, 0.0, 0.0], collection: 'ovq' });
    await db.addNode({ id: 'o3', labels: [], embedding: [0.0, 0.0, 1.0, 0.0], collection: 'ovq' });
    await db.flushIndex();

    const results = await db.hybridSearch({
      queryVector: [0.9, 0.1, 0.0, 0.0],
      k: 1,
      alpha: 0.0,
      collection: 'ovq',
      oversample: 16,
    });
    assert.strictEqual(results.length, 1, 'oversample search must return exactly one result');
    assert.strictEqual(results[0].node.id, 'o1', 'widened oversample must still surface the true nearest');
  } finally {
    cleanup(dbPath);
  }
});

// ---------------------------------------------------------------------------
// Index lag drops to zero after flushIndex — confirms the async indexing
// queue is drained and the synchronous lag counter reflects that.
// ---------------------------------------------------------------------------
test('NAPI: indexLag is zero after flushIndex', async () => {
  const dbPath = tempDb('lag');
  try {
    const db = GenesisDatabase.open({ path: dbPath, vectorDim: 3 });
    for (let i = 0; i < 20; i++) {
      await db.addNode({ id: `n${i}`, labels: [], embedding: [i, 0.0, 0.0] });
    }
    await db.flushIndex();
    assert.strictEqual(db.indexLag(), 0, 'lag must be zero after flush');
  } finally {
    cleanup(dbPath);
  }
});

// ---------------------------------------------------------------------------
// Vector dimension mismatch: adding a node whose embedding length differs from
// the collection dim must reject with an error, not silently corrupt the index.
// ---------------------------------------------------------------------------
test('NAPI: wrong-dim embedding rejects cleanly', async () => {
  const dbPath = tempDb('dim-mismatch');
  try {
    const db = GenesisDatabase.open({ path: dbPath, vectorDim: 3 });
    let threw = false;
    try {
      // Default collection has dim 3; this embedding has dim 4.
      await db.addNode({ id: 'bad', labels: [], embedding: [1.0, 0.0, 0.0, 0.0] });
    } catch {
      threw = true;
    }
    assert.ok(threw, 'wrong-dim vector must throw an error');
  } finally {
    cleanup(dbPath);
  }
});

// ---------------------------------------------------------------------------
// Collection isolation: a node inserted into collection A must NOT appear in
// a hybridSearch scoped to collection B.
// ---------------------------------------------------------------------------
test('NAPI: collection isolation — node in alpha invisible from beta search', async () => {
  const dbPath = tempDb('coll-iso');
  try {
    const db = GenesisDatabase.open({ path: dbPath });
    await db.createCollection('alpha', 'test-model', 3, 'L2');
    await db.createCollection('beta',  'test-model', 3, 'L2');

    await db.addNode({ id: 'alpha-node', labels: ['A'], embedding: [1.0, 0.0, 0.0], collection: 'alpha' });
    // Seed beta with an orthogonal vector so its HNSW index is initialized.
    await db.addNode({ id: 'beta-node', labels: ['B'], embedding: [0.0, 0.0, 1.0], collection: 'beta' });
    await db.flushIndex();

    // Querying [1,0,0] in beta: alpha-node MUST NOT appear; beta-node may.
    const results = await db.hybridSearch({ queryVector: [1.0, 0.0, 0.0], k: 5, alpha: 0.0, collection: 'beta' });
    const ids = results.map(r => r.node.id);
    assert.ok(!ids.includes('alpha-node'),
      `alpha-node must not appear in beta search; got: ${JSON.stringify(ids)}`);
  } finally {
    cleanup(dbPath);
  }
});

// ---------------------------------------------------------------------------
// Edge + node WAL durability: addNode + addEdge, then saveState and reopen;
// the edge must still be traversable after reload.
// ---------------------------------------------------------------------------
test('NAPI: edge survives saveState + reopen', async () => {
  const dbPath = tempDb('edge-persist');
  try {
    {
      const db = GenesisDatabase.open({ path: dbPath });
      await db.addNode({ id: 'src', labels: [] });
      await db.addNode({ id: 'dst', labels: [] });
      await db.addEdge({ from: 'src', to: 'dst', rel: 'LINKS' });
      await db.saveState();
    }
    {
      const db = GenesisDatabase.open({ path: dbPath });
      const neighbors = await db.neighbors('src', { depth: 1, rel: 'LINKS', direction: 'out' });
      assert.strictEqual(neighbors.length, 1, 'edge must survive saveState + reopen');
      assert.strictEqual(neighbors[0].node.id, 'dst');
    }
  } finally {
    cleanup(dbPath);
  }
});

// ---------------------------------------------------------------------------
// P2c: NAPI/REST parity — listCollections() exposes the same per-collection
// quant ops fields the REST /v1/status collections array carries. For a
// quantized+rerank collection the sidecar is on-disk (post-P0), so
// sidecarResidentBytes must be 0 (proves the RAM win); sidecarDiskBytes shows
// where the bytes went; indexLag is the engine-global backlog (also on
// db.indexLag()).
// ---------------------------------------------------------------------------
test('NAPI: listCollections exposes quant ops (sidecar resident bytes ~0, index_lag)', async () => {
  const dbPath = tempDb('quant-ops');
  try {
    const db = GenesisDatabase.open({ path: dbPath });
    // name, model, dim, metric, quant, efSearch, rerank
    await db.createCollection('quantized_rerank', 'bge-m3', 8, 'L2', 'sq8', null, true);
    await db.addNode({
      id: 'n1',
      labels: [],
      embedding: [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8],
      collection: 'quantized_rerank',
    });
    await db.flushIndex();

    const cols = db.listCollections();
    const entry = cols.find(c => c.name === 'quantized_rerank');
    assert.ok(entry, 'quantized_rerank must appear in listCollections()');

    assert.strictEqual(entry.quant, 'sq8', 'quant must be reported');
    // On-disk sidecar ⇒ 0 resident bytes (the P0 RAM win).
    assert.strictEqual(entry.sidecarResidentBytes, 0,
      'on-disk sidecar must report 0 resident bytes');
    // 1 row * dim 8 * 4B = 32 on-disk bytes.
    assert.strictEqual(entry.sidecarDiskBytes, 32,
      'sidecarDiskBytes must reflect on-disk row bytes');
    assert.strictEqual(typeof entry.arenaResidentBytes, 'number',
      'arenaResidentBytes must be numeric');
    assert.strictEqual(typeof entry.indexLag, 'number',
      'per-collection indexLag must be numeric');
    // Parity with the dedicated global accessor.
    assert.strictEqual(entry.indexLag, db.indexLag(),
      'per-collection indexLag must equal the engine-global db.indexLag()');
  } finally {
    cleanup(dbPath);
  }
});
