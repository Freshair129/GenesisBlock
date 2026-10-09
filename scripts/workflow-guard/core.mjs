import { createHash } from 'node:crypto';

const PLAN_KEYS = [
  'schemaVersion', 'id', 'revision', 'baselineSha', 'authorization', 'roles',
  'decisions', 'tasks', 'graph', 'acceptance', 'commands', 'policy',
];
const STATE_KEYS = [
  'schemaVersion', 'runId', 'planDigest', 'version', 'status', 'sessions',
  'tasks', 'locks', 'candidate', 'receipts', 'seenEvents', 'dispatchRequests',
  'lastNow', 'commandRuns',
];
const ROLE_KEYS = [
  'worker', 'research', 'explore', 'verify', 'review', 'build', 'evidence',
  'decision', 'debug', 'integration', 'audit', 'orchestrator', 'final',
];
const LUNA_ROLES = new Set(['worker', 'research', 'explore', 'verify', 'review', 'build', 'evidence']);
const SOL_ROLES = new Set(['decision', 'debug', 'integration', 'audit', 'orchestrator', 'final']);
const EFFORTS = new Set(['minimal', 'low', 'medium', 'high', 'xhigh', 'max', 'ultra']);
const EXPECTED_OUTCOMES = new Set(['pass', 'schema-invalid', 'unsupported', 'expected-red']);
const EVENT_TYPES = new Set([
  'register_session', 'dispatch', 'bind_worker', 'heartbeat', 'command_reserved',
  'command_completed', 'submit_handoff', 'ack_handoff', 'reject_handoff',
  'task_verdict', 'fail', 'stop_confirmed', 'integrate', 'gate',
  'gate_report_reserved', 'invalidate_candidate', 'watchdog', 'abort',
]);

export class GuardError extends Error {
  constructor(code, message) {
    super(message);
    this.name = 'GuardError';
    this.code = code;
  }
}

function fail(code, message) {
  throw new GuardError(code, message);
}

function isRecord(value) {
  if (value === null || typeof value !== 'object' || Array.isArray(value)) return false;
  const proto = Object.getPrototypeOf(value);
  return proto === Object.prototype || proto === null;
}

function isString(value) {
  return typeof value === 'string' && value.trim().length > 0;
}

function isSafeIdentity(value) {
  return isString(value) && !['__proto__', 'constructor', 'prototype'].includes(value.toLowerCase());
}

function setOwn(target, key, value) {
  Object.defineProperty(target, key, { value, enumerable: true, configurable: true, writable: true });
}

function isSafeInt(value, min = 0) {
  return Number.isSafeInteger(value) && value >= min;
}

function sortedUnique(values) {
  return [...new Set(values)].sort();
}

function stableStringify(value) {
  if (value === null || typeof value === 'boolean' || typeof value === 'string') {
    return JSON.stringify(value);
  }
  if (typeof value === 'number') {
    if (!Number.isFinite(value)) throw new TypeError('digest input contains a non-finite number');
    return JSON.stringify(value);
  }
  if (Array.isArray(value)) {
    return `[${value.map((item) => stableStringify(item)).join(',')}]`;
  }
  if (!isRecord(value)) throw new TypeError('digest input must be JSON-compatible');
  const keys = Object.keys(value).sort();
  return `{${keys.map((key) => `${JSON.stringify(key)}:${stableStringify(value[key])}`).join(',')}}`;
}

export function digest(value) {
  return createHash('sha256').update(stableStringify(value), 'utf8').digest('hex');
}

export function normalizePath(path) {
  if (typeof path !== 'string' || path.length === 0) {
    throw new TypeError('path must be a non-empty string');
  }
  const slash = path.replaceAll('\\', '/');
  if (slash.startsWith('/') || /^[a-zA-Z]:/.test(slash) || slash.startsWith('//')) {
    throw new TypeError('absolute, drive-relative, and UNC paths are forbidden');
  }
  if (/[*?\[\]]/.test(slash) || /[\u0000-\u001f\u007f]/.test(slash)) {
    throw new TypeError('wildcards and control characters are forbidden in paths');
  }
  const parts = slash.normalize('NFC').split('/');
  if (parts.some((part) => part === '..')) throw new TypeError('path traversal is forbidden');
  for (const part of parts) {
    if (part !== '.' && (part.endsWith(' ') || part.endsWith('.'))) {
      throw new TypeError('Windows paths ending in a dot or space are forbidden');
    }
    if (/^(con|prn|aux|nul|conin\$|conout\$|com[1-9]|lpt[1-9])(?:\..*)?$/i.test(part)) {
      throw new TypeError('Windows device names are forbidden in paths');
    }
  }
  const normalized = parts.filter((part) => part !== '' && part !== '.').join('/').toLowerCase();
  if (normalized.length === 0) throw new TypeError('path must identify a non-empty relative path');
  if (normalized.split('/').some((part) => part.includes(':'))) {
    throw new TypeError('colon characters are forbidden in relative paths');
  }
  return normalized;
}

export function pathsOverlap(a, b) {
  const left = normalizePath(a);
  const right = normalizePath(b);
  return left === right || left.startsWith(`${right}/`) || right.startsWith(`${left}/`);
}

function deepClone(value) {
  return JSON.parse(JSON.stringify(value));
}

function deepFreeze(value) {
  if (value && typeof value === 'object' && !Object.isFrozen(value)) {
    for (const child of Object.values(value)) deepFreeze(child);
    Object.freeze(value);
  }
  return value;
}

function exactKeys(value, required, optional = []) {
  if (!isRecord(value)) return false;
  const allowed = new Set([...required, ...optional]);
  const keys = Object.keys(value);
  return required.every((key) => Object.hasOwn(value, key)) && keys.every((key) => allowed.has(key));
}

function requireKeys(value, required, optional, code, label) {
  if (!exactKeys(value, required, optional)) fail(code, `${label} has missing or unknown fields`);
}

function validHex(value, lengths = [64]) {
  return typeof value === 'string' && lengths.includes(value.length) && /^[a-f0-9]+$/i.test(value);
}

function cleanSha(value, lengths = [40, 64]) {
  return validHex(value, lengths);
}

function normalizeFingerprint(value) {
  return String(value).normalize('NFC').trim().toLowerCase().replaceAll('\\', '/').replace(/\s+/g, ' ');
}

function topoOrder(tasks) {
  const records = tasks.filter((task) => isRecord(task) && typeof task.id === 'string');
  const byId = new Map(records.map((task) => [task.id, task]));
  const indegree = new Map(records.map((task) => [task.id, 0]));
  const successors = new Map(records.map((task) => [task.id, []]));
  for (const task of records) {
    for (const dependency of Array.isArray(task.dependsOn) ? task.dependsOn : []) {
      if (!byId.has(dependency)) {
        indegree.set(task.id, (indegree.get(task.id) ?? 0) + 1);
        continue;
      }
      indegree.set(task.id, indegree.get(task.id) + 1);
      successors.get(dependency).push(task.id);
    }
  }
  const ready = records.map((task) => task.id).filter((id) => indegree.get(id) === 0);
  const result = [];
  for (let index = 0; index < ready.length; index += 1) {
    const id = ready[index];
    result.push(id);
    for (const successor of successors.get(id) ?? []) {
      indegree.set(successor, indegree.get(successor) - 1);
      if (indegree.get(successor) === 0) ready.push(successor);
    }
  }
  return result;
}

function dependencyReachable(tasks, from, to) {
  const byId = new Map(tasks.filter((task) => isRecord(task) && typeof task.id === 'string').map((task) => [task.id, task]));
  const initial = byId.get(to)?.dependsOn;
  const pending = Array.isArray(initial) ? [...initial] : [];
  const seen = new Set();
  while (pending.length) {
    const id = pending.pop();
    if (id === from) return true;
    if (seen.has(id)) continue;
    seen.add(id);
    const dependencies = byId.get(id)?.dependsOn;
    if (Array.isArray(dependencies)) pending.push(...dependencies);
  }
  return false;
}

function appendError(errors, path, reason) {
  errors.push(`${path}: ${reason}`);
}

