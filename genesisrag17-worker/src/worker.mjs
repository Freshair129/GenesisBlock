import crypto from 'node:crypto';
import fs from 'node:fs';
import fsp from 'node:fs/promises';
import http from 'node:http';
import os from 'node:os';
import path from 'node:path';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import { spawn, spawnSync } from 'node:child_process';
import { createInterface } from 'node:readline';
import { DatabaseSync } from 'node:sqlite';

import { createMspStdioCaller } from './msp-stdio.mjs';

const require = createRequire(import.meta.url);
const { GenesisDatabase } = require('../../index.js');

export const SCHEMA_VERSION = 'genesisrag17.v1';
export const MODEL_ID = 'intfloat/multilingual-e5-small';
export const MODEL_REVISION = '614241f622f53c4eeff9890bdc4f31cfecc418b3';
export const MODEL_DIMENSIONS = 384;
export const MODEL_METRIC = 'cosine';
const VECTOR_COLLECTION_PREFIX = 'genesisrag17_e5_';

const SCOPE_KEYS = ['portfolioId', 'tenantId', 'businessId', 'workspaceId', 'agentId', 'visibility'];
const STAGE_CATALOG_ENTRIES = [
  [9, 'DPS-KI-ENTITY-RESOLVE', 90],
  [10, 'DPS-KI-FACT-EXTRACT', 100],
  [11, 'DPS-KI-ONTOLOGY-MAP', 110],
  [12, 'DPS-KI-TEMPORAL-MAP', 120],
  [13, 'DPS-KI-GRAPH-BUILD', 130],
  [14, 'DPS-KI-ENRICH', 140],
  [15, 'DPS-KI-EMBED', 150],
  [16, 'DPS-KI-INDEX', 160],
  [17, 'DPS-KI-QUALITY-GATE', 170],
];
export const STAGE_CATALOG = Object.freeze(Object.fromEntries(
  STAGE_CATALOG_ENTRIES.map(([stageNumber, pipelineStageId, sequence]) => [
    stageNumber,
    Object.freeze({ stageNumber, pipelineStageId, sequence }),
  ]),
));

// The hashes are the upstream Hugging Face revision/LFS OIDs discovered from
// the model tree, not hashes calculated from an untrusted runtime cache. The
// worker still hashes every local file before opening ONNX Runtime.
export const MODEL_ARTIFACTS = Object.freeze({
  'onnx/model.onnx': Object.freeze({
    size: 470268510,
    sha256: 'ca456c06b3a9505ddfd9131408916dd79290368331e7d76bb621f1cba6bc8665',
    upstreamOid: 'f9c7ec44162ecb2ae1340185b24a43d83d606a00',
  }),
  'tokenizer.json': Object.freeze({
    size: 17082730,
    sha256: '0b44a9d7b51c3c62626640cda0e2c2f70fdacdc25bbbd68038369d14ebdf4c39',
    upstreamOid: 'c9b2fea3119ca8886380e5f47bffc5ea7a6e0ffa',
  }),
  'tokenizer_config.json': Object.freeze({
    size: 443,
    sha256: 'a1d6bc8734a6f635dc158508bef000f8e2e5a759c7d92f984b2c86e5ff53425b',
    upstreamOid: '059214673d9d6d2ee319411e2ffec8c024b816d5',
  }),
  'special_tokens_map.json': Object.freeze({
    size: 167,
    sha256: 'd05497f1da52c5e09554c0cd874037a083e1dc1b9cfd48034d1c717f1afc07a7',
    upstreamOid: 'e0b1d18f602b9acb69e1940d4af2d7ba09a2d626',
  }),
  'config.json': Object.freeze({
    size: 655,
    sha256: '69137736cab8b8903a07fe8afaafdda25aac55415a12a55d1bffa9f581abf959',
    upstreamOid: '60a2a84020f1d74cc53ea9e8c4e91cf4af6c2b68',
  }),
});

function fail(code, detail = '') {
  const suffix = detail ? `:${detail}` : '';
  throw new Error(`${code}${suffix}`);
}

function isFaultInjection(error) {
  return error?.faultInjection === true;
}

function isPlainObject(value) {
  if (value === null || typeof value !== 'object' || Array.isArray(value)) return false;
  const proto = Object.getPrototypeOf(value);
  return proto === Object.prototype || proto === null;
}

/** Canonical JSON: recursively sorted object keys, original array order. */
export function canonicalJson(value, stack = new Set()) {
  if (value === null) return 'null';
  if (typeof value === 'string') return JSON.stringify(value);
  if (typeof value === 'boolean') return value ? 'true' : 'false';
  if (typeof value === 'number') {
    if (!Number.isFinite(value)) fail('CANONICAL_JSON_NONFINITE_NUMBER');
    return JSON.stringify(value);
  }
  if (typeof value === 'undefined' || typeof value === 'function' || typeof value === 'symbol' || typeof value === 'bigint') {
    fail('CANONICAL_JSON_UNSUPPORTED_VALUE');
  }
  if (stack.has(value)) fail('CANONICAL_JSON_CYCLE');
  stack.add(value);
  let result;
  if (Array.isArray(value)) {
    result = `[${value.map((item) => canonicalJson(item, stack)).join(',')}]`;
  } else if (isPlainObject(value)) {
    const parts = Object.keys(value).sort().map((key) => `${JSON.stringify(key)}:${canonicalJson(value[key], stack)}`);
    result = `{${parts.join(',')}}`;
  } else {
    fail('CANONICAL_JSON_NON_PLAIN_OBJECT');
  }
  stack.delete(value);
  return result;
}

export function hashText(text) {
  if (typeof text !== 'string') fail('TEXT_HASH_REQUIRES_STRING');
  return crypto.createHash('sha256').update(Buffer.from(text, 'utf8')).digest('hex');
}

export function hashObject(value) {
  return crypto.createHash('sha256').update(canonicalJson(value), 'utf8').digest('hex');
}

export function validateScope(scope, expected = undefined) {
  if (!isPlainObject(scope)) fail('SCOPE_INVALID');
  const keys = Object.keys(scope).sort();
  if (keys.length !== SCOPE_KEYS.length || keys.some((key, index) => key !== [...SCOPE_KEYS].sort()[index])) {
    fail('SCOPE_KEYS_INVALID');
  }
  for (const key of SCOPE_KEYS) {
    if (typeof scope[key] !== 'string') fail('SCOPE_VALUE_INVALID', key);
  }
  if (!scope.portfolioId || !scope.tenantId || !scope.businessId) fail('SCOPE_REQUIRED_ID_EMPTY');
  if (scope.visibility !== 'private') fail('SCOPE_VISIBILITY_INVALID');
  if (expected !== undefined && canonicalJson(scope) !== canonicalJson(expected)) fail('SCOPE_MISMATCH');
  return scope;
}

function validateIdentity(identity, catalog) {
  if (!isPlainObject(identity)) fail('STAGE_IDENTITY_INVALID');
  for (const key of ['runId', 'pipelineStageId', 'executionStepId', 'attemptId']) {
    if (typeof identity[key] !== 'string' || identity[key].length === 0) fail('STAGE_IDENTITY_FIELD_INVALID', key);
  }
  if (!Number.isInteger(identity.stageNumber) || identity.stageNumber < 9 || identity.stageNumber > 17) {
    fail('STAGE_NUMBER_INVALID');
  }
  const expected = catalog[identity.stageNumber];
  if (!expected || identity.pipelineStageId !== expected.pipelineStageId) fail('STAGE_CATALOG_MISMATCH', String(identity.stageNumber));
}

function validateStages(stages, runId) {
  if (!Array.isArray(stages) || stages.length !== STAGE_CATALOG_ENTRIES.length) fail('STAGES_INVALID');
  const seen = new Set();
  for (const stage of stages) {
    validateIdentity(stage, STAGE_CATALOG);
    if (stage.runId !== runId) fail('STAGE_RUN_MISMATCH');
    if (seen.has(stage.stageNumber)) fail('STAGE_DUPLICATE');
    seen.add(stage.stageNumber);
  }
  for (const [stageNumber] of STAGE_CATALOG_ENTRIES) if (!seen.has(stageNumber)) fail('STAGE_MISSING', String(stageNumber));
}

function validatePolicy(policy) {
  if (!isPlainObject(policy) || typeof policy.allowEmbedding !== 'boolean' || typeof policy.allowPublication !== 'boolean') {
    fail('POLICY_INVALID');
  }
}

function validateSourceAndChunks(source, chunks, mentions) {
  if (!isPlainObject(source) || typeof source.sourceId !== 'string' || typeof source.rawArtifactId !== 'string'
    || typeof source.parsedArtifactId !== 'string' || typeof source.documentId !== 'string'
    || typeof source.version !== 'string' || typeof source.content !== 'string'
    || source.contentHash !== hashText(source.content)) {
    fail('SOURCE_LINEAGE_INVALID');
  }
  if (!Array.isArray(chunks) || chunks.length === 0) fail('CHUNKS_EMPTY');
  const chunkById = new Map();
  for (const chunk of chunks) {
    if (!isPlainObject(chunk) || typeof chunk.chunkId !== 'string' || typeof chunk.parsedArtifactId !== 'string'
      || chunk.parsedArtifactId !== source.parsedArtifactId || !Number.isInteger(chunk.ordinal)
      || typeof chunk.text !== 'string' || chunk.contentHash !== hashText(chunk.text)
      || !Number.isInteger(chunk.startOffset) || !Number.isInteger(chunk.endOffset)
      || chunk.startOffset < 0 || chunk.endOffset < chunk.startOffset) fail('CHUNK_LINEAGE_INVALID');
    if (source.content.slice(chunk.startOffset, chunk.endOffset) !== chunk.text) fail('CHUNK_OFFSET_MISMATCH', chunk.chunkId);
    if (chunkById.has(chunk.chunkId)) fail('CHUNK_DUPLICATE', chunk.chunkId);
    chunkById.set(chunk.chunkId, chunk);
  }
  if (!Array.isArray(mentions)) fail('MENTIONS_INVALID');
  for (const mention of mentions) {
    if (!isPlainObject(mention) || typeof mention.sourceMentionId !== 'string' || typeof mention.resolutionKey !== 'string'
      || typeof mention.semanticType !== 'string' || typeof mention.name !== 'string' || typeof mention.chunkId !== 'string'
      || !Number.isInteger(mention.startOffset) || !Number.isInteger(mention.endOffset)) fail('MENTION_INVALID');
    const chunk = chunkById.get(mention.chunkId);
    if (!chunk) fail('MENTION_CHUNK_MISSING', mention.chunkId);
    if (mention.startOffset < 0 || mention.endOffset < mention.startOffset
      || chunk.text.slice(mention.startOffset, mention.endOffset) !== mention.name) {
      fail('MENTION_OFFSET_MISMATCH', mention.sourceMentionId);
    }
  }
  return chunkById;
}

function sourceRefsOf(row) {
  const rawRefs = row?.sourceReferences ?? row?.sourceRefs ?? row?.provenance;
  const refs = Array.isArray(rawRefs) ? rawRefs[0] : rawRefs;
  if (!isPlainObject(refs)) fail('PROVENANCE_MISSING', row?.id ?? row?.factId ?? 'unknown');
  for (const key of ['sourceId', 'rawArtifactId', 'parsedArtifactId', 'chunkId']) {
    if (typeof refs[key] !== 'string' || refs[key].length === 0) fail('PROVENANCE_FIELD_INVALID', key);
  }
  if (!Array.isArray(refs.sourceMentionIds) || refs.sourceMentionIds.some((id) => typeof id !== 'string')) {
    fail('PROVENANCE_MENTIONS_INVALID');
  }
  return refs;
}

function sourceRefsListOf(row) {
  const rawRefs = row?.sourceReferences ?? row?.sourceRefs ?? row?.provenance;
  const list = Array.isArray(rawRefs) ? rawRefs : [rawRefs];
  if (list.length === 0) fail('PROVENANCE_MISSING', row?.id ?? row?.factId ?? row?.derivedId ?? 'unknown');
  return list.map((refs) => {
    if (!isPlainObject(refs)) fail('PROVENANCE_MISSING', row?.id ?? row?.factId ?? row?.derivedId ?? 'unknown');
    for (const key of ['sourceId', 'rawArtifactId', 'parsedArtifactId', 'chunkId']) {
      if (typeof refs[key] !== 'string' || refs[key].length === 0) fail('PROVENANCE_FIELD_INVALID', key);
    }
    if (!Array.isArray(refs.sourceMentionIds) || refs.sourceMentionIds.some((id) => typeof id !== 'string')) fail('PROVENANCE_MENTIONS_INVALID');
    return refs;
  });
}

function temporalValue(row, key) {
  const temporal = row?.temporal;
  if (!isPlainObject(temporal)) return undefined;
  return temporal[key] ?? temporal[key.replace(/[A-Z]/g, (letter) => `_${letter.toLowerCase()}`)];
}

function temporalValidFrom(row) {
  const value = temporalValue(row, 'validFrom');
  return typeof value === 'string' && value !== 'not_applicable' && Number.isFinite(Date.parse(value)) ? value : null;
}

function temporalClassification(row) {
  const temporal = row?.temporal;
  if (temporal === undefined) return 'not_applicable';
  if (!isPlainObject(temporal)) return 'unsupported';
  const status = temporalValue(row, 'status');
  const validFrom = temporalValue(row, 'validFrom');
  const validTo = temporalValue(row, 'validTo');
  // Facts extracted from language that carries no valid-time assertion are
  // still stamped with transaction time by GKS.  That transaction stamp is
  // not a temporal mapping, so the bitemporal lane is explicitly N/A.
  const noValidTime = (value) => value === undefined || value === null || value === 'not_applicable';
  if (noValidTime(validFrom) && noValidTime(validTo)
    && (status === undefined || status === 'not_applicable')) {
    return 'not_applicable';
  }
  return 'mapped';
}

function validateDerivedRows(rows, chunkById) {
  if (!Array.isArray(rows)) fail('DERIVED_COLLECTION_INVALID');
  for (const row of rows) {
    if (!isPlainObject(row) || typeof (row.id ?? row.derivedId) !== 'string') fail('DERIVED_INVALID');
    for (const refs of sourceRefsListOf(row)) {
      if (!chunkById.has(refs.chunkId)) fail('DERIVED_CHUNK_MISSING', row.id ?? row.derivedId);
    }
  }
  return rows;
}

