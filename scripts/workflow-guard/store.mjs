import { constants as fsConstants } from 'node:fs';
import {
  access,
  lstat,
  mkdir,
  open,
  readFile,
  readdir,
  realpath,
  rename,
  rm,
  stat,
} from 'node:fs/promises';
import { createHash, randomUUID } from 'node:crypto';
import { spawnSync as childSpawnSync } from 'node:child_process';
import os from 'node:os';
import path from 'node:path';
import { spawn } from 'node:child_process';
import {
  applyEvent,
  createState,
  digest,
  normalizePath,
  validatePlan,
} from './core.mjs';

const RECORD_SCHEMA = 1;
const MAX_JSON_BYTES = 16 * 1024 * 1024;
const MAX_COMMAND_OUTPUT_BYTES = 2 * 1024 * 1024;
const MAX_ARTIFACT_BYTES = 64 * 1024 * 1024;
const EVENT_NAME = /^\d{12}\.json$/;
const HASH = /^[a-f0-9]{64}$/;

export class RuntimeError extends Error {
  constructor(code, message, options = {}) {
    super(message, options);
    this.name = 'RuntimeError';
    this.code = code;
  }
}

function fail(code, message, cause) {
  throw new RuntimeError(code, message, cause ? { cause } : {});
}

function isWithin(parent, candidate) {
  const relative = path.relative(parent, candidate);
  return relative === '' || (!relative.startsWith(`..${path.sep}`) && relative !== '..' && !path.isAbsolute(relative));
}

function canonical(value) {
  if (value === null || typeof value !== 'object') return JSON.stringify(value);
  if (Array.isArray(value)) return `[${value.map(canonical).join(',')}]`;
  const keys = Object.keys(value).sort();
  return `{${keys.map((key) => `${JSON.stringify(key)}:${canonical(value[key])}`).join(',')}}`;
}

function sha256(bytes) {
  return createHash('sha256').update(bytes).digest('hex');
}

/** Parse JSON while rejecting duplicate keys at every object depth. */
export function parseJsonNoDuplicates(source, label = 'JSON') {
  if (Buffer.byteLength(source, 'utf8') > MAX_JSON_BYTES) fail('JSON_TOO_LARGE', `${label} exceeds the ${MAX_JSON_BYTES} byte limit.`);
  let offset = 0;
  const ws = () => {
    while (offset < source.length && /[\t\n\r ]/.test(source[offset])) offset += 1;
  };
  const parseString = () => {
    const start = offset;
    if (source[offset] !== '"') fail('JSON_INVALID', `${label} has an invalid string at byte ${offset}.`);
    offset += 1;
    let escaped = false;
    while (offset < source.length) {
      const char = source[offset];
      if (!escaped && char === '"') {
        offset += 1;
        try {
          return JSON.parse(source.slice(start, offset));
        } catch {
          fail('JSON_INVALID', `${label} has an invalid string at byte ${start}.`);
        }
      }
      if (!escaped && char === '\\') escaped = true;
      else escaped = false;
      if (!escaped && char.charCodeAt(0) < 0x20) fail('JSON_INVALID', `${label} has a control character in a string.`);
      offset += 1;
    }
    fail('JSON_INVALID', `${label} has an unterminated string.`);
  };
  const value = () => {
    ws();
    const char = source[offset];
    if (char === '{') {
      offset += 1;
      ws();
      const result = Object.create(null);
      const keys = new Set();
      if (source[offset] === '}') { offset += 1; return result; }
      while (offset < source.length) {
        ws();
        const key = parseString();
        if (keys.has(key)) fail('JSON_DUPLICATE_KEY', `${label} contains duplicate object key ${JSON.stringify(key)}.`);
        keys.add(key);
        ws();
        if (source[offset] !== ':') fail('JSON_INVALID', `${label} is missing ':' after an object key.`);
        offset += 1;
        result[key] = value();
        ws();
        if (source[offset] === '}') { offset += 1; return result; }
        if (source[offset] !== ',') fail('JSON_INVALID', `${label} has an invalid object separator.`);
        offset += 1;
      }
      fail('JSON_INVALID', `${label} has an unterminated object.`);
    }
    if (char === '[') {
      offset += 1;
      ws();
      const result = [];
      if (source[offset] === ']') { offset += 1; return result; }
      while (offset < source.length) {
        result.push(value());
        ws();
        if (source[offset] === ']') { offset += 1; return result; }
        if (source[offset] !== ',') fail('JSON_INVALID', `${label} has an invalid array separator.`);
        offset += 1;
      }
      fail('JSON_INVALID', `${label} has an unterminated array.`);
    }
    if (char === '"') return parseString();
    for (const [token, parsed] of [['true', true], ['false', false], ['null', null]]) {
      if (source.startsWith(token, offset)) { offset += token.length; return parsed; }
    }
    const match = /^-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?/.exec(source.slice(offset));
    if (match) {
      offset += match[0].length;
      const number = Number(match[0]);
      if (!Number.isFinite(number)) fail('JSON_INVALID', `${label} contains a non-finite number.`);
      return number;
    }
    fail('JSON_INVALID', `${label} has an invalid value at byte ${offset}.`);
  };
  const parsed = value();
  ws();
  if (offset !== source.length) fail('JSON_INVALID', `${label} has trailing data.`);
  return parsed;
}

async function readJson(file, label = path.basename(file)) {
  let bytes;
  try {
    bytes = await readFile(file);
  } catch (error) {
    fail('IO_READ', `Could not read ${label}.`, error);
  }
  return parseJsonNoDuplicates(bytes.toString('utf8'), label);
}

async function syncDirectory(directory) {
  if (process.platform === 'win32') return;
  let handle;
  try {
    handle = await open(directory, 'r');
    await handle.sync();
  } catch {
    // Some filesystems do not support syncing directory handles. File data was
    // synced before rename, so the event itself remains the authority.
  } finally {
    await handle?.close().catch(() => {});
  }
}

async function writeDurable(file, bytes, { exclusive = true } = {}) {
  const handle = await open(file, exclusive ? 'wx' : 'w');
  try {
    await handle.writeFile(bytes);
    await handle.sync();
  } finally {
    await handle.close();
  }
}

async function atomicWrite(file, bytes, suffix = 'tmp') {
  const temporary = `${file}.${process.pid}.${randomUUID()}.${suffix}`;
  await writeDurable(temporary, bytes);
  try {
    await rename(temporary, file);
    await syncDirectory(path.dirname(file));
  } catch (error) {
    await rm(temporary, { force: true }).catch(() => {});
    throw error;
  }
}

function gitBaseArgs(workspace) {
  return ['-c', `safe.directory=${workspace}`, '-C', workspace];
}

function runGitSync(workspace, args, { encoding = 'utf8', input, maxBuffer = 32 * 1024 * 1024 } = {}) {
  const result = childSpawnSync('git', [...gitBaseArgs(workspace), ...args], {
    cwd: workspace,
    encoding,
    windowsHide: true,
    maxBuffer,
    shell: false,
    ...(input === undefined ? {} : { input }),
  });
  if (result.error) fail('GIT_EXEC', `Could not run Git ${args[0]}.`, result.error);
  if (result.status !== 0) {
    const detail = Buffer.isBuffer(result.stderr) ? result.stderr.toString('utf8') : result.stderr;
    fail('GIT_FAILED', `Git ${args.join(' ')} failed${detail ? `: ${detail.trim()}` : '.'}`);
  }
  return result.stdout;
}

async function assertGitRoot(workspaceInput) {
  let workspace;
  try {
    workspace = await realpath(workspaceInput);
  } catch (error) {
    fail('WORKSPACE_MISSING', 'Workspace must exist and resolve to a real path.', error);
  }
  const root = runGitSync(workspace, ['rev-parse', '--show-toplevel']).trim();
  const realRoot = await realpath(root).catch((error) => fail('WORKSPACE_GIT_ROOT', 'Could not resolve the Git worktree root.', error));
  if (path.resolve(realRoot) !== path.resolve(workspace)) fail('WORKSPACE_GIT_ROOT', 'Workspace must be the exact Git worktree root.');
  return workspace;
}

async function sourceDigestAt(workspace, commit) {
  const inventory = runGitSync(workspace, ['ls-tree', '-r', '-z', '--full-tree', commit], { encoding: 'buffer' });
  return sha256(inventory);
}

function nulFields(bytes, label) {
  const values = [];
  let start = 0;
  for (let index = 0; index < bytes.length; index += 1) {
    if (bytes[index] !== 0) continue;
    values.push(bytes.subarray(start, index));
    start = index + 1;
  }
  if (start !== bytes.length) fail('TRACKED_INVENTORY_INVALID', `${label} output was not NUL terminated.`);
  return values;
}

