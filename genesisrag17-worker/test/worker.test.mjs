import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import {
  GenesisRag17Worker,
  SCHEMA_VERSION,
  STAGE_CATALOG,
  hashObject,
  hashText,
  verifyModelArtifacts,
} from '../src/index.mjs';

const modelDir = 'C:/Users/pc/.cache/huggingface/hub/models--intfloat--multilingual-e5-small/snapshots/614241f622f53c4eeff9890bdc4f31cfecc418b3';
const scope = {
  portfolioId: 'portfolio-test',
  tenantId: 'tenant-test',
  businessId: 'business-test',
  workspaceId: '',
  agentId: '',
  visibility: 'private',
};

function stages(runId) {
  return Object.values(STAGE_CATALOG).map(({ stageNumber, pipelineStageId }) => ({
    runId,
    pipelineStageId,
    executionStepId: `step-${stageNumber}`,
    attemptId: 'attempt-1',
    stageNumber,
  }));
}

function makeDecision() {
  const first = 'Alice works for Acme Ltd.';
  const second = 'Bob purchased Nimbus.';
  const content = `${first}${second}`;
  const source = {
    sourceId: 'source-1',
    rawArtifactId: 'raw-1',
    parsedArtifactId: 'parsed-1',
    documentId: 'doc-1',
    version: '1',
    content,
    contentHash: hashText(content),
  };
  const chunks = [
    { chunkId: 'chunk-1', parsedArtifactId: 'parsed-1', ordinal: 0, text: first, contentHash: hashText(first), startOffset: 0, endOffset: first.length },
    { chunkId: 'chunk-2', parsedArtifactId: 'parsed-1', ordinal: 1, text: second, contentHash: hashText(second), startOffset: first.length, endOffset: content.length },
  ];
  const mentions = [
    { sourceMentionId: 'mention-alice', resolutionKey: 'person:alice', semanticType: 'Person', name: 'Alice', chunkId: 'chunk-1', startOffset: first.indexOf('Alice'), endOffset: first.indexOf('Alice') + 5 },
    { sourceMentionId: 'mention-acme', resolutionKey: 'org:acme', semanticType: 'Organization', name: 'Acme', chunkId: 'chunk-1', startOffset: first.indexOf('Acme'), endOffset: first.indexOf('Acme') + 4 },
    { sourceMentionId: 'mention-bob', resolutionKey: 'person:bob', semanticType: 'Person', name: 'Bob', chunkId: 'chunk-2', startOffset: second.indexOf('Bob'), endOffset: second.indexOf('Bob') + 3 },
    { sourceMentionId: 'mention-nimbus', resolutionKey: 'product:nimbus', semanticType: 'Product', name: 'Nimbus', chunkId: 'chunk-2', startOffset: second.indexOf('Nimbus'), endOffset: second.indexOf('Nimbus') + 6 },
  ];
  const refs1 = { sourceId: source.sourceId, rawArtifactId: source.rawArtifactId, parsedArtifactId: source.parsedArtifactId, chunkId: 'chunk-1', sourceMentionIds: ['mention-alice', 'mention-acme'] };
  const refs2 = { sourceId: source.sourceId, rawArtifactId: source.rawArtifactId, parsedArtifactId: source.parsedArtifactId, chunkId: 'chunk-2', sourceMentionIds: ['mention-bob', 'mention-nimbus'] };
  const runId = 'run-test-1';
  const decision = {
    schemaVersion: SCHEMA_VERSION,
    decisionId: 'decision-1',
    batchId: 'batch-1',
    scope,
    runId,
    stages: stages(runId),
    source,
    chunks,
    mentions,
    entities: [
      { id: 'person-alice', name: 'Alice', semanticType: 'Person', mentions: ['mention-alice'] },
      { id: 'org-acme', name: 'Acme Ltd.', semanticType: 'Organization', mentions: ['mention-acme'] },
      { id: 'person-bob', name: 'Bob', semanticType: 'Person', mentions: ['mention-bob'] },
      { id: 'product-nimbus', name: 'Nimbus', semanticType: 'Product', mentions: ['mention-nimbus'] },
    ],
    facts: [
      { id: 'fact-works', subjectId: 'person-alice', predicate: 'WORKS_FOR', objectId: 'org-acme', confidence: 0.9, sourceReferences: refs1, temporal: { validFrom: 'not_applicable', validTo: 'not_applicable', txFrom: '2026-09-07T00:00:00.000Z', txTo: 'open' } },
      { id: 'fact-purchased', subjectId: 'person-bob', predicate: 'PURCHASED', objectId: 'product-nimbus', confidence: 0.9, sourceReferences: refs2, temporal: { validFrom: 'not_applicable', validTo: 'not_applicable', txFrom: '2026-09-07T00:00:00.000Z', txTo: 'open' } },
    ],
    held: [],
    derived: [{ id: 'derived-alice', kind: 'enrichment', entityId: 'person-alice', count: 1, sourceReferences: [refs1] }],
    policy: { allowEmbedding: true, allowPublication: true },
    ontologyVersion: 'ontology_v1',
    pipelineVersion: SCHEMA_VERSION,
  };
  decision.decisionHash = hashObject(decision);
  return decision;
}

