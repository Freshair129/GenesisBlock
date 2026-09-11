import test from 'node:test';
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { createRequire } from 'node:module';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import {
  GenesisRag17Worker,
  SCHEMA_VERSION,
  STAGE_CATALOG,
  hashObject,
  hashText,
  validateDecision,
  verifyModelArtifacts,
  writeAtomic,
} from '../src/index.mjs';

const require = createRequire(import.meta.url);
const { GenesisDatabase } = require('../../index.js');

const modelDir = process.env.GENESIS_WORKER_MODEL_DIR
  ?? 'C:/Users/pc/.cache/huggingface/hub/models--intfloat--multilingual-e5-small/snapshots/614241f622f53c4eeff9890bdc4f31cfecc418b3';
let modelFixtureError;
try {
  verifyModelArtifacts(modelDir);
} catch (error) {
  modelFixtureError = error;
}
// The real Stage 15/16 acceptance tests need a large, externally provisioned
// pinned snapshot. Keep them visible as skipped when that TEST prerequisite is
// absent instead of turning a missing fixture into secondary MSP failures.
const modelTestOptions = Object.freeze(modelFixtureError
  ? { skip: `pinned model snapshot unavailable: ${modelFixtureError.message}` }
  : {});
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

function workerOptions(dbPath, mspCall, extra = {}) {
  return {
    dbPath,
    scope,
    credential: 'worker-credential-test',
    workerToken: 'query-token-test',
    modelDir,
    mspCall,
    port: 0,
    ...extra,
  };
}

function makeSingleChunkDecision({
  decisionId,
  batchId,
  runId,
  sourceId,
  documentId,
  version,
  rawArtifactId,
  parsedArtifactId,
  chunkId,
  text,
}) {
  const source = {
    sourceId,
    rawArtifactId,
    parsedArtifactId,
    documentId,
    version,
    content: text,
    contentHash: hashText(text),
  };
  const decision = {
    schemaVersion: SCHEMA_VERSION,
    decisionId,
    batchId,
    scope,
    runId,
    stages: stages(runId),
    source,
    chunks: [{
      chunkId,
      parsedArtifactId,
      ordinal: 0,
      text,
      contentHash: hashText(text),
      startOffset: 0,
      endOffset: text.length,
    }],
    mentions: [],
    entities: [],
    facts: [],
    held: [],
    derived: [],
    policy: { allowEmbedding: true, allowPublication: true },
    ontologyVersion: 'ontology_v1',
    pipelineVersion: SCHEMA_VERSION,
  };
  decision.decisionHash = hashObject(decision);
  return decision;
}

/**
 * ADR-075 Phase 2 (contract revision 2): a decision exercising every
 * ontology_v2 predicate and every new endpoint type (Package, Category,
 * PriceTier) alongside the unchanged v1 predicates. Mirrors makeDecision()'s
 * offset/mention/fact construction, extended to the C-2 vocabulary.
 */