function validatePlanObject(plan, errors) {
  if (!isRecord(plan)) {
    appendError(errors, 'plan', 'must be an object');
    return;
  }
  if (!exactKeys(plan, PLAN_KEYS)) appendError(errors, 'plan', 'has missing or unknown fields');
  if (plan.schemaVersion !== 1) appendError(errors, 'schemaVersion', 'must be 1');
  if (!isSafeIdentity(plan.id)) appendError(errors, 'id', 'must be a safe non-empty identity');
  if (!isSafeInt(plan.revision, 1)) appendError(errors, 'revision', 'must be a positive safe integer');
  if (!cleanSha(plan.baselineSha)) appendError(errors, 'baselineSha', 'must be a full Git SHA');

  if (!exactKeys(plan.authorization, ['accepted', 'dispatchAuthorized', 'scope', 'decisionId'])) {
    appendError(errors, 'authorization', 'has missing or unknown fields');
  } else {
    if (typeof plan.authorization.accepted !== 'boolean') appendError(errors, 'authorization.accepted', 'must be boolean');
    if (typeof plan.authorization.dispatchAuthorized !== 'boolean') appendError(errors, 'authorization.dispatchAuthorized', 'must be boolean');
    if (!isString(plan.authorization.scope)) appendError(errors, 'authorization.scope', 'must be a non-empty string');
    if (!isString(plan.authorization.decisionId)) appendError(errors, 'authorization.decisionId', 'must be a non-empty string');
  }

  const roles = plan.roles;
  const roleNames = new Set();
  if (!isRecord(roles) || Object.keys(roles).length === 0) {
    appendError(errors, 'roles', 'must be a non-empty object');
  } else {
    for (const [role, policy] of Object.entries(roles)) {
      roleNames.add(role);
      if (!ROLE_KEYS.includes(role)) appendError(errors, `roles.${role}`, 'is not a supported role');
      if (!exactKeys(policy, ['model', 'effort'])) {
        appendError(errors, `roles.${role}`, 'has missing or unknown fields');
        continue;
      }
      if (LUNA_ROLES.has(role)) {
        if (policy.model !== 'gpt-6-luna' || policy.effort !== 'max') {
          appendError(errors, `roles.${role}`, 'must pin gpt-6-luna at max effort');
        }
      } else if (SOL_ROLES.has(role)) {
        if (policy.model !== 'gpt-6.1-sol') appendError(errors, `roles.${role}.model`, 'must pin gpt-6.1-sol');
        if (!EFFORTS.has(policy.effort)) appendError(errors, `roles.${role}.effort`, 'must pin a concrete supported effort');
      }
    }
  }

  const decisions = Array.isArray(plan.decisions) ? plan.decisions : [];
  if (!Array.isArray(plan.decisions)) appendError(errors, 'decisions', 'must be an array');
  const decisionIds = new Set();
  for (let i = 0; i < decisions.length; i += 1) {
    const decision = decisions[i];
    const p = `decisions[${i}]`;
    if (!exactKeys(decision, ['id', 'resolved', 'decisionId'])) {
      appendError(errors, p, 'has missing or unknown fields');
      continue;
    }
    if (!isSafeIdentity(decision.id)) appendError(errors, `${p}.id`, 'must be a safe non-empty identity');
    else if (decisionIds.has(decision.id)) appendError(errors, `${p}.id`, 'must be unique');
    else decisionIds.add(decision.id);
    if (typeof decision.resolved !== 'boolean') appendError(errors, `${p}.resolved`, 'must be boolean');
    if (!isString(decision.decisionId)) appendError(errors, `${p}.decisionId`, 'must be a non-empty string');
  }

  const tasks = Array.isArray(plan.tasks) ? plan.tasks : [];
  if (!Array.isArray(plan.tasks) || tasks.length === 0) appendError(errors, 'tasks', 'must be a non-empty array');
  const taskIds = new Set();
  const taskById = new Map();
  for (let i = 0; i < tasks.length; i += 1) {
    const task = tasks[i];
    const p = `tasks[${i}]`;
    const taskFields = ['id', 'role', 'dependsOn', 'requiredDecisions', 'writePaths', 'requiredAc', 'timeoutMs', 'leaseMs', 'maxCommandRuns', 'kind'];
    if (!exactKeys(task, taskFields)) {
      appendError(errors, p, 'has missing or unknown fields');
      continue;
    }
    if (!isSafeIdentity(task.id)) appendError(errors, `${p}.id`, 'must be a safe non-empty identity');
    else if (taskIds.has(task.id)) appendError(errors, `${p}.id`, 'must be unique');
    else {
      taskIds.add(task.id);
      taskById.set(task.id, task);
    }
    if (!ROLE_KEYS.includes(task.role)) appendError(errors, `${p}.role`, 'must use a supported role');
    else if (!roleNames.has(task.role)) appendError(errors, `${p}.role`, 'has no pinned role policy');
    for (const [field, values] of [['dependsOn', task.dependsOn], ['requiredDecisions', task.requiredDecisions], ['writePaths', task.writePaths], ['requiredAc', task.requiredAc]]) {
      if (!Array.isArray(values) || values.some((item) => typeof item !== 'string')) appendError(errors, `${p}.${field}`, 'must be an array of strings');
      else if (new Set(values).size !== values.length) appendError(errors, `${p}.${field}`, 'must not contain duplicates');
    }
    if (!['normal', 'characterization'].includes(task.kind)) appendError(errors, `${p}.kind`, 'must be normal or characterization');
    if (!isSafeInt(task.timeoutMs, 1)) appendError(errors, `${p}.timeoutMs`, 'must be a positive safe integer');
    if (!isSafeInt(task.leaseMs, 1)) appendError(errors, `${p}.leaseMs`, 'must be a positive safe integer');
    if (!isSafeInt(task.maxCommandRuns, 1)) appendError(errors, `${p}.maxCommandRuns`, 'must be a positive safe integer');
    if (Array.isArray(task.dependsOn)) {
      for (const dep of task.dependsOn) {
        if (dep === task.id) appendError(errors, `${p}.dependsOn`, 'cannot depend on itself');
      }
    }
    if (Array.isArray(task.requiredDecisions)) {
      for (const id of task.requiredDecisions) if (!decisionIds.has(id) && decisions.some((d) => d?.id === id) === false) appendError(errors, `${p}.requiredDecisions`, `references unknown decision ${id}`);
    }
    if (Array.isArray(task.writePaths)) {
      const normalized = [];
      for (const value of task.writePaths) {
        try { normalized.push(normalizePath(value)); }
        catch (error) { appendError(errors, `${p}.writePaths`, error.message); }
      }
      if (new Set(normalized).size !== normalized.length) appendError(errors, `${p}.writePaths`, 'contains duplicate normalized paths');
    }
    if (Array.isArray(task.requiredAc) && task.requiredAc.length === 0) appendError(errors, `${p}.requiredAc`, 'must name at least one required acceptance case');
  }
  for (const task of tasks) {
    if (!isRecord(task) || !Array.isArray(task.dependsOn)) continue;
    for (const dep of task.dependsOn) if (!taskIds.has(dep)) appendError(errors, `tasks.${task.id}.dependsOn`, `references unknown task ${dep}`);
  }
  if (tasks.length && taskById.size === tasks.length) {
    const order = topoOrder(tasks);
    if (order.length !== tasks.length) appendError(errors, 'tasks', 'dependency graph contains a cycle');
  }

  const graph = plan.graph;
  if (!exactKeys(graph, ['nodes', 'edges'])) {
    appendError(errors, 'graph', 'has missing or unknown fields');
  } else {
    if (!Array.isArray(graph.nodes) || graph.nodes.some((id) => typeof id !== 'string')) appendError(errors, 'graph.nodes', 'must be an array of task IDs');
    else {
      if (new Set(graph.nodes).size !== graph.nodes.length) appendError(errors, 'graph.nodes', 'must not contain duplicate IDs');
      if (graph.nodes.length !== taskIds.size || graph.nodes.some((id) => !taskIds.has(id)) || [...taskIds].some((id) => !graph.nodes.includes(id))) {
        appendError(errors, 'graph.nodes', 'must equal the task ID set exactly');
      }
    }
    if (!Array.isArray(graph.edges)) appendError(errors, 'graph.edges', 'must be an array of [predecessor, successor] pairs');
    else {
      const actual = [];
      for (let i = 0; i < graph.edges.length; i += 1) {
        const edge = graph.edges[i];
        if (!Array.isArray(edge) || edge.length !== 2 || edge.some((id) => typeof id !== 'string')) {
          appendError(errors, `graph.edges[${i}]`, 'must be a [predecessor, successor] pair');
          continue;
        }
        if (!taskIds.has(edge[0]) || !taskIds.has(edge[1])) appendError(errors, `graph.edges[${i}]`, 'references an unknown task');
        actual.push(`${edge[0]}\u0000${edge[1]}`);
      }
      if (new Set(actual).size !== actual.length) appendError(errors, 'graph.edges', 'must not contain duplicate edges');
      const expected = [];
      for (const task of tasks) if (isRecord(task) && Array.isArray(task.dependsOn)) for (const dep of task.dependsOn) expected.push(`${dep}\u0000${task.id}`);
      if (actual.length !== expected.length || expected.some((edge) => !actual.includes(edge))) appendError(errors, 'graph.edges', 'must exactly equal task dependency edges');
    }
  }

  const acceptance = Array.isArray(plan.acceptance) ? plan.acceptance : [];
  if (!Array.isArray(plan.acceptance) || acceptance.length === 0) appendError(errors, 'acceptance', 'must be a non-empty array');
  const acIds = new Set();
  const acById = new Map();
  const requiredAcByTask = new Map();
  for (let i = 0; i < acceptance.length; i += 1) {
    const ac = acceptance[i];
    const p = `acceptance[${i}]`;
    if (!exactKeys(ac, ['id', 'requirementId', 'taskId', 'caseId', 'expected', 'required', 'commandId'], ['closesAcId'])) {
      appendError(errors, p, 'has missing or unknown fields');
      continue;
    }
    if (!isSafeIdentity(ac.id)) appendError(errors, `${p}.id`, 'must be a safe non-empty identity');
    else if (acIds.has(ac.id)) appendError(errors, `${p}.id`, 'must be unique');
    else { acIds.add(ac.id); acById.set(ac.id, ac); }
    for (const field of ['requirementId', 'taskId', 'caseId', 'commandId']) if (!isSafeIdentity(ac[field])) appendError(errors, `${p}.${field}`, 'must be a safe non-empty identity');
    if (typeof ac.required !== 'boolean') appendError(errors, `${p}.required`, 'must be boolean');
    if (!EXPECTED_OUTCOMES.has(ac.expected)) appendError(errors, `${p}.expected`, 'is not a supported expected outcome');
    if (ac.closesAcId !== undefined && !isSafeIdentity(ac.closesAcId)) appendError(errors, `${p}.closesAcId`, 'must be a safe non-empty acceptance ID');
    if (!taskIds.has(ac.taskId)) appendError(errors, `${p}.taskId`, 'references an unknown task');
    if (ac.required === true) {
      const list = requiredAcByTask.get(ac.taskId) ?? [];
      list.push(ac.id);
      requiredAcByTask.set(ac.taskId, list);
    }
  }

  const commands = plan.commands;
  if (!isRecord(commands) || Object.keys(commands).length === 0) appendError(errors, 'commands', 'must be a non-empty object');
  else {
    for (const [id, command] of Object.entries(commands)) {
      if (!isSafeIdentity(id)) appendError(errors, 'commands', 'command IDs must be safe non-empty identities');
      if (!exactKeys(command, ['executable', 'args', 'cwd', 'timeoutMs'])) {
        appendError(errors, `commands.${id}`, 'has missing or unknown fields');
        continue;
      }
      if (!isString(command.executable)) appendError(errors, `commands.${id}.executable`, 'must be a non-empty executable name');
      if (!Array.isArray(command.args) || command.args.some((arg) => typeof arg !== 'string')) appendError(errors, `commands.${id}.args`, 'must be an array of strings');
      if (!isSafeInt(command.timeoutMs, 1)) appendError(errors, `commands.${id}.timeoutMs`, 'must be a positive safe integer');
      if (typeof command.cwd !== 'string' || (command.cwd !== '.' && (() => { try { normalizePath(command.cwd); return false; } catch { return true; } })())) {
        appendError(errors, `commands.${id}.cwd`, 'must be . or a safe relative path');
      }
    }
  }
  for (const ac of acceptance) {
    if (isRecord(ac) && isRecord(commands) && !Object.hasOwn(commands, ac.commandId)) appendError(errors, `acceptance.${ac.id}.commandId`, 'references an unknown allowlisted command');
  }
  for (const ac of acceptance) {
    if (!isRecord(ac) || ac.closesAcId === undefined) continue;
    const target = acById.get(ac.closesAcId);
    if (!ac.required || ac.expected !== 'pass' || !target || target.expected !== 'expected-red' || !target.required) {
      appendError(errors, `acceptance.${ac.id}.closesAcId`, 'must link a required pass case to a required expected-red case');
      continue;
    }
    if (ac.requirementId !== target.requirementId) appendError(errors, `acceptance.${ac.id}.closesAcId`, 'must refer to the same requirement');
    if (!dependencyReachable(tasks, target.taskId, ac.taskId)) appendError(errors, `acceptance.${ac.id}.closesAcId`, 'closure task must depend on the characterization task');
  }
  for (const ac of acceptance) {
    if (isRecord(ac) && ac.expected === 'expected-red' && !acceptance.some((candidate) => candidate?.closesAcId === ac.id && candidate?.required === true)) {
      appendError(errors, `acceptance.${ac.id}`, 'expected-red case must be explicitly closed by required pass evidence');
    }
    if (isRecord(ac) && ac.expected === 'expected-red' && ac.required !== true) {
      appendError(errors, `acceptance.${ac.id}`, 'expected-red characterization cases must be required');
    }
  }

  for (const task of tasks) {
    if (!isRecord(task) || typeof task.id !== 'string') continue;
    const requiredIds = task.requiredAc;
    if (!Array.isArray(requiredIds)) continue;
    const actual = requiredAcByTask.get(task.id) ?? [];
    if (new Set(requiredIds).size !== requiredIds.length) continue;
    if (requiredIds.length !== actual.length || requiredIds.some((id) => !actual.includes(id))) {
      appendError(errors, `tasks.${task.id}.requiredAc`, 'must equal the task\'s required acceptance IDs');
    }
    for (const id of requiredIds) {
      const ac = acById.get(id);
      if (!ac) appendError(errors, `tasks.${task.id}.requiredAc`, `references unknown acceptance ${id}`);
      else if (ac.taskId !== task.id) appendError(errors, `tasks.${task.id}.requiredAc`, `${id} belongs to another task`);
    }
  }
  for (const ac of acceptance) {
    if (!isRecord(ac) || !isString(ac.taskId)) continue;
    const task = taskById.get(ac.taskId);
    if (task?.kind === 'characterization' && ac.expected !== 'expected-red') appendError(errors, `acceptance.${ac.id}`, 'characterization acceptance must expect expected-red');
    if (task?.kind === 'normal' && ac.expected === 'expected-red') appendError(errors, `acceptance.${ac.id}`, 'expected-red is only valid on characterization tasks');
  }
  for (const task of tasks) {
    if (task?.kind === 'characterization' && !(task.requiredAc ?? []).some((id) => acById.get(id)?.expected === 'expected-red')) {
      appendError(errors, `tasks.${task.id}.requiredAc`, 'characterization task needs a required expected-red case');
    }
  }

  const policy = plan.policy;
  if (!exactKeys(policy, ['maxAttempts', 'infraRetries', 'repeatLimit'])) appendError(errors, 'policy', 'has missing or unknown fields');
  else {
    if (policy.maxAttempts !== 3) appendError(errors, 'policy.maxAttempts', 'must be 3');
    if (policy.infraRetries !== 1) appendError(errors, 'policy.infraRetries', 'must be 1');
    if (policy.repeatLimit !== 2) appendError(errors, 'policy.repeatLimit', 'must be 2');
  }

  if (isRecord(roles)) {
    const requiredGateRoles = ['verify', 'review', 'audit', 'integration'];
    for (const role of [...requiredGateRoles, 'orchestrator']) if (!roleNames.has(role)) appendError(errors, `roles.${role}`, 'is required for the integrated candidate gates and trusted coordinator');
    if (!roleNames.has('final') && !roleNames.has('orchestrator')) appendError(errors, 'roles', 'must pin final or orchestrator for the final gate');
  }

  for (let i = 0; i < tasks.length; i += 1) {
    const left = tasks[i];
    if (!isRecord(left) || !Array.isArray(left.writePaths)) continue;
    for (let j = i + 1; j < tasks.length; j += 1) {
      const right = tasks[j];
      if (!isRecord(right) || !Array.isArray(right.writePaths)) continue;
      const overlap = left.writePaths.some((a) => right.writePaths.some((b) => {
        try { return pathsOverlap(a, b); } catch { return false; }
      }));
      if (overlap && !dependencyReachable(tasks, left.id, right.id) && !dependencyReachable(tasks, right.id, left.id)) {
        appendError(errors, 'tasks', `overlapping scopes for ${left.id} and ${right.id} require dependency ordering`);
      }
    }
  }
}