export function decodeTrackedPath(bytes) {
  const value = bytes.toString('utf8');
  if (!Buffer.from(value, 'utf8').equals(bytes)) fail('TRACKED_PATH_ENCODING', 'Tracked path is not valid UTF-8.');
  if (value.length === 0 || value !== value.normalize('NFC') || /[\\\u0000-\u001f\u007f"]/u.test(value)) {
    fail('TRACKED_PATH_UNSUPPORTED', 'Tracked paths with backslashes, quotes, control characters, or non-NFC names are unsupported.');
  }
  let normalized;
  try { normalized = normalizePath(value); }
  catch (error) { fail('TRACKED_PATH_UNSUPPORTED', `Tracked path is not portable: ${error.message}`); }
  return { value, normalized };
}

function parseTreeEntries(bytes) {
  const entries = [];
  const normalizedPaths = new Set();
  for (const record of nulFields(bytes, 'Git tree')) {
    const tab = record.indexOf(0x09);
    if (tab < 0) fail('TRACKED_INVENTORY_INVALID', 'Git tree entry has no path separator.');
    const header = record.subarray(0, tab).toString('ascii');
    const match = /^(\d{6}) (blob|commit|tree) ([a-f0-9]{40}|[a-f0-9]{64})$/i.exec(header);
    if (!match) fail('TRACKED_INVENTORY_INVALID', 'Git tree entry has an invalid mode, object type, or object ID.');
    const mode = match[1];
    const objectType = match[2];
    const objectId = match[3].toLowerCase();
    const { value: relativePath, normalized } = decodeTrackedPath(record.subarray(tab + 1));
    if (normalizedPaths.has(normalized)) fail('TRACKED_PATH_COLLISION', `Tracked path ${relativePath} collides with another path after portable normalization.`);
    normalizedPaths.add(normalized);
    if (mode === '120000') fail('TRACKED_SYMLINK_UNSUPPORTED', `Tracked symlink ${relativePath} is outside the supported V1 candidate boundary.`);
    if (mode === '160000' || objectType === 'commit') fail('TRACKED_SUBMODULE_UNSUPPORTED', `Tracked submodule ${relativePath} is outside the supported V1 candidate boundary.`);
    if (!['100644', '100755'].includes(mode) || objectType !== 'blob') {
      fail('TRACKED_MODE_UNSUPPORTED', `Tracked entry ${relativePath} has unsupported Git mode ${mode}.`);
    }
    entries.push({ relativePath, normalized, objectId });
  }
  return entries;
}

async function assertIndexMatchesHead(workspace, headTree) {
  const indexTree = runGitSync(workspace, ['write-tree']).trim().toLowerCase();
  if (indexTree !== headTree.toLowerCase()) fail('INDEX_TREE_MISMATCH', 'Git index tree differs from HEAD.');
  const flags = nulFields(runGitSync(workspace, ['ls-files', '-v', '-z', '--cached'], { encoding: 'buffer' }), 'Git index flags');
  for (const entry of flags) {
    if (entry.length < 3 || entry[1] !== 0x20) fail('INDEX_FLAGS_INVALID', 'Git index flag record is malformed.');
    const tag = String.fromCharCode(entry[0]);
    if (tag !== 'H') {
      const path = entry.subarray(2).toString('utf8');
      fail('INDEX_HIDDEN_ENTRY', `Tracked path ${path} has Git index flag ${tag}; assume-unchanged, skip-worktree, and non-merged entries are forbidden.`);
    }
  }
}

export async function assertTrackedWorktreeBytes(workspace, commit) {
  const treeBytes = runGitSync(workspace, ['ls-tree', '-r', '-z', '--full-tree', commit], { encoding: 'buffer' });
  const entries = parseTreeEntries(treeBytes);
  if (entries.length === 0) return;

  // Git clean filters can execute arbitrary configured programs. The guard only
  // supports Git's built-in text/EOL conversion; those conversions are applied
  // below by hash-object --stdin-paths after unsupported attributes are rejected.
  const attributeInput = Buffer.from(`${entries.map((item) => item.relativePath).join('\0')}\0`, 'utf8');
  const attributes = nulFields(runGitSync(workspace, [
    'check-attr', '--stdin', '-z', 'filter', 'working-tree-encoding', 'ident',
  ], { encoding: 'buffer', input: attributeInput }), 'Git attributes');
  if (attributes.length % 3 !== 0) fail('GIT_ATTRIBUTES_INVALID', 'Git attribute query returned an incomplete record.');
  for (let index = 0; index < attributes.length; index += 3) {
    const attribute = attributes[index + 1].toString('utf8');
    const value = attributes[index + 2].toString('utf8');
    if (['filter', 'working-tree-encoding', 'ident'].includes(attribute) && !['unspecified', 'unset'].includes(value)) {
      fail('TRACKED_CONVERSION_UNSUPPORTED', `Tracked files use unsupported Git ${attribute} conversion (${value}).`);
    }
  }

  const relativePaths = [];
  for (const entry of entries) {
    const filePath = await resolveExistingParents(workspace, entry.relativePath);
    const info = await lstat(filePath).catch((error) => fail('TRACKED_FILE_MISSING', `Tracked file ${entry.relativePath} is missing.`, error));
    if (!info.isFile() || info.isSymbolicLink()) fail('TRACKED_FILE_TYPE', `Tracked path ${entry.relativePath} is not a regular file.`);
    const resolved = await realpath(filePath).catch((error) => fail('TRACKED_FILE_PATH', `Could not resolve tracked file ${entry.relativePath}.`, error));
    if (!isWithin(workspace, resolved)) fail('TRACKED_FILE_ESCAPE', `Tracked file ${entry.relativePath} resolves outside the candidate worktree.`);
    relativePaths.push(entry.relativePath);
  }

  // Paths are control-character-free and NFC-validated above, so newline input
  // safely batches hash-object reads while honoring only built-in EOL cleanup.
  const pathInput = Buffer.from(`${relativePaths.join('\n')}\n`, 'utf8');
  const hashes = runGitSync(workspace, ['hash-object', '--stdin-paths'], {
    input: pathInput,
    maxBuffer: Math.max(32 * 1024 * 1024, entries.length * 80),
  }).trim().split(/\r?\n/);
  if (hashes.length !== entries.length) fail('TRACKED_HASH_COUNT', 'Git returned an incomplete tracked-file hash inventory.');
  for (let index = 0; index < entries.length; index += 1) {
    if (!/^(?:[a-f0-9]{40}|[a-f0-9]{64})$/i.test(hashes[index]) || hashes[index].toLowerCase() !== entries[index].objectId) {
      fail('TRACKED_BYTES_MISMATCH', `On-disk bytes for tracked path ${entries[index].relativePath} do not match the HEAD blob.`);
    }
  }
}

async function gitHead(workspace) {
  return runGitSync(workspace, ['rev-parse', '--verify', 'HEAD^{commit}']).trim().toLowerCase();
}

async function gitTree(workspace, commit = 'HEAD') {
  return runGitSync(workspace, ['rev-parse', '--verify', `${commit}^{tree}`]).trim().toLowerCase();
}

async function assertClean(workspace) {
  const status = runGitSync(workspace, ['status', '--porcelain=v1', '-z', '--untracked-files=all'], { encoding: 'buffer' });
  if (status.length !== 0) fail('CANDIDATE_DIRTY', 'Git candidate has tracked or untracked changes.');
}

export async function captureCandidate(workspace) {
  const sha = await gitHead(workspace);
  const tree = await gitTree(workspace, sha);
  await assertIndexMatchesHead(workspace, tree);
  await assertTrackedWorktreeBytes(workspace, sha);
  await assertClean(workspace);
  const sourceDigest = await sourceDigestAt(workspace, sha);
  return { sha, tree, sourceDigest };
}

function sameJson(a, b) {
  return canonical(a) === canonical(b);
}

function requireGateInput(input) {
  const expectedKeys = ['assessment', 'commandIds', 'findings', 'sessionId', 'stage', 'verdict'];
  if (!input || typeof input !== 'object' || Array.isArray(input)
    || Object.keys(input).sort().join(',') !== expectedKeys.join(',')) {
    fail('GATE_INPUT_SCHEMA', 'Gate report input must have exactly stage, sessionId, verdict, findings, assessment, and commandIds fields.');
  }
  if (!['verify', 'review', 'audit', 'final'].includes(input.stage)) fail('GATE_INPUT_STAGE', 'Gate stage must be verify, review, audit, or final.');
  if (typeof input.sessionId !== 'string' || input.sessionId.length === 0) fail('GATE_INPUT_SESSION', 'Gate sessionId must be a non-empty registered session identity.');
  if (!['pass', 'fail'].includes(input.verdict)) fail('GATE_INPUT_VERDICT', 'Gate verdict must be pass or fail.');
  if (typeof input.assessment !== 'string' || input.assessment.trim().length === 0 || Buffer.byteLength(input.assessment, 'utf8') > 64 * 1024) {
    fail('GATE_INPUT_ASSESSMENT', 'Gate assessment must be non-empty text no larger than 64 KiB.');
  }
  if (!Array.isArray(input.commandIds) || input.commandIds.some((id) => typeof id !== 'string' || id.length === 0)
    || new Set(input.commandIds).size !== input.commandIds.length) {
    fail('GATE_INPUT_COMMANDS', 'commandIds must be an array of unique non-empty command IDs.');
  }
  if (!Array.isArray(input.findings)) fail('GATE_INPUT_FINDINGS', 'findings must be an array.');
  const fingerprints = new Set();
  for (const finding of input.findings) {
    if (!finding || typeof finding !== 'object' || Array.isArray(finding)
      || Object.keys(finding).sort().join(',') !== 'blocking,critical,fingerprint'
      || typeof finding.fingerprint !== 'string' || finding.fingerprint.trim().length === 0
      || typeof finding.blocking !== 'boolean' || typeof finding.critical !== 'boolean') {
      fail('GATE_INPUT_FINDINGS', 'Each finding must contain exactly fingerprint, blocking, and critical.');
    }
    const normalized = finding.fingerprint.normalize('NFC').trim().toLowerCase().replaceAll('\\', '/').replace(/\s+/g, ' ');
    if (fingerprints.has(normalized)) fail('GATE_INPUT_FINDINGS', 'findings contains duplicate normalized fingerprints.');
    fingerprints.add(normalized);
  }
  if (input.verdict === 'fail' && !input.findings.some((finding) => finding.blocking)) {
    fail('GATE_INPUT_FINDINGS', 'A requested FAIL report needs at least one blocking finding.');
  }
  if (input.verdict === 'pass' && input.findings.some((finding) => finding.blocking)) {
    fail('GATE_INPUT_FINDINGS', 'A requested PASS report cannot contain a blocking finding.');
  }
}

function uniqueStable(values) {
  const seen = new Set();
  return values.filter((value) => {
    if (seen.has(value)) return false;
    seen.add(value);
    return true;
  });
}

function parseNulPaths(bytes) {
  if (bytes.length === 0) return [];
  return bytes.toString('utf8').split('\0').filter(Boolean);
}

async function changedPathsSince(workspace, baseSha, candidateSha) {
  const ancestry = childSpawnSync('git', [...gitBaseArgs(workspace), 'merge-base', '--is-ancestor', baseSha, candidateSha], {
    cwd: workspace, windowsHide: true, shell: false,
  });
  if (ancestry.error) fail('GIT_EXEC', 'Could not check candidate ancestry.', ancestry.error);
  if (ancestry.status !== 0) fail('CANDIDATE_NOT_DESCENDANT', 'Candidate must descend from the pinned base commit.');
  const bytes = runGitSync(workspace, ['diff', '--no-renames', '--name-only', '-z', '--diff-filter=ACDMRTUXB', baseSha, candidateSha], { encoding: 'buffer' });
  const actual = parseNulPaths(bytes).map((item) => normalizePath(item));
  if (new Set(actual).size !== actual.length) fail('CANDIDATE_PATH_COLLISION', 'Changed paths collide after path normalization.');
  return actual.sort();
}

async function resolveExistingParents(workspace, relativePath) {
  if (relativePath === '.' || relativePath === './' || relativePath === '.\\') return workspace;
  normalizePath(relativePath);
  const segments = relativePath.replaceAll('\\', '/').split('/').filter((segment) => segment !== '' && segment !== '.');
  let current = workspace;
  for (const segment of segments) {
    current = path.join(current, segment);
    try {
      const info = await lstat(current);
      const resolved = await realpath(current);
      if (!isWithin(workspace, resolved)) fail('PATH_ESCAPE', `Path ${relativePath} resolves outside the approved workspace.`);
      if (info.isSymbolicLink() && !isWithin(workspace, resolved)) fail('PATH_ESCAPE', `Path ${relativePath} follows a symlink outside the approved workspace.`);
    } catch (error) {
      if (error?.code === 'ENOENT') break;
      if (error instanceof RuntimeError) throw error;
      fail('PATH_RESOLUTION', `Could not safely resolve ${relativePath}.`, error);
    }
  }
  return path.join(workspace, ...segments);
}

async function validateTaskScopes(workspace, task) {
  for (const scope of task.writePaths) await resolveExistingParents(workspace, scope);
}

function isPathWithinScope(candidate, scope) {
  return candidate === scope || candidate.startsWith(`${scope}/`);
}

async function validatePacketCandidate({ workspace, plan, task, packet, dispatchCandidate }) {
  const current = await captureCandidate(workspace);
  if (!sameJson(packet.candidate, current)) fail('CANDIDATE_HASH_MISMATCH', 'Handoff candidate SHA, tree or source digest does not match the checked Git candidate.');
  const actual = await changedPathsSince(workspace, dispatchCandidate.sha, current.sha);
  const claimed = packet.changedPaths.map((item) => normalizePath(item)).sort();
  if (new Set(claimed).size !== claimed.length || !sameJson(actual, claimed)) fail('CHANGED_PATHS_MISMATCH', 'Handoff changedPaths do not match the Git diff from the reserved candidate.');
  const allowed = task.writePaths.map((item) => normalizePath(item));
  for (const changed of actual) {
    if (!allowed.some((scope) => isPathWithinScope(changed, scope))) fail('WRITE_SCOPE', `Changed path ${changed} is outside task ${task.id}'s write scope.`);
    await resolveExistingParents(workspace, changed);
  }
  return { candidate: current, changedPaths: actual };
}

async function readArtifact(sourcePath) {
  if (typeof sourcePath !== 'string' || !path.isAbsolute(sourcePath)) fail('ARTIFACT_PATH', 'Artifact paths must be absolute filesystem paths.');
  const resolved = await realpath(sourcePath).catch((error) => fail('ARTIFACT_PATH', 'Artifact path does not resolve to an existing file.', error));
  const info = await lstat(sourcePath).catch((error) => fail('ARTIFACT_PATH', 'Could not inspect artifact source.', error));
  if (!info.isFile() || info.isSymbolicLink()) fail('ARTIFACT_TYPE', 'Artifact source must be a regular non-symlink file.');
  if (info.size > MAX_ARTIFACT_BYTES) fail('ARTIFACT_TOO_LARGE', `Artifact exceeds ${MAX_ARTIFACT_BYTES} bytes.`);
  return { resolved, info, bytes: await readFile(resolved) };
}

async function persistBlob(runDir, bytes, expectedHash, label) {
  const actualHash = sha256(bytes);
  if (actualHash !== expectedHash) fail('ARTIFACT_HASH_MISMATCH', `${label} SHA-256 does not match its declared hash.`);
  const objectDir = path.join(runDir, 'objects');
  await mkdir(objectDir, { recursive: true });
  const objectPath = path.join(objectDir, actualHash);
  try {
    await writeDurable(objectPath, bytes);
    await syncDirectory(objectDir);
  } catch (error) {
    if (error?.code !== 'EEXIST') fail('BLOB_WRITE', `Could not persist ${label}.`, error);
    const existing = await readFile(objectPath).catch((readError) => fail('BLOB_READ', `Could not verify stored ${label}.`, readError));
    if (sha256(existing) !== actualHash) fail('BLOB_CORRUPT', `Stored ${label} blob is corrupt.`);
  }
  return { sha256: actualHash, size: bytes.length, object: `objects/${actualHash}` };
}

async function verifyPacketArtifacts(runDir, workspace, packet) {
  const verified = [];
  for (const item of packet.artifacts) {
    const { resolved, bytes } = await readArtifact(item.path);
    if (isWithin(workspace, resolved)) fail('ARTIFACT_IN_SOURCE_TREE', `Artifact ${item.path} is inside the candidate source tree.`);
    if (bytes.length !== item.size) fail('ARTIFACT_SIZE_MISMATCH', `Artifact ${item.path} size does not match its declaration.`);
    const blob = await persistBlob(runDir, bytes, item.sha256, `artifact ${item.path}`);
    verified.push({ path: item.path, ...blob });
  }
  return verified;
}

function findTask(plan, taskId) {
  const task = plan.tasks.find((item) => item.id === taskId);
  if (!task) fail('TASK_UNKNOWN', `Task ${taskId} is not in the immutable plan.`);
  return task;
}

function eventRecordName(sequence) {
  return `${String(sequence).padStart(12, '0')}.json`;
}

function cleanRecord(record) {
  if (!record || typeof record !== 'object' || Array.isArray(record)) fail('LEDGER_CORRUPT', 'Ledger event record must be an object.');
  const allowed = new Set(['schemaVersion', 'sequence', 'previousDigest', 'eventDigest', 'event', 'runtime', 'digest']);
  for (const key of Object.keys(record)) if (!allowed.has(key)) fail('LEDGER_CORRUPT', `Ledger event record has unknown field ${key}.`);
  for (const key of allowed) if (!Object.hasOwn(record, key)) fail('LEDGER_CORRUPT', `Ledger event record is missing ${key}.`);
}

function recordDigest(record) {
  const { digest: ignored, ...body } = record;
  return digest(body);
}

function eventDigest(event) {
  return digest(event);
}

async function pidAlive(pid) {
  if (!Number.isSafeInteger(pid) || pid <= 0) return false;
  try {
    process.kill(pid, 0);
    return true;
  } catch (error) {
    return error?.code === 'EPERM';
  }
}

function eventNeedsCoordinator(type) {
  return new Set([
    'register_session', 'dispatch', 'bind_worker', 'command_reserved',
    'command_completed', 'stop_confirmed', 'invalidate_candidate',
    'watchdog', 'abort',
  ]).has(type);
}

function isTestCrashPoint(name) {
  const requested = process.env.WORKFLOW_GUARD_TEST_CRASH;
  if (!requested) return false;
  if (process.env.NODE_ENV !== 'test') fail('TEST_HOOK_DISABLED', 'Crash hooks are available only with NODE_ENV=test.');
  if (requested === name) {
    process.exit(name === 'before-event-rename' ? 91 : name === 'after-event-rename' ? 92 : 93);
  }
  return false;
}

export class RunStore {
  constructor({ runDir, manifest, plan, workspace, taskWorkspaces }) {
    this.runDir = runDir;
    this.manifest = manifest;
    this.plan = plan;
    this.workspace = workspace;
    this.taskWorkspaces = taskWorkspaces ?? Object.fromEntries(plan.tasks.map((task) => [task.id, workspace]));
    this.eventsDir = path.join(runDir, 'events');
    this.lockPath = path.join(runDir, 'run.lock');
  }

  static async initialize({ plan, planPath, runDir: runDirInput, workspace: workspaceInput, taskWorkspaces: taskWorkspacesInput = {}, runId, coordinatorSessionId, now = Date.now() }) {
    const validation = validatePlan(plan);
    if (!validation?.ok) fail('PLAN_INVALID', (validation?.errors ?? ['Plan failed validation.']).join('\n'));
    if (!path.isAbsolute(runDirInput) || !path.isAbsolute(workspaceInput)) fail('ABSOLUTE_PATH_REQUIRED', 'Run directory and workspace must be absolute paths.');
    const workspace = await assertGitRoot(workspaceInput);
    const parent = await realpath(path.dirname(path.resolve(runDirInput))).catch((error) => fail('RUN_DIR_PARENT', 'Run directory parent must exist.', error));
    const runDir = path.join(parent, path.basename(path.resolve(runDirInput)));
    try {
      await mkdir(runDir);
    } catch (error) {
      if (error?.code === 'EEXIST') fail('RUN_DIR_EXISTS', 'Run directory already exists; initialize never overwrites an existing run.');
      fail('RUN_DIR_CREATE', 'Could not create external run directory.', error);
    }
    const realRunDir = await realpath(runDir).catch((error) => fail('RUN_DIR_RESOLVE', 'Could not resolve run directory.', error));
    let baseline;
    const taskWorkspaces = {};
    const sourceRoots = new Set([workspace]);
    try {
      baseline = await captureCandidate(workspace);
      if (baseline.sha !== plan.baselineSha.toLowerCase()) fail('BASELINE_MISMATCH', 'Workspace HEAD does not match plan.baselineSha.');
      for (const task of plan.tasks) {
        const taskRoot = await assertGitRoot(taskWorkspacesInput[task.id] ?? workspace);
        const taskBaseline = await captureCandidate(taskRoot);
        if (!sameJson(taskBaseline, baseline)) fail('TASK_WORKSPACE_BASELINE', `Task workspace ${task.id} must begin at the same clean baseline commit and tree.`);
        taskWorkspaces[task.id] = taskRoot;
        sourceRoots.add(taskRoot);
      }
      for (const taskId of Object.keys(taskWorkspacesInput)) {
        if (!plan.tasks.some((task) => task.id === taskId)) fail('TASK_WORKSPACE_UNKNOWN', `Task workspace mapping contains unknown task ${taskId}.`);
      }
      if ([...sourceRoots].some((root) => isWithin(root, realRunDir) || isWithin(realRunDir, root))) {
        fail('RUN_DIR_IN_SOURCE_TREE', 'Run directory must be outside every candidate Git worktree and its parent chain.');
      }
    } catch (error) {
      await rm(realRunDir, { recursive: true, force: true }).catch(() => {});
      throw error;
    }
    const runStoreDir = realRunDir;
    await mkdir(path.join(runStoreDir, 'events'));
    await mkdir(path.join(runStoreDir, 'objects'));
    const planCopy = path.join(runStoreDir, 'plan.json');
    const normalizedPlan = `${canonical(plan)}\n`;
    await writeDurable(planCopy, normalizedPlan, { exclusive: true });
    const planDigest = digest(plan);
    const manifest = {
      schemaVersion: RECORD_SCHEMA,
      runId,
      coordinatorSessionId,
      workspace,
      taskWorkspaces,
      planDigest,
      baseline,
      createdAt: now,
      sourcePlanPath: planPath ? path.resolve(planPath) : null,
    };
    await writeDurable(path.join(runStoreDir, 'manifest.json'), `${canonical(manifest)}\n`, { exclusive: true });
    const initial = createState(plan, { runId, now, coordinatorSessionId });
    const store = new RunStore({ runDir: runStoreDir, manifest, plan, workspace, taskWorkspaces });
    await store.writeCache(initial, '0'.repeat(64), 0);
    return store;
  }

  static async open(runDirInput) {
    if (!path.isAbsolute(runDirInput)) fail('ABSOLUTE_PATH_REQUIRED', 'Run directory must be an absolute path.');
    const runDir = await realpath(runDirInput).catch((error) => fail('RUN_DIR_MISSING', 'Run directory does not exist.', error));
    const manifest = await readJson(path.join(runDir, 'manifest.json'), 'run manifest');
    const plan = await readJson(path.join(runDir, 'plan.json'), 'immutable plan copy');
    const validation = validatePlan(plan);
    if (!validation?.ok) fail('PLAN_INVALID', (validation?.errors ?? ['Plan failed validation.']).join('\n'));
    const planDigest = digest(plan);
    if (manifest.schemaVersion !== RECORD_SCHEMA || manifest.planDigest !== planDigest) fail('PLAN_DIGEST_MISMATCH', 'Saved plan does not match the run manifest.');
    if (typeof manifest.workspace !== 'string' || !path.isAbsolute(manifest.workspace)) fail('RUN_MANIFEST_INVALID', 'Run manifest workspace must be absolute.');
    const workspace = await assertGitRoot(manifest.workspace);
    const taskWorkspaces = {};
    for (const task of plan.tasks) {
      const taskRootPath = manifest.taskWorkspaces?.[task.id] ?? workspace;
      if (!path.isAbsolute(taskRootPath)) fail('RUN_MANIFEST_INVALID', `Task workspace ${task.id} must be absolute.`);
      const taskRoot = await assertGitRoot(taskRootPath);
      if (isWithin(taskRoot, runDir) || isWithin(runDir, taskRoot)) fail('RUN_DIR_IN_SOURCE_TREE', 'Run directory is no longer outside every candidate Git worktree.');
      taskWorkspaces[task.id] = taskRoot;
    }
    if (isWithin(workspace, runDir) || isWithin(runDir, workspace)) fail('RUN_DIR_IN_SOURCE_TREE', 'Run directory is no longer outside the candidate Git worktree.');
    return new RunStore({ runDir, manifest, plan, workspace, taskWorkspaces });
  }

  workspaceForTask(taskId) {
    const taskWorkspace = this.taskWorkspaces[taskId];
    if (!taskWorkspace) fail('TASK_WORKSPACE_MISSING', `Task ${taskId} has no pinned Git worktree.`);
    return taskWorkspace;
  }

  async acquireLock(ackStaleNonce) {
    const nonce = randomUUID();
    const owner = { pid: process.pid, hostname: os.hostname(), nonce, startedAt: Date.now() };
    try {
      const handle = await open(this.lockPath, 'wx', 0o600);
      try { await handle.writeFile(`${canonical(owner)}\n`); await handle.sync(); }
      finally { await handle.close(); }
      await syncDirectory(this.runDir);
      return async () => {
        try {
          const current = await readJson(this.lockPath, 'run lock');
          if (current.nonce === nonce) await rm(this.lockPath, { force: true });
        } catch { /* A changed or damaged lock is left for explicit recovery. */ }
      };
    } catch (error) {
      if (error?.code !== 'EEXIST') fail('LOCK_CREATE', 'Could not acquire run lock.', error);
    }
    let previous;
    try { previous = await readJson(this.lockPath, 'run lock'); }
    catch (error) { fail('LOCK_AMBIGUOUS', `Run lock is unreadable; preserve it and resolve ownership before recovery (${error.code ?? 'invalid lock'}).`); }
    const liveHere = previous.hostname === os.hostname() && await pidAlive(previous.pid);
    if (liveHere) fail('LOCK_HELD', `Run is locked by process ${previous.pid} on this host.`);
    if (typeof previous.nonce !== 'string' || !ackStaleNonce || ackStaleNonce !== previous.nonce) {
      fail('LOCK_STALE_REQUIRES_ACK', `Stale or remote run lock requires trusted recovery acknowledgment. Lock nonce: ${String(previous.nonce ?? 'unavailable')}.`);
    }
    const reread = await readJson(this.lockPath, 'run lock');
    if (reread.nonce !== previous.nonce) fail('LOCK_CHANGED', 'Run lock changed while recovery was being acknowledged.');
    await rm(this.lockPath);
    return this.acquireLock(undefined);
  }

  async withLock(callback, { ackStaleNonce } = {}) {
    const release = await this.acquireLock(ackStaleNonce);
    try { return await callback(); }
    finally { await release(); }
  }

  async replay({ writeCache: rebuild = true } = {}) {
    const entries = await readdir(this.eventsDir).catch((error) => fail('LEDGER_READ', 'Could not read event ledger directory.', error));
    const names = entries.filter((entry) => EVENT_NAME.test(entry)).sort();
    const unexpected = entries.filter((entry) => !EVENT_NAME.test(entry) && !entry.includes('.tmp') && !entry.includes('.pending'));
    if (unexpected.length) fail('LEDGER_EXTRA_FILE', `Event ledger contains unexpected file ${unexpected[0]}.`);
    let state = createState(this.plan, {
      runId: this.manifest.runId,
      now: this.manifest.createdAt,
      coordinatorSessionId: this.manifest.coordinatorSessionId,
    });
    let previousDigest = '0'.repeat(64);
    let sequence = 0;
    const records = [];
    for (const name of names) {
      sequence += 1;
      if (name !== eventRecordName(sequence)) fail('LEDGER_GAP', `Ledger is missing sequence ${eventRecordName(sequence)}.`);
      const record = await readJson(path.join(this.eventsDir, name), `event ${sequence}`);
      cleanRecord(record);
      if (record.schemaVersion !== RECORD_SCHEMA || record.sequence !== sequence) fail('LEDGER_CORRUPT', `Event ${sequence} has an invalid schema or sequence.`);
      if (record.previousDigest !== previousDigest) fail('LEDGER_CHAIN', `Event ${sequence} does not extend the previous digest.`);
      if (record.eventDigest !== eventDigest(record.event)) fail('LEDGER_EVENT_DIGEST', `Event ${sequence} payload digest is invalid.`);
      if (record.digest !== recordDigest(record)) fail('LEDGER_RECORD_DIGEST', `Event ${sequence} record digest is invalid.`);
      let next;
      try { next = applyEvent(this.plan, state, record.event, { now: record.runtime?.now ?? Date.now() }); }
      catch (error) { fail('LEDGER_REPLAY', `Accepted event ${sequence} no longer replays: ${error.message}`, error); }
      if (!next || next.version !== state.version + 1) fail('LEDGER_VERSION', `Accepted event ${sequence} did not advance state version exactly once.`);
      state = next;
      previousDigest = record.digest;
      records.push(record);
    }
    const tempFiles = entries.filter((entry) => entry.includes('.tmp') || entry.includes('.pending'));
    if (rebuild) await this.writeCache(state, previousDigest, sequence);
    return { state, records, headDigest: previousDigest, sequence, orphanTemps: tempFiles };
  }

  async writeCache(state, headDigest, sequence) {
    const body = { schemaVersion: RECORD_SCHEMA, sequence, headDigest, stateDigest: digest(state), state };
    await atomicWrite(path.join(this.runDir, 'state-cache.json'), `${canonical(body)}\n`, 'cache-tmp');
  }

  async appendEvent(event, { runtime = {}, now = Date.now(), dispatchCandidate = null, artifactBlobs = null, allowRunnerEvent = false, allowGateEvents = false } = {}) {
    if (eventNeedsCoordinator(event?.type) && event.actorSessionId !== this.manifest.coordinatorSessionId) {
      fail('COORDINATOR_ACTOR', 'Coordinator-only event must use the trusted coordinator session from the run manifest.');
    }
    if (!allowRunnerEvent && (event?.type === 'command_reserved' || event?.type === 'command_completed')) {
      fail('RUNNER_EVENT_ONLY', 'Command reservation/completion events can only be created by the approved command runner.');
    }
    if (!allowGateEvents && (event?.type === 'gate' || event?.type === 'gate_report_reserved')) {
      fail('GATE_REPORT_REQUIRED', 'Gate and gate reservation events can only be created by the bounded gate-report runner.');
    }
    const replayed = await this.replay({ writeCache: false });
    const { state } = replayed;
    const acceptedAt = Math.max(now, state.lastNow ?? now);
    if (now < (state.lastNow ?? now)) fail('CLOCK_BACKWARD', 'Trusted CLI time moved backward from the last accepted event.');
    let next;
    try { next = applyEvent(this.plan, state, event, { now: acceptedAt }); }
    catch (error) {
      if (error instanceof RuntimeError) throw error;
      fail(error.code ?? 'EVENT_REJECTED', error.message ?? 'Event was rejected.', error);
    }
    if (next.version === state.version) return { state, idempotent: true, sequence: replayed.sequence, headDigest: replayed.headDigest, dispatchRequest: null };
    if (next.version !== state.version + 1) fail('STATE_VERSION', 'Guard reducer must increment state version exactly once per accepted event.');
    const sequence = replayed.sequence + 1;
    const newlyDispatched = event.type === 'dispatch'
      ? Object.values(next.dispatchRequests ?? {}).find((item) => item.taskId === event.payload.taskId
        && !Object.hasOwn(state.dispatchRequests ?? {}, item.attemptId))
      : null;
    const savedDispatchCandidate = dispatchCandidate && newlyDispatched
      ? { ...dispatchCandidate, attemptId: newlyDispatched.attemptId, fence: newlyDispatched.fence }
      : dispatchCandidate;
    const record = {
      schemaVersion: RECORD_SCHEMA,
      sequence,
      previousDigest: replayed.headDigest,
      eventDigest: eventDigest(event),
      event,
      runtime: { now: acceptedAt, ...runtime, ...(savedDispatchCandidate ? { dispatchCandidate: savedDispatchCandidate } : {}), ...(artifactBlobs ? { artifactBlobs } : {}) },
    };
    record.digest = recordDigest(record);
    const target = path.join(this.eventsDir, eventRecordName(sequence));
    const temporary = `${target}.${process.pid}.${randomUUID()}.tmp`;
    await writeDurable(temporary, `${canonical(record)}\n`);
    isTestCrashPoint('before-event-rename');
    try {
      await rename(temporary, target);
      await syncDirectory(this.eventsDir);
    } catch (error) {
      await rm(temporary, { force: true }).catch(() => {});
      fail('EVENT_COMMIT', 'Could not commit event record; no event was accepted.', error);
    }
    isTestCrashPoint('after-event-rename');
    try {
      await this.writeCache(next, record.digest, sequence);
    } catch {
      // Event rename already committed. Replay will reconstruct the cache.
    }
    isTestCrashPoint('after-cache-rename');
    return {
      state: next,
      idempotent: false,
      sequence,
      headDigest: record.digest,
      dispatchRequest: newlyDispatched ?? null,
      record,
    };
  }

  async dispatchRequestFor(eventResult) {
    return eventResult?.dispatchRequest ?? null;
  }

  async verifySubmission(packet) {
    const task = findTask(this.plan, packet.taskId);
    const taskWorkspace = this.workspaceForTask(packet.taskId);
    const replayed = await this.replay({ writeCache: false });
    const dispatchRecord = [...replayed.records].reverse().find((record) => record.event.type === 'dispatch'
      && record.event.payload.taskId === packet.taskId
      && record.runtime?.dispatchCandidate
      && Object.values(replayed.state.tasks ?? {}).some((taskState) => {
        const attempt = taskState?.attempt ?? taskState?.currentAttempt;
        return attempt?.id === packet.attemptId || attempt?.attemptId === packet.attemptId;
      }));
    // The reducer remains authoritative for attempt/session/fence state; this lookup
    // only recovers the candidate captured when its dispatch was durably reserved.
    const matchingContext = [...replayed.records].reverse().find((record) => record.event.type === 'dispatch'
      && record.event.payload.taskId === packet.taskId
      && record.runtime?.dispatchCandidate
      && (record.runtime.dispatchCandidate.attemptId === packet.attemptId
        || record.runtime.dispatchCandidate.taskId === packet.taskId));
    const exactContext = [...replayed.records].reverse().find((record) => record.event.type === 'dispatch'
      && record.event.payload.taskId === packet.taskId
      && record.runtime?.dispatchCandidate?.attemptId === packet.attemptId);
    const context = exactContext ?? matchingContext ?? dispatchRecord;
    if (!context?.runtime?.dispatchCandidate) fail('DISPATCH_CANDIDATE_MISSING', 'No durable candidate snapshot exists for this task reservation.');
    await validateTaskScopes(taskWorkspace, task);
    const candidate = await validatePacketCandidate({
      workspace: taskWorkspace,
      plan: this.plan,
      task,
      packet,
      dispatchCandidate: context.runtime.dispatchCandidate,
    });
    const artifacts = await verifyPacketArtifacts(this.runDir, taskWorkspace, packet);
    const commandReceipts = this.verifyPacketCommands(packet, replayed.records);
    return { task, candidate, artifacts, commandReceipts, dispatchRecord: context };
  }

  verifyPacketCommands(packet, records) {
    const checked = [];
    for (const command of packet.commands) {
      const record = [...records].reverse().find((item) => item.event.type === 'command_completed'
        && item.event.payload.commandRunId === command.commandRunId);
      if (!record) fail('COMMAND_RECEIPT_MISSING', `Command run ${command.commandRunId} has no durable completion receipt.`);
      const reservation = [...records].reverse().find((item) => item.event.type === 'command_reserved'
        && item.event.payload.commandRunId === command.commandRunId);
      if (!reservation || reservation.event.payload.taskId !== packet.taskId || reservation.event.payload.attemptId !== packet.attemptId || reservation.event.payload.fence !== packet.fence) {
        fail('COMMAND_RECEIPT_SCOPE', `Command run ${command.commandRunId} belongs to a different task attempt.`);
      }
      const actual = record.event.payload.receipt;
      const { commandRunId, ...declared } = command;
      if (!sameJson(actual, declared)) fail('COMMAND_RECEIPT_MISMATCH', `Command run ${command.commandRunId} receipt differs from captured execution.`);
      if (record.runtime?.candidateMutation || (record.runtime?.candidateBefore && !sameJson(record.runtime.candidateBefore, record.runtime.candidateAfter))) {
        fail('COMMAND_MUTATED_CANDIDATE', `Command run ${command.commandRunId} changed the Git candidate.`);
      }
      if (!sameJson(record.runtime?.candidateBefore, packet.candidate) || !sameJson(record.runtime?.candidateAfter, packet.candidate)) {
        fail('COMMAND_CANDIDATE_MISMATCH', `Command run ${commandRunId} did not execute against the handoff candidate.`);
      }
      checked.push({ commandRunId, receipt: actual, eventDigest: record.eventDigest });
    }
    return checked;
  }

  async verifyPacketBytes(packet, records) {
    const record = [...records].reverse().find((item) => item.event.type === 'submit_handoff'
      && item.event.payload.packet.packetId === packet.packetId);
    if (!record) fail('PACKET_RECORD_MISSING', `Packet ${packet.packetId} has no accepted submission record.`);
    const saved = record.runtime?.artifactBlobs;
    if (!Array.isArray(saved) || saved.length !== packet.artifacts.length) fail('ARTIFACT_EVIDENCE_MISSING', `Packet ${packet.packetId} artifact snapshots are incomplete.`);
    for (const artifact of packet.artifacts) {
      const blob = saved.find((item) => item.path === artifact.path && item.sha256 === artifact.sha256 && item.size === artifact.size);
      if (!blob || !/^objects\/[a-f0-9]{64}$/.test(blob.object)) fail('ARTIFACT_EVIDENCE_MISSING', `Packet artifact ${artifact.path} has no matching immutable blob.`);
      const file = path.join(this.runDir, blob.object);
      const bytes = await readFile(file).catch((error) => fail('ARTIFACT_BLOB_MISSING', `Stored artifact blob ${blob.sha256} is missing.`, error));
      if (bytes.length !== artifact.size || sha256(bytes) !== artifact.sha256) fail('ARTIFACT_BLOB_CORRUPT', `Stored artifact blob ${blob.sha256} failed its hash check.`);
    }
    const checkedCommands = this.verifyPacketCommands(packet, records);
    for (const checked of checkedCommands) {
      const completion = [...records].reverse().find((item) => item.event.type === 'command_completed'
        && item.event.payload.commandRunId === checked.commandRunId);
      for (const [field, expectedDigest] of [['stdoutBlob', checked.receipt.stdoutDigest], ['stderrBlob', checked.receipt.stderrDigest]]) {
        const reference = completion?.runtime?.[field];
        if (!reference || reference.sha256 !== expectedDigest || !/^objects\/[a-f0-9]{64}$/.test(reference.object)) {
          fail('COMMAND_OUTPUT_MISSING', `Command ${checked.commandRunId} has no matching immutable ${field} blob.`);
        }
        const bytes = await readFile(path.join(this.runDir, reference.object)).catch((error) => fail('COMMAND_OUTPUT_MISSING', `Command ${checked.commandRunId} ${field} blob is missing.`, error));
        if (bytes.length !== reference.size || sha256(bytes) !== expectedDigest) fail('COMMAND_OUTPUT_CORRUPT', `Command ${checked.commandRunId} ${field} blob failed its hash check.`);
      }
    }
    return record;
  }

  async verifyEvidenceDigests(digests) {
    for (const value of digests) {
      if (!/^[a-f0-9]{64}$/i.test(value)) fail('EVIDENCE_DIGEST', 'Evidence digests must be SHA-256 hex values.');
      const objectPath = path.join(this.runDir, 'objects', value.toLowerCase());
      const bytes = await readFile(objectPath).catch((error) => fail('EVIDENCE_BLOB_MISSING', `Evidence blob ${value} is missing.`, error));
      if (sha256(bytes) !== value.toLowerCase()) fail('EVIDENCE_BLOB_CORRUPT', `Evidence blob ${value} failed its hash check.`);
    }
  }

  async verifyAcceptedPackets(state, records) {
    for (const task of this.plan.tasks) {
      const packetId = state.tasks[task.id]?.acceptedPacketId;
      if (!packetId) fail('TASK_UNVERIFIED', `Task ${task.id} has no currently accepted handoff.`);
      await this.verifyCurrentPacketEvent({ payload: { packetId, taskId: task.id } }, records);
    }
  }

  async readGateReport(reportDigest) {
    if (!HASH.test(reportDigest)) fail('GATE_REPORT_DIGEST', 'Stored gate report digest is invalid.');
    const bytes = await readFile(path.join(this.runDir, 'objects', reportDigest)).catch((error) => fail('GATE_REPORT_MISSING', `Gate report ${reportDigest} is missing.`, error));
    if (sha256(bytes) !== reportDigest) fail('GATE_REPORT_CORRUPT', `Gate report ${reportDigest} failed its hash check.`);
    return { bytes, report: parseJsonNoDuplicates(bytes.toString('utf8'), `gate report ${reportDigest}`) };
  }

  async executeGateReport({ input, now = Date.now(), ackStaleNonce } = {}) {
    requireGateInput(input);
    return this.withLock(async () => {
      const before = await this.replay({ writeCache: false });
      const { state } = before;
      if (!state.candidate || !state.receipts.integration) fail('NO_INTEGRATED_CANDIDATE', 'Gate reports require a currently integrated candidate.');
      const frozenCandidate = state.candidate;
      const actualCandidate = await captureCandidate(this.workspace);
      if (!sameJson(actualCandidate, frozenCandidate)) fail('STALE_CANDIDATE', 'Integrated candidate changed before the gate report reservation.');
      await this.verifyAcceptedPackets(state, before.records);

      const stageOrder = ['verify', 'review', 'audit', 'final'];
      const priorStages = {
        verify: [], review: ['verify'], audit: ['verify', 'review'], final: ['verify', 'review', 'audit'],
      }[input.stage];
      const priorEvidenceDigests = priorStages.map((stage) => state.receipts.gates[stage]?.reportDigest);
      if (priorEvidenceDigests.some((value) => typeof value !== 'string')) fail('PRIOR_REPORT_MISSING', `Gate ${input.stage} is missing a prior report.`);
      const priorReports = [];
      for (let index = 0; index < priorStages.length; index += 1) {
        const report = await this.readGateReport(priorEvidenceDigests[index]);
        if (report.report.stage !== priorStages[index] || report.report.candidateDigest !== digest(frozenCandidate)) {
          fail('PRIOR_REPORT_MISMATCH', `Prior ${priorStages[index]} report does not bind this integrated candidate.`);
        }
        priorReports.push({ stage: priorStages[index], digest: priorEvidenceDigests[index] });
      }
      await this.verifyEvidenceDigests(priorEvidenceDigests);

      const requiredAc = this.plan.acceptance.filter((ac) => ac.required);
      const expectedCommandIds = uniqueStable(requiredAc.filter((ac) => ac.expected !== 'expected-red').map((ac) => ac.commandId));
      if (input.stage === 'verify') {
        if (!sameJson(input.commandIds, expectedCommandIds)) fail('GATE_INPUT_COMMANDS', 'Verify commandIds must exactly cover required normal and closure ACs in plan order.');
      } else if (input.commandIds.length !== 0) {
        fail('GATE_INPUT_COMMANDS', `${input.stage} reports do not rerun Verify commands; commandIds must be empty.`);
      }

      const reportRunId = randomUUID();
      const reservationEvent = {
        id: randomUUID(),
        type: 'gate_report_reserved',
        expectedVersion: state.version,
        actorSessionId: input.sessionId,
        payload: { stage: input.stage, candidateDigest: digest(frozenCandidate), reportRunId },
      };
      const reserved = await this.appendEvent(reservationEvent, {
        now, runtime: { candidateBefore: frozenCandidate }, allowGateEvents: true,
      });

      let verdict = input.verdict;
      let findings = input.findings.map((finding) => ({ ...finding }));
      let runnerErrors = [];
      let commandReceipts = [];
      let freshAcResults = new Map();
      let commandEvidenceDigests = [];
      let candidateAfter = frozenCandidate;

      if (input.stage === 'verify') {
        const totalTimeoutMs = input.commandIds.reduce((sum, commandId) => sum + this.plan.commands[commandId].timeoutMs, 0);
        const verifyDeadline = Date.now() + totalTimeoutMs;
        for (const commandId of input.commandIds) {
          const command = this.plan.commands[commandId];
          const commandRunId = randomUUID();
          const expectedRows = this.plan.acceptance.filter((ac) => ac.required && ac.expected !== 'expected-red' && ac.commandId === commandId);
          const started = Date.now();
          let receipt;
          let stdoutBlob = null;
          let stderrBlob = null;
          let commandBefore = null;
          let commandAfter = null;
          try {
            commandBefore = await captureCandidate(this.workspace);
            if (!sameJson(commandBefore, frozenCandidate)) fail('GATE_CANDIDATE_CHANGED', `Integrated candidate changed before Verify command ${commandId}.`);
            const remaining = Math.min(command.timeoutMs, verifyDeadline - Date.now());
            if (remaining <= 0) fail('GATE_VERIFY_BUDGET', 'Integrated Verify exhausted its pinned total command timeout.');
            const executable = await resolveExecutable(command.executable);
            const cwd = await resolveExistingParents(this.workspace, command.cwd);
            const cwdInfo = await stat(cwd).catch((error) => fail('GATE_COMMAND_CWD', `Verify command ${commandId} cwd is missing.`, error));
            if (!cwdInfo.isDirectory() || !isWithin(this.workspace, await realpath(cwd))) fail('GATE_COMMAND_CWD', `Verify command ${commandId} cwd escapes the integrated workspace.`);
            const envDigest = await environmentFingerprint(executable.path);
            const execution = await spawnBounded(executable.path, command.args, {
              cwd, timeoutMs: remaining, outputLimit: MAX_COMMAND_OUTPUT_BYTES, env: process.env,
            });
            const durationMs = Math.max(0, Date.now() - started);
            const stdoutDigest = sha256(execution.stdout);
            const stderrDigest = sha256(execution.stderr);
            stdoutBlob = await persistBlob(this.runDir, execution.stdout, stdoutDigest, `Verify ${commandId} stdout`);
            stderrBlob = await persistBlob(this.runDir, execution.stderr, stderrDigest, `Verify ${commandId} stderr`);
            const parsed = parseStructuredResult(execution.stdout.toString('utf8'), commandId, expectedRows);
            const isCleanSuccess = execution.exitCode === 0 && execution.signal === null && !execution.timedOut && !execution.spawnError;
            const outputMatches = parsed.error === null;
            if (!isCleanSuccess) runnerErrors.push(`Verify command ${commandId} did not exit successfully without signal or timeout.`);
            if (!outputMatches) runnerErrors.push(`Verify command ${commandId} result marker was invalid: ${parsed.error}.`);
            if (execution.stdoutTruncated || execution.stderrTruncated) runnerErrors.push(`Verify command ${commandId} output was truncated.`);
            if (execution.completionUncertain) runnerErrors.push(`Verify command ${commandId} completion could not be confirmed after process termination.`);
            if (execution.spawnError) runnerErrors.push(`Verify command ${commandId} could not spawn: ${execution.spawnError}.`);
            try {
              commandAfter = await captureCandidate(this.workspace);
              if (!sameJson(commandBefore, commandAfter)) runnerErrors.push(`Verify command ${commandId} changed the integrated candidate.`);
            } catch (error) {
              runnerErrors.push(`Verify command ${commandId} left a dirty or unreadable candidate: ${error.message}.`);
            }
            if (commandBefore && commandAfter && sameJson(commandBefore, frozenCandidate)
              && sameJson(commandAfter, frozenCandidate) && outputMatches) {
              for (const row of parsed.caseResults) {
                freshAcResults.set(row.acId, {
                  acId: row.acId, caseId: row.caseId, outcome: row.outcome,
                  testCount: row.testCount, evidenceDigest: stdoutDigest,
                });
              }
            }
            receipt = {
              commandRunId, commandId, cwd: command.cwd,
              exitCode: execution.exitCode, signal: execution.signal, timedOut: execution.timedOut,
              durationMs, testCount: parsed.testCount, caseResults: parsed.caseResults,
              environmentDigest: envDigest, stdoutDigest, stderrDigest,
              stdoutBlob, stderrBlob, candidateBefore: commandBefore, candidateAfter: commandAfter,
              stdoutTruncated: execution.stdoutTruncated, stderrTruncated: execution.stderrTruncated,
              completionUncertain: execution.completionUncertain, spawnError: execution.spawnError,
              resultMarkerError: parsed.error,
            };
            commandEvidenceDigests.push(stdoutDigest, stderrDigest);
          } catch (error) {
            runnerErrors.push(`Verify command ${commandId} could not be captured: ${error.message}.`);
            receipt = {
              commandRunId, commandId, cwd: command.cwd, exitCode: null, signal: null,
              timedOut: false, durationMs: Math.max(0, Date.now() - started), testCount: 0,
              caseResults: [], environmentDigest: null, stdoutDigest: null, stderrDigest: null,
              stdoutBlob, stderrBlob, candidateBefore: commandBefore, candidateAfter,
              runnerError: { code: error.code ?? 'GATE_COMMAND_ERROR', message: error.message },
            };
          }
          commandReceipts.push(receipt);
          if (receipt.candidateAfter) candidateAfter = receipt.candidateAfter;
          if (runnerErrors.some((message) => message.includes(`command ${commandId} changed the integrated candidate`)
            || message.includes(`command ${commandId} left a dirty or unreadable candidate`))) {
            break;
          }
        }
        if (commandReceipts.length !== input.commandIds.length) {
          for (const commandId of input.commandIds.slice(commandReceipts.length)) {
            if (commandReceipts.some((receipt) => receipt.commandId === commandId)) continue;
            commandReceipts.push({ commandId, notRun: 'candidate changed or Verify stopped after an earlier command failure' });
          }
        }
        const historicalRed = new Map();
        for (const ac of requiredAc.filter((item) => item.expected === 'expected-red')) {
          const result = state.tasks[ac.taskId].acceptedAcResults.find((item) => item.acId === ac.id && item.caseId === ac.caseId);
          if (!result || result.outcome !== 'expected-red' || result.testCount < 1) {
            runnerErrors.push(`Historical expected-red case ${ac.id} has no accepted task evidence.`);
            continue;
          }
          historicalRed.set(ac.id, { ...result });
        }
        const allGateResults = [];
        for (const ac of requiredAc) {
          const result = ac.expected === 'expected-red' ? historicalRed.get(ac.id) : freshAcResults.get(ac.id);
          if (result) allGateResults.push(result);
          else runnerErrors.push(`Required Verify AC ${ac.id} has no accepted evidence.`);
        }
        freshAcResults = new Map(allGateResults.map((row) => [row.acId, row]));
      } else {
        freshAcResults = new Map((state.receipts.gates.verify?.acResults ?? []).map((row) => [row.acId, { ...row }]));
      }

      try {
        candidateAfter = await captureCandidate(this.workspace);
        if (!sameJson(candidateAfter, frozenCandidate)) runnerErrors.push('Integrated candidate changed while the gate report was being generated.');
      } catch (error) {
        candidateAfter = null;
        runnerErrors.push(`Integrated candidate is dirty or unreadable after gate execution: ${error.message}.`);
      }
      if (runnerErrors.length > 0) verdict = 'fail';
      if (findings.some((finding) => finding.blocking)) verdict = 'fail';
      if (verdict === 'fail' && !findings.some((finding) => finding.blocking)) {
        findings.push({ fingerprint: `gate-runner/${input.stage}/${reportRunId}`, blocking: true, critical: false });
      }

      const acResults = [...freshAcResults.values()];
      const priorAcEvidence = acResults.map((row) => row.evidenceDigest);
      const commandDigests = commandEvidenceDigests.filter((value) => typeof value === 'string' && HASH.test(value));
      const referencedDigests = uniqueStable([...priorEvidenceDigests, ...priorAcEvidence, ...commandDigests]);
      await this.verifyEvidenceDigests(referencedDigests);

      const report = {
        schemaVersion: 1,
        runId: this.manifest.runId,
        planDigest: this.manifest.planDigest,
        stage: input.stage,
        reportRunId,
        sessionId: input.sessionId,
        candidate: frozenCandidate,
        candidateDigest: digest(frozenCandidate),
        candidateAfter,
        verdict,
        findings,
        assessment: input.assessment,
        commandIds: [...input.commandIds],
        commandReceipts,
        runnerErrors,
        acResults,
        priorEvidenceDigests,
        priorReports,
        generatedAt: Date.now(),
      };
      const reportBytes = Buffer.from(`${canonical(report)}\n`, 'utf8');
      const reportDigest = sha256(reportBytes);
      const reportBlob = await persistBlob(this.runDir, reportBytes, reportDigest, `${input.stage} gate report`);
      const evidenceDigests = uniqueStable([reportDigest, ...referencedDigests]);
      await this.verifyEvidenceDigests(evidenceDigests);
      const storedReport = await this.readGateReport(reportDigest);
      if (!sameJson(storedReport.report, report)) fail('GATE_REPORT_MISMATCH', 'Persisted report bytes differ from the report used for the gate event.');

      const gateEvent = {
        id: randomUUID(),
        type: 'gate',
        expectedVersion: reserved.state.version,
        actorSessionId: input.sessionId,
        payload: {
          stage: input.stage,
          candidateDigest: digest(frozenCandidate),
          reportRunId,
          priorEvidenceDigests,
          verdict,
          acResults,
          findings,
          evidenceDigests,
        },
      };
      const completed = await this.appendEvent(gateEvent, {
        now: Date.now(),
        runtime: { candidateBefore: frozenCandidate, candidateAfter, reportBlob },
        allowGateEvents: true,
      });
      return {
        state: completed.state,
        stage: input.stage,
        verdict,
        reportRunId,
        reportDigest,
        candidate: frozenCandidate,
        candidateAfter,
        commandReceipts,
        runnerErrors,
        evidenceDigests,
        reservationSequence: reserved.sequence,
        completionSequence: completed.sequence,
      };
    }, { ackStaleNonce });
  }

  async verifyIntegratedCandidate(event, state, records) {
    const candidate = await captureCandidate(this.workspace);
    if (!sameJson(event.payload.candidate, candidate)) fail('INTEGRATION_CANDIDATE_MISMATCH', 'Integration candidate does not match the clean checked Git worktree.');
    const integratedChanges = await changedPathsSince(this.workspace, this.plan.baselineSha, candidate.sha);
    const approvedScopes = this.plan.tasks.flatMap((task) => task.writePaths.map((scope) => normalizePath(scope)));
    for (const changed of integratedChanges) {
      if (!approvedScopes.some((scope) => isPathWithinScope(changed, scope))) {
        fail('INTEGRATION_SCOPE', `Integrated change ${changed} is outside all approved task write scopes.`);
      }
    }
    for (const commit of event.payload.taskCommits) {
      const packetRecord = [...records].reverse().find((record) => record.event.type === 'submit_handoff'
        && record.event.payload.packet.taskId === commit.taskId
        && state.tasks[commit.taskId]?.acceptedPacketId === record.event.payload.packet.packetId);
      if (!packetRecord || packetRecord.event.payload.packet.candidate.sha !== commit.sha) fail('INTEGRATION_COMMIT_MISMATCH', `Task commit ${commit.taskId} is not the accepted packet candidate.`);
      await validatePacketCandidate({
        workspace: this.workspaceForTask(commit.taskId),
        plan: this.plan,
        task: findTask(this.plan, commit.taskId),
        packet: packetRecord.event.payload.packet,
        dispatchCandidate: packetRecord.runtime?.dispatchCandidate,
      });
      const ancestry = childSpawnSync('git', [...gitBaseArgs(this.workspace), 'merge-base', '--is-ancestor', commit.sha, candidate.sha], {
        cwd: this.workspace, windowsHide: true, shell: false,
      });
      if (ancestry.error || ancestry.status !== 0) fail('INTEGRATION_COMMIT_UNREACHABLE', `Task commit ${commit.taskId} is not reachable from the integrated candidate.`);
      await this.verifyPacketBytes(packetRecord.event.payload.packet, records);
    }
    await this.verifyEvidenceDigests(event.payload.artifactDigests);
  }

  async verifyCurrentPacketEvent(event, records) {
    const submitted = [...records].reverse().find((record) => record.event.type === 'submit_handoff'
      && record.event.payload.packet.packetId === event.payload.packetId);
    if (!submitted) fail('PACKET_RECORD_MISSING', `Packet ${event.payload.packetId} has no accepted submission record.`);
    const packet = submitted.event.payload.packet;
    if (packet.taskId !== event.payload.taskId) fail('PACKET_TASK_MISMATCH', 'Event taskId does not match its accepted packet.');
    const task = findTask(this.plan, packet.taskId);
    const taskWorkspace = this.workspaceForTask(packet.taskId);
    await validatePacketCandidate({
      workspace: taskWorkspace,
      plan: this.plan,
      task,
      packet,
      dispatchCandidate: submitted.runtime?.dispatchCandidate,
    });
    await this.verifyPacketBytes(packet, records);
  }

  async ensureSharedWorkspaceIdle(taskId, state) {
    const selected = this.workspaceForTask(taskId);
    for (const otherTask of this.plan.tasks) {
      if (otherTask.id === taskId || this.workspaceForTask(otherTask.id) !== selected) continue;
      const otherState = state.tasks[otherTask.id];
      const current = otherState.currentAttemptId
        ? otherState.attempts.find((attempt) => attempt.attemptId === otherState.currentAttemptId)
        : null;
      if (current && ['reserved', 'running', 'handoff_pending', 'verifying', 'quarantined'].includes(current.status)) {
        fail('WORKSPACE_BUSY', `Task workspace is already in use by active attempt ${current.attemptId}.`);
      }
    }
  }

  async processEvent(event, { now = Date.now(), ackStaleNonce } = {}) {
    return this.withLock(async () => {
      if (event?.type === 'gate' || event?.type === 'gate_report_reserved') {
        fail('GATE_REPORT_REQUIRED', 'Gate and gate reservation events can only be created by the bounded gate-report runner.');
      }
      const before = await this.replay({ writeCache: false });
      const preflight = applyEvent(this.plan, before.state, event, { now });
      if (preflight.version === before.state.version) {
        return { state: before.state, idempotent: true, sequence: before.sequence, headDigest: before.headDigest, dispatchRequest: null };
      }
      let dispatchCandidate = null;
      let artifactBlobs = null;
      if (event.type === 'dispatch') {
        const taskWorkspace = this.workspaceForTask(event.payload.taskId);
        await this.ensureSharedWorkspaceIdle(event.payload.taskId, before.state);
        dispatchCandidate = { ...(await captureCandidate(taskWorkspace)), taskId: event.payload.taskId, workspace: taskWorkspace };
        await validateTaskScopes(taskWorkspace, findTask(this.plan, event.payload.taskId));
      } else if (event.type === 'submit_handoff') {
        const verified = await this.verifySubmission(event.payload.packet);
        dispatchCandidate = verified.dispatchRecord.runtime.dispatchCandidate;
        artifactBlobs = verified.artifacts;
      } else if (['ack_handoff', 'reject_handoff', 'task_verdict'].includes(event.type)) {
        await this.verifyCurrentPacketEvent(event, before.records);
      } else if (event.type === 'integrate') {
        await this.verifyIntegratedCandidate(event, before.state, before.records);
      }
      const result = await this.appendEvent(event, { now, dispatchCandidate, artifactBlobs });
      const dispatchRequest = event.type === 'dispatch' ? await this.dispatchRequestFor(result) : null;
      return { ...result, dispatchRequest };
    }, { ackStaleNonce });
  }

  async registerEvent(type, payload, { id = randomUUID(), expectedVersion, actorSessionId, now = Date.now() } = {}) {
    const replayed = await this.replay({ writeCache: false });
    const event = {
      id,
      type,
      expectedVersion: expectedVersion ?? replayed.state.version,
      actorSessionId: actorSessionId ?? this.manifest.coordinatorSessionId,
      payload,
    };
    return this.processEvent(event, { now });
  }

  async executeApprovedCommand({ taskId, attemptId, commandId, now = Date.now(), ackStaleNonce }) {
    return this.withLock(async () => {
      findTask(this.plan, taskId);
      const command = this.plan.commands[commandId];
      if (!command) fail('COMMAND_UNKNOWN', `Command ${commandId} is not in the immutable plan.`);
      const beforeState = (await this.replay({ writeCache: false })).state;
      const request = beforeState.dispatchRequests?.[attemptId];
      if (!request || request.taskId !== taskId) fail('ATTEMPT_NOT_DISPATCHED', 'Command attempt has no current dispatch request.');
      const taskWorkspace = this.workspaceForTask(taskId);
      const before = await captureCandidate(taskWorkspace);
      const remainingMs = Math.min(request.deadline, request.leaseUntil) - Date.now();
      if (remainingMs <= 0) fail('ATTEMPT_EXPIRED', 'Task deadline or provider lease expired before the command could start.');
      const timeoutMs = Math.min(command.timeoutMs, remainingMs);
      const executable = await resolveExecutable(command.executable);
      const cwd = await resolveExistingParents(taskWorkspace, command.cwd);
      const cwdInfo = await stat(cwd).catch((error) => fail('COMMAND_CWD', `Command working directory ${command.cwd} does not exist.`, error));
      if (!cwdInfo.isDirectory() || !isWithin(taskWorkspace, await realpath(cwd))) fail('COMMAND_CWD', 'Command working directory must resolve inside the task workspace.');
      const envFingerprint = await environmentFingerprint(executable.path);
      const commandRunId = randomUUID();
      const reserve = {
        id: randomUUID(),
        type: 'command_reserved',
        expectedVersion: beforeState.version,
        actorSessionId: this.manifest.coordinatorSessionId,
        payload: { taskId, attemptId, fence: request.fence, commandRunId, commandId },
      };
      const reservation = await this.appendEvent(reserve, { now, runtime: { candidateBefore: before }, allowRunnerEvent: true });
      const started = Date.now();
      const execution = await spawnBounded(executable.path, command.args, {
        cwd,
        timeoutMs,
        outputLimit: MAX_COMMAND_OUTPUT_BYTES,
        env: process.env,
      });
      const durationMs = Math.max(0, Date.now() - started);
      const outputHashes = {
        stdoutDigest: sha256(execution.stdout),
        stderrDigest: sha256(execution.stderr),
      };
      const stdoutBlob = await persistBlob(this.runDir, execution.stdout, outputHashes.stdoutDigest, 'command stdout');
      const stderrBlob = await persistBlob(this.runDir, execution.stderr, outputHashes.stderrDigest, 'command stderr');
      const resultEnvelope = parseStructuredResult(
        execution.stdout.toString('utf8'),
        commandId,
        this.plan.acceptance.filter((ac) => ac.taskId === taskId && ac.commandId === commandId),
      );
      let after = null;
      let candidateMutation = false;
      try { after = await captureCandidate(taskWorkspace); }
      catch {
        // Any failed post-command snapshot is unsafe evidence, including byte
        // mismatches that status/ls-files can hide. Preserve the command receipt
        // and quarantine the writer instead of abandoning its durable ledger.
        candidateMutation = true;
      }
      if (after && !sameJson(before, after)) candidateMutation = true;
      const receipt = {
        commandId,
        cwd: command.cwd,
        exitCode: execution.exitCode,
        signal: execution.signal,
        timedOut: execution.timedOut,
        durationMs,
        testCount: resultEnvelope.testCount,
        caseResults: resultEnvelope.caseResults,
        environmentDigest: envFingerprint,
        stdoutDigest: outputHashes.stdoutDigest,
        stderrDigest: outputHashes.stderrDigest,
      };
      const completeNow = Date.now();
      const completedEvent = {
        id: randomUUID(),
        type: 'command_completed',
        expectedVersion: reservation.state.version,
        actorSessionId: this.manifest.coordinatorSessionId,
        payload: { commandRunId, receipt },
      };
      const completed = await this.appendEvent(completedEvent, {
        now: completeNow,
        runtime: {
          candidateBefore: before,
          candidateAfter: after,
          candidateMutation,
          commandExecutable: executable.path,
          executableDigest: executable.sha256,
          stdoutBlob,
          stderrBlob,
          stdoutTruncated: execution.stdoutTruncated,
          stderrTruncated: execution.stderrTruncated,
          spawnError: execution.spawnError,
          completionUncertain: execution.completionUncertain,
          resultMarkerError: resultEnvelope.error,
        },
        allowRunnerEvent: true,
      });
      let failure = null;
      if (candidateMutation || execution.timedOut || execution.completionUncertain || execution.stdoutTruncated || execution.stderrTruncated || execution.spawnError) {
        const category = execution.timedOut || execution.completionUncertain ? 'timeout' : candidateMutation ? 'code' : 'infra';
        failure = await this.appendEvent({
          id: randomUUID(),
          type: 'fail',
          expectedVersion: completed.state.version,
          actorSessionId: this.manifest.coordinatorSessionId,
          payload: { taskId, attemptId, category, fingerprint: `command/${category}/${commandRunId}` },
        }, { now: Date.now() });
      }
      return {
        state: failure?.state ?? completed.state,
        commandRunId,
        receipt,
        candidateMutation,
        outputs: { stdout: stdoutBlob, stderr: stderrBlob },
        reservationSequence: reservation.sequence,
        completionSequence: completed.sequence,
        failureSequence: failure?.sequence ?? null,
      };
    }, { ackStaleNonce });
  }

  async recover({ now = Date.now(), removeTemps = true, ackStaleNonce } = {}) {
    return this.withLock(async () => {
      const before = await this.replay({ writeCache: false });
      if (removeTemps && before.orphanTemps.length) {
        for (const file of before.orphanTemps) await rm(path.join(this.eventsDir, file), { force: true });
        await syncDirectory(this.eventsDir);
      }
      const event = {
        id: randomUUID(),
        type: 'watchdog',
        expectedVersion: before.state.version,
        actorSessionId: this.manifest.coordinatorSessionId,
        payload: {},
      };
      return this.appendEvent(event, { now });
    }, { ackStaleNonce });
  }

  async status({ ackStaleNonce } = {}) {
    return this.withLock(async () => {
      const replayed = await this.replay();
      const current = await captureCandidate(this.workspace);
      return { state: replayed.state, headDigest: replayed.headDigest, sequence: replayed.sequence, candidate: current, orphanTemps: replayed.orphanTemps };
    }, { ackStaleNonce });
  }
}

async function resolveExecutable(executable) {
  if (typeof executable !== 'string' || executable.length === 0) fail('COMMAND_EXECUTABLE', 'Command executable must be a non-empty string.');
  const candidates = [];
  if (path.isAbsolute(executable) || executable.includes('/') || executable.includes('\\')) {
    candidates.push(path.resolve(executable));
  } else {
    const extensions = process.platform === 'win32' ? (process.env.PATHEXT ?? '.EXE;.CMD;.BAT;.COM').split(';') : [''];
    for (const directory of (process.env.PATH ?? '').split(path.delimiter)) {
      if (!directory) continue;
      for (const extension of extensions) {
        const name = process.platform === 'win32' && !path.extname(executable) ? `${executable}${extension.toLowerCase()}` : executable;
        candidates.push(path.join(directory, name));
      }
    }
  }
  for (const candidate of candidates) {
    try {
      const resolved = await realpath(candidate);
      const info = await stat(resolved);
      if (!info.isFile()) continue;
      await access(resolved, fsConstants.X_OK).catch(() => {
        if (process.platform !== 'win32') throw new Error('not executable');
      });
      const bytes = await readFile(resolved);
      return { path: resolved, sha256: sha256(bytes) };
    } catch { /* Try the next immutable PATH candidate. */ }
  }
  fail('COMMAND_EXECUTABLE', `Could not resolve executable ${executable} using the current PATH.`);
}

async function environmentFingerprint(executablePath) {
  const relevantKeys = [
    'CI', 'LANG', 'LC_ALL', 'NODE_OPTIONS', 'PATH', 'PATHEXT', 'RUSTUP_HOME',
    'CARGO_HOME', 'JAVA_HOME', 'TZ',
  ];
  const env = Object.fromEntries(relevantKeys.filter((key) => process.env[key] !== undefined).map((key) => [key, process.env[key]]));
  const executableBytes = await readFile(executablePath);
  return sha256(Buffer.from(canonical({ env, executablePath, executableDigest: sha256(executableBytes) }), 'utf8'));
}

function parseStructuredResult(stdout, commandId, expected) {
  const markers = stdout.split(/\r?\n/).filter((line) => line.startsWith('WORKFLOW_GUARD_RESULT='));
  if (markers.length !== 1) return { testCount: 0, caseResults: [], error: markers.length === 0 ? 'missing result marker' : 'duplicate result markers' };
  let envelope;
  try { envelope = parseJsonNoDuplicates(markers[0].slice('WORKFLOW_GUARD_RESULT='.length), 'command result marker'); }
  catch (error) { return { testCount: 0, caseResults: [], error: error.message }; }
  if (!envelope || typeof envelope !== 'object' || Array.isArray(envelope) || Object.keys(envelope).length !== 1 || !Array.isArray(envelope.acResults)) {
    return { testCount: 0, caseResults: [], error: 'result marker must contain exactly one acResults array' };
  }
  const caseResults = [];
  const seen = new Set();
  for (const row of envelope.acResults) {
    if (!row || typeof row !== 'object' || Array.isArray(row)
      || Object.keys(row).sort().join(',') !== 'acId,caseId,outcome,testCount'
      || typeof row.acId !== 'string' || typeof row.caseId !== 'string'
      || !['pass', 'schema-invalid', 'unsupported', 'expected-red'].includes(row.outcome)
      || !Number.isSafeInteger(row.testCount) || row.testCount <= 0) {
      return { testCount: 0, caseResults: [], error: 'result marker has an invalid AC result row' };
    }
    const key = `${row.acId}\u0000${row.caseId}`;
    if (seen.has(key)) return { testCount: 0, caseResults: [], error: 'result marker has a duplicate AC result' };
    seen.add(key);
    const declared = expected.find((ac) => ac.id === row.acId && ac.caseId === row.caseId);
    if (!declared || declared.expected !== row.outcome) return { testCount: 0, caseResults: [], error: `result marker AC ${row.acId}/${row.caseId} does not match the immutable plan` };
    caseResults.push({ acId: row.acId, caseId: row.caseId, outcome: row.outcome, testCount: row.testCount });
  }
  if (caseResults.length !== expected.length) return { testCount: 0, caseResults: [], error: 'result marker does not contain the exact command AC inventory' };
  const sum = caseResults.reduce((total, row) => total + row.testCount, 0);
  if (!Number.isSafeInteger(sum)) return { testCount: 0, caseResults: [], error: 'result marker test count is not a safe integer' };
  return { testCount: sum, caseResults, error: null };
}

function spawnBounded(executable, args, { cwd, timeoutMs, outputLimit, env }) {
  return new Promise((resolve) => {
    let child;
    let stdout = Buffer.alloc(0);
    let stderr = Buffer.alloc(0);
    let stdoutTruncated = false;
    let stderrTruncated = false;
    let timedOut = false;
    let spawnError = null;
    let completionUncertain = false;
    let settled = false;
    let timer;
    let hardKillTimer;
    let finalizerTimer;
    const append = (prior, chunk, streamName) => {
      const available = Math.max(0, outputLimit - prior.length);
      const accepted = chunk.subarray(0, available);
      const overflow = chunk.length > accepted.length;
      if (overflow) {
        if (streamName === 'stdout') stdoutTruncated = true;
        else stderrTruncated = true;
      }
      return Buffer.concat([prior, accepted]);
    };
    const finish = (result) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      clearTimeout(hardKillTimer);
      clearTimeout(finalizerTimer);
      resolve({ stdout, stderr, stdoutTruncated, stderrTruncated, completionUncertain, ...result });
    };
    try {
      child = spawn(executable, args, { cwd, env, shell: false, windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'] });
    } catch (error) {
      spawnError = error.message;
      finish({ exitCode: null, signal: null, timedOut: false, spawnError });
      return;
    }
    child.stdout.on('data', (chunk) => {
      stdout = append(stdout, chunk, 'stdout');
      if (stdoutTruncated) child.kill('SIGTERM');
    });
    child.stderr.on('data', (chunk) => {
      stderr = append(stderr, chunk, 'stderr');
      if (stderrTruncated) child.kill('SIGTERM');
    });
    child.on('error', (error) => {
      spawnError = error.message;
      setTimeout(() => finish({ exitCode: null, signal: null, timedOut, spawnError }), 25).unref();
    });
    timer = setTimeout(() => {
      timedOut = true;
      child.kill('SIGTERM');
      hardKillTimer = setTimeout(() => {
        child.kill('SIGKILL');
        finalizerTimer = setTimeout(() => {
          completionUncertain = true;
          child.stdout.destroy();
          child.stderr.destroy();
          finish({ exitCode: child.exitCode, signal: child.signalCode ?? 'SIGKILL', timedOut, spawnError });
        }, 750);
        finalizerTimer.unref();
      }, 500);
      hardKillTimer.unref();
    }, timeoutMs);
    timer.unref();
    child.on('close', (code, signal) => finish({ exitCode: code, signal, timedOut, spawnError }));
  });
}

export { canonical, sha256, sameJson, eventDigest, isWithin, changedPathsSince, resolveExistingParents, persistBlob };