function makeOntologyV2Decision() {
  const segments = [
    { chunkId: 'chunk-1', text: 'Alice works for Acme Ltd.' },
    { chunkId: 'chunk-2', text: 'Bob purchased Nimbus.' },
    { chunkId: 'chunk-3', text: 'BundleX contains Nimbus.' },
    { chunkId: 'chunk-4', text: 'Nimbus priced at Tier100.' },
    { chunkId: 'chunk-5', text: 'Nimbus in category Drinkware.' },
  ];
  let cursor = 0;
  const chunks = segments.map(({ chunkId, text }, ordinal) => {
    const startOffset = cursor;
    cursor += text.length;
    return { chunkId, parsedArtifactId: 'parsed-v2', ordinal, text, contentHash: hashText(text), startOffset, endOffset: cursor };
  });
  const content = segments.map((segment) => segment.text).join('');
  const source = {
    sourceId: 'source-v2', rawArtifactId: 'raw-v2', parsedArtifactId: 'parsed-v2',
    documentId: 'doc-v2', version: '1', content, contentHash: hashText(content),
  };
  const mentionSpec = (sourceMentionId, resolutionKey, semanticType, name, chunkId) => {
    const chunk = chunks.find((candidate) => candidate.chunkId === chunkId);
    const startOffset = chunk.text.indexOf(name);
    return { sourceMentionId, resolutionKey, semanticType, name, chunkId, startOffset, endOffset: startOffset + name.length };
  };
  const mentions = [
    mentionSpec('mention-alice', 'person:alice', 'Person', 'Alice', 'chunk-1'),
    mentionSpec('mention-acme', 'org:acme', 'Organization', 'Acme', 'chunk-1'),
    mentionSpec('mention-bob', 'person:bob', 'Person', 'Bob', 'chunk-2'),
    mentionSpec('mention-nimbus-2', 'product:nimbus', 'Product', 'Nimbus', 'chunk-2'),
    mentionSpec('mention-bundlex', 'package:bundlex', 'PACKAGE', 'BundleX', 'chunk-3'),
    mentionSpec('mention-nimbus-3', 'product:nimbus', 'Product', 'Nimbus', 'chunk-3'),
    mentionSpec('mention-nimbus-4', 'product:nimbus', 'Product', 'Nimbus', 'chunk-4'),
    mentionSpec('mention-tier100', 'PM-NIMBUS:qty100:100000', 'PRICE_TIER', 'Tier100', 'chunk-4'),
    mentionSpec('mention-nimbus-5', 'product:nimbus', 'Product', 'Nimbus', 'chunk-5'),
    mentionSpec('mention-drinkware', 'category:drinkware', 'CATEGORY', 'Drinkware', 'chunk-5'),
  ];
  const refsFor = (chunkId, sourceMentionIds) => ({
    sourceId: source.sourceId, rawArtifactId: source.rawArtifactId, parsedArtifactId: source.parsedArtifactId,
    chunkId, sourceMentionIds,
  });
  const notApplicable = { validFrom: 'not_applicable', validTo: 'not_applicable', txFrom: '2026-09-11T00:00:00.000Z', txTo: 'open' };
  const runId = 'run-v2-1';
  const decision = {
    schemaVersion: SCHEMA_VERSION,
    decisionId: 'decision-v2-1',
    batchId: 'batch-v2-1',
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
      { id: 'product-nimbus', name: 'Nimbus', semanticType: 'Product', mentions: ['mention-nimbus-2', 'mention-nimbus-3', 'mention-nimbus-4', 'mention-nimbus-5'] },
      // Case-insensitive semanticType matching (like the existing entries): PACKAGE/CATEGORY/PRICE_TIER.
      { id: 'package-bundlex', name: 'BundleX', semanticType: 'PACKAGE', mentions: ['mention-bundlex'] },
      { id: 'pricetier-100', name: 'Tier100', semanticType: 'PRICE_TIER', mentions: ['mention-tier100'] },
      { id: 'category-drinkware', name: 'Drinkware', semanticType: 'CATEGORY', mentions: ['mention-drinkware'] },
    ],
    facts: [
      { id: 'fact-works', subjectId: 'person-alice', predicate: 'WORKS_FOR', objectId: 'org-acme', confidence: 0.9, sourceReferences: refsFor('chunk-1', ['mention-alice', 'mention-acme']), temporal: notApplicable },
      { id: 'fact-purchased', subjectId: 'person-bob', predicate: 'PURCHASED', objectId: 'product-nimbus', confidence: 0.9, sourceReferences: refsFor('chunk-2', ['mention-bob', 'mention-nimbus-2']), temporal: notApplicable },
      { id: 'fact-component', subjectId: 'package-bundlex', predicate: 'HAS_COMPONENT', objectId: 'product-nimbus', confidence: 0.9, sourceReferences: refsFor('chunk-3', ['mention-bundlex', 'mention-nimbus-3']), temporal: notApplicable },
      { id: 'fact-priced', subjectId: 'product-nimbus', predicate: 'PRICED_AT', objectId: 'pricetier-100', confidence: 0.9, sourceReferences: refsFor('chunk-4', ['mention-nimbus-4', 'mention-tier100']), temporal: notApplicable },
      { id: 'fact-category', subjectId: 'product-nimbus', predicate: 'IN_CATEGORY', objectId: 'category-drinkware', confidence: 0.9, sourceReferences: refsFor('chunk-5', ['mention-nimbus-5', 'mention-drinkware']), temporal: notApplicable },
    ],
    held: [],
    derived: [],
    policy: { allowEmbedding: true, allowPublication: true },
    ontologyVersion: 'ontology_v2',
    pipelineVersion: SCHEMA_VERSION,
  };
  decision.decisionHash = hashObject(decision);
  return decision;
}

function rehash(decision) {
  const hashable = { ...decision };
  delete hashable.decisionHash;
  decision.decisionHash = hashObject(hashable);
  return decision;
}