/** Validate the frozen decision wire before any native write. */
export function validateDecision(decision, expectedScope = undefined) {
  if (!isPlainObject(decision) || decision.schemaVersion !== SCHEMA_VERSION) fail('DECISION_SCHEMA_INVALID');
  if (typeof decision.decisionId !== 'string' || typeof decision.decisionHash !== 'string'
    || typeof decision.batchId !== 'string' || typeof decision.runId !== 'string') fail('DECISION_IDENTITY_INVALID');
  validateScope(decision.scope, expectedScope);
  validateStages(decision.stages, decision.runId);
  validatePolicy(decision.policy);
  if (decision.ontologyVersion !== 'ontology_v1' || decision.pipelineVersion !== SCHEMA_VERSION) fail('DECISION_VERSION_INVALID');
  const hashable = { ...decision };
  delete hashable.decisionHash;
  if (hashObject(hashable) !== decision.decisionHash) fail('DECISION_HASH_MISMATCH');
  const chunkById = validateSourceAndChunks(decision.source, decision.chunks, decision.mentions ?? []);
  if (!Array.isArray(decision.entities) || !Array.isArray(decision.facts) || !Array.isArray(decision.held) || !Array.isArray(decision.derived)) {
    fail('DECISION_COLLECTIONS_INVALID');
  }
  const entityById = new Map();
  for (const entity of decision.entities) {
    if (!isPlainObject(entity) || typeof entity.id !== 'string' || typeof entity.name !== 'string'
      || typeof entity.semanticType !== 'string' || !Array.isArray(entity.mentions)) fail('ENTITY_INVALID');
    entityById.set(entity.id, entity);
  }
  const validateFact = (row) => {
    if (!isPlainObject(row) || typeof (row.id ?? row.factId) !== 'string' || typeof row.predicate !== 'string') fail('FACT_INVALID');
    if (!['WORKS_FOR', 'PURCHASED'].includes(row.predicate)) fail('FACT_PREDICATE_NONCANONICAL', row.predicate);
    const refs = sourceRefsOf(row);
    if (!chunkById.has(refs.chunkId)) fail('FACT_CHUNK_MISSING', row.id ?? row.factId);
    if (row.subjectId !== undefined && !entityById.has(row.subjectId)) fail('FACT_SUBJECT_MISSING', row.id ?? row.factId);
    if (row.objectId !== undefined && !entityById.has(row.objectId)) fail('FACT_OBJECT_MISSING', row.id ?? row.factId);
    const subject = entityById.get(row.subjectId);
    const object = entityById.get(row.objectId);
    const endpointsValid = row.predicate === 'WORKS_FOR'
      ? entityKind(subject) === 'Person' && entityKind(object) === 'Organization'
      : ['Person', 'Organization'].includes(entityKind(subject)) && entityKind(object) === 'Product';
    if (!endpointsValid) fail('FACT_ENDPOINT_INVALID', row.id ?? row.factId);
    if (row.confidence !== undefined && (!Number.isFinite(row.confidence) || row.confidence < 0 || row.confidence > 1)) fail('FACT_CONFIDENCE_INVALID');
    if (Number(row.confidence ?? 0) < 0.8) fail('FACT_CONFIDENCE_BELOW_WRITE_FLOOR', row.id ?? row.factId);
    return refs;
  };
  for (const row of decision.facts) validateFact(row);
  for (const row of decision.held) {
    if (!isPlainObject(row) || typeof (row.id ?? row.factId) !== 'string' || typeof row.predicate !== 'string') fail('HELD_FACT_INVALID');
    const refs = sourceRefsOf(row);
    if (!chunkById.has(refs.chunkId)) fail('HELD_CHUNK_MISSING', row.id ?? row.factId);
    if (row.confidence !== undefined && (!Number.isFinite(row.confidence) || row.confidence < 0 || row.confidence > 1)) fail('HELD_CONFIDENCE_INVALID');
  }
  for (const row of decision.derived) {
    if (!isPlainObject(row) || typeof (row.id ?? row.derivedId) !== 'string') fail('DERIVED_INVALID');
    for (const refs of sourceRefsListOf(row)) {
      if (!chunkById.has(refs.chunkId)) fail('DERIVED_CHUNK_MISSING', row.id ?? row.derivedId);
    }
  }
  return { decision, chunkById, entityById };
}

function expectedArtifactPath(modelDir, relative) {
  return path.join(modelDir, ...relative.split('/'));
}

export function verifyModelArtifacts(modelDir) {
  if (typeof modelDir !== 'string' || !path.isAbsolute(modelDir)) fail('MODEL_DIR_MUST_BE_ABSOLUTE');
  const artifactHashes = {};
  for (const [relative, expected] of Object.entries(MODEL_ARTIFACTS)) {
    const filename = expectedArtifactPath(modelDir, relative);
    if (!fs.existsSync(filename)) fail('MODEL_ARTIFACT_MISSING', relative);
    const stat = fs.statSync(filename);
    if (!stat.isFile() || stat.size !== expected.size) fail('MODEL_ARTIFACT_SIZE_MISMATCH', relative);
    const digest = crypto.createHash('sha256').update(fs.readFileSync(filename)).digest('hex');
    if (digest !== expected.sha256) fail('MODEL_ARTIFACT_HASH_MISMATCH', `${relative}:${digest}`);
    artifactHashes[relative] = digest;
  }
  return artifactHashes;
}

function cloneJson(value) {
  return JSON.parse(JSON.stringify(value));
}

function parseJsonText(text, code = 'JSON_INVALID') {
  try {
    return JSON.parse(text);
  } catch (error) {
    fail(code, error.message);
  }
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function nowIso() {
  return new Date().toISOString();
}

function nonNegativeMetrics(recordsIn, recordsOut, quarantined, errors, retries, durationMs) {
  const values = [recordsIn, recordsOut, quarantined, errors, retries, durationMs];
  if (values.some((value) => !Number.isFinite(value) || value < 0)) fail('METRICS_INVALID');
  return {
    records_in: Math.floor(recordsIn),
    records_out: Math.floor(recordsOut),
    records_quarantined: Math.floor(quarantined),
    error_count: Math.floor(errors),
    retry_count: Math.floor(retries),
    duration_ms: Math.floor(durationMs),
  };
}

function stageFailureCode(error) {
  const candidate = typeof error?.code === 'string' ? error.code : String(error?.message ?? 'WORKER_STAGE_FAILED');
  return candidate.match(/^[A-Z][A-Z0-9_]*/)?.[0] ?? 'WORKER_STAGE_FAILED';
}

function stageFailureMessage(error, secrets = []) {
  let message = String(error?.message ?? error ?? 'stage failed').slice(0, 1000);
  for (const secret of secrets) {
    if (typeof secret === 'string' && secret.length > 0) message = message.split(secret).join('[redacted]');
  }
  return message || 'stage failed';
}

function stateFilePath(root) {
  return path.join(root, 'state.json');
}

function safeFilePart(value) {
  return value.replace(/[^a-zA-Z0-9._-]/g, '_').slice(0, 80);
}

function publishedSnapshotHistory(pointer, snapshotId) {
  const prior = Array.isArray(pointer?.publishedSnapshotIds)
    ? pointer.publishedSnapshotIds
    : pointer?.snapshotId ? [pointer.snapshotId] : [];
  if (prior.some((id) => typeof id !== 'string' || id.length === 0)) fail('PUBLISHED_HISTORY_INVALID');
  if (typeof snapshotId !== 'string' || snapshotId.length === 0) fail('SNAPSHOT_ID_INVALID');
  return [...new Set([...prior, snapshotId])];
}

function snapshotRecord(pointer, decision) {
  return {
    schemaVersion: pointer.schemaVersion,
    scope: cloneJson(pointer.scope),
    snapshotId: pointer.snapshotId,
    generation: pointer.generation,
    vectorCollection: pointer.vectorCollection,
    decisionId: pointer.decisionId,
    decisionHash: pointer.decisionHash,
    receiptHash: pointer.receiptHash,
    createdAt: nowIso(),
    sourceIds: [decision.source.sourceId],
    chunkIds: decision.chunks.map((chunk) => chunk.chunkId),
  };
}

function fsyncDirectory(directory) {
  if (process.platform === 'win32') return;
  let fd;
  try {
    fd = fs.openSync(directory, 'r');
    fs.fsyncSync(fd);
  } catch (error) {
    if (!['EINVAL', 'ENOTDIR', 'EPERM'].includes(error?.code)) throw error;
  } finally {
    if (fd !== undefined) fs.closeSync(fd);
  }
}

function fsyncFile(filename) {
  let fd;
  try {
    fd = fs.openSync(filename, process.platform === 'win32' ? 'r+' : 'r');
    fs.fsyncSync(fd);
  } catch (error) {
    if (!['EINVAL', 'EPERM'].includes(error?.code)) throw error;
  } finally {
    if (fd !== undefined) fs.closeSync(fd);
  }
}

function powershellLiteral(value) {
  return `'${String(value).replaceAll("'", "''")}'`;
}

function replaceFileOnWindows(source, destination) {
  const backup = `${destination}.backup-${process.pid}-${crypto.randomBytes(4).toString('hex')}`;
  const command = [
    '$ErrorActionPreference = "Stop"',
    `[System.IO.File]::Replace(${powershellLiteral(source)}, ${powershellLiteral(destination)}, ${powershellLiteral(backup)}, $true)`,
  ].join('; ');
  const encoded = Buffer.from(command, 'utf16le').toString('base64');
  const result = spawnSync('powershell.exe', [
    '-NoProfile',
    '-NonInteractive',
    '-EncodedCommand',
    encoded,
  ], { windowsHide: true, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] });
  if (result.error) throw result.error;
  if (result.status !== 0) {
    const error = new Error(`ATOMIC_REPLACE_FAILED:${String(result.stderr ?? '').trim().slice(-500)}`);
    error.code = 'EPERM';
    throw error;
  }
  try { fs.rmSync(backup, { force: true }); } catch { /* preserve the replaced pointer; cleanup is best effort */ }
}

function isTransientAtomicReplaceError(error) {
  return ['EACCES', 'EBUSY', 'EEXIST', 'EPERM', 'ENOTEMPTY'].includes(error?.code);
}

function waitForAtomicReplaceRetry() {
  const signal = new Int32Array(new SharedArrayBuffer(4));
  Atomics.wait(signal, 0, 0, 25);
}

function replaceFileSafely(source, destination, replaceFile = undefined) {
  for (let attempt = 0; attempt < 4; attempt += 1) {
    try {
      if (replaceFile) {
        replaceFile(source, destination);
      } else if (process.platform === 'win32' && fs.existsSync(destination)) {
        // File.Replace maps to ReplaceFileW: destination remains in place if
        // the atomic replacement fails, including when a transient sharing
        // violation/EPERM is returned.
        replaceFileOnWindows(source, destination);
      } else {
        fs.renameSync(source, destination);
      }
      break;
    } catch (error) {
      if (!isTransientAtomicReplaceError(error) || attempt === 3) throw error;
      waitForAtomicReplaceRetry();
    }
  }
  fsyncFile(destination);
  fsyncDirectory(path.dirname(destination));
}

export function writeAtomic(filename, content, replaceFile = undefined) {
  fs.mkdirSync(path.dirname(filename), { recursive: true });
  const temp = `${filename}.tmp-${process.pid}-${crypto.randomBytes(4).toString('hex')}`;
  const fd = fs.openSync(temp, 'w');
  let replaced = false;
  try {
    fs.writeFileSync(fd, content, 'utf8');
    fs.fsyncSync(fd);
  } finally {
    fs.closeSync(fd);
  }
  try {
    replaceFileSafely(temp, filename, replaceFile);
    replaced = true;
  } finally {
    if (!replaced) {
      try { fs.rmSync(temp, { force: true }); } catch { /* preserve the original replacement error */ }
    }
  }
}

function processIsAlive(pid) {
  if (!Number.isInteger(pid) || pid <= 0) return false;
  try {
    process.kill(pid, 0);
    return true;
  } catch (error) {
    return error.code === 'EPERM';
  }
}

function acquireWorkerLock(filename, dbPath) {
  const lock = { pid: process.pid, dbPath, createdAt: nowIso(), token: crypto.randomBytes(16).toString('hex') };
  fs.mkdirSync(path.dirname(filename), { recursive: true });
  try {
    const fd = fs.openSync(filename, 'wx');
    fs.writeFileSync(fd, `${JSON.stringify(lock)}\n`, 'utf8');
    fs.fsyncSync(fd);
    fs.closeSync(fd);
    return lock;
  } catch (error) {
    if (error.code !== 'EEXIST') throw error;
    let current;
    try { current = parseJsonText(fs.readFileSync(filename, 'utf8'), 'WORKER_LOCK_INVALID'); } catch { fail('WORKER_LOCK_UNPROVABLY_STALE'); }
    if (current.dbPath !== dbPath || processIsAlive(current.pid)) fail('WORKER_STORE_ALREADY_OWNED', String(current.pid));
    // A dead owner is the only condition that permits stale-lock recovery.
    fs.rmSync(filename, { force: true });
    return acquireWorkerLock(filename, dbPath);
  }
}

function releaseWorkerLock(filename, lock) {
  if (!lock || !fs.existsSync(filename)) return;
  try {
    const current = parseJsonText(fs.readFileSync(filename, 'utf8'), 'WORKER_LOCK_INVALID');
    if (current.token === lock.token && current.pid === lock.pid) fs.rmSync(filename, { force: true });
  } catch {
    // A corrupt lock is not ours to delete; leave it for safe manual recovery.
  }
}

function physicalId(generation, kind, logicalId) {
  return `g17:${generation}:${kind}:${hashText(logicalId).slice(0, 24)}`;
}

function parseNativeFrontier(value) {
  const number = Number(value);
  if (!Number.isSafeInteger(number) || number < 0) fail('NATIVE_FRONTIER_INVALID');
  return String(number);
}

class Embedder {
  constructor({ modelDir, pythonCommand = process.env.GENESISRAG17_PYTHON ?? 'python' }) {
    this.modelDir = modelDir;
    this.pythonCommand = pythonCommand;
    this.child = undefined;
    this.lines = undefined;
    this.nextId = 1;
    this.pending = new Map();
    this.ready = undefined;
  }