test('worker verifies pinned model artifacts and performs native publish/query', async () => {
  assert.ok(fs.existsSync(modelDir), 'pinned model snapshot must be present for the real integration test');
  const artifactHashes = verifyModelArtifacts(modelDir);
  assert.equal(artifactHashes['onnx/model.onnx'], 'ca456c06b3a9505ddfd9131408916dd79290368331e7d76bb621f1cba6bc8665');
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'genesisrag17-worker-'));
  const dbPath = path.join(root, 'db');
  const decision = makeDecision();
  const fixture = {
    fixtureVersion: 'worker-test-v1',
    queries: [
      { query: 'Which company employs Alice?', relevantTexts: ['Alice works for Acme Ltd.'] },
      { query: 'What product did Bob buy?', relevantTexts: ['Bob purchased Nimbus.'] },
    ],
  };
  let claimed = true;
  let receipt;
  const calls = [];
  const response = (body) => ({ schemaVersion: SCHEMA_VERSION, scope, ...body });
  const mspCall = async (name, args) => {
    calls.push(name);
    if (name === 'msp_pipeline_claim') return response({ decisions: claimed ? [decision] : [] });
    if (name === 'msp_pipeline_write_receipt') {
      receipt = args.receipt;
      return response({ accepted: true, receiptHash: hashObject(receipt) });
    }
    if (name === 'msp_pipeline_graph_receipt') {
      return response({ accepted: true, graphReceiptHash: hashObject(args.receipt), derived: decision.derived, derivedHash: hashObject(decision.derived) });
    }
    if (name === 'msp_pipeline_gate') {
      return response({
        verdict: {
          schemaVersion: SCHEMA_VERSION,
          scope,
          runId: decision.runId,
          decisionId: decision.decisionId,
          decisionHash: decision.decisionHash,
          snapshotId: receipt.snapshotId,
          generation: receipt.generation,
          receiptHash: hashObject(receipt),
          verdict: 'PASS',
          allowPublication: true,
          dimensions: {
            data: { result: 'PASS', critical: false, reasons: [] },
            graph: { result: 'PASS', critical: false, reasons: [] },
            knowledge: { result: 'PASS', critical: false, reasons: [] },
            security: { result: 'PASS', critical: false, reasons: [] },
            retrieval: { result: 'PASS', critical: false, reasons: [] },
          },
        },
      });
    }
    if (name === 'msp_pipeline_publication_receipt') {
      claimed = false;
      return response({ accepted: true });
    }
    throw new Error(`unexpected MSP tool ${name}`);
  };
  const worker = GenesisRag17Worker.create({
    dbPath,
    scope,
    credential: 'worker-credential-test',
    workerToken: 'query-token-test',
    modelDir,
    mspCall,
    benchmarkFixture: fixture,
    port: 0,
  });
  try {
    const result = await worker.runOnce();
    assert.equal(result.status, 'published');
    assert.equal(receipt.model.dimensions, 384);
    assert.equal(receipt.model.revision, '614241f622f53c4eeff9890bdc4f31cfecc418b3');
    assert.equal(receipt.readback.ok, true);
    assert.equal(receipt.laneManifest.vector.status, 'ready');
    assert.equal(receipt.laneManifest.lexical.status, 'ready');
    assert.equal(receipt.laneManifest.graph.status, 'ready');
    assert.equal(receipt.laneManifest.sqlite.status, 'ready');
    assert.equal(receipt.laneManifest.bitemporal.status, 'not_applicable');
    assert.equal(receipt.laneManifest.provenance.status, 'ready');
    assert.equal(receipt.benchmark.queryCount, 2);
    assert.equal(receipt.benchmark.crossTenantLeaks, 0);
    assert.ok(calls.includes('msp_pipeline_write_receipt'));
    assert.ok(calls.includes('msp_pipeline_gate'));
    assert.ok(calls.includes('msp_pipeline_publication_receipt'));

    const endpoint = await worker.listen();
    const response = await fetch(`${endpoint.url}/query`, {
      method: 'POST',
      headers: { authorization: 'Bearer query-token-test', 'content-type': 'application/json' },
      body: JSON.stringify({ schemaVersion: SCHEMA_VERSION, scope, query: 'Which company employs Alice?', topK: 5 }),
    });
    assert.equal(response.status, 200);
    const body = await response.json();
    assert.equal(body.scope.tenantId, scope.tenantId);
    assert.equal(body.generation, result.generation);
    assert.ok(body.results.some((row) => row.text === 'Alice works for Acme Ltd.'));
    assert.equal(body.results[0].citation.sourceId, 'source-1');
    assert.equal(body.results[0].citation.contentHash, hashText(body.results[0].text));
  } finally {
    await worker.close();
    try { fs.rmSync(root, { recursive: true, force: true }); } catch { /* Windows native handles are released at process exit. */ }
  }
});