test('worker verifies pinned model artifacts and performs native publish/query', modelTestOptions, async () => {
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

test('worker resumes after pointer replacement without rewriting the snapshot', modelTestOptions, async () => {
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

test('worker keeps a prepared snapshot private until pointer replacement', modelTestOptions, async () => {
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

test('worker persists the exact native transaction intent before commit and rejects same-id payload drift', async () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'genesisrag17-native-intent-'));
  const dbPath = path.join(root, 'db');
  const worker = GenesisRag17Worker.create(workerOptions(dbPath, async () => ({ decisions: [] })));
  try {
    const decision = makeDecision();
    const graphDecision = { ...decision, derived: [] };
    const validated = validateDecision(decision, scope);
    const candidate = worker.buildPhysicalCandidate(graphDecision, validated, []);
    const committed = await worker.commitCandidate(graphDecision, candidate, 'graph');
    const intentPath = path.join(dbPath, 'genesisrag17', 'transactions', `graph-${decision.decisionId}.json`);
    assert.ok(fs.existsSync(intentPath), 'the intent must remain available until receipt acknowledgement');
    const intent = JSON.parse(fs.readFileSync(intentPath, 'utf8'));
    assert.equal(intent.phase, 'graph');
    assert.equal(intent.transaction.transaction_id, committed.id);
    assert.equal(intent.transaction.expected_frontier, 0);
    assert.equal(intent.payloadHash, hashText(intent.payloadJson));
    assert.equal(intent.payloadJson, JSON.stringify(intent.transaction));

    const replay = await worker.commitCandidate(graphDecision, candidate, 'graph');
    assert.equal(replay.id, committed.id);
    assert.equal(replay.frontier, committed.frontier);

    const drifted = JSON.parse(intent.payloadJson);
    drifted.graph.nodes[0].props.text = 'payload drift';
    await assert.rejects(worker.db.commitTransaction(JSON.stringify(drifted)), /transaction identity conflict/);
  } finally {
    await worker.close();
    try { fs.rmSync(root, { recursive: true, force: true }); } catch { /* native handles are released at process exit. */ }
  }
});

test('worker recovers an actual native graph commit after accepted receipt state is interrupted', async () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'genesisrag17-graph-recovery-'));
  const dbPath = path.join(root, 'db');
  const decision = makeDecision();
  decision.policy.allowEmbedding = false;
  const hashable = { ...decision };
  delete hashable.decisionHash;
  decision.decisionHash = hashObject(hashable);
  let claimed = true;
  let inject = true;
  let graphCalls = 0;
  let firstGraphReceipt;
  let failureCall = 0;
  const response = (body) => ({ schemaVersion: SCHEMA_VERSION, scope, ...body });
  const mspCall = async (name, args) => {
    if (name === 'msp_pipeline_claim') return response({ decisions: claimed ? [decision] : [] });
    if (name === 'msp_pipeline_graph_receipt') {
      graphCalls += 1;
      if (!firstGraphReceipt) firstGraphReceipt = structuredClone(args.receipt);
      else assert.deepEqual(args.receipt, firstGraphReceipt, 'graph receipt replay must be byte-equivalent in content');
      return response({ accepted: true, graphReceiptHash: hashObject(args.receipt), derived: decision.derived, derivedHash: hashObject(decision.derived) });
    }
    if (name === 'msp_pipeline_stage_failure') {
      failureCall += 1;
      claimed = false;
      const normalizedFailure = { ...args };
      delete normalizedFailure.credential;
      return response({ accepted: true, failureHash: hashObject(normalizedFailure) });
    }
    throw new Error(`unexpected MSP tool ${name}`);
  };
  const worker = GenesisRag17Worker.create(workerOptions(dbPath, mspCall, {
    faultInjector: async (point) => {
      if (point === 'after-graph-receipt-accepted-before-local-state' && inject) {
        inject = false;
        const error = new Error('TEST_GRAPH_ACCEPTED_BEFORE_LOCAL_STATE');
        error.faultInjection = true;
        throw error;
      }
    },
  }));
  try {
    await assert.rejects(worker.runOnce(), /TEST_GRAPH_ACCEPTED_BEFORE_LOCAL_STATE/);
    const outboxPath = path.join(dbPath, 'genesisrag17', 'outbox', `graph-receipt-${decision.decisionId}.json`);
    assert.ok(fs.existsSync(outboxPath), 'graph outbox must remain until accepted derived state is durable');
    assert.equal(worker.lexical.count(
      `g17-${hashObject({ decisionId: decision.decisionId, decisionHash: decision.decisionHash }).slice(0, 32)}`,
      scope,
    ), 0, 'Stage13 must not write lexical rows');

    const result = await worker.runOnce();
    assert.equal(result.status, 'failed');
    assert.equal(result.stageNumber, 15);
    assert.equal(graphCalls, 2, 'the accepted graph receipt must be replayed from the durable outbox');
    assert.equal(failureCall, 1);
    assert.ok(!fs.existsSync(outboxPath));
    assert.ok(!fs.existsSync(path.join(dbPath, 'genesisrag17', 'transactions', `graph-${decision.decisionId}.json`)));
    const graphTransactionRows = JSON.parse(await worker.db.querySql(
      'SELECT transaction_id FROM applied_transactions WHERE transaction_id LIKE ?',
      JSON.stringify([`txn-graph-%`]),
    ));
    assert.equal(graphTransactionRows.length, 1, 'native graph transaction must be applied once');
  } finally {
    await worker.close();
    try { fs.rmSync(root, { recursive: true, force: true }); } catch { /* native handles are released at process exit. */ }
  }
});