  async start() {
    if (this.ready) return this.ready;
    this.ready = new Promise((resolve, reject) => {
      const normalizedScript = fileURLToPath(new URL('./embedder.py', import.meta.url));
      this.child = spawn(this.pythonCommand, [normalizedScript, this.modelDir], {
        shell: false,
        stdio: ['pipe', 'pipe', 'pipe'],
        env: { ...process.env, PYTHONIOENCODING: 'utf-8' },
      });
      this.lines = createInterface({ input: this.child.stdout, crlfDelay: Infinity });
      this.lines.on('line', (line) => {
        if (!line.trim()) return;
        let message;
        try { message = JSON.parse(line); } catch (error) {
          reject(error);
          return;
        }
        if (message.ready) {
          resolve(message);
          return;
        }
        const waiter = this.pending.get(message.id);
        if (!waiter) return;
        this.pending.delete(message.id);
        if (message.error) waiter.reject(new Error(`EMBEDDER_ERROR:${message.error}`));
        else waiter.resolve(message.vectors);
      });
      let stderr = '';
      this.child.stderr.on('data', (chunk) => { stderr += chunk.toString(); });
      this.child.once('error', reject);
      this.child.once('exit', (code, signal) => {
        const error = new Error(`EMBEDDER_EXIT:${code ?? 'null'}:${signal ?? 'null'}:${stderr.slice(-400)}`);
        for (const waiter of this.pending.values()) waiter.reject(error);
        this.pending.clear();
        if (this.ready) this.ready = undefined;
        else reject(error);
      });
    });
    await this.ready;
    return this.ready;
  }

  async embed(texts, mode = 'passage') {
    if (!Array.isArray(texts) || texts.length === 0) return [];
    await this.start();
    const id = this.nextId++;
    const result = new Promise((resolve, reject) => this.pending.set(id, { resolve, reject }));
    this.child.stdin.write(`${JSON.stringify({ id, mode, texts })}\n`);
    const vectors = await result;
    if (!Array.isArray(vectors) || vectors.length !== texts.length) fail('EMBEDDER_RESULT_COUNT');
    for (const vector of vectors) {
      if (!Array.isArray(vector) || vector.length !== MODEL_DIMENSIONS || vector.some((n) => !Number.isFinite(n))) {
        fail('EMBEDDER_VECTOR_INVALID');
      }
    }
    return vectors;
  }

  async close() {
    this.lines?.close();
    if (this.child && !this.child.killed) this.child.kill();
    this.child = undefined;
    this.ready = undefined;
  }
}

class LexicalIndex {
  constructor(filename) {
    this.filename = filename;
    fs.mkdirSync(path.dirname(filename), { recursive: true });
    this.db = new DatabaseSync(filename);
    this.db.exec(`
      PRAGMA journal_mode = WAL;
      CREATE TABLE IF NOT EXISTS lexical_documents (
        physical_id TEXT PRIMARY KEY,
        logical_id TEXT NOT NULL,
        generation TEXT NOT NULL,
        scope_json TEXT NOT NULL,
        text TEXT NOT NULL,
        content_hash TEXT NOT NULL,
        citation_json TEXT NOT NULL
      );
      CREATE VIRTUAL TABLE IF NOT EXISTS lexical_fts USING fts5(
        physical_id UNINDEXED,
        logical_id UNINDEXED,
        generation UNINDEXED,
        scope_json UNINDEXED,
        text,
        content_hash UNINDEXED,
        citation_json UNINDEXED
      );
    `);
  }

  put(rows) {
    if (rows.length === 0) return;
    this.db.exec('BEGIN IMMEDIATE');
    try {
      const get = this.db.prepare('SELECT content_hash FROM lexical_documents WHERE physical_id = ?');
      const insertDoc = this.db.prepare(`INSERT INTO lexical_documents
        (physical_id,logical_id,generation,scope_json,text,content_hash,citation_json) VALUES (?,?,?,?,?,?,?)`);
      const insertFts = this.db.prepare(`INSERT INTO lexical_fts
        (physical_id,logical_id,generation,scope_json,text,content_hash,citation_json) VALUES (?,?,?,?,?,?,?)`);
      for (const row of rows) {
        const existing = get.get(row.physicalId);
        if (existing && existing.content_hash !== row.contentHash) fail('LEXICAL_ID_CONFLICT', row.physicalId);
        if (existing) continue;
        const scopeJson = canonicalJson(row.scope);
        const citationJson = canonicalJson(row.citation);
        insertDoc.run(row.physicalId, row.logicalId, row.generation, scopeJson, row.text, row.contentHash, citationJson);
      insertFts.run(row.physicalId, row.logicalId, row.generation, scopeJson, row.text, row.contentHash, citationJson);
      }
      this.db.exec('COMMIT');
    } catch (error) {
      try { this.db.exec('ROLLBACK'); } catch { /* preserve original failure */ }
      throw error;
    }
  }

  search(query, generation, scope, limit = 20) {
    const terms = query.normalize('NFKC').match(/[\p{L}\p{N}_-]+/gu) ?? [];
    const uniqueTerms = [...new Set(terms.map((term) => term.slice(0, 64)).filter(Boolean))];
    if (uniqueTerms.length === 0) return [];
    const match = uniqueTerms.map((term) => `"${term.replaceAll('"', '""')}"`).join(' OR ');
    const rows = this.db.prepare(`SELECT physical_id,logical_id,generation,scope_json,text,content_hash,citation_json,bm25(lexical_fts) AS rank
      FROM lexical_fts WHERE lexical_fts MATCH ? AND generation = ? AND scope_json = ? ORDER BY rank LIMIT ?`)
      .all(match, generation, canonicalJson(scope), limit);
    return rows.map((row) => ({
      physicalId: row.physical_id,
      logicalId: row.logical_id,
      generation: row.generation,
      text: row.text,
      contentHash: row.content_hash,
      citation: parseJsonText(row.citation_json, 'LEXICAL_CITATION_INVALID'),
      score: 1 / (1 + Math.max(0, Number(row.rank))),
    }));
  }

  searchAll(query, generation, limit = 20) {
    const terms = query.normalize('NFKC').match(/[\p{L}\p{N}_-]+/gu) ?? [];
    const uniqueTerms = [...new Set(terms.map((term) => term.slice(0, 64)).filter(Boolean))];
    if (uniqueTerms.length === 0) return [];
    const match = uniqueTerms.map((term) => `"${term.replaceAll('"', '""')}"`).join(' OR ');
    const rows = this.db.prepare(`SELECT physical_id,logical_id,generation,scope_json,text,content_hash,citation_json,bm25(lexical_fts) AS rank
      FROM lexical_fts WHERE lexical_fts MATCH ? AND generation = ? ORDER BY rank LIMIT ?`)
      .all(match, generation, limit);
    return rows.map((row) => ({
      physicalId: row.physical_id,
      logicalId: row.logical_id,
      generation: row.generation,
      scope: parseJsonText(row.scope_json, 'LEXICAL_SCOPE_INVALID'),
      text: row.text,
      contentHash: row.content_hash,
      citation: parseJsonText(row.citation_json, 'LEXICAL_CITATION_INVALID'),
      score: 1 / (1 + Math.max(0, Number(row.rank))),
    }));
  }

  count(generation, scope) {
    return Number(this.db.prepare('SELECT COUNT(*) AS count FROM lexical_documents WHERE generation = ? AND scope_json = ?')
      .get(generation, canonicalJson(scope)).count);
  }

  close() {
    this.db.close();
  }
}

function entityKind(entity) {
  const type = String(entity?.semanticType ?? '').trim().toLowerCase();
  if (type === 'person') return 'Person';
  if (type === 'organization' || type === 'company') return 'Organization';
  if (type === 'product') return 'Product';
  return entity?.semanticType ?? '';
}

function sameScopeJson(a, b) {
  return isPlainObject(a) && isPlainObject(b) && canonicalJson(a) === canonicalJson(b);
}

function parseRows(text, code) {
  const rows = typeof text === 'string' ? parseJsonText(text, code) : text;
  if (!Array.isArray(rows)) fail(code, 'expected array');
  return rows;
}

function readPayload(row) {
  const payload = row?.payload;
  if (typeof payload === 'string') return parseJsonText(payload, 'NATIVE_PAYLOAD_INVALID');
  if (isPlainObject(payload)) return payload;
  fail('NATIVE_PAYLOAD_INVALID');
}

function fixtureFromOption(option) {
  if (!option) return undefined;
  if (typeof option === 'string') {
    if (!path.isAbsolute(option)) fail('BENCHMARK_FIXTURE_PATH_MUST_BE_ABSOLUTE');
    if (!fs.existsSync(option)) fail('BENCHMARK_FIXTURE_MISSING', option);
    return parseJsonText(fs.readFileSync(option, 'utf8'), 'BENCHMARK_FIXTURE_INVALID');
  }
  return cloneJson(option);
}

function validateBenchmarkFixture(fixture) {
  if (!fixture) return undefined;
  if (!isPlainObject(fixture) || typeof fixture.fixtureVersion !== 'string' || !Array.isArray(fixture.queries) || fixture.queries.length === 0) {
    fail('BENCHMARK_FIXTURE_INVALID');
  }
  for (const row of fixture.queries) {
    if (!isPlainObject(row) || typeof row.query !== 'string' || row.query.trim() === ''
      || !Array.isArray(row.relevantTexts) || row.relevantTexts.length === 0
      || row.relevantTexts.some((text) => typeof text !== 'string' || text.length === 0)) {
      fail('BENCHMARK_FIXTURE_QUERY_INVALID');
    }
  }
  return fixture;
}

function citationFor(source, chunk) {
  return {
    sourceId: source.sourceId,
    rawArtifactId: source.rawArtifactId,
    parsedArtifactId: source.parsedArtifactId,
    chunkId: chunk.chunkId,
    contentHash: chunk.contentHash,
  };
}

export class GenesisRag17Worker {
  constructor({
    dbPath,
    scope,
    credential,
    workerToken,
    modelDir,
    mspCall,
    mspCommand,
    pythonCommand,
    benchmarkFixture,
    faultInjector,
    port,
    host = '127.0.0.1',
    pollIntervalMs = 1000,
  } = {}) {
    if (typeof dbPath !== 'string' || !path.isAbsolute(dbPath)) fail('DB_PATH_MUST_BE_ABSOLUTE');
    validateScope(scope);
    if (typeof credential !== 'string' || credential.length === 0) fail('WORKER_CREDENTIAL_REQUIRED');
    if (typeof workerToken !== 'string' || workerToken.length === 0) fail('WORKER_TOKEN_REQUIRED');
    if (typeof modelDir !== 'string' || !path.isAbsolute(modelDir)) fail('MODEL_DIR_MUST_BE_ABSOLUTE');
    if (typeof mspCall !== 'function' && !mspCommand) fail('MSP_CALLER_REQUIRED');
    if (!Number.isInteger(pollIntervalMs) || pollIntervalMs < 10) fail('POLL_INTERVAL_INVALID');
    if (host !== '127.0.0.1' && host !== '::1') fail('WORKER_HOST_MUST_BE_LOOPBACK');
    if (port !== undefined && (!Number.isInteger(port) || port < 0 || port > 65535)) fail('WORKER_PORT_INVALID');

    this.dbPath = dbPath;
    this.scope = cloneJson(scope);
    this.credential = credential;
    this.workerToken = workerToken;
    this.modelDir = modelDir;
    this.pythonCommand = pythonCommand;
    this.mspCall = mspCall ?? createMspStdioCaller(mspCommand);
    this.benchmarkFixture = validateBenchmarkFixture(fixtureFromOption(benchmarkFixture));
    this.benchmarkFixtureHash = this.benchmarkFixture ? hashObject(this.benchmarkFixture) : undefined;
    this.faultInjector = typeof faultInjector === 'function' ? faultInjector : undefined;
    this.host = host;
    this.port = port;
    this.pollIntervalMs = pollIntervalMs;
    this.root = path.join(dbPath, 'genesisrag17');
    this.decisionsRoot = path.join(this.root, 'decisions');
    this.transactionsRoot = path.join(this.root, 'transactions');
    this.snapshotsRoot = path.join(this.root, 'snapshots');
    this.outboxRoot = path.join(this.root, 'outbox');
    this.lockPath = path.join(this.root, 'worker.lock');
    fs.mkdirSync(this.root, { recursive: true });
    fs.mkdirSync(this.decisionsRoot, { recursive: true });
    fs.mkdirSync(this.transactionsRoot, { recursive: true });
    fs.mkdirSync(this.snapshotsRoot, { recursive: true });
    fs.mkdirSync(this.outboxRoot, { recursive: true });

    this.lock = acquireWorkerLock(this.lockPath, dbPath);
    try {
      this.db = GenesisDatabase.open({
        path: dbPath,
        pageCacheMb: 64,
        readOnly: false,
        vectorDim: MODEL_DIMENSIONS,
        retention: 'full',
      });
      this.lexical = new LexicalIndex(path.join(this.root, 'lexical.sqlite'));
      this.embedder = new Embedder({ modelDir, pythonCommand });
      this.state = this.loadState();
    } catch (error) {
      releaseWorkerLock(this.lockPath, this.lock);
      throw error;
    }
    this.server = undefined;
    this.loopTimer = undefined;
    this.running = false;
    this.activeRun = undefined;
  }

  static create(options) {
    return new GenesisRag17Worker(options);
  }

  loadState() {
    const filename = stateFilePath(this.root);
    if (!fs.existsSync(filename)) return { schemaVersion: SCHEMA_VERSION, scope: cloneJson(this.scope), decisions: {} };
    const state = parseJsonText(fs.readFileSync(filename, 'utf8'), 'WORKER_STATE_INVALID');
    if (state.schemaVersion !== SCHEMA_VERSION) fail('WORKER_STATE_SCHEMA_INVALID');
    validateScope(state.scope, this.scope);
    if (!isPlainObject(state.decisions)) fail('WORKER_STATE_DECISIONS_INVALID');
    return state;
  }

  saveState() {
    writeAtomic(stateFilePath(this.root), `${JSON.stringify({
      schemaVersion: SCHEMA_VERSION,
      scope: this.scope,
      decisions: this.state.decisions,
    })}\n`);
  }

  decisionPath(decisionId) {
    return path.join(this.decisionsRoot, `${safeFilePart(decisionId)}.json`);
  }