test('worker rejects a second owner for the same store', async () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'genesisrag17-lock-'));
  const dbPath = path.join(root, 'db');
  const noop = async () => ({ decisions: [] });
  const options = {
    dbPath,
    scope,
    credential: 'worker-credential-test',
    workerToken: 'query-token-test',
    modelDir,
    mspCall: noop,
  };
  const first = GenesisRag17Worker.create(options);
  try {
    assert.throws(() => GenesisRag17Worker.create(options), /WORKER_STORE_ALREADY_OWNED/);
  } finally {
    await first.close();
    try { fs.rmSync(root, { recursive: true, force: true }); } catch { /* native handle cleanup is process scoped */ }
  }
});

test('worker records embedding policy denial as an actual Stage15 failure', async () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'genesisrag17-failure-'));
  const dbPath = path.join(root, 'db');
  const decision = makeDecision();
  decision.policy.allowEmbedding = false;
  const hashable = { ...decision };
  delete hashable.decisionHash;
  decision.decisionHash = hashObject(hashable);
  let claimed = true;
  const failures = [];
  const calls = [];
  const response = (body) => ({ schemaVersion: SCHEMA_VERSION, scope, ...body });
  const mspCall = async (name, args) => {
    calls.push(name);
    if (name === 'msp_pipeline_claim') return response({ decisions: claimed ? [decision] : [] });
    if (name === 'msp_pipeline_graph_receipt') {
      return response({ accepted: true, graphReceiptHash: hashObject(args.receipt), derived: decision.derived, derivedHash: hashObject(decision.derived) });
    }
    if (name === 'msp_pipeline_stage_failure') {
      failures.push(args);
      claimed = false;
      return response({ accepted: true });
    }
    throw new Error(`unexpected MSP tool ${name}`);
  };
  const worker = GenesisRag17Worker.create({
    dbPath,
    scope,
    credential: 'worker-credential-test',
    workerToken: 'query-token-test',
    modelDir,
    mspCall,
  });
  try {
    const result = await worker.runOnce();
    assert.equal(result.status, 'failed');
    assert.equal(result.stageNumber, 15);
    assert.equal(failures.length, 1);
    assert.equal(failures[0].stage.stageNumber, 15);
    assert.equal(failures[0].metrics.records_in, decision.chunks.length);
    assert.equal(failures[0].metrics.records_out, 0);
    assert.equal(failures[0].error.code, 'EMBEDDING_POLICY_DENIED');
    assert.equal(worker.state.decisions[decision.decisionId].status, 'failed');
    assert.ok(calls.includes('msp_pipeline_stage_failure'));
    assert.ok(!calls.includes('msp_pipeline_write_receipt'));
    assert.ok(!calls.includes('msp_pipeline_gate'));
  } finally {
    await worker.close();
    try { fs.rmSync(root, { recursive: true, force: true }); } catch { /* native handle cleanup is process scoped */ }
  }
});