test('worker reuses an actual native final transaction after commit-to-receipt interruption and indexes lexical rows only in Stage16', modelTestOptions, async () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'genesisrag17-final-recovery-'));
  const dbPath = path.join(root, 'db');
  const decision = makeDecision();
  let claimed = true;
  let inject = true;
  let writeCalls = 0;
  let writeReceipt;
  let firstFinalIntent;
  const response = (body) => ({ schemaVersion: SCHEMA_VERSION, scope, ...body });
  const mspCall = async (name, args) => {
    if (name === 'msp_pipeline_claim') return response({ decisions: claimed ? [decision] : [] });
    if (name === 'msp_pipeline_graph_receipt') {
      return response({ accepted: true, graphReceiptHash: hashObject(args.receipt), derived: decision.derived, derivedHash: hashObject(decision.derived) });
    }
    if (name === 'msp_pipeline_write_receipt') {
      writeCalls += 1;
      writeReceipt = structuredClone(args.receipt);
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
  const worker = GenesisRag17Worker.create(workerOptions(dbPath, mspCall, {
    benchmarkFixture: {
      fixtureVersion: 'worker-test-v1',
      queries: [{ query: 'Which company employs Alice?', relevantTexts: ['Alice works for Acme Ltd.'] }],
    },
    faultInjector: async (point, details) => {
      if (point === 'after-native-commit-before-receipt' && details.phase === 'final' && inject) {
        inject = false;
        firstFinalIntent = fs.readFileSync(path.join(dbPath, 'genesisrag17', 'transactions', `final-${decision.decisionId}.json`), 'utf8');
        const error = new Error('TEST_FINAL_NATIVE_COMMIT_BEFORE_RECEIPT');
        error.faultInjection = true;
        throw error;
      }
    },
  }));
  try {
    await assert.rejects(worker.runOnce(), /TEST_FINAL_NATIVE_COMMIT_BEFORE_RECEIPT/);
    assert.equal(writeCalls, 0);
    assert.equal(worker.lexical.count(
      `g17-${hashObject({ decisionId: decision.decisionId, decisionHash: decision.decisionHash }).slice(0, 32)}`,
      scope,
    ), 0, 'lexical indexing must not occur before the final Stage16 boundary');
    const intentPath = path.join(dbPath, 'genesisrag17', 'transactions', `final-${decision.decisionId}.json`);
    assert.equal(fs.readFileSync(intentPath, 'utf8'), firstFinalIntent);

    const result = await worker.runOnce();
    assert.equal(result.status, 'published');
    assert.equal(writeCalls, 1);
    assert.ok(worker.lexical.count(result.generation, scope) > 0);
    const finalTransactionRows = JSON.parse(await worker.db.querySql(
      'SELECT transaction_id FROM applied_transactions WHERE transaction_id LIKE ?',
      JSON.stringify([`txn-final-%`]),
    ));
    assert.equal(finalTransactionRows.length, 1, 'native final replay must remain one applied transaction');
    assert.ok(!fs.existsSync(intentPath), 'final intent is removed only after the write receipt is accepted');
  } finally {
    await worker.close();
    try { fs.rmSync(root, { recursive: true, force: true }); } catch { /* native handles are released at process exit. */ }
  }
});