export function validatePlan(plan) {
  const errors = [];
  validatePlanObject(plan, errors);
  return { ok: errors.length === 0, errors };
}

function validateNow(now) {
  if (!isSafeInt(now)) fail('INVALID_TIME', 'now must be a non-negative epoch-millisecond safe integer');
}

function assertPlan(plan) {
  const result = validatePlan(plan);
  if (!result.ok) fail('INVALID_PLAN', result.errors.join('; '));
}

function planTaskMap(plan) {
  return new Map(plan.tasks.map((task) => [task.id, task]));
}

function rolePolicy(plan, role) {
  return plan.roles[role];
}

function initialTaskStatus(task) {
  return task.dependsOn.length === 0 ? 'ready' : 'proposed';
}

export function createState(plan, { runId, now, coordinatorSessionId } = {}) {
  assertPlan(plan);
  validateNow(now);
  if (!isSafeIdentity(runId)) fail('INVALID_RUN_ID', 'runId must be a safe non-empty identity');
  if (!isSafeIdentity(coordinatorSessionId)) fail('INVALID_COORDINATOR', 'coordinatorSessionId must be a safe non-empty trusted identity');
  const coordinatorPolicy = rolePolicy(plan, 'orchestrator');
  if (!coordinatorPolicy) fail('INVALID_PLAN', 'roles.orchestrator is required for the trusted coordinator session');
  const sessions = {};
  setOwn(sessions, coordinatorSessionId, {
      sessionId: coordinatorSessionId,
      role: 'orchestrator',
      model: coordinatorPolicy.model,
      effort: coordinatorPolicy.effort,
      providerReceiptId: 'trusted-initialization',
      registeredAt: now,
      coordinator: true,
      uses: [],
    });
  const tasks = {};
  for (const task of plan.tasks) setOwn(tasks, task.id, {
    status: initialTaskStatus(task),
    attemptCount: 0,
    codeAttempts: 0,
    infraRetries: 0,
    currentCodeInfraRetries: 0,
    commandRuns: 0,
    deadline: null,
    attempts: [],
    currentAttemptId: null,
    pendingRetryKind: null,
    findingCounts: {},
    lastProgressDigest: null,
    noProgressCount: 0,
    acceptedPacketId: null,
    acceptedAcResults: [],
  });
  const state = {
    schemaVersion: 1,
    runId,
    planDigest: digest(plan),
    version: 0,
    status: 'active',
    sessions,
    tasks,
    locks: {},
    candidate: null,
    receipts: { packets: {}, integration: null, gates: {}, gateReports: {} },
    seenEvents: {},
    dispatchRequests: {},
    lastNow: now,
    commandRuns: {},
  };
  return deepFreeze(state);
}

function assertState(plan, state) {
  if (!exactKeys(state, STATE_KEYS)) fail('INVALID_STATE', 'state has missing or unknown fields');
  if (state.schemaVersion !== 1 || !isString(state.runId) || state.planDigest !== digest(plan)) fail('INVALID_STATE', 'state schema, run ID, or plan digest is invalid');
  if (!isSafeInt(state.version) || !isSafeInt(state.lastNow)) fail('INVALID_STATE', 'state version or lastNow is invalid');
  if (!['active', 'blocked', 'aborted', 'done'].includes(state.status)) fail('INVALID_STATE', 'state status is invalid');
  if (!isRecord(state.sessions) || !isRecord(state.tasks) || !isRecord(state.locks) || !isRecord(state.receipts) || !isRecord(state.seenEvents) || !isRecord(state.dispatchRequests) || !isRecord(state.commandRuns)) {
    fail('INVALID_STATE', 'state maps are invalid');
  }
  const expectedTasks = plan.tasks.map((task) => task.id).sort();
  const actualTasks = Object.keys(state.tasks).sort();
  if (stableStringify(expectedTasks) !== stableStringify(actualTasks)) fail('INVALID_STATE', 'state task IDs do not match the immutable plan');
  if (!Object.hasOwn(state.receipts, 'packets') || !Object.hasOwn(state.receipts, 'integration') || !isRecord(state.receipts.gates) || !isRecord(state.receipts.gateReports)) fail('INVALID_STATE', 'state receipts are invalid');
}

function validateEventEnvelope(event) {
  requireKeys(event, ['id', 'type', 'expectedVersion', 'actorSessionId', 'payload'], [], 'INVALID_EVENT', 'event');
  if (!isSafeIdentity(event.id) || !EVENT_TYPES.has(event.type) || !isSafeInt(event.expectedVersion) || !isSafeIdentity(event.actorSessionId) || !isRecord(event.payload)) {
    fail('INVALID_EVENT', 'event envelope contains an invalid identity, type, version, actor, or payload');
  }
}

function actorSession(state, actorSessionId) {
  if (!Object.hasOwn(state.sessions, actorSessionId)) fail('UNKNOWN_SESSION', 'actor session is not registered');
  const session = state.sessions[actorSessionId];
  if (!session) fail('UNKNOWN_SESSION', 'actor session is not registered');
  return session;
}

function requireCoordinator(state, actorSessionId) {
  const session = actorSession(state, actorSessionId);
  if (session.coordinator !== true) fail('FORBIDDEN', 'event requires the trusted coordinator session');
  return session;
}

function requireRole(state, actorSessionId, role) {
  const session = actorSession(state, actorSessionId);
  if (session.role !== role) fail('FORBIDDEN', `event requires a registered ${role} session`);
  return session;
}

function expectedGateRole(plan, stage) {
  if (stage === 'final') return Object.hasOwn(plan.roles, 'final') ? 'final' : 'orchestrator';
  return stage;
}

function authorizeGateSession(plan, state, stage, actorSessionId) {
  const usedGateSessions = Object.values(state.receipts.gates).map((receipt) => receipt.sessionId);
  if (usedGateSessions.includes(actorSessionId)) fail('SESSION_REUSED', 'Verify, Review, Audit, and Final Gate require distinct sessions');
  const role = expectedGateRole(plan, stage);
  const session = requireRole(state, actorSessionId, role);
  const authorSessions = plan.tasks.flatMap((task) => state.tasks[task.id].attempts.map((attempt) => attempt.sessionId).filter(Boolean));
  if (authorSessions.includes(actorSessionId)) fail('NON_INDEPENDENT_GATE', `${stage} gate must be independent from task authors`);
  if (stage !== 'verify' && actorSessionId === state.receipts.integration?.integratorSessionId) {
    fail('NON_INDEPENDENT_GATE', 'Review, Audit, and Final Gate must be independent from the integrator');
  }
  if (stage === 'audit' && session.model !== 'gpt-6.1-sol') fail('NON_INDEPENDENT_GATE', 'final Audit requires a Sol session');
  if (stage === 'final' && session.model !== 'gpt-6.1-sol') fail('NON_INDEPENDENT_GATE', 'Final Gate requires a Sol session');
  return { session, role };
}