test('worker retries the exact persisted stage failure after a lost reply', async () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'genesisrag17-failure-retry-'));
  const dbPath = path.join(root, 'db');
  const decision = makeDecision();
  decision.policy.allowEmbedding = false;
  const hashable = { ...decision };
  delete hashable.decisionHash;
  decision.decisionHash = hashObject(hashable);
  let claimed = true;
  let failureCall = 0;
  let firstFailure;
  const response = (body) => ({ schemaVersion: SCHEMA_VERSION, scope, ...body });
  const mspCall = async (name, args) => {
    if (name === 'msp_pipeline_claim') return response({ decisions: claimed ? [decision] : [] });
    if (name === 'msp_pipeline_graph_receipt') {
      return response({ accepted: true, graphReceiptHash: hashObject(args.receipt), derived: decision.derived, derivedHash: hashObject(decision.derived) });
    }
    if (name === 'msp_pipeline_stage_failure') {
      failureCall += 1;
      if (failureCall === 1) {
        firstFailure = structuredClone(args);
        throw new Error('STAGE_FAILURE_REPLY_LOST');
      }
      assert.deepEqual(args, firstFailure);
      claimed = false;
      const normalizedFailure = { ...args };
      delete normalizedFailure.credential;
      return response({ accepted: true, failureHash: hashObject(normalizedFailure) });
    }
    throw new Error(`unexpected MSP tool ${name}`);
  };
  const worker = GenesisRag17Worker.create({
    dbPath,
    scope,
    credential: 'worker-credential-test',
    workerToken: 'query-token-test',
    modelDir,
    mspCall,
  });
  try {
    await assert.rejects(worker.runOnce(), /STAGE_FAILURE_REPLY_LOST/);
    const saved = structuredClone(worker.state.decisions[decision.decisionId].stageFailure);
    const savedHash = worker.state.decisions[decision.decisionId].stageFailureHash;
    assert.equal(worker.state.decisions[decision.decisionId].status, 'stage_failure_pending');
    assert.equal((await worker.runOnce()).status, 'idle');
    assert.equal(failureCall, 2);
    assert.equal(worker.state.decisions[decision.decisionId].status, 'failed');
    assert.deepEqual(worker.state.decisions[decision.decisionId].stageFailure, saved);
    assert.equal(worker.state.decisions[decision.decisionId].stageFailureHash, savedHash);
  } finally {
    await worker.close();
    try { fs.rmSync(root, { recursive: true, force: true }); } catch { /* native handle cleanup is process scoped */ }
  }
});