test('worker checkpoints a new vector collection before a crash can replay its first vector transaction', modelTestOptions, async () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'genesisrag17-collection-recovery-'));
  const dbPath = path.join(root, 'db');
  const collection = 'genesisrag17_recovery_collection';
  const workerUrl = new URL('../src/worker.mjs', import.meta.url).href;
  const childScript = `
    import { GenesisRag17Worker } from ${JSON.stringify(workerUrl)};
    const scope = ${JSON.stringify(scope)};
    const collection = ${JSON.stringify(collection)};
    const worker = GenesisRag17Worker.create({
      dbPath: ${JSON.stringify(dbPath)},
      scope,
      credential: 'worker-credential-test',
      workerToken: 'query-token-test',
      modelDir: ${JSON.stringify(modelDir)},
      mspCall: async () => ({}),
      port: 0,
    });
    await worker.db.addNode({
      id: 'collection-recovery-node',
      labels: ['Chunk'],
      props: { scope, generation: 'g17-collection-recovery', text: 'recovery' },
      validFrom: new Date().toISOString(),
    });
    await worker.db.saveState();
    await worker.ensureEmbedder(collection);
    const transaction = {
      transaction_id: 'txn-collection-recovery',
      expected_frontier: Number(worker.db.txnFrontier()),
      relational: [],
      graph: {
        nodes: [],
        edges: [],
      },
      vectors: [{
        node_id: 'collection-recovery-node',
        collection,
        embedding: Array.from({ length: 384 }, (_, index) => index === 0 ? 1 : 0),
      }],
    };
    await worker.db.commitTransaction(JSON.stringify(transaction));
    process.exit(0);
  `;
  try {
    execFileSync(process.execPath, ['--input-type=module', '--eval', childScript], { stdio: 'inherit' });
    const db = GenesisDatabase.open({ path: dbPath, vectorDim: 384, retention: 'full' });
    try {
      const recovered = db.listCollections().find((entry) => entry.name === collection);
      assert.ok(recovered, 'the first vector transaction must not auto-provision a replacement collection');
      assert.equal(recovered.model, 'intfloat/multilingual-e5-small');
      assert.equal(recovered.dim, 384);
      assert.equal(recovered.metric.toLowerCase(), 'cosine');
    } finally {
      // The pinned N-API binding has no close method; the handle is released at
      // process exit. Keep this disposable store isolated for that lifetime.
    }
  } finally {
    try { fs.rmSync(root, { recursive: true, force: true }); } catch { /* native handle cleanup is process scoped. */ }
  }
});

test('atomic pointer replacement failure preserves the existing pointer', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'genesisrag17-pointer-failure-'));
  const pointer = path.join(root, 'published-pointer.json');
  try {
    fs.writeFileSync(pointer, 'old-pointer\n', 'utf8');
    assert.throws(() => writeAtomic(pointer, 'new-pointer\n', () => {
      const error = new Error('TEST_ATOMIC_REPLACE_FAILURE');
      error.code = 'EPERM';
      throw error;
    }), /TEST_ATOMIC_REPLACE_FAILURE/);
    assert.equal(fs.readFileSync(pointer, 'utf8'), 'old-pointer\n');
  } finally {
    try { fs.rmSync(root, { recursive: true, force: true }); } catch { /* test directory is disposable. */ }
  }
});

