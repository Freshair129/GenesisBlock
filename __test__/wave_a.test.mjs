import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { GenesisDatabase } from '../index.js';

test('NAPI rejected unified constraints preserve frontier and allow corrected retry', async () => {
  const dbPath = fs.mkdtempSync(path.join(os.tmpdir(), 'genesis-wave-a-'));
  const db = GenesisDatabase.open({ path: dbPath, vectorDim: 2 });
  await db.registerRelationalSchema(JSON.stringify({
    namespace: 'audit', schema_version: 1, previous_version: null,
    package_id: '00000000-0000-4000-8000-000000000002', schema_hash: '', named_queries: [],
    tables: [{ name: 'items', columns: [{ name: 'id', column_type: 'Text', nullable: false }],
      primary_key: ['id'], foreign_keys: [], indexes: [] }],
  }));
  const before = db.stableFrontier();
  const row = { table: 'items', kind: 'Insert', values: { id: 'a' } };
  const request = { transaction_id: 'napi-retry', relational: [{ namespace: 'audit', mutations: [row, row] }],
    graph: { nodes: [{ id: 'mixed', labels: [], embedding: [1, 0] }], edges: [] }, vectors: [] };
  await assert.rejects(db.commitTransaction(JSON.stringify(request)), /UNIQUE/);
  assert.equal(db.stableFrontier(), before);
  request.relational[0].mutations.pop();
  const result = JSON.parse(await db.commitTransaction(JSON.stringify(request)));
  assert.equal(result.commit_sequence, before + 1);
  assert.equal(JSON.parse(await db.commitTransaction(JSON.stringify(request))).commit_sequence, result.commit_sequence);
  // The binding owns live worker threads until GC/process exit; retain this
  // unique temp directory rather than deleting an open database on Windows.
});