test('worker resumes after pointer replacement without rewriting the snapshot', async () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'genesisrag17-pointer-recovery-'));
  const dbPath = path.join(root, 'db');
  const decision = makeDecision();
  const fixture = {
    fixtureVersion: 'worker-test-v1',
    queries: [
      { query: 'Which company employs Alice?', relevantTexts: ['Alice works for Acme Ltd.'] },
      { query: 'What product did Bob buy?', relevantTexts: ['Bob purchased Nimbus.'] },
    ],
  };
  let claimed = true;
  let writeReceipt;
  let publicationReceipt;
  const response = (body) => ({ schemaVersion: SCHEMA_VERSION, scope, ...body });
  const mspCall = async (name, args) => {
    if (name === 'msp_pipeline_claim') return response({ decisions: claimed ? [decision] : [] });
    if (name === 'msp_pipeline_graph_receipt') {
      return response({ accepted: true, graphReceiptHash: hashObject(args.receipt), derived: decision.derived, derivedHash: hashObject(decision.derived) });
    }
    if (name === 'msp_pipeline_write_receipt') {
      writeReceipt = args.receipt;
      return response({ accepted: true, receiptHash: hashObject(args.receipt) });
    }
    if (name === 'msp_pipeline_gate') {
      return response({ verdict: {
        schemaVersion: SCHEMA_VERSION,
        scope,
        runId: decision.runId,
        decisionId: decision.decisionId,
        decisionHash: decision.decisionHash,
        snapshotId: writeReceipt.snapshotId,
        generation: writeReceipt.generation,
        receiptHash: hashObject(writeReceipt),
        verdict: 'PASS',
        allowPublication: true,
        dimensions: {
          data: { result: 'PASS', critical: false, reasons: [] },
          graph: { result: 'PASS', critical: false, reasons: [] },
          knowledge: { result: 'PASS', critical: false, reasons: [] },
          security: { result: 'PASS', critical: false, reasons: [] },
          retrieval: { result: 'PASS', critical: false, reasons: [] },
        },
      } });
    }
    if (name === 'msp_pipeline_publication_receipt') {
      publicationReceipt = args.receipt;
      claimed = false;
      return response({ accepted: true });
    }
    throw new Error(`unexpected MSP tool ${name}`);
  };
  let inject = true;
  const options = {
    dbPath,
    scope,
    credential: 'worker-credential-test',
    workerToken: 'query-token-test',
    modelDir,
    mspCall,
    benchmarkFixture: fixture,
    port: 0,
    faultInjector: async (point) => {
      if (point === 'after-pointer-replacement-before-publication-outbox' && inject) {
        inject = false;
        throw new Error('TEST_POINTER_CRASH');
      }
    },
  };
  const first = GenesisRag17Worker.create(options);
  let pointerBefore;
  let snapshotBefore;
  try {
    await assert.rejects(first.runOnce(), /TEST_POINTER_CRASH/);
    pointerBefore = fs.readFileSync(path.join(dbPath, 'genesisrag17', 'published-pointer.json'), 'utf8');
    snapshotBefore = fs.readFileSync(path.join(dbPath, 'genesisrag17', 'snapshots', `${JSON.parse(pointerBefore).snapshotId}.json`), 'utf8');
    const result = await first.runOnce();
    assert.equal(result.status, 'published');
    assert.equal(fs.readFileSync(path.join(dbPath, 'genesisrag17', 'published-pointer.json'), 'utf8'), pointerBefore);
    const snapshotAfter = fs.readFileSync(path.join(dbPath, 'genesisrag17', 'snapshots', `${JSON.parse(pointerBefore).snapshotId}.json`), 'utf8');
    assert.equal(snapshotAfter, snapshotBefore);
    assert.equal(publicationReceipt.pointerHash, JSON.parse(pointerBefore).pointerHash);
  } finally {
    await first.close();
    try { fs.rmSync(root, { recursive: true, force: true }); } catch { /* native handle cleanup is process scoped */ }
  }
});