function requireTask(plan, state, taskId) {
  const taskDef = planTaskMap(plan).get(taskId);
  const taskState = Object.hasOwn(state.tasks, taskId) ? state.tasks[taskId] : null;
  if (!taskDef || !taskState) fail('UNKNOWN_TASK', `task ${taskId} is not in the plan`);
  return { taskDef, taskState };
}

function findAttempt(taskState, attemptId) {
  return taskState.attempts.find((attempt) => attempt.attemptId === attemptId) ?? null;
}

function currentAttempt(taskState, attemptId) {
  const attempt = findAttempt(taskState, attemptId);
  if (!attempt || taskState.currentAttemptId !== attemptId) fail('STALE_ATTEMPT', 'attempt is not the current task attempt');
  return attempt;
}

function assertLiveAttempt(attempt, now) {
  if (!['running', 'reserved', 'handoff_pending', 'verifying'].includes(attempt.status) || attempt.fenced) fail('STALE_ATTEMPT', 'attempt is fenced or no longer live');
  if (now >= attempt.deadline || now >= attempt.leaseUntil) fail('LEASE_EXPIRED', 'attempt lease or absolute deadline has expired');
}

function addFindingCounts(taskState, findings) {
  let repeated = false;
  let critical = false;
  for (const finding of findings) {
    const fingerprint = normalizeFingerprint(finding.fingerprint);
    const count = (Object.hasOwn(taskState.findingCounts, fingerprint) ? taskState.findingCounts[fingerprint] : 0) + 1;
    setOwn(taskState.findingCounts, fingerprint, count);
    if (count >= 2) repeated = true;
    if (finding.critical) critical = true;
  }
  return { repeated, critical };
}

function validateFindingList(findings, label = 'findings') {
  if (!Array.isArray(findings)) fail('INVALID_FINDINGS', `${label} must be an array`);
  const fingerprints = new Set();
  for (const finding of findings) {
    requireKeys(finding, ['fingerprint', 'blocking', 'critical'], [], 'INVALID_FINDINGS', label);
    if (!isSafeIdentity(finding.fingerprint) || typeof finding.blocking !== 'boolean' || typeof finding.critical !== 'boolean') {
      fail('INVALID_FINDINGS', `${label} contains an invalid finding`);
    }
    const normalized = normalizeFingerprint(finding.fingerprint);
    if (fingerprints.has(normalized)) fail('INVALID_FINDINGS', `${label} contains a duplicate fingerprint`);
    fingerprints.add(normalized);
  }
}

function blockRun(state, taskState, attempt, reason) {
  state.status = 'blocked';
  if (attempt) quarantineAttempt(state, taskState, attempt, reason);
  if (taskState) taskState.status = 'blocked';
}

function quarantineAttempt(state, taskState, attempt, reason) {
  attempt.status = 'quarantined';
  attempt.fenced = true;
  attempt.quarantineReason = reason;
  attempt.quarantinedAt = state.lastNow;
  if (taskState) taskState.status = 'rework';
  for (const path of attempt.writePaths) {
    setOwn(state.locks, path, {
      taskId: attempt.taskId,
      attemptId: attempt.attemptId,
      fence: attempt.fence,
      quarantined: true,
    });
  }
}

function releaseAttemptLocks(state, attempt) {
  for (const [path, lock] of Object.entries(state.locks)) {
    if (lock.attemptId === attempt.attemptId && lock.fence === attempt.fence) delete state.locks[path];
  }
}

function refreshReadyTasks(plan, state) {
  for (const task of plan.tasks) {
    const current = state.tasks[task.id];
    if (current.status === 'proposed' && task.dependsOn.every((id) => state.tasks[id].status === 'verified')) current.status = 'ready';
  }
}

function validateCandidate(candidate, label) {
  requireKeys(candidate, ['sha', 'tree', 'sourceDigest'], [], 'INVALID_CANDIDATE', label);
  if (!cleanSha(candidate.sha) || !cleanSha(candidate.tree) || !validHex(candidate.sourceDigest, [64])) {
    fail('INVALID_CANDIDATE', `${label} must contain Git commit/tree SHAs and a SHA-256 source digest`);
  }
}

function validateAcResultList(results, plan, allowedTaskIds = null, includeOptional = false, requireComplete = true) {
  if (!Array.isArray(results)) fail('INVALID_AC_RESULTS', 'acResults must be an array');
  const scoped = plan.acceptance.filter((ac) => !allowedTaskIds || allowedTaskIds.has(ac.taskId));
  const required = scoped.filter((ac) => ac.required);
  const allowed = includeOptional ? scoped : required;
  const allowedById = new Map(allowed.map((ac) => [ac.id, ac]));
  const found = new Set();
  for (const result of results) {
    requireKeys(result, ['acId', 'caseId', 'outcome', 'testCount', 'evidenceDigest'], [], 'INVALID_AC_RESULTS', 'acResults entry');
    const ac = allowedById.get(result.acId);
    if (!ac || result.caseId !== ac.caseId) fail('INVALID_AC_RESULTS', 'acResults contains an unknown or mismatched AC/case');
    if (found.has(result.acId)) fail('INVALID_AC_RESULTS', `duplicate result for ${result.acId}`);
    if (result.outcome === 'skip') {
      if (ac.required && requireComplete) fail('REQUIRED_AC_SKIPPED', `required acceptance ${result.acId} cannot be skipped`);
      if (result.testCount !== 0) fail('INVALID_AC_RESULTS', `${result.acId} skipped testCount must be zero`);
    } else {
      if (requireComplete && result.outcome !== ac.expected) fail('INVALID_AC_RESULTS', `${result.acId} outcome does not match its immutable expected outcome`);
      if (!isSafeInt(result.testCount, 1)) fail('INVALID_AC_RESULTS', `${result.acId} needs positive test evidence`);
    }
    if (!validHex(result.evidenceDigest, [64])) fail('INVALID_AC_RESULTS', `${result.acId} needs a SHA-256 evidence digest`);
    found.add(result.acId);
  }
  if (requireComplete) for (const ac of required) if (!found.has(ac.id)) fail('INCOMPLETE_AC', `required acceptance ${ac.id} is missing`);
  return results.map((result) => deepClone(result));
}

function validatePacket(packet, plan, state, taskDef, attempt) {
  const keys = ['packetId', 'runId', 'planDigest', 'taskId', 'attemptId', 'fence', 'senderSessionId', 'receiverSessionId', 'candidate', 'changedPaths', 'artifacts', 'commands', 'acResults', 'findings'];
  requireKeys(packet, keys, [], 'INVALID_PACKET', 'packet');
  for (const field of ['packetId', 'runId', 'planDigest', 'taskId', 'attemptId', 'senderSessionId', 'receiverSessionId']) {
    if (!isSafeIdentity(packet[field])) fail('INVALID_PACKET', `packet.${field} must be a safe non-empty identity`);
  }
  if (packet.runId !== state.runId || packet.planDigest !== state.planDigest || packet.taskId !== taskDef.id || packet.attemptId !== attempt.attemptId || packet.fence !== attempt.fence) {
    fail('STALE_PACKET', 'packet is not bound to this run, plan, task, attempt, and fence');
  }
  if (packet.senderSessionId !== attempt.sessionId) fail('INVALID_PACKET', 'packet sender does not match its bound worker session');
  validateCandidate(packet.candidate, 'packet.candidate');
  if (!Array.isArray(packet.changedPaths)) fail('INVALID_PACKET', 'packet.changedPaths must be an array');
  const changed = [];
  for (const path of packet.changedPaths) {
    let normalized;
    try { normalized = normalizePath(path); } catch (error) { fail('INVALID_PACKET', `invalid changed path: ${error.message}`); }
    if (!taskDef.writePaths.some((scope) => pathsOverlap(scope, normalized) && (normalized === normalizePath(scope) || normalized.startsWith(`${normalizePath(scope)}/`)))) {
      fail('OUT_OF_SCOPE', `changed path ${path} is outside the task write scopes`);
    }
    changed.push(normalized);
  }
  if (new Set(changed).size !== changed.length) fail('INVALID_PACKET', 'changedPaths contains duplicate normalized paths');

  if (!Array.isArray(packet.artifacts)) fail('INVALID_PACKET', 'packet.artifacts must be an array');
  for (const artifact of packet.artifacts) {
    requireKeys(artifact, ['path', 'sha256', 'size'], [], 'INVALID_PACKET', 'artifact');
    if (!isString(artifact.path) || !validHex(artifact.sha256, [64]) || !isSafeInt(artifact.size)) fail('INVALID_PACKET', 'artifact metadata is invalid');
  }

  if (!Array.isArray(packet.commands)) fail('INVALID_PACKET', 'packet.commands must be an array');
  const packetCommandIds = new Set();
  const taskRunIds = Object.keys(state.commandRuns).filter((id) => state.commandRuns[id].taskId === taskDef.id && state.commandRuns[id].attemptId === attempt.attemptId).sort();
  const completedIds = taskRunIds.filter((id) => state.commandRuns[id].status === 'completed').sort();
  if (taskRunIds.length !== completedIds.length) fail('INCOMPLETE_COMMANDS', 'all reserved commands must complete before handoff');
  for (const receipt of packet.commands) {
    requireKeys(receipt, ['commandRunId', 'commandId', 'cwd', 'exitCode', 'signal', 'timedOut', 'durationMs', 'testCount', 'caseResults', 'environmentDigest', 'stdoutDigest', 'stderrDigest'], [], 'INVALID_COMMAND_RECEIPT', 'packet command');
    if (!isSafeIdentity(receipt.commandRunId)) fail('INVALID_COMMAND_RECEIPT', 'packet commandRunId is invalid');
    const stored = Object.hasOwn(state.commandRuns, receipt.commandRunId) ? state.commandRuns[receipt.commandRunId] : null;
    if (!stored || stored.status !== 'completed' || stored.taskId !== taskDef.id || stored.attemptId !== attempt.attemptId || stored.fence !== attempt.fence) {
      fail('INVALID_COMMAND_RECEIPT', 'packet references an unknown, incomplete, or stale command reservation');
    }
    if (packetCommandIds.has(receipt.commandRunId)) fail('INVALID_COMMAND_RECEIPT', 'packet repeats a command reservation');
    if (stableStringify(receipt) !== stableStringify(stored.receipt)) fail('INVALID_COMMAND_RECEIPT', 'packet command differs from the captured runner receipt');
    packetCommandIds.add(receipt.commandRunId);
  }
  if (stableStringify([...packetCommandIds].sort()) !== stableStringify(completedIds)) fail('INCOMPLETE_COMMANDS', 'packet must bind every completed command reservation for this attempt');

  const acResults = validateAcResultList(packet.acResults, plan, new Set([taskDef.id]), true);
  validateFindingList(packet.findings, 'packet.findings');
  const taskAc = plan.acceptance.filter((ac) => ac.taskId === taskDef.id);
  for (const receipt of packet.commands) {
    const rows = taskAc.filter((ac) => ac.commandId === receipt.commandId);
    const expectedRed = rows.some((ac) => ac.expected === 'expected-red');
    const success = receipt.exitCode === 0 && receipt.signal === null && receipt.timedOut === false;
    const capturedRed = receipt.caseResults.some((result) => rows.some((ac) => ac.id === result.acId && ac.expected === 'expected-red' && result.outcome === 'expected-red'));
    if (expectedRed && (!capturedRed || !Number.isSafeInteger(receipt.exitCode) || receipt.exitCode <= 0 || receipt.signal !== null || receipt.timedOut)) {
      fail('EXPECTED_RED_MISSING', `command ${receipt.commandId} did not capture a normal nonzero expected-red result`);
    }
    if (!success && !expectedRed) fail('COMMAND_FAILED', `command ${receipt.commandId} failed without an expected-red AC`);
    if (success && rows.some((ac) => ac.expected === 'expected-red')) fail('EXPECTED_RED_MISSING', `command ${receipt.commandId} did not produce the expected failure`);
    for (const row of receipt.caseResults) {
      const ac = taskAc.find((candidate) => candidate.id === row.acId);
      const optionalSkip = ac && !ac.required && row.outcome === 'skip' && row.testCount === 0;
      if (!ac || ac.commandId !== receipt.commandId || ac.caseId !== row.caseId || (!optionalSkip && ac.expected !== row.outcome)) {
        fail('INVALID_COMMAND_RECEIPT', `command ${receipt.commandId} contains a case result outside the immutable AC contract`);
      }
    }
  }
  for (const result of acResults) {
    const ac = taskAc.find((candidate) => candidate.id === result.acId);
    const matches = packet.commands.flatMap((receipt) => receipt.commandId === ac.commandId
      ? receipt.caseResults.filter((row) => row.acId === ac.id && row.caseId === ac.caseId).map((row) => ({ receipt, row }))
      : []);
    if (matches.length !== 1) fail('INCOMPLETE_AC', `${ac.id} must have exactly one parsed command case result`);
    const { receipt, row } = matches[0];
    if (result.outcome !== row.outcome || result.testCount !== row.testCount || result.evidenceDigest !== receipt.stdoutDigest) {
      fail('AC_RECEIPT_MISMATCH', `${ac.id} differs from its captured allowlisted command output`);
    }
  }
  for (const ac of taskAc.filter((item) => item.required)) {
    if (!acResults.some((result) => result.acId === ac.id)) fail('INCOMPLETE_AC', `${ac.id} has no accepted result`);
  }
  return { packet: deepClone(packet), acResults };
}