  outboxPath(kind, decisionId) {
    return path.join(this.outboxRoot, `${safeFilePart(kind)}-${safeFilePart(decisionId)}.json`);
  }

  vectorCollectionFor(generation) {
    if (typeof generation !== 'string' || generation.length === 0) fail('VECTOR_GENERATION_INVALID');
    return `${VECTOR_COLLECTION_PREFIX}${hashObject({ scope: this.scope, generation }).slice(0, 32)}`;
  }

  writeDecision(decision) {
    writeAtomic(this.decisionPath(decision.decisionId), `${JSON.stringify(decision)}\n`);
  }

  saveOutbox(kind, decisionId, payload) {
    writeAtomic(this.outboxPath(kind, decisionId), `${JSON.stringify({ kind, decisionId, payload })}\n`);
  }

  removeOutbox(kind, decisionId) {
    fs.rmSync(this.outboxPath(kind, decisionId), { force: true });
    fsyncDirectory(this.outboxRoot);
  }

  transactionIntentPath(decisionId, phase) {
    if (typeof decisionId !== 'string' || decisionId.length === 0) fail('TRANSACTION_DECISION_ID_INVALID');
    if (phase !== 'graph' && phase !== 'final') fail('TRANSACTION_PHASE_INVALID', phase);
    return path.join(this.transactionsRoot, `${safeFilePart(phase)}-${safeFilePart(decisionId)}.json`);
  }

  transactionIdFor(decision, phase) {
    return `txn-${phase}-${hashObject({ decisionId: decision.decisionId, decisionHash: decision.decisionHash, phase }).slice(0, 44)}`;
  }

  saveTransactionIntent(decision, phase, payloadJson, candidate) {
    if (typeof payloadJson !== 'string' || payloadJson.length === 0) fail('TRANSACTION_PAYLOAD_INVALID');
    const transaction = parseJsonText(payloadJson, 'TRANSACTION_PAYLOAD_INVALID');
    if (!isPlainObject(transaction) || typeof transaction.transaction_id !== 'string'
      || !Number.isSafeInteger(transaction.expected_frontier) || transaction.expected_frontier < 0) {
      fail('TRANSACTION_INTENT_INVALID');
    }
    const expectedId = this.transactionIdFor(decision, phase);
    if (transaction.transaction_id !== expectedId) fail('TRANSACTION_ID_INVALID');
    const intent = {
      schemaVersion: SCHEMA_VERSION,
      scope: cloneJson(this.scope),
      decisionId: decision.decisionId,
      decisionHash: decision.decisionHash,
      phase,
      transactionId: transaction.transaction_id,
      transaction_id: transaction.transaction_id,
      expectedFrontier: transaction.expected_frontier,
      expected_frontier: transaction.expected_frontier,
      payloadJson,
      payloadHash: hashText(payloadJson),
      generation: candidate.generation,
      snapshotId: candidate.snapshotId,
      vectorCollection: candidate.vectorCollection,
      expectedNodeCount: candidate.expectedNodeCount,
      expectedEdgeCount: candidate.expectedEdgeCount,
      expectedVectorCount: candidate.expectedVectorCount,
      transaction: cloneJson(transaction),
    };
    writeAtomic(this.transactionIntentPath(decision.decisionId, phase), `${JSON.stringify(intent)}\n`);
    return { ...intent, transaction };
  }

  readTransactionIntent(decision, phase) {
    const filename = this.transactionIntentPath(decision.decisionId, phase);
    if (!fs.existsSync(filename)) return undefined;
    const intent = parseJsonText(fs.readFileSync(filename, 'utf8'), 'TRANSACTION_INTENT_INVALID');
    if (!isPlainObject(intent) || intent.schemaVersion !== SCHEMA_VERSION
      || intent.decisionId !== decision.decisionId || intent.decisionHash !== decision.decisionHash
      || intent.phase !== phase || !sameScopeJson(intent.scope, this.scope)
      || typeof intent.payloadJson !== 'string' || intent.payloadJson.length === 0
      || intent.payloadHash !== hashText(intent.payloadJson)
      || intent.transactionId !== this.transactionIdFor(decision, phase)
      || (intent.transaction_id !== undefined && intent.transaction_id !== intent.transactionId)
      || (intent.expected_frontier !== undefined && intent.expected_frontier !== intent.expectedFrontier)
      || !Number.isSafeInteger(intent.expectedFrontier) || intent.expectedFrontier < 0) {
      fail('TRANSACTION_INTENT_IDENTITY_INVALID');
    }
    const transaction = parseJsonText(intent.payloadJson, 'TRANSACTION_PAYLOAD_INVALID');
    if (!isPlainObject(transaction) || transaction.transaction_id !== intent.transactionId
      || transaction.expected_frontier !== intent.expectedFrontier) {
      fail('TRANSACTION_INTENT_PAYLOAD_MISMATCH');
    }
    if (intent.transaction !== undefined && canonicalJson(intent.transaction) !== canonicalJson(transaction)) {
      fail('TRANSACTION_INTENT_PAYLOAD_MISMATCH');
    }
    return { ...intent, transaction };
  }

  removeTransactionIntent(decisionId, phase) {
    fs.rmSync(this.transactionIntentPath(decisionId, phase), { force: true });
    fsyncDirectory(this.transactionsRoot);
  }

  async ensureEmbedder(vectorCollection) {
    if (!this.artifactHashes) this.artifactHashes = verifyModelArtifacts(this.modelDir);
    if (typeof vectorCollection !== 'string' || vectorCollection.length === 0) fail('VECTOR_COLLECTION_REQUIRED');
    const existing = this.db.listCollections().find((collection) => collection.name === vectorCollection);
    if (!existing) {
      await this.db.createCollection(vectorCollection, MODEL_ID, MODEL_DIMENSIONS, MODEL_METRIC, null, 100, false);
      // Collection declarations live in the native manifest, while a later
      // vector transaction can replay independently after a crash. Checkpoint
      // the declaration before that first vector commit so replay cannot
      // auto-provision the collection with the engine's recovery defaults.
      await this.db.saveState();
    } else if (existing.dim !== MODEL_DIMENSIONS || existing.metric.toLowerCase() !== MODEL_METRIC) {
      fail('NATIVE_VECTOR_COLLECTION_MISMATCH');
    }
    await this.embedder.start();
  }

  pipelineArgs(extra) {
    return { schemaVersion: SCHEMA_VERSION, scope: cloneJson(this.scope), credential: this.credential, ...extra };
  }

  async callMsp(name, args) {
    const result = await this.mspCall(name, this.pipelineArgs(args));
    if (!isPlainObject(result)) fail('MSP_RESPONSE_INVALID', name);
    if (result.schemaVersion !== SCHEMA_VERSION || !sameScopeJson(result.scope, this.scope)) {
      fail('MSP_RESPONSE_IDENTITY_MISMATCH', name);
    }
    return result;
  }

  async claim() {
    const result = await this.callMsp('msp_pipeline_claim', { limit: 1 });
    if (!Array.isArray(result.decisions)) fail('MSP_CLAIM_RESPONSE_INVALID');
    return result.decisions;
  }

  makeNode(id, labels, props, validFrom = nowIso()) {
    return { id, labels, props, valid_from: validFrom };
  }

  makeEdge(id, from, to, rel, props, validFrom = nowIso()) {
    return { id, from, to, rel, props, valid_from: validFrom };
  }

  buildPhysicalCandidate(decision, validated, vectors) {
    const { chunkById, entityById } = validated;
    const generation = `g17-${hashObject({ decisionId: decision.decisionId, decisionHash: decision.decisionHash }).slice(0, 32)}`;
    const snapshotId = `snap-${hashObject({ scope: decision.scope, generation }).slice(0, 32)}`;
    const nodes = [];
    const edges = [];
    const nodeByLogical = new Map();
    const addNode = (kind, logicalId, labels, props, validFrom) => {
      const id = physicalId(generation, kind, logicalId);
      nodeByLogical.set(`${kind}:${logicalId}`, id);
      nodes.push(this.makeNode(id, labels, {
        ...props,
        scope: cloneJson(decision.scope),
        generation,
        snapshotId,
        logicalId,
      }, validFrom));
      return id;
    };
    const addEdge = (kind, logicalId, from, to, rel, props, validFrom) => {
      if (!from || !to) fail('GRAPH_ENDPOINT_MISSING', logicalId);
      edges.push(this.makeEdge(physicalId(generation, kind, logicalId), from, to, rel, {
        ...props,
        scope: cloneJson(decision.scope),
        generation,
        snapshotId,
        logicalId,
      }, validFrom));
    };

    const source = decision.source;
    const sourcePhysicalId = addNode('source', source.sourceId, ['RawExternalRecord'], {
      objectType: 'source', sourceId: source.sourceId, rawArtifactId: source.rawArtifactId,
      parsedArtifactId: source.parsedArtifactId, documentId: source.documentId, version: source.version,
      contentHash: source.contentHash, text: source.content,
    });
    const parsedPhysicalId = addNode('parsed', source.parsedArtifactId, ['ParsedArtifact'], {
      objectType: 'parsedArtifact', sourceId: source.sourceId, rawArtifactId: source.rawArtifactId,
      parsedArtifactId: source.parsedArtifactId, contentHash: source.contentHash,
    });
    addEdge('lineage', `${source.sourceId}->${source.parsedArtifactId}`, sourcePhysicalId, parsedPhysicalId, 'PARSED_AS', {
      sourceId: source.sourceId, rawArtifactId: source.rawArtifactId, parsedArtifactId: source.parsedArtifactId,
    });

    const chunkPhysicalById = new Map();
    const lexicalRows = [];
    const chunkVectorByPhysical = new Map();
    for (const [index, chunk] of decision.chunks.entries()) {
      const citation = citationFor(source, chunk);
      const chunkPhysicalId = addNode('chunk', chunk.chunkId, ['Chunk'], {
        objectType: 'chunk', sourceId: source.sourceId, rawArtifactId: source.rawArtifactId,
        parsedArtifactId: source.parsedArtifactId, chunkId: chunk.chunkId, ordinal: chunk.ordinal,
        startOffset: chunk.startOffset, endOffset: chunk.endOffset, contentHash: chunk.contentHash,
        text: chunk.text, citation,
      });
      chunkPhysicalById.set(chunk.chunkId, chunkPhysicalId);
      addEdge('lineage', `${source.parsedArtifactId}->${chunk.chunkId}`, parsedPhysicalId, chunkPhysicalId, 'CONTAINS_CHUNK', {
        sourceId: source.sourceId, rawArtifactId: source.rawArtifactId, parsedArtifactId: source.parsedArtifactId,
        chunkId: chunk.chunkId,
      });
      lexicalRows.push({
        physicalId: chunkPhysicalId,
        logicalId: chunk.chunkId,
        generation,
        scope: decision.scope,
        text: chunk.text,
        contentHash: chunk.contentHash,
        citation,
      });
      if (vectors[index]) chunkVectorByPhysical.set(chunkPhysicalId, vectors[index]);
    }

    for (const entity of decision.entities) {
      const id = addNode('entity', entity.id, ['Entity', entityKind(entity)], {
        objectType: 'entity', entityId: entity.id, name: entity.name, semanticType: entity.semanticType,
        mentions: entity.mentions,
      });
      for (const mentionId of entity.mentions) {
        const mention = (decision.mentions ?? []).find((candidate) => candidate.sourceMentionId === mentionId);
        if (!mention) continue;
        const chunkId = chunkPhysicalById.get(mention.chunkId);
        if (!chunkId) continue;
        addEdge('mention', mention.sourceMentionId, id, chunkId, 'MENTIONED_IN', {
          sourceMentionId: mention.sourceMentionId, semanticType: mention.semanticType,
          resolutionKey: mention.resolutionKey, sourceId: source.sourceId,
          rawArtifactId: source.rawArtifactId, parsedArtifactId: source.parsedArtifactId,
          chunkId: mention.chunkId,
        });
      }
    }

    const quarantinedFacts = decision.held.length;
    const acceptedFacts = [];
    for (const fact of decision.facts) {
      const factId = fact.id ?? fact.factId;
      const predicate = fact.predicate;
      const confidence = Number(fact.confidence ?? 0);
      const subject = entityById.get(fact.subjectId);
      const object = fact.objectId === undefined ? undefined : entityById.get(fact.objectId);
      const validEndpoints = predicate === 'WORKS_FOR'
        ? entityKind(subject) === 'Person' && entityKind(object) === 'Organization'
        : predicate === 'PURCHASED'
          ? ['Person', 'Organization'].includes(entityKind(subject)) && entityKind(object) === 'Product'
          : false;
      if (!predicate || confidence < 0.8 || !validEndpoints) fail('FACT_ENDPOINT_INVALID', factId);
      acceptedFacts.push(fact);
      const refs = sourceRefsOf(fact);
      const factPhysicalId = addNode('fact', factId, ['Fact'], {
        objectType: 'fact', factId, predicate, confidence,
        subjectId: fact.subjectId, objectId: fact.objectId, value: fact.value,
        sourceReferences: refs, temporal: fact.temporal ?? { status: 'not_applicable' },
      }, temporalValidFrom(fact));
      const subjectId = nodeByLogical.get(`entity:${fact.subjectId}`);
      const objectId = fact.objectId === undefined ? undefined : nodeByLogical.get(`entity:${fact.objectId}`);
      const assertionTo = objectId ?? factPhysicalId;
      addEdge('assertion', factId, subjectId, assertionTo, predicate, {
        factId, sourceReferences: refs, confidence, temporal: fact.temporal ?? { status: 'not_applicable' },
      }, temporalValidFrom(fact));
      const evidenceChunk = chunkPhysicalById.get(refs.chunkId);
      if (evidenceChunk) addEdge('evidence', `${factId}:${refs.chunkId}`, evidenceChunk, factPhysicalId, 'EVIDENCE_FOR', { sourceReferences: refs }, temporalValidFrom(fact));
    }

    for (const held of decision.held) {
      const heldId = held.id ?? held.factId;
      const refs = sourceRefsOf(held);
      const heldPhysicalId = addNode('held', heldId, ['HeldFact'], {
        objectType: 'heldFact', factId: heldId, predicate: held.predicate,
        confidence: Number(held.confidence ?? 0), subjectId: held.subjectId, objectId: held.objectId,
        value: held.value, held: true, reason: held.reason ?? 'below_write_floor', sourceReferences: refs,
        temporal: held.temporal ?? { status: 'not_applicable' },
      }, temporalValidFrom(held));
      const evidenceChunk = chunkPhysicalById.get(refs.chunkId);
      if (evidenceChunk) addEdge('evidence', `${heldId}:${refs.chunkId}`, evidenceChunk, heldPhysicalId, 'HELD_EVIDENCE', { sourceReferences: refs }, temporalValidFrom(held));
    }

    for (const derived of decision.derived) {
      const derivedId = derived.id ?? derived.derivedId;
      const references = sourceRefsListOf(derived);
      const refs = references[0];
      const derivedPhysicalId = addNode('derived', derivedId, ['Derived'], {
        objectType: 'derived', derivedId, kind: derived.kind ?? 'enrichment',
        entityId: derived.entityId, value: derived.value, count: derived.count,
        sourceReferences: references, temporal: derived.temporal ?? { status: 'not_applicable' },
      }, temporalValidFrom(derived));
      if (derived.entityId && nodeByLogical.has(`entity:${derived.entityId}`)) {
        addEdge('derived', derivedId, nodeByLogical.get(`entity:${derived.entityId}`), derivedPhysicalId, 'HAS_DERIVED', { sourceReferences: references }, temporalValidFrom(derived));
      }
      for (const reference of references) {
        const evidenceChunk = chunkPhysicalById.get(reference.chunkId);
        if (evidenceChunk) addEdge('evidence', `${derivedId}:${reference.chunkId}`, evidenceChunk, derivedPhysicalId, 'EVIDENCE_FOR', { sourceReferences: reference }, temporalValidFrom(derived));
      }
    }

    return {
      generation,
      snapshotId,
      vectorCollection: this.vectorCollectionFor(generation),
      nodes,
      edges,
      lexicalRows,
      chunkVectorByPhysical,
      acceptedFacts,
      quarantinedFacts,
      chunkPhysicalById,
      expectedNodeCount: nodes.length,
      expectedEdgeCount: edges.length,
      expectedVectorCount: chunkVectorByPhysical.size,
    };
  }