test('worker publishes two queued documents and retains versioned historical snapshots after correction', modelTestOptions, async () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'genesisrag17-two-documents-'));
  const dbPath = path.join(root, 'db');
  const aliceV1 = makeSingleChunkDecision({
    decisionId: 'decision-alice-v1',
    batchId: 'batch-alice-v1',
    runId: 'run-alice-v1',
    sourceId: 'source-alice',
    documentId: 'doc-alice',
    version: '1',
    rawArtifactId: 'raw-alice-v1',
    parsedArtifactId: 'parsed-alice-v1',
    chunkId: 'chunk-alice-v1',
    text: 'Alice works for Acme Ltd.',
  });
  const bobV1 = makeSingleChunkDecision({
    decisionId: 'decision-bob-v1',
    batchId: 'batch-bob-v1',
    runId: 'run-bob-v1',
    sourceId: 'source-bob',
    documentId: 'doc-bob',
    version: '1',
    rawArtifactId: 'raw-bob-v1',
    parsedArtifactId: 'parsed-bob-v1',
    chunkId: 'chunk-bob-v1',
    text: 'Bob purchased Nimbus.',
  });
  const aliceV2 = makeSingleChunkDecision({
    decisionId: 'decision-alice-v2',
    batchId: 'batch-alice-v2',
    runId: 'run-alice-v2',
    sourceId: 'source-alice',
    documentId: 'doc-alice',
    version: '2',
    rawArtifactId: 'raw-alice-v2',
    parsedArtifactId: 'parsed-alice-v2',
    chunkId: 'chunk-alice-v2',
    text: 'Alice works for Globex Corp.',
  });
  const decisions = new Map([aliceV1, bobV1, aliceV2].map((decision) => [decision.decisionId, decision]));
  const pending = [aliceV1, bobV1];
  const receipts = new Map();
  const published = [];
  const response = (body) => ({ schemaVersion: SCHEMA_VERSION, scope, ...body });
  const passDimensions = {
    data: { result: 'PASS', critical: false, reasons: [] },
    graph: { result: 'PASS', critical: false, reasons: [] },
    knowledge: { result: 'PASS', critical: false, reasons: [] },
    security: { result: 'PASS', critical: false, reasons: [] },
    retrieval: { result: 'PASS', critical: false, reasons: [] },
  };
  const mspCall = async (name, args) => {
    if (name === 'msp_pipeline_claim') {
      assert.equal(args.limit, 1, 'the worker must claim one document at a time');
      return response({ decisions: pending.length > 0 ? [pending.shift()] : [] });
    }
    if (name === 'msp_pipeline_graph_receipt') {
      const decision = decisions.get(args.receipt.decisionId);
      assert.ok(decision, 'graph receipt must reference a queued decision');
      assert.equal(args.receipt.readback.ok, true, 'graph receipt must contain native readback evidence');
      return response({
        accepted: true,
        graphReceiptHash: hashObject(args.receipt),
        derived: decision.derived,
        derivedHash: hashObject(decision.derived),
      });
    }
    if (name === 'msp_pipeline_write_receipt') {
      const decision = decisions.get(args.receipt.decisionId);
      assert.ok(decision, 'write receipt must reference a queued decision');
      assert.equal(args.receipt.readback.ok, true, 'write receipt must contain native readback evidence');
      assert.equal(args.receipt.laneManifest.vector.status, 'ready');
      assert.equal(args.receipt.laneManifest.lexical.status, 'ready');
      assert.equal(args.receipt.laneManifest.graph.status, 'ready');
      assert.equal(args.receipt.laneManifest.sqlite.status, 'ready');
      assert.equal(args.receipt.laneManifest.provenance.status, 'ready');
      assert.equal(args.receipt.benchmark.citationCorrectness, 1);
      receipts.set(decision.decisionId, structuredClone(args.receipt));
      return response({ accepted: true, receiptHash: hashObject(args.receipt) });
    }
    if (name === 'msp_pipeline_gate') {
      const decision = decisions.get(args.decisionId);
      const receipt = receipts.get(args.decisionId);
      assert.ok(decision && receipt, 'quality gate must use the accepted native receipt');
      return response({ verdict: {
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
        dimensions: passDimensions,
      } });
    }
    if (name === 'msp_pipeline_publication_receipt') {
      const decision = decisions.get(args.receipt.decisionId);
      assert.ok(decision, 'publication receipt must reference a queued decision');
      published.push(structuredClone(args.receipt));
      return response({ accepted: true });
    }
    throw new Error(`unexpected MSP tool ${name}`);
  };
  const benchmarkFixture = {
    fixtureVersion: 'worker-two-document-v1',
    queries: [{
      query: 'What does this document say?',
      relevantTexts: [aliceV1.source.content, bobV1.source.content, aliceV2.source.content],
    }],
  };
  const worker = GenesisRag17Worker.create(workerOptions(dbPath, mspCall, { benchmarkFixture }));
  const sourceRows = async () => JSON.parse(await worker.db.querySql('SELECT payload FROM props', '[]'))
    .map(({ payload }) => typeof payload === 'string' ? JSON.parse(payload) : payload)
    .filter((row) => row.objectType === 'source');
  const assertVersionedHit = async (result, decision) => {
    assert.equal(result.results.length, 1, 'a generation-scoped query must return only its document');
    const hit = result.results[0];
    assert.equal(hit.text, decision.source.content);
    assert.equal(hit.citation.sourceId, decision.source.sourceId);
    assert.equal(hit.citation.chunkId, decision.chunks[0].chunkId);
    assert.equal(hit.citation.contentHash, decision.chunks[0].contentHash);
    const source = (await sourceRows()).find((row) => row.generation === result.generation
      && row.sourceId === decision.source.sourceId);
    assert.ok(source, `native source row for ${decision.source.documentId} must be present`);
    assert.equal(source.documentId, decision.source.documentId);
    assert.equal(source.version, decision.source.version, 'citation content must resolve to the source version');
    assert.equal(source.contentHash, decision.source.contentHash);
  };
  try {
    const first = await worker.runOnce();
    assert.equal(first.status, 'published');
    const second = await worker.runOnce();
    assert.equal(second.status, 'published');
    pending.push(aliceV2);
    const correction = await worker.runOnce();
    assert.equal(correction.status, 'published');
    assert.equal(published.length, 3, 'each document version must receive one publication receipt');

    const pointer = worker.readPointer();
    assert.equal(pointer.snapshotId, correction.snapshotId);
    assert.deepEqual(pointer.publishedSnapshotIds, [first.snapshotId, second.snapshotId, correction.snapshotId]);

    const aliceOld = await worker.queryPublished({
      scope,
      query: 'Where does Alice work?',
      topK: 5,
      snapshotId: first.snapshotId,
    });
    const bobHistorical = await worker.queryPublished({
      scope,
      query: 'What product did Bob purchase?',
      topK: 5,
      snapshotId: second.snapshotId,
    });
    const aliceCurrent = await worker.queryPublished({
      scope,
      query: 'Where does Alice work?',
      topK: 5,
      snapshotId: correction.snapshotId,
    });
    await assertVersionedHit(aliceOld, aliceV1);
    await assertVersionedHit(bobHistorical, bobV1);
    await assertVersionedHit(aliceCurrent, aliceV2);

    const latest = await worker.queryPublished({ scope, query: 'Where does Alice work?', topK: 5 });
    assert.equal(latest.snapshotId, correction.snapshotId);
    await assertVersionedHit(latest, aliceV2);
  } finally {
    await worker.close();
    try { fs.rmSync(root, { recursive: true, force: true }); } catch { /* native handles are released at process exit. */ }
  }
});