function candidateProgressDigest(packet) {
  return packet.candidate.sourceDigest.toLowerCase();
}

function recordFailedProgress(taskState, packet) {
  const progress = candidateProgressDigest(packet);
  if (taskState.lastProgressDigest === progress) taskState.noProgressCount += 1;
  else taskState.noProgressCount = 1;
  taskState.lastProgressDigest = progress;
  return taskState.noProgressCount >= 2;
}

function validatePlanAuthorization(plan) {
  if (plan.authorization.accepted !== true || plan.authorization.dispatchAuthorized !== true) {
    fail('UNAUTHORIZED_PLAN', 'plan is not accepted and explicitly dispatch-authorized');
  }
}

function getAttemptPathLocks(state) {
  return Object.entries(state.locks).map(([path, lock]) => ({ path, lock }));
}

function reserveDispatch(plan, state, taskId, now) {
  validatePlanAuthorization(plan);
  const { taskDef, taskState } = requireTask(plan, state, taskId);
  if (!['ready', 'rework'].includes(taskState.status)) fail('TASK_NOT_READY', `task ${taskId} is not ready for dispatch`);
  if (taskDef.requiredDecisions.some((id) => !plan.decisions.find((decision) => decision.id === id)?.resolved)) {
    fail('UNRESOLVED_DECISION', `task ${taskId} requires an unresolved decision`);
  }
  if (!taskDef.dependsOn.every((id) => state.tasks[id].status === 'verified')) fail('PREDECESSOR_UNVERIFIED', `task ${taskId} has an unverified predecessor`);
  if (taskState.attempts.some((attempt) => ['reserved', 'running', 'handoff_pending', 'verifying', 'quarantined'].includes(attempt.status))) {
    fail('ATTEMPT_ALREADY_ACTIVE', `task ${taskId} already has a live or quarantined attempt`);
  }
  if (taskState.codeAttempts === 0) {
    taskState.codeAttempts = 1;
  } else if (taskState.pendingRetryKind === 'infra') {
    if (taskState.currentCodeInfraRetries >= plan.policy.infraRetries) fail('RETRY_BUDGET_EXHAUSTED', `task ${taskId} exhausted infrastructure retries for this code attempt`);
    taskState.infraRetries += 1;
    taskState.currentCodeInfraRetries += 1;
  } else {
    if (taskState.codeAttempts >= plan.policy.maxAttempts) fail('RETRY_BUDGET_EXHAUSTED', `task ${taskId} exhausted code attempts`);
    taskState.codeAttempts += 1;
    taskState.currentCodeInfraRetries = 0;
  }
  taskState.pendingRetryKind = null;
  if (taskState.deadline === null) taskState.deadline = now + taskDef.timeoutMs;
  if (!Number.isSafeInteger(taskState.deadline) || now >= taskState.deadline) {
    blockRun(state, taskState, null, 'task-deadline-exhausted');
    fail('TASK_DEADLINE_EXHAUSTED', `task ${taskId} absolute deadline has expired`);
  }
  const scopes = taskDef.writePaths.map((path) => normalizePath(path));
  for (const scope of scopes) {
    for (const { path, lock } of getAttemptPathLocks(state)) {
      if (pathsOverlap(scope, path)) fail(lock.quarantined ? 'SCOPE_QUARANTINED' : 'SCOPE_LOCKED', `task ${taskId} overlaps locked scope ${path}`);
    }
  }
  const attemptNumber = taskState.attemptCount + 1;
  taskState.attemptCount = attemptNumber;
  const attemptId = `${taskId}-${attemptNumber}`;
  if (taskState.attempts.some((attempt) => attempt.attemptId === attemptId)) fail('INVALID_STATE', 'attempt ID collision');
  const previousFence = Object.values(state.dispatchRequests).reduce((max, request) => Math.max(max, request.fence), 0);
  const fence = previousFence + 1;
  const leaseUntil = Math.min(now + taskDef.leaseMs, taskState.deadline);
  const role = taskDef.role;
  const policy = rolePolicy(plan, role);
  const attempt = {
    runId: state.runId,
    taskId,
    attemptId,
    codeAttempt: taskState.codeAttempts,
    infraRetry: taskState.currentCodeInfraRetries,
    fence,
    role,
    model: policy.model,
    effort: policy.effort,
    status: 'reserved',
    sessionId: null,
    leaseUntil,
    deadline: taskState.deadline,
    dispatchAt: now,
    boundAt: null,
    writePaths: scopes,
    packetId: null,
    acked: false,
    fenced: false,
    quarantineReason: null,
    reservedCommandIds: [],
  };
  taskState.attempts.push(attempt);
  taskState.currentAttemptId = attemptId;
  taskState.status = 'running';
  const dispatchRequest = {
    runId: state.runId,
    planDigest: state.planDigest,
    taskId,
    attemptId,
    fence,
    role,
    model: policy.model,
    effort: policy.effort,
    writePaths: [...scopes],
    deadline: taskState.deadline,
    leaseUntil,
  };
  setOwn(state.dispatchRequests, attemptId, dispatchRequest);
  for (const path of scopes) setOwn(state.locks, path, { taskId, attemptId, fence, quarantined: false });
}

function markTaskFailure(plan, state, taskState, attempt, failureKind, findings = [], now = state.lastNow) {
  const counts = addFindingCounts(taskState, findings);
  const critical = counts.critical;
  quarantineAttempt(state, taskState, attempt, failureKind);
  if (critical || counts.repeated) {
    blockRun(state, taskState, attempt, critical ? 'critical-finding' : 'repeated-finding');
    return;
  }
  if (failureKind === 'timeout' || now >= attempt.deadline) {
    blockRun(state, taskState, attempt, 'task-deadline-exhausted');
    return;
  }
  if (failureKind === 'infra' || failureKind === 'process-loss' || failureKind === 'lease-expired') {
    if (taskState.currentCodeInfraRetries >= plan.policy.infraRetries) {
      blockRun(state, taskState, attempt, 'infrastructure-retry-exhausted');
      return;
    }
    taskState.pendingRetryKind = 'infra';
  } else {
    if (taskState.codeAttempts >= plan.policy.maxAttempts) {
      blockRun(state, taskState, attempt, 'code-attempts-exhausted');
      return;
    }
    taskState.pendingRetryKind = 'code';
  }
}

function validateCommandReceipt(receipt, commandDef, commandId) {
  requireKeys(receipt, ['commandId', 'cwd', 'exitCode', 'signal', 'timedOut', 'durationMs', 'testCount', 'caseResults', 'environmentDigest', 'stdoutDigest', 'stderrDigest'], [], 'INVALID_COMMAND_RECEIPT', 'receipt');
  if (receipt.commandId !== commandId || receipt.cwd !== commandDef.cwd) fail('INVALID_COMMAND_RECEIPT', 'command receipt does not match its allowlisted command and cwd');
  if (!(receipt.exitCode === null || Number.isSafeInteger(receipt.exitCode))) fail('INVALID_COMMAND_RECEIPT', 'exitCode must be an integer or null');
  if (!(receipt.signal === null || isString(receipt.signal))) fail('INVALID_COMMAND_RECEIPT', 'signal must be null or a non-empty string');
  if (typeof receipt.timedOut !== 'boolean' || !isSafeInt(receipt.durationMs) || !isSafeInt(receipt.testCount)) fail('INVALID_COMMAND_RECEIPT', 'command timing or test count is invalid');
  if (!Array.isArray(receipt.caseResults)) fail('INVALID_COMMAND_RECEIPT', 'caseResults must be an array');
  const caseIds = new Set();
  let caseTestTotal = 0;
  for (const result of receipt.caseResults) {
    requireKeys(result, ['acId', 'caseId', 'outcome', 'testCount'], [], 'INVALID_COMMAND_RECEIPT', 'case result');
    const optionalSkip = result.outcome === 'skip' && result.testCount === 0;
    if (!isSafeIdentity(result.acId) || !isSafeIdentity(result.caseId) || (!EXPECTED_OUTCOMES.has(result.outcome) && !optionalSkip) || (!optionalSkip && !isSafeInt(result.testCount, 1))) {
      fail('INVALID_COMMAND_RECEIPT', 'case result has invalid IDs, outcome, or testCount');
    }
    if (caseIds.has(result.acId)) fail('INVALID_COMMAND_RECEIPT', 'caseResults contains a duplicate AC ID');
    caseIds.add(result.acId);
    caseTestTotal += result.testCount;
  }
  if (caseTestTotal !== receipt.testCount) fail('INVALID_COMMAND_RECEIPT', 'aggregate testCount must equal parsed case test counts');
  for (const field of ['environmentDigest', 'stdoutDigest', 'stderrDigest']) if (!validHex(receipt[field], [64])) fail('INVALID_COMMAND_RECEIPT', `${field} must be a SHA-256 digest`);
}