test('worker keeps a prepared snapshot private until pointer replacement', async () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'genesisrag17-prepared-'));
  const dbPath = path.join(root, 'db');
  const decision = makeDecision();
  const fixture = {
    fixtureVersion: 'worker-test-v1',
    queries: [{ query: 'Which company employs Alice?', relevantTexts: ['Alice works for Acme Ltd.'] }],
  };
  let claimed = true;
  let writeReceipt;
  let inject = true;
  const response = (body) => ({ schemaVersion: SCHEMA_VERSION, scope, ...body });
  const mspCall = async (name, args) => {
    if (name === 'msp_pipeline_claim') return response({ decisions: claimed ? [decision] : [] });
    if (name === 'msp_pipeline_graph_receipt') {
      return response({ accepted: true, graphReceiptHash: hashObject(args.receipt), derived: decision.derived, derivedHash: hashObject(decision.derived) });
    }
    if (name === 'msp_pipeline_write_receipt') {
      writeReceipt = args.receipt;
      return response({ accepted: true, receiptHash: hashObject(args.receipt) });
    }
    if (name === 'msp_pipeline_gate') {
      return response({ verdict: {
        schemaVersion: SCHEMA_VERSION,
        scope,
        runId: decision.runId,
        decisionId: decision.decisionId,
        decisionHash: decision.decisionHash,
        snapshotId: writeReceipt.snapshotId,
        generation: writeReceipt.generation,
        receiptHash: hashObject(writeReceipt),
        verdict: 'PASS',
        allowPublication: true,
        dimensions: {
          data: { result: 'PASS', critical: false, reasons: [] },
          graph: { result: 'PASS', critical: false, reasons: [] },
          knowledge: { result: 'PASS', critical: false, reasons: [] },
          security: { result: 'PASS', critical: false, reasons: [] },
          retrieval: { result: 'PASS', critical: false, reasons: [] },
        },
      } });
    }
    if (name === 'msp_pipeline_publication_receipt') {
      claimed = false;
      return response({ accepted: true });
    }
    throw new Error(`unexpected MSP tool ${name}`);
  };
  const worker = GenesisRag17Worker.create({
    dbPath,
    scope,
    credential: 'worker-credential-test',
    workerToken: 'query-token-test',
    modelDir,
    mspCall,
    benchmarkFixture: fixture,
    port: 0,
    faultInjector: async (point) => {
      if (point === 'before-pointer-replacement' && inject) {
        inject = false;
        throw new Error('TEST_PREPARED_SNAPSHOT');
      }
    },
  });
  try {
    await assert.rejects(worker.runOnce(), /TEST_PREPARED_SNAPSHOT/);
    const preparedSnapshotId = worker.state.decisions[decision.decisionId].snapshotId;
    assert.ok(preparedSnapshotId);
    assert.ok(fs.existsSync(path.join(dbPath, 'genesisrag17', 'snapshots', `${preparedSnapshotId}.json`)));
    await assert.rejects(
      worker.queryPublished({ scope, query: 'Which company employs Alice?', topK: 5, snapshotId: preparedSnapshotId }),
      /PUBLISHED_POINTER_MISSING|SNAPSHOT_NOT_PUBLISHED/,
    );

    const result = await worker.runOnce();
    assert.equal(result.status, 'published');
    const query = await worker.queryPublished({ scope, query: 'Which company employs Alice?', topK: 5, snapshotId: preparedSnapshotId });
    assert.equal(query.snapshotId, preparedSnapshotId);
    assert.ok(query.results.some((row) => row.text === 'Alice works for Acme Ltd.'));
    assert.ok(worker.readPointer().publishedSnapshotIds.includes(preparedSnapshotId));
  } finally {
    await worker.close();
    try { fs.rmSync(root, { recursive: true, force: true }); } catch { /* native handle cleanup is process scoped */ }
  }
});
