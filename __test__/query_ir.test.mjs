import test from 'node:test';
import assert from 'node:assert';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { GenesisDatabase } from '../index.js';

function tempDb(suffix) {
  return path.join(os.tmpdir(), `genesis-query-ir-${suffix}-${process.pid}`);
}

function cleanup(dbPath) {
  try {
    fs.rmSync(dbPath, { recursive: true, force: true });
  } catch (error) {
    if (!['ENOTEMPTY', 'EBUSY', 'EPERM', 'EACCES'].includes(error.code)) throw error;
  }
}

test('NAPI: executeQueryIr supports search and traverse envelopes', async () => {
  const dbPath = tempDb('parity');
  try {
    const db = GenesisDatabase.open({ path: dbPath, vectorDim: 3 });
    const capabilities = db.queryIrCapabilities();
    assert.equal(capabilities.contract_version, 'query-ir.v1');
    assert.equal(capabilities.operations.search, 'implemented');
    assert.equal(capabilities.operations.traverse, 'implemented');
    await db.addNode({ id: 'query-ir-napi-src', labels: [], embedding: [1.0, 0.0, 0.0] });
    await db.addNode({ id: 'query-ir-napi-dst', labels: [] });
    await db.addEdge({
      id: 'query-ir-napi-edge',
      from: 'query-ir-napi-src',
      to: 'query-ir-napi-dst',
      rel: 'KNOWS',
    });
    await db.flushIndex();

    const search = await db.executeQueryIr({
      contract_version: 'query-ir.v1',
      request_id: 'napi-search',
      operation: {
        kind: 'search',
        mode: 'vector',
        query_vector: [1.0, 0.0, 0.0],
        k: 1,
      },
    });
    assert.equal(search.operation_kind, 'search');
    assert.equal(search.data[0].node.id, 'query-ir-napi-src');

    const traverse = await db.executeQueryIr({
      contract_version: 'query-ir.v1',
      request_id: 'napi-traverse',
      operation: {
        kind: 'traverse',
        seed_id: 'query-ir-napi-src',
        depth: 1,
        relations: ['KNOWS'],
        direction: 'out',
      },
    });
    assert.equal(traverse.operation_kind, 'traverse');
    assert.equal(traverse.data[0].node.id, 'query-ir-napi-dst');
  } finally {
    cleanup(dbPath);
  }
});

test('NAPI: executeQueryIr rejects unknown envelope fields', async () => {
  const dbPath = tempDb('validation');
  try {
    const db = GenesisDatabase.open({ path: dbPath, vectorDim: 3 });
    await assert.rejects(
      db.executeQueryIr({
        contract_version: 'query-ir.v1',
        request_id: 'napi-invalid',
        unknown: true,
        operation: {
          kind: 'traverse',
          seed_id: 'missing',
          depth: 1,
          relations: ['KNOWS'],
          direction: 'out',
        },
      }),
      /QUERY_IR_VALIDATION_FAILED/,
    );
  } finally {
    cleanup(dbPath);
  }
});