function handleEvent(plan, state, event, now) {
  const p = event.payload;
  switch (event.type) {
    case 'register_session': {
      requireKeys(p, ['sessionId', 'role', 'model', 'effort', 'providerReceiptId'], [], 'INVALID_PAYLOAD', 'register_session payload');
      requireCoordinator(state, event.actorSessionId);
      if (!isSafeIdentity(p.sessionId) || ![p.role, p.model, p.effort, p.providerReceiptId].every(isString)) fail('INVALID_PAYLOAD', 'session identity and provider receipt fields must be non-empty strings');
      if (Object.hasOwn(state.sessions, p.sessionId)) fail('DUPLICATE_SESSION', 'session identity is already registered');
      const policy = rolePolicy(plan, p.role);
      if (!policy || policy.model !== p.model || policy.effort !== p.effort) fail('SESSION_POLICY_MISMATCH', 'session role, model, and effort do not match the immutable plan');
      setOwn(state.sessions, p.sessionId, {
        sessionId: p.sessionId, role: p.role, model: p.model, effort: p.effort,
        providerReceiptId: p.providerReceiptId, registeredAt: now, coordinator: false, uses: [],
      });
      break;
    }
    case 'dispatch': {
      requireKeys(p, ['taskId'], [], 'INVALID_PAYLOAD', 'dispatch payload');
      requireCoordinator(state, event.actorSessionId);
      if (!isString(p.taskId)) fail('INVALID_PAYLOAD', 'taskId must be a non-empty string');
      reserveDispatch(plan, state, p.taskId, now);
      break;
    }
    case 'bind_worker': {
      requireKeys(p, ['taskId', 'attemptId', 'sessionId'], [], 'INVALID_PAYLOAD', 'bind_worker payload');
      requireCoordinator(state, event.actorSessionId);
      const { taskDef, taskState } = requireTask(plan, state, p.taskId);
      const attempt = currentAttempt(taskState, p.attemptId);
      if (attempt.status !== 'reserved' || attempt.sessionId !== null) fail('ATTEMPT_ALREADY_BOUND', 'attempt is not an unbound reservation');
      assertLiveAttempt(attempt, now);
      const session = Object.hasOwn(state.sessions, p.sessionId) ? state.sessions[p.sessionId] : null;
      if (!session || session.role !== taskDef.role || session.model !== attempt.model || session.effort !== attempt.effort || session.coordinator) {
        fail('SESSION_POLICY_MISMATCH', 'worker session does not match the reserved role, model, and effort');
      }
      if (session.uses.length !== 0) fail('SESSION_REUSED', 'provider session is already bound to another actor');
      attempt.sessionId = p.sessionId;
      attempt.status = 'running';
      attempt.boundAt = now;
      session.uses.push({ kind: 'worker', taskId: p.taskId, attemptId: p.attemptId });
      break;
    }
    case 'heartbeat': {
      requireKeys(p, ['taskId', 'attemptId', 'fence'], [], 'INVALID_PAYLOAD', 'heartbeat payload');
      const { taskDef, taskState } = requireTask(plan, state, p.taskId);
      const attempt = currentAttempt(taskState, p.attemptId);
      if (attempt.fence !== p.fence || attempt.sessionId !== event.actorSessionId) fail('STALE_FENCE', 'heartbeat session or fence does not match the current attempt');
      if (attempt.status !== 'running') fail('STALE_ATTEMPT', 'heartbeat requires a running attempt before handoff');
      assertLiveAttempt(attempt, now);
      const nextLease = Math.min(now + taskDef.leaseMs, attempt.deadline);
      if (nextLease <= attempt.leaseUntil) fail('LEASE_NOT_EXTENDED', 'heartbeat would not extend the lease');
      attempt.leaseUntil = nextLease;
      break;
    }
    case 'command_reserved': {
      requireKeys(p, ['taskId', 'attemptId', 'fence', 'commandRunId', 'commandId'], [], 'INVALID_PAYLOAD', 'command_reserved payload');
      requireCoordinator(state, event.actorSessionId);
      const { taskDef, taskState } = requireTask(plan, state, p.taskId);
      const attempt = currentAttempt(taskState, p.attemptId);
      assertLiveAttempt(attempt, now);
      if (attempt.status !== 'running' || attempt.fence !== p.fence || !attempt.sessionId) fail('STALE_ATTEMPT', 'command reservation requires the live bound attempt');
      if (!isSafeIdentity(p.commandRunId) || !isSafeIdentity(p.commandId) || !Object.hasOwn(plan.commands, p.commandId)) fail('INVALID_PAYLOAD', 'command run ID or allowlisted command is invalid');
      if (Object.hasOwn(state.commandRuns, p.commandRunId)) fail('DUPLICATE_COMMAND_RUN', 'commandRunId is already reserved');
      if (taskState.commandRuns >= taskDef.maxCommandRuns) fail('COMMAND_BUDGET_EXHAUSTED', `task ${p.taskId} exhausted its cumulative command budget`);
      taskState.commandRuns += 1;
      attempt.reservedCommandIds.push(p.commandRunId);
      setOwn(state.commandRuns, p.commandRunId, {
        commandRunId: p.commandRunId, taskId: p.taskId, attemptId: p.attemptId,
        fence: p.fence, commandId: p.commandId, reservedAt: now, status: 'reserved', receipt: null,
      });
      break;
    }
    case 'command_completed': {
      requireKeys(p, ['commandRunId', 'receipt'], [], 'INVALID_PAYLOAD', 'command_completed payload');
      requireCoordinator(state, event.actorSessionId);
      if (!isSafeIdentity(p.commandRunId)) fail('INVALID_PAYLOAD', 'commandRunId must be a safe non-empty identity');
      const run = Object.hasOwn(state.commandRuns, p.commandRunId) ? state.commandRuns[p.commandRunId] : null;
      if (!run) fail('UNKNOWN_COMMAND_RUN', 'command completion has no reservation');
      if (run.status !== 'reserved') fail('DUPLICATE_COMMAND_COMPLETION', 'command run already has a captured completion');
      validateCommandReceipt(p.receipt, plan.commands[run.commandId], run.commandId);
      run.status = 'completed';
      run.completedAt = now;
      run.receipt = { commandRunId: p.commandRunId, ...deepClone(p.receipt) };
      break;
    }
    case 'submit_handoff': {
      requireKeys(p, ['packet'], [], 'INVALID_PAYLOAD', 'submit_handoff payload');
      if (!isRecord(p.packet)) fail('INVALID_PACKET', 'packet must be an object');
      const { taskDef, taskState } = requireTask(plan, state, p.packet.taskId);
      const attempt = currentAttempt(taskState, p.packet.attemptId);
      if (attempt.sessionId !== event.actorSessionId || p.packet.fence !== attempt.fence) fail('STALE_FENCE', 'handoff actor or fence does not match current worker attempt');
      assertLiveAttempt(attempt, now);
      if (attempt.status !== 'running') fail('STALE_ATTEMPT', 'handoff requires a live running attempt');
    const { packet } = validatePacket(p.packet, plan, state, taskDef, attempt);
    const receiver = Object.hasOwn(state.sessions, packet.receiverSessionId) ? state.sessions[packet.receiverSessionId] : null;
      if (!receiver || receiver.role !== 'verify' || receiver.coordinator || receiver.sessionId === attempt.sessionId) fail('INVALID_RECEIVER', 'handoff receiver must be a registered independent Verify session');
      if (Object.hasOwn(state.receipts.packets, packet.packetId)) fail('DUPLICATE_PACKET', 'packetId is already in use');
      attempt.packetId = packet.packetId;
      attempt.status = 'handoff_pending';
      taskState.status = 'handoff_pending';
      setOwn(state.receipts.packets, packet.packetId, {
        packet,
        taskId: taskDef.id,
        attemptId: attempt.attemptId,
        fence: attempt.fence,
        senderSessionId: attempt.sessionId,
        receiverSessionId: packet.receiverSessionId,
        acResults: packet.acResults,
        candidateDigest: digest(packet.candidate),
        status: 'submitted',
      });
      break;
    }
    case 'ack_handoff': {
      requireKeys(p, ['taskId', 'packetId'], [], 'INVALID_PAYLOAD', 'ack_handoff payload');
      const { taskState } = requireTask(plan, state, p.taskId);
      const packetReceipt = Object.hasOwn(state.receipts.packets, p.packetId) ? state.receipts.packets[p.packetId] : null;
      if (!packetReceipt || packetReceipt.taskId !== p.taskId) fail('UNKNOWN_PACKET', 'packet is not pending for this task');
      if (event.actorSessionId !== packetReceipt.receiverSessionId) fail('FORBIDDEN', 'only the packet receiver may acknowledge handoff');
      requireRole(state, event.actorSessionId, 'verify');
      if (packetReceipt.status !== 'submitted') fail('PACKET_NOT_PENDING', 'packet is not awaiting acknowledgment');
      const attempt = currentAttempt(taskState, packetReceipt.attemptId);
      if (attempt.fence !== packetReceipt.fence || attempt.status !== 'handoff_pending' || attempt.fenced) fail('STALE_FENCE', 'packet attempt is stale');
      assertLiveAttempt(attempt, now);
      packetReceipt.status = 'acked';
      packetReceipt.ackedAt = now;
      attempt.status = 'verifying';
      attempt.acked = true;
      taskState.status = 'verifying';
      break;
    }
    case 'reject_handoff': {
      requireKeys(p, ['taskId', 'packetId', 'reason'], [], 'INVALID_PAYLOAD', 'reject_handoff payload');
      const { taskState } = requireTask(plan, state, p.taskId);
      const packetReceipt = Object.hasOwn(state.receipts.packets, p.packetId) ? state.receipts.packets[p.packetId] : null;
      if (!packetReceipt || packetReceipt.taskId !== p.taskId) fail('UNKNOWN_PACKET', 'packet is not pending for this task');
      if (event.actorSessionId !== packetReceipt.receiverSessionId) fail('FORBIDDEN', 'only the packet receiver may reject handoff');
      requireRole(state, event.actorSessionId, 'verify');
      if (packetReceipt.status !== 'submitted') fail('PACKET_NOT_PENDING', 'packet is not awaiting acknowledgment');
      if (!isString(p.reason)) fail('INVALID_PAYLOAD', 'reason must be a non-empty string');
      const attempt = currentAttempt(taskState, packetReceipt.attemptId);
      if (attempt.fence !== packetReceipt.fence || attempt.status !== 'handoff_pending' || attempt.fenced) fail('STALE_FENCE', 'packet attempt is stale');
      assertLiveAttempt(attempt, now);
      const stuck = recordFailedProgress(taskState, packetReceipt.packet);
      packetReceipt.status = 'rejected';
      packetReceipt.rejectionReason = p.reason;
      markTaskFailure(plan, state, taskState, attempt, 'code', [{ fingerprint: `handoff/${p.taskId}/rejected`, blocking: true, critical: false }], now);
      if (stuck && state.status !== 'blocked') blockRun(state, taskState, attempt, 'no-progress-limit');
      break;
    }
    case 'task_verdict': {
      requireKeys(p, ['taskId', 'packetId', 'verdict', 'findings'], [], 'INVALID_PAYLOAD', 'task_verdict payload');
      const { taskDef, taskState } = requireTask(plan, state, p.taskId);
      const packetReceipt = Object.hasOwn(state.receipts.packets, p.packetId) ? state.receipts.packets[p.packetId] : null;
      if (!packetReceipt || packetReceipt.taskId !== p.taskId) fail('UNKNOWN_PACKET', 'packet is not registered for this task');
      if (event.actorSessionId !== packetReceipt.receiverSessionId) fail('FORBIDDEN', 'only the packet receiver may issue its task verdict');
      requireRole(state, event.actorSessionId, 'verify');
      if (packetReceipt.status !== 'acked') fail('PACKET_NOT_ACKED', 'task verdict requires a separate accepted handoff acknowledgment');
      if (!['pass', 'fail'].includes(p.verdict)) fail('INVALID_VERDICT', 'verdict must be pass or fail');
      validateFindingList(p.findings);
      const attempt = currentAttempt(taskState, packetReceipt.attemptId);
      if (attempt.status !== 'verifying' || attempt.fence !== packetReceipt.fence || attempt.fenced) fail('STALE_FENCE', 'task verdict targets a stale attempt');
      assertLiveAttempt(attempt, now);
      const blocking = p.findings.some((finding) => finding.blocking);
      if (p.verdict === 'pass') {
        if (blocking) fail('BLOCKING_FINDING', 'task cannot pass with a blocking finding');
        const acResults = validateAcResultList(packetReceipt.acResults, plan, new Set([taskDef.id]), true);
        attempt.status = 'verified';
        attempt.fenced = true;
        attempt.completedAt = now;
        taskState.status = 'verified';
        taskState.acceptedPacketId = p.packetId;
        taskState.acceptedAcResults = acResults;
        taskState.currentAttemptId = null;
        taskState.pendingRetryKind = null;
        packetReceipt.status = 'verified';
        packetReceipt.verdictAt = now;
        packetReceipt.findings = deepClone(p.findings);
        releaseAttemptLocks(state, attempt);
        refreshReadyTasks(plan, state);
      } else {
        const stuck = recordFailedProgress(taskState, packetReceipt.packet);
        packetReceipt.status = 'failed';
        packetReceipt.verdictAt = now;
        packetReceipt.findings = deepClone(p.findings);
        markTaskFailure(plan, state, taskState, attempt, 'code', p.findings, now);
        if (stuck && state.status !== 'blocked') blockRun(state, taskState, attempt, 'no-progress-limit');
      }
      break;
    }
    case 'fail': {
      requireKeys(p, ['taskId', 'attemptId', 'category', 'fingerprint'], ['critical', 'progressDigest'], 'INVALID_PAYLOAD', 'fail payload');
      const { taskState } = requireTask(plan, state, p.taskId);
      const attempt = currentAttempt(taskState, p.attemptId);
      if (!['code', 'infra', 'timeout', 'process-loss'].includes(p.category) || !isString(p.fingerprint)) fail('INVALID_PAYLOAD', 'failure category or fingerprint is invalid');
      if (p.critical !== undefined && typeof p.critical !== 'boolean') fail('INVALID_PAYLOAD', 'critical must be boolean');
      if (p.progressDigest !== undefined && !validHex(p.progressDigest, [64])) fail('INVALID_PAYLOAD', 'progressDigest must be a SHA-256 digest');
      const isCoordinator = state.sessions[event.actorSessionId]?.coordinator === true;
      if (!isCoordinator && attempt.sessionId !== event.actorSessionId) fail('FORBIDDEN', 'failure must come from coordinator or bound worker');
      if (attempt.status !== 'running' || attempt.fenced) fail('STALE_ATTEMPT', 'failure targets a non-running or fenced attempt');
      const findings = [{ fingerprint: p.fingerprint, blocking: true, critical: p.critical === true }];
      // progressDigest is advisory and deliberately does not affect the no-progress counter.
      markTaskFailure(plan, state, taskState, attempt, p.category, findings, now);
      break;
    }
    case 'stop_confirmed': {
      requireKeys(p, ['taskId', 'attemptId', 'stopReceiptId'], [], 'INVALID_PAYLOAD', 'stop_confirmed payload');
      requireCoordinator(state, event.actorSessionId);
      const { taskState } = requireTask(plan, state, p.taskId);
      if (!isString(p.stopReceiptId)) fail('INVALID_PAYLOAD', 'stopReceiptId must be a non-empty trusted receipt ID');
      const attempt = findAttempt(taskState, p.attemptId);
      if (!attempt || attempt.status !== 'quarantined') fail('NOT_QUARANTINED', 'stop confirmation requires a quarantined attempt');
      attempt.status = 'stopped';
      attempt.stopReceiptId = p.stopReceiptId;
      attempt.stoppedAt = now;
      releaseAttemptLocks(state, attempt);
      taskState.currentAttemptId = null;
      if (state.status === 'active' && taskState.status !== 'blocked') taskState.status = 'rework';
      break;
    }
    case 'integrate': {
      requireKeys(p, ['candidate', 'taskCommits', 'groups', 'artifactDigests'], [], 'INVALID_PAYLOAD', 'integrate payload');
      const integrationSession = requireRole(state, event.actorSessionId, 'integration');
      if (state.status !== 'active' || state.candidate !== null || state.receipts.integration !== null) fail('INVALID_STATE_TRANSITION', 'run is not ready for candidate integration');
      validateCandidate(p.candidate, 'candidate');
      if (!Array.isArray(p.taskCommits)) fail('INVALID_INTEGRATION', 'taskCommits must be an array');
      const expectedOrder = topoOrder(plan.tasks);
      if (p.taskCommits.length !== expectedOrder.length) fail('INVALID_INTEGRATION', 'taskCommits must include every task exactly once');
      const taskCommits = [];
      for (let i = 0; i < p.taskCommits.length; i += 1) {
        const commit = p.taskCommits[i];
        requireKeys(commit, ['taskId', 'sha'], [], 'INVALID_INTEGRATION', 'task commit');
        if (commit.taskId !== expectedOrder[i] || !cleanSha(commit.sha)) fail('INVALID_INTEGRATION', 'taskCommits must follow dependency order and contain full SHAs');
        const taskState = state.tasks[commit.taskId];
        const packetReceipt = state.receipts.packets[taskState.acceptedPacketId];
        if (taskState.status !== 'verified' || !packetReceipt || packetReceipt.status !== 'verified') fail('TASKS_UNVERIFIED', `task ${commit.taskId} is not currently verified`);
        if (packetReceipt.packet.candidate.sha !== commit.sha) fail('INVALID_INTEGRATION', `task commit for ${commit.taskId} does not match its accepted packet`);
        taskCommits.push(deepClone(commit));
      }
      const allArtifactDigests = sortedUnique(Object.values(state.tasks).flatMap((taskState) => {
        const receipt = state.receipts.packets[taskState.acceptedPacketId];
        return receipt?.packet.artifacts.map((artifact) => artifact.sha256.toLowerCase()) ?? [];
      }));
      if (!Array.isArray(p.artifactDigests) || p.artifactDigests.some((value) => !validHex(value, [64])) || stableStringify(sortedUnique(p.artifactDigests.map((value) => value.toLowerCase()))) !== stableStringify(allArtifactDigests)) {
        fail('INVALID_INTEGRATION', 'artifactDigests must exactly bind the accepted task artifact set');
      }
      if (!Array.isArray(p.groups)) fail('INVALID_INTEGRATION', 'groups must be an array');
      const groupKeys = new Set();
      for (const group of p.groups) {
        requireKeys(group, ['tasks', 'outcome'], [], 'INVALID_INTEGRATION', 'integration group');
        if (!Array.isArray(group.tasks) || group.tasks.length < 2 || group.tasks.some((id) => !expectedOrder.includes(id)) || new Set(group.tasks).size !== group.tasks.length || group.outcome !== 'pass') {
          fail('INVALID_INTEGRATION', 'each integration group must contain unique known tasks and outcome pass');
        }
        const key = sortedUnique(group.tasks).join('\u0000');
        if (groupKeys.has(key)) fail('INVALID_INTEGRATION', 'duplicate integration group');
        groupKeys.add(key);
      }
      const characterizationTasks = plan.tasks.filter((task) => task.kind === 'characterization');
      for (const characterization of characterizationTasks) {
        const expectedRedCases = plan.acceptance.filter((ac) => ac.taskId === characterization.id && ac.expected === 'expected-red');
        for (const redCase of expectedRedCases) {
          const closures = plan.acceptance.filter((ac) => ac.closesAcId === redCase.id);
          const closureTaskIds = sortedUnique(closures.map((ac) => ac.taskId));
          if (closureTaskIds.length === 0) fail('INVALID_INTEGRATION', `expected-red case ${redCase.id} has no linked positive closure`);
          const group = p.groups.find((item) => item.tasks.includes(characterization.id) && closureTaskIds.every((id) => item.tasks.includes(id)));
          if (!group) fail('INVALID_INTEGRATION', `atomic GREEN group must include ${characterization.id} and every task that closes ${redCase.id}`);
          for (const closure of closures) {
            const hasPositive = state.tasks[closure.taskId].acceptedAcResults.some((result) => result.acId === closure.id && result.outcome === 'pass');
            if (!hasPositive) fail('INVALID_INTEGRATION', `closure ${closure.id} lacks accepted positive evidence`);
          }
        }
      }
      integrationSession.uses.push({ kind: 'integration', candidateDigest: digest(p.candidate) });
      state.candidate = deepClone(p.candidate);
      state.receipts.integration = {
        candidateDigest: digest(p.candidate), candidate: deepClone(p.candidate),
        taskCommits, groups: deepClone(p.groups), artifactDigests: allArtifactDigests,
        integratorSessionId: event.actorSessionId, integratedAt: now,
      };
      state.status = 'active';
      break;
    }
    case 'gate_report_reserved': {
      requireKeys(p, ['stage', 'candidateDigest', 'reportRunId'], [], 'INVALID_PAYLOAD', 'gate_report_reserved payload');
      const stages = ['verify', 'review', 'audit', 'final'];
      if (!stages.includes(p.stage) || !validHex(p.candidateDigest, [64]) || !isSafeIdentity(p.reportRunId)) fail('INVALID_GATE_REPORT', 'stage, candidate digest, or reportRunId is invalid');
      if (!state.candidate || !state.receipts.integration || p.candidateDigest !== digest(state.candidate)) fail('STALE_CANDIDATE', 'gate report reservation does not bind the current candidate');
      const expectedStage = stages[Object.keys(state.receipts.gates).length];
      if (p.stage !== expectedStage || state.receipts.gates[p.stage]) fail('GATE_ORDER', `next gate stage is ${expectedStage ?? 'none'}`);
      const { role } = authorizeGateSession(plan, state, p.stage, event.actorSessionId);
      const reservationKey = `${p.candidateDigest}:${p.stage}`;
      if (Object.hasOwn(state.receipts.gateReports, reservationKey)) fail('GATE_REPORT_ALREADY_RESERVED', 'this stage/candidate report attempt was already reserved');
      if (Object.values(state.receipts.gateReports).some((record) => record.reportRunId === p.reportRunId)) fail('DUPLICATE_REPORT_RUN', 'reportRunId is already reserved');
      setOwn(state.receipts.gateReports, reservationKey, {
        stage: p.stage, role, sessionId: event.actorSessionId,
        candidateDigest: p.candidateDigest, reportRunId: p.reportRunId, reservedAt: now,
      });
      break;
    }
    case 'gate': {
      requireKeys(p, ['stage', 'candidateDigest', 'reportRunId', 'priorEvidenceDigests', 'verdict', 'acResults', 'findings', 'evidenceDigests'], [], 'INVALID_PAYLOAD', 'gate payload');
      const stages = ['verify', 'review', 'audit', 'final'];
      if (!stages.includes(p.stage) || !['pass', 'fail'].includes(p.verdict) || !isSafeIdentity(p.reportRunId)) fail('INVALID_GATE', 'stage, reportRunId, or verdict is invalid');
      if (!state.candidate || !state.receipts.integration || p.candidateDigest !== digest(state.candidate)) fail('STALE_CANDIDATE', 'gate does not bind the current integrated candidate');
      const expectedStage = stages[Object.keys(state.receipts.gates).length];
      if (p.stage !== expectedStage || state.receipts.gates[p.stage]) fail('GATE_ORDER', `next gate stage is ${expectedStage ?? 'none'}`);
      const reservationKey = `${p.candidateDigest}:${p.stage}`;
      const reservation = Object.hasOwn(state.receipts.gateReports, reservationKey) ? state.receipts.gateReports[reservationKey] : null;
      if (!reservation || reservation.reportRunId !== p.reportRunId || reservation.sessionId !== event.actorSessionId) fail('GATE_REPORT_NOT_RESERVED', 'gate event must commit the single reserved report for this candidate and session');
      const { session: gateSession, role } = authorizeGateSession(plan, state, p.stage, event.actorSessionId);
      const expectedPriorStages = { verify: [], review: ['verify'], audit: ['verify', 'review'], final: ['verify', 'review', 'audit'] }[p.stage];
      const expectedPriorDigests = expectedPriorStages.map((stage) => state.receipts.gates[stage]?.reportDigest);
      if (!Array.isArray(p.priorEvidenceDigests) || p.priorEvidenceDigests.some((value) => !validHex(value, [64])) || stableStringify(p.priorEvidenceDigests.map((value) => value.toLowerCase())) !== stableStringify(expectedPriorDigests)) {
        fail('INVALID_PRIOR_EVIDENCE', 'priorEvidenceDigests must exactly cite all required earlier gate reports');
      }
      validateAcResultList(p.acResults, plan, null, true, p.verdict === 'pass');
      validateFindingList(p.findings);
      if (!Array.isArray(p.evidenceDigests) || p.evidenceDigests.length === 0 || p.evidenceDigests.some((value) => !validHex(value, [64])) || new Set(p.evidenceDigests).size !== p.evidenceDigests.length) {
        fail('INVALID_GATE', 'evidenceDigests must contain unique SHA-256 digests');
      }
      const blocking = p.findings.some((finding) => finding.blocking);
      if (p.verdict === 'pass' && blocking) fail('BLOCKING_FINDING', 'gate cannot pass with an open blocking finding');
      if (p.verdict === 'fail' && !blocking) fail('FAILED_GATE_NEEDS_FINDING', 'a failed gate report must retain at least one blocking finding');
      const counts = addFindingCounts({ findingCounts: state.receipts.findingCounts ?? (state.receipts.findingCounts = {}) }, p.findings);
      if (counts.critical || counts.repeated) {
        state.status = 'blocked';
      }
      const orderedEvidence = p.evidenceDigests.map((value) => value.toLowerCase());
      const receipt = {
        stage: p.stage, sessionId: event.actorSessionId, role,
        candidateDigest: p.candidateDigest, reportRunId: p.reportRunId,
        reportDigest: orderedEvidence[0], priorEvidenceDigests: p.priorEvidenceDigests.map((value) => value.toLowerCase()), verdict: p.verdict,
        acResults: deepClone(p.acResults), findings: deepClone(p.findings),
        evidenceDigests: orderedEvidence,
        recordedAt: now,
      };
      state.receipts.gates[p.stage] = receipt;
      gateSession.uses.push({ kind: 'gate', stage: p.stage, candidateDigest: p.candidateDigest });
      if (p.verdict === 'fail') state.status = 'blocked';
      else if (p.stage === 'final' && state.status !== 'blocked') state.status = 'done';
      break;
    }
    case 'invalidate_candidate': {
      requireKeys(p, ['reason'], [], 'INVALID_PAYLOAD', 'invalidate_candidate payload');
      requireCoordinator(state, event.actorSessionId);
      if (!isString(p.reason)) fail('INVALID_PAYLOAD', 'reason must be a non-empty string');
      if (!state.candidate && !state.receipts.integration && Object.keys(state.receipts.gates).length === 0) fail('NO_CANDIDATE', 'there is no candidate evidence to invalidate');
      for (const taskDef of plan.tasks) {
        const taskState = state.tasks[taskDef.id];
        if (taskState.status === 'verified') {
          taskState.status = 'rework';
          taskState.acceptedPacketId = null;
          taskState.acceptedAcResults = [];
          taskState.pendingRetryKind = 'code';
        }
      }
      state.candidate = null;
      state.receipts.integration = null;
      state.receipts.gates = {};
      state.receipts.invalidations ??= [];
      state.receipts.invalidations.push({ reason: p.reason, invalidatedAt: now });
      state.status = 'active';
      break;
    }
    case 'watchdog': {
      requireKeys(p, [], [], 'INVALID_PAYLOAD', 'watchdog payload');
      requireCoordinator(state, event.actorSessionId);
      for (const taskDef of plan.tasks) {
        const taskState = state.tasks[taskDef.id];
        const attempt = taskState.currentAttemptId ? findAttempt(taskState, taskState.currentAttemptId) : null;
        if (!attempt || !['reserved', 'running', 'handoff_pending', 'verifying'].includes(attempt.status)) continue;
        if (now < attempt.leaseUntil && now < attempt.deadline) continue;
        if (now >= attempt.deadline) {
          blockRun(state, taskState, attempt, 'task-deadline-exhausted');
        } else {
          markTaskFailure(plan, state, taskState, attempt, 'lease-expired', [], now);
        }
      }
      break;
    }
    case 'abort': {
      requireKeys(p, ['reason'], [], 'INVALID_PAYLOAD', 'abort payload');
      requireCoordinator(state, event.actorSessionId);
      if (!isString(p.reason)) fail('INVALID_PAYLOAD', 'reason must be a non-empty string');
      for (const taskState of Object.values(state.tasks)) {
        const attempt = taskState.currentAttemptId ? findAttempt(taskState, taskState.currentAttemptId) : null;
        if (attempt && ['reserved', 'running', 'handoff_pending', 'verifying'].includes(attempt.status)) quarantineAttempt(state, taskState, attempt, 'run-aborted');
      }
      state.status = 'aborted';
      state.receipts.abort = { reason: p.reason, abortedAt: now };
      break;
    }
    default:
      fail('UNKNOWN_EVENT', `unsupported event ${event.type}`);
  }
}