  buildDerivedCandidate(decision, baseCandidate) {
    const nodes = [];
    const edges = [];
    const addNode = (kind, logicalId, labels, props, validFrom) => {
      const id = physicalId(baseCandidate.generation, kind, logicalId);
      nodes.push(this.makeNode(id, labels, {
        ...props,
        scope: cloneJson(decision.scope),
        generation: baseCandidate.generation,
        snapshotId: baseCandidate.snapshotId,
        logicalId,
      }, validFrom));
      return id;
    };
    const addEdge = (kind, logicalId, from, to, rel, props, validFrom) => {
      if (!from || !to) fail('DERIVED_GRAPH_ENDPOINT_MISSING', logicalId);
      edges.push(this.makeEdge(physicalId(baseCandidate.generation, kind, logicalId), from, to, rel, {
        ...props,
        scope: cloneJson(decision.scope),
        generation: baseCandidate.generation,
        snapshotId: baseCandidate.snapshotId,
        logicalId,
      }, validFrom));
    };
    const chunkPhysicalById = baseCandidate.chunkPhysicalById;
    for (const derived of decision.derived) {
      const derivedId = derived.id ?? derived.derivedId;
      const references = sourceRefsListOf(derived);
      const refs = references[0];
      const derivedPhysicalId = addNode('derived', derivedId, ['Derived'], {
        objectType: 'derived', derivedId, kind: derived.kind ?? 'enrichment',
        entityId: derived.entityId, value: derived.value, count: derived.count,
        sourceReferences: references, temporal: derived.temporal ?? { status: 'not_applicable' },
      }, temporalValidFrom(derived));
      if (derived.entityId) {
        addEdge('derived', derivedId, physicalId(baseCandidate.generation, 'entity', derived.entityId), derivedPhysicalId, 'HAS_DERIVED', { sourceReferences: references }, temporalValidFrom(derived));
      }
      for (const reference of references) {
        const evidenceChunk = chunkPhysicalById.get(reference.chunkId);
        if (evidenceChunk) addEdge('evidence', `${derivedId}:${reference.chunkId}`, evidenceChunk, derivedPhysicalId, 'EVIDENCE_FOR', { sourceReferences: reference }, temporalValidFrom(derived));
      }
    }
    return {
      generation: baseCandidate.generation,
      snapshotId: baseCandidate.snapshotId,
      vectorCollection: baseCandidate.vectorCollection,
      nodes,
      edges,
      lexicalRows: [],
      chunkVectorByPhysical: new Map(),
      acceptedFacts: [],
      quarantinedFacts: decision.held.length,
      chunkPhysicalById: baseCandidate.chunkPhysicalById,
      expectedNodeCount: nodes.length,
      expectedEdgeCount: edges.length,
      expectedVectorCount: 0,
    };
  }

  async commitCandidate(decision, candidate, phase = 'final') {
    const persistedIntent = this.readTransactionIntent(decision, phase);
    let intent = persistedIntent;
    if (!intent) {
      const transactionId = this.transactionIdFor(decision, phase);
      const vectorRows = [...candidate.chunkVectorByPhysical.entries()].map(([nodeId, embedding]) => ({
        node_id: nodeId,
        collection: candidate.vectorCollection,
        embedding,
      }));
      const expectedFrontier = Number(this.db.txnFrontier());
      if (!Number.isSafeInteger(expectedFrontier) || expectedFrontier < 0) fail('NATIVE_FRONTIER_INVALID');
      const transaction = {
        transaction_id: transactionId,
        expected_frontier: expectedFrontier,
        relational: [],
        graph: { nodes: candidate.nodes, edges: candidate.edges },
        vectors: vectorRows,
      };
      intent = this.saveTransactionIntent(decision, phase, JSON.stringify(transaction), candidate);
    }
    const nativeResult = parseJsonText(await this.db.commitTransaction(intent.payloadJson), 'NATIVE_COMMIT_RESULT_INVALID');
    const transaction = intent.transaction;
    const transactionId = intent.transactionId;
    if (nativeResult.transaction_id !== transactionId && nativeResult.transactionId !== transactionId) fail('NATIVE_TRANSACTION_ID_MISMATCH');
    await this.faultInjector?.('after-native-commit-before-receipt', {
      phase,
      decisionId: decision.decisionId,
      transaction: cloneJson(transaction),
      payloadJson: intent.payloadJson,
    });
    await this.db.flushIndex();
    await this.db.saveState();
    const commitSequence = nativeResult.commit_sequence ?? nativeResult.commitSequence;
    return {
      id: transactionId,
      frontier: parseNativeFrontier(commitSequence),
      checkpoint: parseNativeFrontier(this.db.stableFrontier()),
      vectorRows: transaction.vectors,
    };
  }

  async nativeReadback(candidate, decision) {
    const propsRows = parseRows(await this.db.querySql('SELECT node_u32,payload FROM props', '[]'), 'NATIVE_PROPS_READBACK_INVALID');
    const nodes = propsRows.map((row) => readPayload(row))
      .filter((props) => sameScopeJson(props.scope, decision.scope) && props.generation === candidate.generation);
    const edgeRows = parseRows(await this.db.querySql('SELECT id,from_id,to_id,rel,props FROM edges', '[]'), 'NATIVE_EDGES_READBACK_INVALID');
    const edges = edgeRows.map((row) => ({ ...row, props: typeof row.props === 'string' ? parseJsonText(row.props, 'NATIVE_EDGE_PAYLOAD_INVALID') : row.props }))
      .filter((row) => sameScopeJson(row.props?.scope, decision.scope) && row.props?.generation === candidate.generation);
    if (nodes.length === 0 || edges.length === 0) fail('NATIVE_READBACK_EMPTY');
    const chunkVectors = [...candidate.chunkVectorByPhysical.entries()];
    let vectorCount = 0;
    if (chunkVectors.length > 0) {
      const vectorResults = await this.db.hybridSearch({
        queryVector: chunkVectors[0][1],
        k: Math.max(chunkVectors.length, 5),
        alpha: 0,
        collection: candidate.vectorCollection,
      });
      const expectedIds = new Set(chunkVectors.map(([id]) => id));
      vectorCount = new Set(vectorResults
        .filter((result) => expectedIds.has(result?.node?.id)
          && sameScopeJson(result.node.props?.scope, decision.scope)
          && result.node.props?.generation === candidate.generation)
        .map((result) => result.node.id)).size;
      if (vectorCount !== chunkVectors.length) fail('NATIVE_VECTOR_READBACK_INCOMPLETE', `${vectorCount}/${chunkVectors.length}`);
    }
    const citationCount = nodes.filter((props) => props.citation && props.citation.contentHash === hashText(props.text ?? '')).length;
    if (citationCount < decision.chunks.length) fail('NATIVE_CITATION_READBACK_INCOMPLETE', `${citationCount}/${decision.chunks.length}`);
    const firstNode = nodes.find((props) => props.objectType === 'chunk');
    const firstPhysicalId = firstNode?.logicalId ? physicalId(candidate.generation, 'chunk', firstNode.logicalId) : undefined;
    if (firstPhysicalId) {
      const versions = await this.db.nodeVersions(firstPhysicalId, null);
      if (!versions || !Array.isArray(versions.versions) || versions.versions.length === 0) fail('NATIVE_VERSION_READBACK_EMPTY');
    }
    return {
      ok: true,
      nodeCount: nodes.length,
      edgeCount: edges.length,
      vectorCount,
      citationCount,
    };
  }

  async searchGeneration(generation, snapshotId, query, topK = 5, vectorCollection = this.vectorCollectionFor(generation)) {
    if (typeof query !== 'string' || query.trim() === '') fail('QUERY_EMPTY');
    const lexicalResults = this.lexical.search(query, generation, this.scope, Math.max(20, topK * 4));
    const queryVector = await this.embedder.embed([query], 'query');
    let vectorResults = [];
    try {
      vectorResults = await this.db.hybridSearch({ queryVector: queryVector[0], k: Math.max(20, topK * 4), alpha: 0, collection: vectorCollection });
    } catch (error) {
      // A published snapshot with no vectors is valid only when its policy
      // explicitly denied embedding; lexical-only search remains honest.
      if (lexicalResults.length === 0) throw error;
    }
    const byId = new Map();
    for (const row of lexicalResults) {
      byId.set(row.physicalId, { ...row, vectorScore: 0, lexicalScore: row.score });
    }
    for (const result of vectorResults) {
      const props = result?.node?.props;
      if (!result?.node?.id || !sameScopeJson(props?.scope, this.scope) || props?.generation !== generation) continue;
      const citation = props.citation;
      const row = byId.get(result.node.id) ?? {
        physicalId: result.node.id,
        logicalId: props.chunkId ?? props.logicalId,
        generation,
        text: props.text,
        contentHash: props.contentHash,
        citation,
        lexicalScore: 0,
      };
      row.vectorScore = Math.max(0, Number(result.score ?? 0));
      row.text = props.text;
      row.contentHash = props.contentHash;
      row.citation = citation;
      byId.set(result.node.id, row);
    }
    const ranked = [...byId.values()]
      .filter((row) => row.generation === generation && row.citation && row.contentHash === hashText(row.text ?? ''))
      .map((row) => ({ ...row, score: (row.vectorScore * 0.7) + (row.lexicalScore * 0.3) }))
      .sort((a, b) => b.score - a.score || a.logicalId.localeCompare(b.logicalId))
      .slice(0, topK);
    return ranked.map((row) => ({
      id: row.logicalId,
      score: row.score,
      text: row.text,
      citation: row.citation,
      snapshotId,
      generation,
    }));
  }

  async benchmark(candidate) {
    const fixture = validateBenchmarkFixture(this.benchmarkFixture);
    if (!fixture) fail('BENCHMARK_FIXTURE_REQUIRED');
    const queryRows = [];
    const persisted = new Map(candidate.lexicalRows.map((row) => [row.chunkId ?? row.logicalId, row]));
    let crossTenantLeaks = 0;
    for (const query of fixture.queries) {
      const results = await this.searchGeneration(candidate.generation, candidate.snapshotId, query.query, 5);
      const relevant = new Set(query.relevantTexts);
      const retrievedRelevant = new Set(results.filter((result) => relevant.has(result.text)).map((result) => result.text));
      const first = results.findIndex((result) => relevant.has(result.text));
      const citationCorrect = results.every((result) => {
        const citation = result.citation;
        const stored = persisted.get(citation?.chunkId);
        return Boolean(stored && citation.sourceId === stored.citation.sourceId
          && citation.rawArtifactId === stored.citation.rawArtifactId
          && citation.parsedArtifactId === stored.citation.parsedArtifactId
          && citation.contentHash === stored.contentHash
          && citation.contentHash === hashText(result.text));
      });
      const unscopedLexical = this.lexical.searchAll(query.query, candidate.generation, 20);
      crossTenantLeaks += unscopedLexical.filter((row) => !sameScopeJson(row.scope, this.scope)).length;
      const queryVector = await this.embedder.embed([query.query], 'query');
      const unscopedVector = await this.db.hybridSearch({ queryVector: queryVector[0], k: 20, alpha: 0, collection: candidate.vectorCollection });
      crossTenantLeaks += unscopedVector.filter((row) => !sameScopeJson(row.node?.props?.scope, this.scope)).length;
      queryRows.push({ retrieved: retrievedRelevant.size, totalRelevant: relevant.size, rank: first, citationCorrect, results });
    }
    if (queryRows.length === 0) fail('BENCHMARK_EMPTY');
    const retrieved = queryRows.reduce((sum, row) => sum + row.retrieved, 0);
    const totalRelevant = queryRows.reduce((sum, row) => sum + row.totalRelevant, 0);
    const mrr = queryRows.reduce((sum, row) => sum + (row.rank >= 0 ? 1 / (row.rank + 1) : 0), 0) / queryRows.length;
    const citationCorrectness = queryRows.every((row) => row.citationCorrect) ? 1 : 0;
    const metrics = {
      fixtureVersion: fixture.fixtureVersion,
      queryCount: queryRows.length,
      recallAt5: totalRelevant > 0 ? retrieved / totalRelevant : 0,
      mrr,
      citationCorrectness,
      crossTenantLeaks,
    };
    if (!Number.isFinite(metrics.recallAt5) || !Number.isFinite(metrics.mrr)) fail('BENCHMARK_METRICS_INVALID');
    return metrics;
  }

