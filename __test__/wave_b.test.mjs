import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { GenesisDatabase } from '../index.js';

test('NAPI collection definition survives child exit without checkpoint', async () => {
  const dbPath = fs.mkdtempSync(path.join(os.tmpdir(), 'genesis-wave-b-crash-'));
  const script = `import { GenesisDatabase } from './index.js';
    const db=GenesisDatabase.open({path:process.env.GENESIS_WAVE_B_PATH,vectorDim:2});
    await db.createCollection('empty','model-x',2,'cosine','f16',123,false);
    process.exit(0);`;
  const child = spawnSync(process.execPath, ['--input-type=module', '-e', script], {
    cwd: path.resolve(import.meta.dirname, '..'), encoding: 'utf8',
    env: { ...process.env, GENESIS_WAVE_B_PATH: dbPath },
  });
  assert.equal(child.status, 0, child.stderr);
  const db = GenesisDatabase.open({ path: dbPath, vectorDim: 2 });
  const collection = (await db.listCollections()).find(c => c.name === 'empty');
  assert.equal(collection.model, 'model-x');
  assert.equal(collection.metric, 'Cosine');
  assert.equal(collection.quant, 'f16');
  assert.equal(collection.efSearch, 123);
  assert.equal(db.queryIrCapabilities().storage_schema_version, 4);
  const before = db.stableFrontier();
  await assert.rejects(db.createCollection('bad', 'm', 65537), /DIM_INVALID/);
  assert.equal(db.stableFrontier(), before);
});

test('NAPI edge rebind preserves transaction-time endpoints', async () => {
  const db = GenesisDatabase.open({ path: fs.mkdtempSync(path.join(os.tmpdir(), 'genesis-wave-b-edge-')), vectorDim: 2, retention: 'full' });
  for (const id of ['A', 'B', 'C', 'D']) await db.addNode({ id, labels: [] });
  await db.addEdge({ id: 'e', from: 'A', to: 'B', rel: 'R' });
  const first = db.stableFrontier();
  await db.addEdge({ id: 'e', from: 'C', to: 'D', rel: 'R' });
  const query = { contract_version: 'query-ir.v1', request_id: 'wave-b',
    operation: { kind: 'traverse', seed_id: 'A', depth: 1, direction: 'out', relations: ['R'] } };
  assert.equal((await db.executeQueryIr(query)).data.length, 0);
  query.temporal = { tx_as_of: first };
  assert.equal((await db.executeQueryIr(query)).data[0].node.id, 'B');
});
