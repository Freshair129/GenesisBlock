import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { GenesisDatabase } from '../index.js';

function tempDb(suffix) {
  return path.join(os.tmpdir(), `genesis-api-error-${suffix}-${process.pid}`);
}

function cleanup(dbPath) {
  try {
    fs.rmSync(dbPath, { recursive: true, force: true });
  } catch (error) {
    if (!['ENOTEMPTY', 'EBUSY', 'EPERM', 'EACCES'].includes(error.code)) throw error;
  }
}

test('NAPI typed query contract returns versioned validation envelopes and success data', async () => {
  const dbPath = tempDb('query');
  try {
    const db = GenesisDatabase.open({ path: dbPath, vectorDim: 2 });
    const malformed = await db.queryV2Contract(
      JSON.stringify({ principal: 'api-error-test', namespace: 'default' }),
      '{',
    );
    assert.equal(malformed.contract_version, 'genesis.api.errors.v1');
    assert.equal(malformed.ok, false);
    assert.equal(malformed.error.category, 'validation');
    assert.equal(malformed.error.code, 'GENESIS_VALIDATION_ERROR');
    assert.ok(malformed.error.message.length > 0);

    const success = await db.queryV2Contract(
      JSON.stringify({ principal: 'api-error-test', namespace: 'default' }),
      JSON.stringify({
        contract_version: 'genesis.api.v2',
        request_id: 'api-error-success',
        namespace: 'default',
        language_version: 'hql.v2',
        hql: 'USE default FROM NODES AS n |> RETURN n.id AS id',
        params: {},
      }),
    );
    assert.equal(success.contract_version, 'genesis.api.errors.v1');
    assert.equal(success.ok, true, JSON.stringify(success));
    assert.equal(success.data.request_id, 'api-error-success');
    assert.ok(Array.isArray(success.data.rows));
  } finally {
    cleanup(dbPath);
  }
});

test('NAPI query contract preserves the structured legacy code beside the stable code', async () => {
  const dbPath = tempDb('query-legacy-code');
  try {
    const db = GenesisDatabase.open({ path: dbPath, vectorDim: 2 });
    const result = await db.queryV2Contract(
      JSON.stringify({ principal: 'api-error-test', namespace: 'default' }),
      JSON.stringify({
        contract_version: 'genesis.api.v3',
        request_id: 'api-error-legacy-code',
        namespace: 'default',
        language_version: 'hql.v2',
        hql: 'MATCH (n) RETURN n.id',
        params: {},
      }),
    );

    assert.equal(result.contract_version, 'genesis.api.errors.v1');
    assert.equal(result.ok, false);
    assert.equal(result.error.category, 'validation');
    assert.equal(result.error.code, 'GENESIS_VALIDATION_ERROR');
    assert.equal(result.error.legacy_code, 'VERSION_UNSUPPORTED');
    assert.equal(result.code, 'VERSION_UNSUPPORTED');
    assert.ok(result.error.message.length > 0);
  } finally {
    cleanup(dbPath);
  }
});

test('NAPI transaction contract returns stable conflict while legacy commit remains rejected', async () => {
  const dbPath = tempDb('transaction');
  try {
    const db = GenesisDatabase.open({ path: dbPath, vectorDim: 2 });
    const transaction = {
      transaction_id: 'api-error-conflict',
      expected_frontier: 1,
      relational: [],
      graph: { nodes: [], edges: [] },
      vectors: [],
    };
    const encoded = JSON.stringify(transaction);

    const typed = await db.commitTransactionContract(encoded);
    assert.equal(typed.contract_version, 'genesis.api.errors.v1');
    assert.equal(typed.ok, false);
    assert.equal(typed.error.category, 'conflict');
    assert.equal(typed.error.code, 'GENESIS_CONFLICT');
    assert.equal(typed.error.message, 'expected frontier conflict');

    await assert.rejects(db.commitTransaction(encoded), /expected frontier conflict/);
  } finally {
    cleanup(dbPath);
  }
});