  temporalRows(decision) {
    return [
      ...decision.facts.map((row) => ({ kind: 'fact', row, id: row.id ?? row.factId })),
      ...decision.held.map((row) => ({ kind: 'held', row, id: row.id ?? row.factId })),
    ];
  }

  async verifyTemporalLane(candidate, decision) {
    const rows = this.temporalRows(decision);
    if (rows.length === 0) {
      return { status: 'not_applicable', reason: 'no_temporal_assertions_in_fixture', objects: 0 };
    }
    const classifications = rows.map((entry) => ({ ...entry, classification: temporalClassification(entry.row) }));
    if (classifications.some(({ classification }) => classification === 'unsupported')) {
      return { status: 'unsupported', reason: 'temporal_mapping_invalid', objects: 0 };
    }
    const mapped = classifications.filter(({ classification }) => classification === 'mapped');
    if (mapped.length === 0) {
      return { status: 'not_applicable', reason: 'all_temporal_assertions_not_applicable', objects: 0 };
    }
    if (mapped.some(({ row }) => !temporalValidFrom(row))) {
      return { status: 'unsupported', reason: 'temporal_mapping_missing_valid_from', objects: 0 };
    }
    if (typeof this.db.queryIrCapabilities !== 'function' || typeof this.db.executeQueryIr !== 'function') {
      return { status: 'unsupported', reason: 'native_temporal_query_api_unavailable', objects: 0 };
    }
    let capabilities;
    try {
      capabilities = this.db.queryIrCapabilities();
    } catch {
      return { status: 'unsupported', reason: 'native_temporal_query_api_unavailable', objects: 0 };
    }
    if (capabilities?.operations?.traverse !== 'implemented') {
      return { status: 'unsupported', reason: 'native_temporal_traverse_unavailable', objects: 0 };
    }
    const now = nowIso();
    for (const { kind, row, id } of mapped) {
      const nodeId = physicalId(candidate.generation, kind, id);
      const validFrom = temporalValidFrom(row);
      const expectedVisible = Date.parse(validFrom) <= Date.parse(now);
      let result;
      try {
        result = await this.db.executeQueryIr({
          contract_version: 'query-ir.v1',
          request_id: `g17-temporal-${hashObject({ generation: candidate.generation, kind, id, validFrom }).slice(0, 32)}`,
          temporal: { valid_at: now },
          consistency: { index: 'read_your_write' },
          operation: {
            kind: 'traverse',
            seed_id: nodeId,
            depth: 1,
            relations: [kind === 'held' ? 'HELD_EVIDENCE' : 'EVIDENCE_FOR'],
            direction: 'both',
            limit: 1,
          },
        });
      } catch {
        return { status: 'unsupported', reason: 'native_temporal_query_failed', objects: 0 };
      }
      if (!result || result.contract_version !== 'query-ir.v1' || result.status !== 'ok' || !Array.isArray(result.data)) {
        return { status: 'unsupported', reason: 'native_temporal_query_invalid', objects: 0 };
      }
      if (expectedVisible !== (result.data.length > 0)) {
        return { status: 'unsupported', reason: 'native_temporal_visibility_mismatch', objects: 0 };
      }
    }
    return { status: 'ready', reason: 'native_query_ir_temporal_readback', objects: mapped.length };
  }

  laneManifest(candidate, decision, readback, temporal) {
    const provenanceReady = decision.chunks.length > 0 && [...decision.facts, ...decision.held, ...decision.derived]
      .every((row) => row.sourceReferences || row.sourceRefs || row.provenance);
    return {
      vector: {
        status: candidate.expectedVectorCount > 0 ? 'ready' : 'not_applicable',
        reason: candidate.expectedVectorCount > 0 ? 'native_hnsw_after_flush' : 'policy_denied_or_no_chunks',
        objects: candidate.expectedVectorCount,
      },
      lexical: {
        status: candidate.lexicalRows.length > 0 ? 'ready' : 'not_applicable',
        reason: 'worker_sqlite_fts5',
        objects: this.lexical.count(candidate.generation, decision.scope),
      },
      graph: {
        status: readback.edgeCount > 0 && readback.nodeCount > 0 ? 'ready' : 'unsupported',
        reason: 'native_graph_projection_readback',
        objects: readback.edgeCount,
      },
      sqlite: {
        status: readback.nodeCount > 0 ? 'ready' : 'unsupported',
        reason: 'native_projection_sql_readback',
        objects: readback.nodeCount,
      },
      bitemporal: {
        status: temporal.status,
        reason: temporal.reason,
        objects: temporal.objects,
      },
      provenance: {
        status: provenanceReady && readback.citationCount >= decision.chunks.length ? 'ready' : 'unsupported',
        reason: provenanceReady ? 'source_refs_and_citations_verified' : 'source_refs_missing',
        objects: readback.citationCount,
      },
    };
  }

  makeReceipt(decision, candidate, transaction, readback, laneManifest, benchmark, metrics, graphReceiptHash, derivedHash, executionTimes) {
    const receipt = {
      schemaVersion: SCHEMA_VERSION,
      scope: cloneJson(this.scope),
      runId: decision.runId,
      decisionId: decision.decisionId,
      decisionHash: decision.decisionHash,
      stages: cloneJson(decision.stages),
      snapshotId: candidate.snapshotId,
      generation: candidate.generation,
      model: {
        id: MODEL_ID,
        revision: MODEL_REVISION,
        dimensions: MODEL_DIMENSIONS,
        metric: MODEL_METRIC,
        artifactHashes: cloneJson(this.artifactHashes),
      },
      transaction: {
        id: transaction.id,
        frontier: transaction.frontier,
        checkpoint: transaction.checkpoint,
      },
      readback,
      laneManifest,
      metrics,
      benchmark,
    };
    if (graphReceiptHash !== undefined) receipt.graphReceiptHash = graphReceiptHash;
    if (derivedHash !== undefined) receipt.derivedHash = derivedHash;
    if (executionTimes !== undefined) receipt.executionTimes = executionTimes;
    return receipt;
  }

  recordDecision(decision, status, extra = {}) {
    const previous = this.state.decisions[decision.decisionId] ?? {};
    this.state.decisions[decision.decisionId] = {
      ...previous,
      decisionId: decision.decisionId,
      decisionHash: decision.decisionHash,
      batchId: decision.batchId,
      runId: decision.runId,
      status,
      updatedAt: nowIso(),
      ...extra,
    };
    this.saveState();
  }

  recordStored(decisionId, status, extra = {}) {
    const previous = this.state.decisions[decisionId] ?? { decisionId };
    this.state.decisions[decisionId] = { ...previous, status, updatedAt: nowIso(), ...extra };
    this.saveState();
  }

  stageFor(decision, stageNumber) {
    const stage = decision.stages.find((candidate) => candidate.stageNumber === stageNumber);
    if (!stage) fail('STAGE_IDENTITY_MISSING', String(stageNumber));
    return cloneJson(stage);
  }

  async reportStageFailure(decision, stageNumber, startedAt, metrics, error) {
    const previous = this.state.decisions[decision.decisionId];
    if (previous?.stageFailure && Number(previous.failedStage) !== stageNumber) {
      fail('STAGE_FAILURE_CONFLICT', `${previous.failedStage}:${stageNumber}`);
    }
    // A transport interruption can re-enter the same stage after its failure
    // payload was already persisted locally. Reuse that immutable payload and
    // hash instead of creating a new interval/error object that MSP would
    // correctly reject as different terminal evidence.
    const failure = previous?.stageFailure ?? (() => {
      const stage = this.stageFor(decision, stageNumber);
      const finishedAt = nowIso();
      return {
        schemaVersion: SCHEMA_VERSION,
        scope: cloneJson(this.scope),
        runId: decision.runId,
        decisionId: decision.decisionId,
        decisionHash: decision.decisionHash,
        stage,
        startedAt,
        finishedAt,
        metrics: nonNegativeMetrics(
          metrics.records_in,
          metrics.records_out,
          metrics.records_quarantined,
          metrics.error_count,
          metrics.retry_count,
          metrics.duration_ms,
        ),
        error: {
          code: stageFailureCode(error),
          message: stageFailureMessage(error, [this.credential, this.workerToken]),
        },
      };
    })();
    const failureHash = previous?.stageFailureHash ?? hashObject(failure);
    const kind = `stage-failure-${stageNumber}`;
    const payload = { failure, failureHash };
    this.saveOutbox(kind, decision.decisionId, payload);
    this.recordDecision(decision, 'stage_failure_pending', {
      failedStage: stageNumber,
      stageFailure: failure,
      stageFailureHash: failureHash,
      lastError: failure.error.message,
    });
    try {
      const response = await this.callMsp('msp_pipeline_stage_failure', failure);
      if (response.accepted !== true) fail('MSP_STAGE_FAILURE_NOT_ACCEPTED');
      if (response.failureHash !== undefined && response.failureHash !== failureHash) {
        fail('MSP_STAGE_FAILURE_HASH_MISMATCH');
      }
      this.removeOutbox(kind, decision.decisionId);
      this.recordDecision(decision, 'failed', {
        failedStage: stageNumber,
        stageFailure: failure,
        stageFailureHash: failureHash,
      });
      return {
        status: 'failed',
        decisionId: decision.decisionId,
        stageNumber,
        error: failure.error,
      };
    } catch (reportError) {
      this.recordDecision(decision, 'stage_failure_pending', {
        failedStage: stageNumber,
        stageFailure: failure,
        stageFailureHash: failureHash,
        lastError: reportError.message,
      });
      throw reportError;
    }
  }

  makeGraphReceipt(decision, candidate, transaction, readback, durationMs) {
    return {
      schemaVersion: SCHEMA_VERSION,
      scope: cloneJson(this.scope),
      runId: decision.runId,
      decisionId: decision.decisionId,
      decisionHash: decision.decisionHash,
      stages: cloneJson(decision.stages),
      transaction: {
        id: transaction.id,
        frontier: transaction.frontier,
        checkpoint: transaction.checkpoint,
      },
      readback: {
        ok: readback.ok,
        nodeCount: readback.nodeCount,
        edgeCount: readback.edgeCount,
      },
      metrics: nonNegativeMetrics(decision.chunks.length, candidate.expectedNodeCount, candidate.quarantinedFacts, 0, 0, durationMs),
      startedAt: new Date(Date.now() - durationMs).toISOString(),
      finishedAt: nowIso(),
    };
  }

  async resumeFromReceipt(decision, known) {
    const receipt = known.receipt;
    const receiptHash = hashObject(receipt);
    if (receiptHash !== known.receiptHash || receipt.decisionId !== decision.decisionId
      || receipt.decisionHash !== decision.decisionHash || receipt.generation === undefined
      || receipt.snapshotId === undefined || !sameScopeJson(receipt.scope, this.scope)) {
      fail('PERSISTED_RECEIPT_IDENTITY_INVALID');
    }
    if (known.status === 'receipt_pending') {
      this.saveOutbox('write-receipt', decision.decisionId, { receipt });
      const response = await this.callMsp('msp_pipeline_write_receipt', { receipt });
      if (response.accepted !== true || response.receiptHash !== receiptHash) fail('MSP_RECEIPT_NOT_ACCEPTED');
      this.recordDecision(decision, 'receipt_written', { receiptHash, receipt });
      this.removeTransactionIntent(decision.decisionId, 'final');
      this.removeOutbox('write-receipt', decision.decisionId);
    }
    let verdict = known.verdict;
    if (!verdict || !['PASS', 'WARN', 'FAIL'].includes(verdict.verdict)) {
      const gate = await this.callMsp('msp_pipeline_gate', { decisionId: decision.decisionId, decisionHash: decision.decisionHash });
      verdict = gate.verdict ?? gate;
      if (!isPlainObject(verdict) || verdict.schemaVersion !== SCHEMA_VERSION
        || verdict.decisionId !== decision.decisionId || verdict.decisionHash !== decision.decisionHash
        || verdict.snapshotId !== receipt.snapshotId || verdict.generation !== receipt.generation
        || verdict.receiptHash !== receiptHash || !sameScopeJson(verdict.scope, this.scope)) fail('MSP_GATE_IDENTITY_MISMATCH');
      this.recordDecision(decision, 'gate_received', { receiptHash, receipt, verdict });
    }
    const allowed = verdict.verdict === 'PASS'
      && verdict.allowPublication === true && decision.policy.allowPublication === true;
    if (!allowed) {
      this.recordDecision(decision, 'held_by_quality_gate', { receiptHash, receipt, verdict });
      return { status: 'held', decisionId: decision.decisionId, verdict };
    }
    const snapshotFilename = path.join(this.snapshotsRoot, `${safeFilePart(receipt.snapshotId)}.json`);
    if (fs.existsSync(snapshotFilename)) {
      const savedSnapshot = this.readSnapshot(receipt.snapshotId);
      if (savedSnapshot.generation !== receipt.generation
        || savedSnapshot.decisionId !== decision.decisionId
        || savedSnapshot.decisionHash !== decision.decisionHash
        || savedSnapshot.receiptHash !== receiptHash
        || savedSnapshot.vectorCollection !== this.vectorCollectionFor(receipt.generation)) {
        fail('SNAPSHOT_IDENTITY_CONFLICT');
      }
    }
    const existingPointer = fs.existsSync(path.join(this.root, 'published-pointer.json')) ? this.readPointer() : undefined;
    const pointerBody = {
      schemaVersion: SCHEMA_VERSION,
      scope: cloneJson(this.scope),
      snapshotId: receipt.snapshotId,
      generation: receipt.generation,
      vectorCollection: this.vectorCollectionFor(receipt.generation),
      decisionId: decision.decisionId,
      decisionHash: decision.decisionHash,
      receiptHash,
      publishedSnapshotIds: publishedSnapshotHistory(existingPointer, receipt.snapshotId),
    };
    const expectedPointer = { ...pointerBody, pointerHash: hashObject(pointerBody) };
    let pointer = expectedPointer;
    const pointerMatches = existingPointer
      && existingPointer.snapshotId === expectedPointer.snapshotId
      && existingPointer.generation === expectedPointer.generation
      && existingPointer.vectorCollection === expectedPointer.vectorCollection
      && existingPointer.decisionId === expectedPointer.decisionId
      && existingPointer.decisionHash === expectedPointer.decisionHash
      && existingPointer.receiptHash === expectedPointer.receiptHash
      && existingPointer.pointerHash === expectedPointer.pointerHash;
    if (pointerMatches) {
      if (!fs.existsSync(snapshotFilename)) fail('SNAPSHOT_MISSING_FOR_POINTER');
      pointer = existingPointer;
    } else {
      if (!fs.existsSync(snapshotFilename)) {
        writeAtomic(snapshotFilename, `${JSON.stringify(snapshotRecord(expectedPointer, decision))}\n`);
      }
      await this.faultInjector?.('before-pointer-replacement', { decisionId: decision.decisionId, pointer: expectedPointer });
      writeAtomic(path.join(this.root, 'published-pointer.json'), `${JSON.stringify(expectedPointer)}\n`);
      pointer = expectedPointer;
      await this.faultInjector?.('after-pointer-replacement-before-publication-outbox', { decisionId: decision.decisionId, pointer });
    }
    const publicationReceipt = known.publicationReceipt ?? {
      schemaVersion: SCHEMA_VERSION,
      scope: cloneJson(this.scope),
      runId: decision.runId,
      decisionId: decision.decisionId,
      decisionHash: decision.decisionHash,
      snapshotId: receipt.snapshotId,
      generation: receipt.generation,
      receiptHash,
      publishedAt: nowIso(),
      pointerHash: pointer.pointerHash,
      modelRevision: MODEL_REVISION,
      transactionFrontier: receipt.transaction.frontier,
      readback: { ok: true },
    };
    this.saveOutbox('publication-receipt', decision.decisionId, { receipt: publicationReceipt });
    const published = await this.callMsp('msp_pipeline_publication_receipt', { receipt: publicationReceipt });
    if (published.accepted !== true) fail('MSP_PUBLICATION_NOT_ACCEPTED');
    this.recordDecision(decision, 'published', {
      receiptHash, receipt, verdict, publicationReceipt,
      generation: receipt.generation, snapshotId: receipt.snapshotId,
    });
    this.removeOutbox('publication-receipt', decision.decisionId);
    return { status: 'published', decisionId: decision.decisionId, snapshotId: receipt.snapshotId, generation: receipt.generation, benchmark: receipt.benchmark };
  }