test('worker fails closed when a quality gate returns WARN with publication allowed', async () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'genesisrag17-warn-gate-'));
  const dbPath = path.join(root, 'db');
  const decision = makeDecision();
  const receipt = {
    schemaVersion: SCHEMA_VERSION,
    scope,
    runId: decision.runId,
    decisionId: decision.decisionId,
    decisionHash: decision.decisionHash,
    snapshotId: 'snap-warn-test',
    generation: 'g17-warn-test',
    transaction: { id: 'txn-final-warn-test', frontier: '1', checkpoint: '1' },
  };
  const receiptHash = hashObject(receipt);
  const response = (body) => ({ schemaVersion: SCHEMA_VERSION, scope, ...body });
  let publicationCalls = 0;
  const worker = GenesisRag17Worker.create(workerOptions(dbPath, async (name) => {
    if (name === 'msp_pipeline_gate') return response({ verdict: {
      schemaVersion: SCHEMA_VERSION,
      scope,
      runId: decision.runId,
      decisionId: decision.decisionId,
      decisionHash: decision.decisionHash,
      snapshotId: receipt.snapshotId,
      generation: receipt.generation,
      receiptHash,
      verdict: 'WARN',
      allowPublication: true,
    } });
    if (name === 'msp_pipeline_publication_receipt') publicationCalls += 1;
    throw new Error(`unexpected MSP tool ${name}`);
  }));
  try {
    const result = await worker.resumeFromReceipt({
      ...decision,
      policy: { ...decision.policy, allowPublication: true },
    }, { status: 'receipt_written', receipt, receiptHash });
    assert.equal(result.status, 'held');
    assert.equal(worker.state.decisions[decision.decisionId].status, 'held_by_quality_gate');
    assert.equal(publicationCalls, 0);
  } finally {
    await worker.close();
    try { fs.rmSync(root, { recursive: true, force: true }); } catch { /* test directory is disposable. */ }
  }
});

test('worker keeps explicit unmapped temporal status unsupported while accepting legacy null applicability', async () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'genesisrag17-temporal-status-'));
  const dbPath = path.join(root, 'db');
  const worker = GenesisRag17Worker.create(workerOptions(dbPath, async () => ({ decisions: [] })));
  try {
    const decision = makeDecision();
    const candidate = { generation: 'g17-temporal-status-test' };
    decision.facts = decision.facts.map((fact) => ({
      ...fact,
      temporal: { validFrom: null, validTo: null, txFrom: '2026-09-08T00:00:00.000Z', txTo: 'open' },
    }));
    const legacy = await worker.verifyTemporalLane(candidate, decision);
    assert.equal(legacy.status, 'not_applicable');
    decision.facts[0].temporal.status = 'unmapped';
    const unmapped = await worker.verifyTemporalLane(candidate, decision);
    assert.equal(unmapped.status, 'unsupported');
    assert.equal(unmapped.reason, 'temporal_mapping_missing_valid_from');
  } finally {
    await worker.close();
    try { fs.rmSync(root, { recursive: true, force: true }); } catch { /* test directory is disposable. */ }
  }
});

// ---------------------------------------------------------------------------
// ADR-075 Phase 2 (contract revision 2): worker accepts {ontology_v1, ontology_v2}
// ---------------------------------------------------------------------------