export function applyEvent(plan, state, event, { now } = {}) {
  assertPlan(plan);
  assertState(plan, state);
  validateEventEnvelope(event);
  const eventDigest = digest(event);
  const existing = Object.hasOwn(state.seenEvents, event.id) ? state.seenEvents[event.id] : null;
  if (existing) {
    if (existing.digest === eventDigest) return state;
    fail('DUPLICATE_EVENT_ID', 'event ID was already accepted with different content');
  }
  if (event.expectedVersion !== state.version) fail('VERSION_CONFLICT', `expected version ${state.version}, received ${event.expectedVersion}`);
  validateNow(now);
  if (now < state.lastNow) fail('CLOCK_ROLLBACK', 'trusted now must not move backward');
  if (state.status === 'done') fail('RUN_COMPLETE', 'completed run accepts no further events');
  if (state.status === 'aborted' && !['stop_confirmed', 'watchdog'].includes(event.type)) fail('RUN_ABORTED', 'aborted run accepts only stop confirmation or watchdog recovery');
  if (state.status === 'blocked' && !['stop_confirmed', 'watchdog', 'abort'].includes(event.type)) fail('RUN_BLOCKED', 'blocked run accepts only quarantine recovery or abort');
  const next = deepClone(state);
  next.lastNow = now;
  try {
    handleEvent(plan, next, event, now);
  } catch (error) {
    if (error instanceof GuardError) throw error;
    throw new GuardError('INVALID_EVENT', error instanceof Error ? error.message : String(error));
  }
  next.version = state.version + 1;
  next.lastNow = now;
  setOwn(next.seenEvents, event.id, { digest: eventDigest, version: next.version });
  return deepFreeze(next);
}