  async processDecision(decision) {
    const startedAt = Date.now();
    const validated = validateDecision(decision, this.scope);
    const known = this.state.decisions[decision.decisionId];
    if (known?.receipt && known.status !== 'candidate_received' && known.status !== 'candidate_materialized') {
      return this.resumeFromReceipt(decision, known);
    }
    this.writeDecision(decision);
    this.recordDecision(decision, 'candidate_received');
    const graphDecision = { ...decision, derived: [] };
    let graphReceipt = known?.graphReceipt;
    let graphReceiptHash = known?.graphReceiptHash;
    let derived = known?.derived;
    let derivedHash = known?.derivedHash;
    let baseCandidate;
    if (!graphReceipt || !Array.isArray(derived)) {
      this.recordDecision(decision, 'candidate_materialized', {
        generation: `g17-${hashObject({ decisionId: decision.decisionId, decisionHash: decision.decisionHash }).slice(0, 32)}`,
        snapshotId: `snap-${hashObject({ scope: decision.scope, generation: `g17-${hashObject({ decisionId: decision.decisionId, decisionHash: decision.decisionHash }).slice(0, 32)}` }).slice(0, 32)}`,
      });
      const graphStartedAt = nowIso();
      const graphStarted = Date.now();
      let graphTransaction;
      let graphReadback;
      try {
        baseCandidate = this.buildPhysicalCandidate(graphDecision, validated, []);
        graphTransaction = await this.commitCandidate(graphDecision, baseCandidate, 'graph');
        this.recordDecision(decision, 'graph_native_committed', { transaction: graphTransaction, generation: baseCandidate.generation, snapshotId: baseCandidate.snapshotId });
        graphReadback = await this.nativeReadback(baseCandidate, graphDecision);
      } catch (error) {
        if (isFaultInjection(error)) throw error;
        return this.reportStageFailure(decision, 13, graphStartedAt, nonNegativeMetrics(
          decision.chunks.length + decision.entities.length + decision.facts.length + decision.held.length,
          baseCandidate?.expectedNodeCount ?? 0,
          baseCandidate?.quarantinedFacts ?? 0,
          1,
          0,
          Date.now() - graphStarted,
        ), error);
      }
      const graphDuration = Date.now() - graphStarted;
      graphReceipt = this.makeGraphReceipt(decision, baseCandidate, graphTransaction, graphReadback, graphDuration);
      graphReceipt.startedAt = graphStartedAt;
      graphReceipt.finishedAt = nowIso();
      graphReceiptHash = hashObject(graphReceipt);
      this.recordDecision(decision, 'graph_receipt_pending', {
        graphReceiptHash, graphReceipt, generation: baseCandidate.generation, snapshotId: baseCandidate.snapshotId,
      });
      this.saveOutbox('graph-receipt', decision.decisionId, { receipt: graphReceipt });
      try {
        const graphResponse = await this.callMsp('msp_pipeline_graph_receipt', { receipt: graphReceipt });
        if (graphResponse.schemaVersion !== SCHEMA_VERSION || !sameScopeJson(graphResponse.scope, this.scope)
          || graphResponse.accepted !== true || graphResponse.graphReceiptHash !== graphReceiptHash
          || !Array.isArray(graphResponse.derived) || typeof graphResponse.derivedHash !== 'string'
          || graphResponse.derivedHash !== hashObject(graphResponse.derived)) fail('MSP_GRAPH_RECEIPT_NOT_ACCEPTED');
        validateDerivedRows(graphResponse.derived, validated.chunkById);
        derived = graphResponse.derived;
        derivedHash = graphResponse.derivedHash;
        await this.faultInjector?.('after-graph-receipt-accepted-before-local-state', {
          phase: 'graph',
          decisionId: decision.decisionId,
          receipt: graphReceipt,
          derived: cloneJson(derived),
          derivedHash,
        });
        this.recordDecision(decision, 'graph_receipt_written', {
          graphReceiptHash, graphReceipt, derived, derivedHash,
          generation: baseCandidate.generation, snapshotId: baseCandidate.snapshotId,
        });
        this.removeTransactionIntent(decision.decisionId, 'graph');
        this.removeOutbox('graph-receipt', decision.decisionId);
      } catch (error) {
        this.recordDecision(decision, 'graph_receipt_pending', {
          graphReceiptHash, graphReceipt, generation: baseCandidate.generation, snapshotId: baseCandidate.snapshotId,
          lastError: error.message,
        });
        throw error;
      }
    } else {
      // The graph receipt and GKS-derived rows were durably saved by the MSP
      // outbox retry. Reopen the candidate metadata without re-writing graph.
      const generation = known.generation ?? `g17-${hashObject({ decisionId: decision.decisionId, decisionHash: decision.decisionHash }).slice(0, 32)}`;
      const snapshotId = known.snapshotId ?? `snap-${hashObject({ scope: decision.scope, generation }).slice(0, 32)}`;
      baseCandidate = {
        generation,
        snapshotId,
        vectorCollection: this.vectorCollectionFor(generation),
        nodes: [],
        edges: [],
        lexicalRows: [],
        chunkVectorByPhysical: new Map(),
        acceptedFacts: decision.facts,
        quarantinedFacts: 0,
        chunkPhysicalById: new Map(decision.chunks.map((chunk) => [chunk.chunkId, physicalId(generation, 'chunk', chunk.chunkId)])),
        expectedNodeCount: graphReceipt.readback.nodeCount,
        expectedEdgeCount: graphReceipt.readback.edgeCount,
        expectedVectorCount: 0,
      };
      const lexicalRows = decision.chunks.map((chunk) => ({
        physicalId: physicalId(generation, 'chunk', chunk.chunkId),
        logicalId: chunk.chunkId,
        generation: baseCandidate.generation,
        scope: decision.scope,
        text: chunk.text,
        contentHash: chunk.contentHash,
        citation: citationFor(decision.source, chunk),
      }));
      baseCandidate.lexicalRows = lexicalRows;
    }

    const graphStartedAt = graphReceipt.startedAt;
    const embeddingStartedAt = nowIso();
    const embeddingStarted = Date.now();
    if (!decision.policy.allowEmbedding) {
      return this.reportStageFailure(decision, 15, embeddingStartedAt, nonNegativeMetrics(
        decision.chunks.length,
        0,
        0,
        1,
        0,
        Date.now() - embeddingStarted,
      ), new Error('EMBEDDING_POLICY_DENIED'));
    }
    let vectors = [];
    try {
      await this.ensureEmbedder(baseCandidate.vectorCollection);
      vectors = await this.embedder.embed(decision.chunks.map((chunk) => chunk.text), 'passage');
    } catch (error) {
      return this.reportStageFailure(decision, 15, embeddingStartedAt, nonNegativeMetrics(
        decision.chunks.length,
        vectors.length,
        0,
        1,
        0,
        Date.now() - embeddingStarted,
      ), error);
    }
    const embeddingDuration = Date.now() - embeddingStarted;
    const embeddingFinishedAt = nowIso();
    const enrichedDecision = { ...decision, derived };
    const indexStartedAt = nowIso();
    const indexStarted = Date.now();
    let derivedCandidate;
    let finalWriteCandidate;
    let candidate;
    let transaction;
    let readback;
    let benchmark;
    let laneManifest;
    let temporal;
    try {
      derivedCandidate = this.buildDerivedCandidate(enrichedDecision, baseCandidate);
      const vectorMap = new Map(decision.chunks.map((chunk, index) => [
        physicalId(baseCandidate.generation, 'chunk', chunk.chunkId), vectors[index],
      ].filter(([, vector]) => vector)));
      finalWriteCandidate = {
        ...derivedCandidate,
        chunkVectorByPhysical: vectorMap,
        expectedVectorCount: vectorMap.size,
      };
      candidate = {
        ...finalWriteCandidate,
        nodes: [...baseCandidate.nodes, ...derivedCandidate.nodes],
        edges: [...baseCandidate.edges, ...derivedCandidate.edges],
        lexicalRows: baseCandidate.lexicalRows,
        expectedNodeCount: baseCandidate.expectedNodeCount + derivedCandidate.expectedNodeCount,
        expectedEdgeCount: baseCandidate.expectedEdgeCount + derivedCandidate.expectedEdgeCount,
      };
      this.recordDecision(decision, 'candidate_materialized', {
        generation: candidate.generation,
        snapshotId: candidate.snapshotId,
        graphReceiptHash,
        derivedHash,
        derived,
      });

      transaction = await this.commitCandidate(enrichedDecision, finalWriteCandidate, 'final');
      this.lexical.put(candidate.lexicalRows);
      readback = await this.nativeReadback(candidate, enrichedDecision);
      benchmark = await this.benchmark(candidate);
      temporal = await this.verifyTemporalLane(candidate, enrichedDecision);
      laneManifest = this.laneManifest(candidate, enrichedDecision, readback, temporal);
    } catch (error) {
      if (isFaultInjection(error)) throw error;
      return this.reportStageFailure(decision, 16, indexStartedAt, nonNegativeMetrics(
        candidate?.expectedVectorCount ?? vectors.length,
        candidate?.expectedVectorCount ?? vectors.length,
        0,
        1,
        0,
        Date.now() - indexStarted,
      ), error);
    }
    const indexDuration = Date.now() - indexStarted;
    const indexFinishedAt = nowIso();
    const metrics = {
      13: graphReceipt.metrics,
      15: nonNegativeMetrics(decision.chunks.length, candidate.expectedVectorCount, 0, 0, 0, embeddingDuration),
      16: nonNegativeMetrics(candidate.expectedVectorCount, candidate.expectedVectorCount, 0, 0, 0, indexDuration),
    };
    const receipt = this.makeReceipt(decision, candidate, transaction, readback, laneManifest, benchmark, metrics, graphReceiptHash, derivedHash, {
      13: { startedAt: graphStartedAt, finishedAt: graphReceipt.finishedAt },
      15: { startedAt: embeddingStartedAt, finishedAt: embeddingFinishedAt },
      16: { startedAt: indexStartedAt, finishedAt: indexFinishedAt },
    });
    const receiptHash = hashObject(receipt);
    this.recordDecision(decision, 'receipt_pending', { receiptHash, receipt });
    this.saveOutbox('write-receipt', decision.decisionId, { receipt });
    try {
      const accepted = await this.callMsp('msp_pipeline_write_receipt', { receipt });
      if (accepted.accepted !== true || accepted.receiptHash !== receiptHash) fail('MSP_RECEIPT_NOT_ACCEPTED');
    } catch (error) {
      this.recordDecision(decision, 'receipt_pending', { receiptHash, receipt, lastError: error.message });
      throw error;
    }
    this.recordDecision(decision, 'receipt_written', { receiptHash, receipt });
    this.removeTransactionIntent(decision.decisionId, 'final');
    this.removeOutbox('write-receipt', decision.decisionId);

    const gate = await this.callMsp('msp_pipeline_gate', {
      decisionId: decision.decisionId,
      decisionHash: decision.decisionHash,
    });
    if (gate.verdict === undefined || gate.verdict === null) fail('MSP_GATE_RESPONSE_INVALID');
    const verdict = gate.verdict ?? gate;
    if (!isPlainObject(verdict) || verdict.schemaVersion !== SCHEMA_VERSION
      || verdict.decisionId !== decision.decisionId || verdict.decisionHash !== decision.decisionHash
      || verdict.snapshotId !== candidate.snapshotId || verdict.generation !== candidate.generation
      || verdict.receiptHash !== receiptHash || !['PASS', 'WARN', 'FAIL'].includes(verdict.verdict)
      || !sameScopeJson(verdict.scope, this.scope)) fail('MSP_GATE_IDENTITY_MISMATCH');
    this.recordDecision(decision, 'gate_received', { receiptHash, receipt, verdict });
    const allowed = verdict.verdict === 'PASS'
      && verdict.allowPublication === true && decision.policy.allowPublication === true;
    if (!allowed) {
      this.recordDecision(decision, 'held_by_quality_gate', { receiptHash, receipt, verdict });
      return { status: 'held', decisionId: decision.decisionId, verdict };
    }

    const existingPointer = fs.existsSync(path.join(this.root, 'published-pointer.json')) ? this.readPointer() : undefined;
    const pointerBody = {
      schemaVersion: SCHEMA_VERSION,
      scope: cloneJson(this.scope),
      snapshotId: candidate.snapshotId,
      generation: candidate.generation,
      vectorCollection: candidate.vectorCollection,
      decisionId: decision.decisionId,
      decisionHash: decision.decisionHash,
      receiptHash,
      publishedSnapshotIds: publishedSnapshotHistory(existingPointer, candidate.snapshotId),
    };
    const pointer = { ...pointerBody, pointerHash: hashObject(pointerBody) };
    const snapshotFilename = path.join(this.snapshotsRoot, `${safeFilePart(candidate.snapshotId)}.json`);
    if (!fs.existsSync(snapshotFilename)) {
      writeAtomic(snapshotFilename, `${JSON.stringify(snapshotRecord(pointer, decision))}\n`);
    } else {
      const savedSnapshot = this.readSnapshot(candidate.snapshotId);
      if (savedSnapshot.generation !== candidate.generation
        || savedSnapshot.decisionId !== decision.decisionId
        || savedSnapshot.decisionHash !== decision.decisionHash
        || savedSnapshot.receiptHash !== receiptHash) {
        fail('SNAPSHOT_IDENTITY_CONFLICT');
      }
    }
    await this.faultInjector?.('before-pointer-replacement', { decisionId: decision.decisionId, pointer });
    writeAtomic(path.join(this.root, 'published-pointer.json'), `${JSON.stringify(pointer)}\n`);
    await this.faultInjector?.('after-pointer-replacement-before-publication-outbox', { decisionId: decision.decisionId, pointer });
    const publicationReceipt = {
      schemaVersion: SCHEMA_VERSION,
      scope: cloneJson(this.scope),
      runId: decision.runId,
      decisionId: decision.decisionId,
      decisionHash: decision.decisionHash,
      snapshotId: candidate.snapshotId,
      generation: candidate.generation,
      receiptHash,
      publishedAt: nowIso(),
      pointerHash: pointer.pointerHash,
      modelRevision: MODEL_REVISION,
      transactionFrontier: transaction.frontier,
      readback: { ok: true },
    };
    this.saveOutbox('publication-receipt', decision.decisionId, { receipt: publicationReceipt });
    try {
      const published = await this.callMsp('msp_pipeline_publication_receipt', { receipt: publicationReceipt });
      if (published.accepted !== true) fail('MSP_PUBLICATION_NOT_ACCEPTED');
    } catch (error) {
      this.recordDecision(decision, 'publication_receipt_pending', { receiptHash, receipt, verdict, publicationReceipt, lastError: error.message });
      throw error;
    }
    this.recordDecision(decision, 'published', {
      generation: candidate.generation,
      snapshotId: candidate.snapshotId,
      receiptHash,
      receipt,
      verdict,
      publicationReceipt,
      durationMs: Date.now() - startedAt,
    });
    this.removeOutbox('publication-receipt', decision.decisionId);
    return { status: 'published', decisionId: decision.decisionId, snapshotId: candidate.snapshotId, generation: candidate.generation, benchmark };
  }