test('worker Stage13 accepts an ontology_v2 decision exercising each new predicate and endpoint type', async () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'genesisrag17-ontology-v2-'));
  const dbPath = path.join(root, 'db');
  const worker = GenesisRag17Worker.create(workerOptions(dbPath, async () => ({ decisions: [] })));
  try {
    const decision = makeOntologyV2Decision();
    const validated = validateDecision(decision, scope);
    assert.equal(validated.decision.ontologyVersion, 'ontology_v2');
    assert.equal(validated.decision.facts.length, 5);

    const candidate = worker.buildPhysicalCandidate(decision, validated, []);
    // entityKind() must map the new semanticType values case-insensitively.
    const nodeFor = (entityId) => candidate.nodes.find((node) => node.props.entityId === entityId);
    assert.deepEqual(nodeFor('package-bundlex').labels, ['Entity', 'Package']);
    assert.deepEqual(nodeFor('pricetier-100').labels, ['Entity', 'PriceTier']);
    assert.deepEqual(nodeFor('category-drinkware').labels, ['Entity', 'Category']);
    // All five facts (2 unchanged v1 predicates + 3 new v2 predicates) built without FACT_ENDPOINT_INVALID.
    const acceptedFactIds = candidate.nodes
      .filter((node) => node.props.objectType === 'fact')
      .map((node) => node.props.factId)
      .sort();
    assert.deepEqual(acceptedFactIds, ['fact-category', 'fact-component', 'fact-priced', 'fact-purchased', 'fact-works']);

    // Real native Stage13 physical write, not just candidate construction.
    const committed = await worker.commitCandidate(decision, candidate, 'graph');
    assert.ok(committed.id);
    assert.match(committed.frontier, /^\d+$/, 'a real native commit returns a numeric frontier string');
  } finally {
    await worker.close();
    try { fs.rmSync(root, { recursive: true, force: true }); } catch { /* test directory is disposable. */ }
  }
});

test('worker Stage13 still accepts an ontology_v1 decision unchanged (regression)', () => {
  const decision = makeDecision();
  assert.equal(decision.ontologyVersion, 'ontology_v1');
  const validated = validateDecision(decision, scope);
  assert.equal(validated.decision.facts.length, 2);
  assert.equal(validated.decision.facts[0].predicate, 'WORKS_FOR');
  assert.equal(validated.decision.facts[1].predicate, 'PURCHASED');
});

test('worker rejects a v2-only predicate inside a v1 decision as FACT_PREDICATE_NONCANONICAL', () => {
  const decision = makeDecision();
  decision.facts.push({
    id: 'fact-component-in-v1',
    subjectId: 'org-acme',
    predicate: 'HAS_COMPONENT',
    objectId: 'product-nimbus',
    confidence: 0.9,
    sourceReferences: {
      sourceId: decision.source.sourceId, rawArtifactId: decision.source.rawArtifactId,
      parsedArtifactId: decision.source.parsedArtifactId, chunkId: 'chunk-1', sourceMentionIds: ['mention-acme'],
    },
    temporal: { validFrom: 'not_applicable', validTo: 'not_applicable', txFrom: '2026-09-11T00:00:00.000Z', txTo: 'open' },
  });
  rehash(decision);
  assert.throws(() => validateDecision(decision, scope), /FACT_PREDICATE_NONCANONICAL:HAS_COMPONENT/);
});

test('worker rejects an ontology_v2 fact with reversed endpoints as FACT_ENDPOINT_INVALID', () => {
  const decision = makeOntologyV2Decision();
  const categoryFact = decision.facts.find((fact) => fact.id === 'fact-category');
  // IN_CATEGORY is Product|Package -> Category; swap subject/object so it reads Category -> Product.
  categoryFact.subjectId = 'category-drinkware';
  categoryFact.objectId = 'product-nimbus';
  rehash(decision);
  assert.throws(() => validateDecision(decision, scope), /FACT_ENDPOINT_INVALID:fact-category/);
});

test('worker rejects an unsupported ontologyVersion as DECISION_VERSION_INVALID', () => {
  const decision = makeDecision();
  decision.ontologyVersion = 'ontology_v3';
  rehash(decision);
  assert.throws(() => validateDecision(decision, scope), /DECISION_VERSION_INVALID/);
});

test('worker bitemporal lane counts only the mapped (dated) fact in a mixed generation (C-10)', async () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'genesisrag17-mixed-temporal-'));
  const dbPath = path.join(root, 'db');
  const worker = GenesisRag17Worker.create(workerOptions(dbPath, async () => ({ decisions: [] })));
  try {
    const decision = makeDecision();
    // One dated (mapped) fact and one not_applicable fact in the same generation.
    decision.facts[0].temporal = {
      validFrom: '2020-01-01T00:00:00.000Z', validTo: 'not_applicable',
      txFrom: '2026-09-11T00:00:00.000Z', txTo: 'open',
    };
    // decision.facts[1] keeps makeDecision()'s default not_applicable temporal.
    rehash(decision);
    const graphDecision = { ...decision, derived: [] };
    const validated = validateDecision(decision, scope);
    const candidate = worker.buildPhysicalCandidate(graphDecision, validated, []);
    await worker.commitCandidate(graphDecision, candidate, 'graph');

    const lane = await worker.verifyTemporalLane(candidate, graphDecision);
    assert.equal(lane.status, 'ready');
    assert.equal(lane.objects, 1, 'bitemporal objects must equal the mapped/dated count, not facts.length');
  } finally {
    await worker.close();
    try { fs.rmSync(root, { recursive: true, force: true }); } catch { /* test directory is disposable. */ }
  }
});