  async retryOutbox() {
    const files = fs.readdirSync(this.outboxRoot).filter((filename) => filename.endsWith('.json'));
    for (const filename of files) {
      const item = parseJsonText(fs.readFileSync(path.join(this.outboxRoot, filename), 'utf8'), 'OUTBOX_INVALID');
      const stageFailure = typeof item.kind === 'string' ? item.kind.match(/^stage-failure-(\d+)$/) : null;
      const isStageFailure = Boolean(stageFailure);
      if (!['graph-receipt', 'write-receipt', 'publication-receipt'].includes(item.kind) && !isStageFailure) fail('OUTBOX_KIND_INVALID');
      if (!item.payload) fail('OUTBOX_PAYLOAD_INVALID');
      const tool = isStageFailure
        ? 'msp_pipeline_stage_failure'
        : item.kind === 'graph-receipt'
          ? 'msp_pipeline_graph_receipt'
          : item.kind === 'write-receipt' ? 'msp_pipeline_write_receipt' : 'msp_pipeline_publication_receipt';
      const request = isStageFailure ? item.payload.failure : item.payload;
      if (isStageFailure && (!request || Number(request.stage?.stageNumber) !== Number(stageFailure[1]))) fail('OUTBOX_STAGE_FAILURE_INVALID');
      const response = await this.callMsp(tool, request);
      if (response.accepted !== true) fail('OUTBOX_RETRY_NOT_ACCEPTED', item.kind);
      if (isStageFailure) {
        const failureHash = item.payload.failureHash ?? hashObject(request);
        if (response.failureHash !== undefined && response.failureHash !== failureHash) fail('OUTBOX_STAGE_FAILURE_HASH_MISMATCH');
        this.recordStored(item.decisionId, 'failed', {
          failedStage: request.stage.stageNumber,
          stageFailure: request,
          stageFailureHash: failureHash,
        });
      }
      if (item.kind === 'write-receipt' && response.receiptHash !== hashObject(item.payload.receipt)) fail('OUTBOX_RECEIPT_HASH_MISMATCH');
      if (item.kind === 'graph-receipt') {
        const graphReceiptHash = hashObject(item.payload.receipt);
        if (response.graphReceiptHash !== graphReceiptHash || !Array.isArray(response.derived)
          || typeof response.derivedHash !== 'string' || response.derivedHash !== hashObject(response.derived)) {
          fail('OUTBOX_GRAPH_RECEIPT_INVALID');
        }
        await this.faultInjector?.('after-graph-receipt-accepted-before-local-state', {
          phase: 'graph',
          decisionId: item.decisionId,
          receipt: item.payload.receipt,
          derived: cloneJson(response.derived),
          derivedHash: response.derivedHash,
          replay: true,
        });
        this.recordStored(item.decisionId, 'graph_receipt_written', {
          graphReceiptHash,
          graphReceipt: item.payload.receipt,
          derived: response.derived,
          derivedHash: response.derivedHash,
          generation: this.state.decisions[item.decisionId]?.generation,
          snapshotId: this.state.decisions[item.decisionId]?.snapshotId,
        });
      }
      if (item.kind === 'write-receipt') {
        const receiptHash = hashObject(item.payload.receipt);
        this.recordStored(item.decisionId, 'receipt_written', {
          receiptHash,
          receipt: item.payload.receipt,
        });
        this.removeTransactionIntent(item.decisionId, 'final');
      }
      if (item.kind === 'publication-receipt') {
        const receipt = item.payload.receipt;
        this.recordStored(item.decisionId, 'published', {
          receiptHash: receipt.receiptHash,
          publicationReceipt: receipt,
          generation: receipt.generation,
          snapshotId: receipt.snapshotId,
        });
      }
      if (item.kind === 'graph-receipt') this.removeTransactionIntent(item.decisionId, 'graph');
      this.removeOutbox(item.kind, item.decisionId);
    }
  }

  async runOnce() {
    await this.retryOutbox();
    const decisions = await this.claim();
    if (decisions.length === 0) return { status: 'idle' };
    const decision = decisions[0];
    const known = this.state.decisions[decision.decisionId];
    if (known?.status === 'published') return { status: 'already_published', decisionId: decision.decisionId };
    if (known?.status === 'failed') {
      return {
        status: 'failed',
        decisionId: decision.decisionId,
        stageNumber: known.failedStage,
        error: known.stageFailure?.error,
      };
    }
    return this.processDecision(decision);
  }

  readPointer() {
    const filename = path.join(this.root, 'published-pointer.json');
    if (!fs.existsSync(filename)) fail('PUBLISHED_POINTER_MISSING');
    const pointer = parseJsonText(fs.readFileSync(filename, 'utf8'), 'PUBLISHED_POINTER_INVALID');
    if (pointer.schemaVersion !== SCHEMA_VERSION) fail('PUBLISHED_POINTER_SCHEMA_INVALID');
    validateScope(pointer.scope, this.scope);
    for (const key of ['snapshotId', 'generation', 'vectorCollection', 'decisionId', 'decisionHash', 'receiptHash', 'pointerHash']) {
      if (typeof pointer[key] !== 'string' || pointer[key].length === 0) fail('PUBLISHED_POINTER_FIELD_INVALID', key);
    }
    if (!Array.isArray(pointer.publishedSnapshotIds) || pointer.publishedSnapshotIds.length === 0
      || pointer.publishedSnapshotIds.some((snapshotId) => typeof snapshotId !== 'string' || snapshotId.length === 0)
      || new Set(pointer.publishedSnapshotIds).size !== pointer.publishedSnapshotIds.length
      || !pointer.publishedSnapshotIds.includes(pointer.snapshotId)) {
      fail('PUBLISHED_HISTORY_INVALID');
    }
    if (pointer.vectorCollection !== this.vectorCollectionFor(pointer.generation)) fail('PUBLISHED_POINTER_VECTOR_COLLECTION_MISMATCH');
    const body = { ...pointer };
    delete body.pointerHash;
    if (hashObject(body) !== pointer.pointerHash) fail('PUBLISHED_POINTER_HASH_MISMATCH');
    return pointer;
  }

  readSnapshot(snapshotId) {
    if (typeof snapshotId !== 'string' || snapshotId.length === 0) fail('SNAPSHOT_ID_INVALID');
    const filename = path.join(this.snapshotsRoot, `${safeFilePart(snapshotId)}.json`);
    if (!fs.existsSync(filename)) fail('SNAPSHOT_NOT_FOUND', snapshotId);
    const snapshot = parseJsonText(fs.readFileSync(filename, 'utf8'), 'SNAPSHOT_INVALID');
    if (snapshot.schemaVersion !== SCHEMA_VERSION) fail('SNAPSHOT_SCHEMA_INVALID');
    validateScope(snapshot.scope, this.scope);
    if (snapshot.snapshotId !== snapshotId) fail('SNAPSHOT_ID_MISMATCH');
    for (const key of ['generation', 'vectorCollection', 'decisionId', 'decisionHash', 'receiptHash']) {
      if (typeof snapshot[key] !== 'string' || snapshot[key].length === 0) fail('SNAPSHOT_FIELD_INVALID', key);
    }
    if (snapshot.vectorCollection !== this.vectorCollectionFor(snapshot.generation)) fail('SNAPSHOT_VECTOR_COLLECTION_MISMATCH');
    return snapshot;
  }

  async queryPublished({ scope, query, topK = 5, snapshotId } = {}) {
    validateScope(scope, this.scope);
    if (!Number.isSafeInteger(topK) || topK < 1 || topK > 100) fail('QUERY_TOP_K_INVALID');
    // Bind one authoritative pointer read before selecting a historical
    // snapshot. Prepared candidate files are retained for crash recovery but
    // become queryable only after the atomic pointer publishes their id.
    const publishedPointer = this.readPointer();
    const selectedSnapshotId = snapshotId ?? publishedPointer.snapshotId;
    if (!publishedPointer.publishedSnapshotIds.includes(selectedSnapshotId)) {
      fail('SNAPSHOT_NOT_PUBLISHED', selectedSnapshotId);
    }
    const snapshot = this.readSnapshot(selectedSnapshotId);
    const results = await this.searchGeneration(snapshot.generation, snapshot.snapshotId, query, topK, snapshot.vectorCollection);
    return {
      schemaVersion: SCHEMA_VERSION,
      scope: cloneJson(this.scope),
      snapshotId: snapshot.snapshotId,
      generation: snapshot.generation,
      results,
    };
  }

  async listen() {
    if (this.server) {
      const current = this.server.address();
      return { host: this.host, port: typeof current === 'object' && current ? current.port : this.port };
    }
    this.server = http.createServer((request, response) => {
      const failResponse = (status, error) => {
        if (response.writableEnded) return;
        response.statusCode = status;
        response.setHeader('content-type', 'application/json');
        response.end(JSON.stringify({ error: error.message ?? String(error) }));
      };
      const remote = request.socket.remoteAddress;
      if (remote !== '127.0.0.1' && remote !== '::1' && remote !== '::ffff:127.0.0.1') {
        failResponse(403, new Error('LOOPBACK_ONLY'));
        return;
      }
      if (request.headers.authorization !== `Bearer ${this.workerToken}`) {
        failResponse(401, new Error('WORKER_TOKEN_INVALID'));
        return;
      }
      if (request.method !== 'POST' || request.url !== '/query') {
        failResponse(404, new Error('QUERY_ROUTE_NOT_FOUND'));
        return;
      }
      let body = '';
      let tooLarge = false;
      request.setEncoding('utf8');
      request.on('data', (chunk) => {
        body += chunk;
        if (Buffer.byteLength(body, 'utf8') > 2 * 1024 * 1024) {
          tooLarge = true;
          request.destroy();
        }
      });
      request.on('error', (error) => failResponse(400, error));
      request.on('end', async () => {
        if (tooLarge) return;
        try {
          const result = await this.queryPublished(parseJsonText(body, 'QUERY_JSON_INVALID'));
          response.statusCode = 200;
          response.setHeader('content-type', 'application/json');
          response.end(JSON.stringify(result));
        } catch (error) {
          failResponse(400, error);
        }
      });
    });
    await new Promise((resolve, reject) => {
      this.server.once('error', reject);
      this.server.listen(this.port ?? 0, this.host, () => {
        this.server.off('error', reject);
        resolve();
      });
    });
    const address = this.server.address();
    return { host: this.host, port: address.port, url: `http://${this.host}:${address.port}` };
  }

  start() {
    if (this.running) return this;
    this.running = true;
    const tick = async () => {
      if (!this.running) return;
      this.activeRun = this.runOnce();
      try {
        await this.activeRun;
      } catch (error) {
        this.lastError = error;
      } finally {
        this.activeRun = undefined;
      }
      if (this.running) this.loopTimer = setTimeout(tick, this.pollIntervalMs);
    };
    this.loopTimer = setTimeout(tick, 0);
    return this;
  }

  resume() {
    return this.start();
  }

  stop() {
    this.running = false;
    if (this.loopTimer) clearTimeout(this.loopTimer);
    this.loopTimer = undefined;
    return this;
  }

  async close() {
    this.stop();
    if (this.activeRun) {
      try { await this.activeRun; } catch { /* preserve durable outbox for the next process */ }
      this.activeRun = undefined;
    }
    if (this.server) {
      await new Promise((resolve) => this.server.close(() => resolve()));
      this.server = undefined;
    }
    await this.embedder.close();
    this.lexical.close();
    this.mspCall.close?.();
    releaseWorkerLock(this.lockPath, this.lock);
  }
}
