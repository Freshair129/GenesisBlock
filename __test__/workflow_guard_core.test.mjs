import assert from 'node:assert/strict';
import test from 'node:test';

import {
  GuardError,
  applyEvent,
  createState,
  digest,
  normalizePath,
  pathsOverlap,
  validatePlan,
} from '../scripts/workflow-guard/core.mjs';

const sha = (letter) => letter.repeat(64);
const gitSha = (letter) => letter.repeat(40);

function makePlan(overrides = {}) {
  const base = {
    schemaVersion: 1,
    id: 'guard-test',
    revision: 1,
    baselineSha: gitSha('a'),
    authorization: { accepted: true, dispatchAuthorized: true, scope: 'synthetic-pilot', decisionId: 'owner-1' },
    roles: {
      worker: { model: 'gpt-6-luna', effort: 'max' },
      verify: { model: 'gpt-6-luna', effort: 'max' },
      review: { model: 'gpt-6-luna', effort: 'max' },
      audit: { model: 'gpt-6.1-sol', effort: 'high' },
      integration: { model: 'gpt-6.1-sol', effort: 'high' },
      orchestrator: { model: 'gpt-6.1-sol', effort: 'high' },
    },
    decisions: [],
    tasks: [{
      id: 'T1', role: 'worker', dependsOn: [], requiredDecisions: [],
      writePaths: ['pilot/input.txt'], requiredAc: ['AC-1'], timeoutMs: 120_000,
      leaseMs: 60_000, maxCommandRuns: 4, kind: 'normal',
    }],
    graph: { nodes: ['T1'], edges: [] },
    acceptance: [{
      id: 'AC-1', requirementId: 'WF-AC-05', taskId: 'T1', caseId: 'positive',
      expected: 'pass', required: true, commandId: 'check',
    }],
    commands: { check: { executable: 'node', args: ['check.mjs'], cwd: '.', timeoutMs: 10_000 } },
    policy: { maxAttempts: 3, infraRetries: 1, repeatLimit: 2 },
  };
  return { ...base, ...overrides };
}

function harness(plan = makePlan(), { leaseMs, maxCommandRuns } = {}) {
  const selectedPlan = structuredClone(plan);
  if (leaseMs !== undefined) selectedPlan.tasks[0].leaseMs = leaseMs;
  if (maxCommandRuns !== undefined) selectedPlan.tasks[0].maxCommandRuns = maxCommandRuns;
  let state = createState(selectedPlan, { runId: 'run-1', now: 100, coordinatorSessionId: 'coord' });
  let sequence = 0;
  const emit = (type, actor, payload, explicitNow) => {
    const event = {
      id: `event-${++sequence}`,
      type,
      expectedVersion: state.version,
      actorSessionId: actor,
      payload,
    };
    state = applyEvent(selectedPlan, state, event, { now: explicitNow ?? state.lastNow + 1 });
    return state;
  };
  const register = (sessionId, role, model = selectedPlan.roles[role].model, effort = selectedPlan.roles[role].effort) => emit('register_session', 'coord', {
    sessionId, role, model, effort, providerReceiptId: `provider-${sessionId}`,
  });
  return { plan: selectedPlan, get state() { return state; }, emit, register };
}

function prepareHandoff(h = harness(), {
  commandTestCount = 1, path = 'pilot/input.txt', outcome = 'pass',
  workerId = 'worker-1', verifyId = 'verify-1', packetId = 'packet-1',
  commandRunId = 'run-check-1', sourceDigest = sha('a'),
} = {}) {
  h.register(workerId, 'worker');
  h.register(verifyId, 'verify');
  h.emit('dispatch', 'coord', { taskId: 'T1' });
  const attempt = h.state.tasks.T1.attempts.at(-1);
  h.emit('bind_worker', 'coord', { taskId: 'T1', attemptId: attempt.attemptId, sessionId: workerId });
  h.emit('command_reserved', 'coord', {
    taskId: 'T1', attemptId: attempt.attemptId, fence: attempt.fence,
    commandRunId, commandId: 'check',
  });
  const stdoutDigest = sha('b');
  const caseResults = commandTestCount > 0
    ? [{ acId: 'AC-1', caseId: 'positive', outcome, testCount: commandTestCount }]
    : [];
  const runnerReceipt = {
    commandId: 'check', cwd: '.', exitCode: outcome === 'expected-red' ? 1 : 0,
    signal: null, timedOut: false, durationMs: 5,
    testCount: commandTestCount, caseResults,
    environmentDigest: sha('c'), stdoutDigest, stderrDigest: sha('d'),
  };
  h.emit('command_completed', 'coord', { commandRunId, receipt: runnerReceipt });
  const packet = {
    packetId, runId: h.state.runId, planDigest: h.state.planDigest,
    taskId: 'T1', attemptId: attempt.attemptId, fence: attempt.fence,
    senderSessionId: workerId, receiverSessionId: verifyId,
    candidate: { sha: gitSha('e'), tree: gitSha('f'), sourceDigest },
    changedPaths: [path], artifacts: [],
    commands: [{ commandRunId, ...runnerReceipt }],
    acResults: commandTestCount > 0 ? [{
      acId: 'AC-1', caseId: 'positive', outcome, testCount: commandTestCount,
      evidenceDigest: stdoutDigest,
    }] : [],
    findings: [],
  };
  return { h, attempt, packet, stdoutDigest };
}

function acceptedTask(h, packet) {
  h.emit('submit_handoff', 'worker-1', { packet });
  h.emit('ack_handoff', 'verify-1', { taskId: 'T1', packetId: packet.packetId });
  h.emit('task_verdict', 'verify-1', { taskId: 'T1', packetId: packet.packetId, verdict: 'pass', findings: [] });
}

function characterizePlan() {
  const base = makePlan();
  const c0 = { ...base.tasks[0], id: 'C0', kind: 'characterization', writePaths: ['pilot/c0.txt'], requiredAc: ['AC-RED'] };
  const c1 = { ...base.tasks[0], id: 'C1', dependsOn: ['C0'], writePaths: ['pilot/c1.txt'], requiredAc: ['AC-GREEN'] };
  return {
    ...base,
    tasks: [c0, c1],
    graph: { nodes: ['C0', 'C1'], edges: [['C0', 'C1']] },
    acceptance: [
      { id: 'AC-RED', requirementId: 'WF-AC-09', taskId: 'C0', caseId: 'old-behavior', expected: 'expected-red', required: true, commandId: 'check' },
      { id: 'AC-GREEN', requirementId: 'WF-AC-09', taskId: 'C1', caseId: 'regression', expected: 'pass', required: true, commandId: 'check', closesAcId: 'AC-RED' },
    ],
  };
}

function completeTask(h, taskId, {
  workerId = `worker-${taskId}`, verifyId = `verify-${taskId}`, outcomeOverrides = {},
  omitOptional = false, runnerOverrides = {}, commandRunId = `run-check-${taskId}`, packetId = `packet-${taskId}`,
} = {}) {
  const task = h.plan.tasks.find((item) => item.id === taskId);
  const acceptance = h.plan.acceptance.filter((item) => item.taskId === taskId);
  const commandId = acceptance[0].commandId;
  h.register(workerId, task.role);
  h.register(verifyId, 'verify');
  h.emit('dispatch', 'coord', { taskId });
  const attempt = h.state.tasks[taskId].attempts.at(-1);
  h.emit('bind_worker', 'coord', { taskId, attemptId: attempt.attemptId, sessionId: workerId });
  h.emit('command_reserved', 'coord', {
    taskId, attemptId: attempt.attemptId, fence: attempt.fence, commandRunId, commandId,
  });
  const stdoutDigest = sha(taskId === 'C0' ? '3' : taskId === 'C1' ? '4' : 'b');
  const caseResults = acceptance.filter((ac) => !omitOptional || ac.required).map((ac) => {
    const rowOutcome = Object.hasOwn(outcomeOverrides, ac.id) ? outcomeOverrides[ac.id] : ac.expected;
    return { acId: ac.id, caseId: ac.caseId, outcome: rowOutcome, testCount: rowOutcome === 'skip' ? 0 : 1 };
  });
  const runnerReceipt = {
    commandId, cwd: '.', exitCode: caseResults.some((row) => row.outcome === 'expected-red') ? 1 : 0,
    signal: null, timedOut: false, durationMs: 5,
    testCount: caseResults.reduce((sum, row) => sum + row.testCount, 0), caseResults,
    environmentDigest: sha('c'), stdoutDigest, stderrDigest: sha('d'),
    ...runnerOverrides,
  };
  h.emit('command_completed', 'coord', { commandRunId, receipt: runnerReceipt });
  const packet = {
    packetId, runId: h.state.runId, planDigest: h.state.planDigest,
    taskId, attemptId: attempt.attemptId, fence: attempt.fence,
    senderSessionId: workerId, receiverSessionId: verifyId,
    candidate: { sha: gitSha(taskId === 'C0' ? '1' : taskId === 'C1' ? '2' : 'e'), tree: gitSha('f'), sourceDigest: sha(taskId === 'C0' ? '5' : taskId === 'C1' ? '6' : 'a') },
    changedPaths: [task.writePaths[0]], artifacts: [],
    commands: [{ commandRunId, ...runnerReceipt }],
    acResults: caseResults.map((row) => ({ ...row, evidenceDigest: stdoutDigest })),
    findings: [],
  };
  h.emit('submit_handoff', workerId, { packet });
  h.emit('ack_handoff', verifyId, { taskId, packetId });
  h.emit('task_verdict', verifyId, { taskId, packetId, verdict: 'pass', findings: [] });
  return { packet, workerId, verifyId, stdoutDigest };
}

function integrate(h, groups = []) {
  h.register('integration-1', 'integration');
  const taskCommits = h.plan.tasks.map((task) => ({ taskId: task.id, sha: h.state.receipts.packets[h.state.tasks[task.id].acceptedPacketId].packet.candidate.sha }));
  const candidate = { sha: gitSha('e'), tree: gitSha('f'), sourceDigest: sha('a') };
  h.emit('integrate', 'integration-1', { candidate, taskCommits, groups, artifactDigests: [] });
  return candidate;
}

const REPORT_CHARS = { verify: '9', review: '8', audit: '7', final: '6' };

function emitGate(h, stage, sessionId, verdict = 'pass', findings = [], acResults = undefined) {
  const candidateDigest = digest(h.state.candidate);
  const reportRunId = `report-${stage}-1`;
  h.emit('gate_report_reserved', sessionId, { stage, candidateDigest, reportRunId });
  const priorStages = { verify: [], review: ['verify'], audit: ['verify', 'review'], final: ['verify', 'review', 'audit'] }[stage];
  const priorEvidenceDigests = priorStages.map((prior) => h.state.receipts.gates[prior].reportDigest);
  const effectiveResults = acResults ?? h.plan.acceptance.filter((ac) => ac.required).map((ac) => {
    return h.state.tasks[ac.taskId].acceptedAcResults.find((row) => row.acId === ac.id);
  });
  const evidenceDigests = [sha(REPORT_CHARS[stage]), sha('5')];
  h.emit('gate', sessionId, {
    stage, candidateDigest, reportRunId, priorEvidenceDigests,
    verdict, acResults: effectiveResults, findings, evidenceDigests,
  });
}

function fullHappyPath() {
  const h = harness();
  const task = completeTask(h, 'T1', { workerId: 'worker-1', verifyId: 'verify-1' });
  h.register('review-1', 'review');
  h.register('audit-1', 'audit');
  integrate(h);
  emitGate(h, 'verify', 'verify-1');
  emitGate(h, 'review', 'review-1');
  emitGate(h, 'audit', 'audit-1');
  emitGate(h, 'final', 'coord');
  return { h, task };
}

function assertRejectedWithoutMutation(h, event, code, now = h.state.lastNow + 1) {
  const before = h.state;
  assert.throws(() => applyEvent(h.plan, before, event, { now }), (error) => error instanceof GuardError && error.code === code);
  assert.equal(before.version, h.state.version);
  assert.equal(before.lastNow, h.state.lastNow);
}

test('normalizes relative scopes and rejects traversal, aliases, and parent-child conflicts', () => {
  assert.equal(normalizePath('Pilot\\Sub\\File.TXT'), 'pilot/sub/file.txt');
  assert.equal(pathsOverlap('src', 'src/lib.rs'), true);
  assert.equal(pathsOverlap('src-a', 'src'), false);
  for (const path of ['/absolute/path', 'C:relative', 'C:\\absolute', '..\\escape', 'src/../secret', 'NUL.txt', 'src/name.']) {
    assert.throws(() => normalizePath(path));
  }
  const plan = makePlan({
    tasks: [
      { ...makePlan().tasks[0], id: 'T1', writePaths: ['src'] },
      { ...makePlan().tasks[0], id: 'T2', writePaths: ['src/lib.rs'] },
    ],
    graph: { nodes: ['T1', 'T2'], edges: [] },
    acceptance: [
      { ...makePlan().acceptance[0], taskId: 'T1', id: 'AC-1' },
      { ...makePlan().acceptance[0], taskId: 'T2', id: 'AC-2' },
    ],
  });
  plan.tasks[0].requiredAc = ['AC-1'];
  plan.tasks[1].requiredAc = ['AC-2'];
  assert.equal(validatePlan(plan).ok, false);
});

test('validates exact task and graph sets, cycle freedom, model policy, and poison IDs', () => {
  const plan = makePlan();
  assert.equal(validatePlan(plan).ok, true);
  const duplicateNode = structuredClone(plan);
  duplicateNode.graph.nodes = ['T1', 'T1'];
  assert.equal(validatePlan(duplicateNode).ok, false);
  const wrongModel = structuredClone(plan);
  wrongModel.roles.worker.effort = 'high';
  assert.equal(validatePlan(wrongModel).ok, false);
  const poison = structuredClone(plan);
  poison.tasks[0].id = '__proto__';
  poison.graph.nodes = ['__proto__'];
  poison.acceptance[0].taskId = '__proto__';
  assert.equal(validatePlan(poison).ok, false);
  const duplicateTask = structuredClone(plan);
  duplicateTask.tasks.push(structuredClone(duplicateTask.tasks[0]));
  duplicateTask.graph.nodes = ['T1'];
  assert.equal(validatePlan(duplicateTask).ok, false);
  const cycle = characterizePlan();
  cycle.tasks[0].dependsOn = ['C1'];
  cycle.graph.edges = [['C1', 'C0'], ['C0', 'C1']];
  assert.equal(validatePlan(cycle).ok, false);
  const poisonEvent = { id: '__proto__', type: 'dispatch', expectedVersion: 0, actorSessionId: 'coord', payload: { taskId: 'T1' } };
  assert.throws(() => applyEvent(plan, createState(plan, { runId: 'run-1', now: 100, coordinatorSessionId: 'coord' }), poisonEvent, { now: 101 }), (error) => error.code === 'INVALID_EVENT');
});

test('rejects unauthorized dispatch and worker-authored session registration without changing state', () => {
  const unauthorized = makePlan({ authorization: { accepted: true, dispatchAuthorized: false, scope: 'synthetic-pilot', decisionId: 'owner-1' } });
  const h = harness(unauthorized);
  const event = { id: 'dispatch-1', type: 'dispatch', expectedVersion: 0, actorSessionId: 'coord', payload: { taskId: 'T1' } };
  assertRejectedWithoutMutation(h, event, 'UNAUTHORIZED_PLAN');
  const worker = prepareHandoff();
  const bad = { id: 'self-register', type: 'register_session', expectedVersion: worker.h.state.version, actorSessionId: 'worker-1', payload: {
    sessionId: 'attacker', role: 'worker', model: 'gpt-6-luna', effort: 'max', providerReceiptId: 'fake',
  } };
  assertRejectedWithoutMutation(worker.h, bad, 'FORBIDDEN');
});

test('requires parsed command evidence and rejects zero tests or changed scope before handoff', () => {
  const noTests = prepareHandoff(harness(), { commandTestCount: 0 });
  const event = { id: 'submit-empty', type: 'submit_handoff', expectedVersion: noTests.h.state.version, actorSessionId: 'worker-1', payload: { packet: noTests.packet } };
  assertRejectedWithoutMutation(noTests.h, event, 'INCOMPLETE_AC');
  const outOfScope = prepareHandoff();
  outOfScope.packet.changedPaths = ['other/file.txt'];
  const badScope = { id: 'submit-scope', type: 'submit_handoff', expectedVersion: outOfScope.h.state.version, actorSessionId: 'worker-1', payload: { packet: outOfScope.packet } };
  assertRejectedWithoutMutation(outOfScope.h, badScope, 'OUT_OF_SCOPE');
  const forged = prepareHandoff();
  forged.packet.acResults[0].evidenceDigest = sha('7');
  const badEvidence = { id: 'submit-forged', type: 'submit_handoff', expectedVersion: forged.h.state.version, actorSessionId: 'worker-1', payload: { packet: forged.packet } };
  assertRejectedWithoutMutation(forged.h, badEvidence, 'AC_RECEIPT_MISMATCH');
});

test('keeps a packet from passing after its lease expires even before watchdog runs', () => {
  const { h, packet } = prepareHandoff(harness(), { leaseMs: 5 });
  h.emit('submit_handoff', 'worker-1', { packet });
  const ack = { id: 'late-ack', type: 'ack_handoff', expectedVersion: h.state.version, actorSessionId: 'verify-1', payload: { taskId: 'T1', packetId: packet.packetId } };
  assertRejectedWithoutMutation(h, ack, 'LEASE_EXPIRED', h.state.tasks.T1.attempts[0].leaseUntil);
});

test('rejects an expired submitted packet without changing failure attribution', () => {
  const { h, packet, attempt } = prepareHandoff(harness(makePlan(), { leaseMs: 5 }));
  h.emit('submit_handoff', 'worker-1', { packet });
  const reject = { id: 'late-reject', type: 'reject_handoff', expectedVersion: h.state.version, actorSessionId: 'verify-1', payload: {
    taskId: 'T1', packetId: packet.packetId, reason: 'late review',
  } };
  assertRejectedWithoutMutation(h, reject, 'LEASE_EXPIRED', attempt.leaseUntil);
  assert.equal(h.state.receipts.packets[packet.packetId].status, 'submitted');
  assert.equal(h.state.tasks.T1.noProgressCount, 0);
  assert.equal(h.state.tasks.T1.findingCounts['handoff/T1/rejected'], undefined);
});

test('does not accept an ACKed packet after its lease expires before the separate verdict', () => {
  const { h, packet, attempt } = prepareHandoff(harness(makePlan(), { leaseMs: 100 }));
  h.emit('submit_handoff', 'worker-1', { packet });
  h.emit('ack_handoff', 'verify-1', { taskId: 'T1', packetId: packet.packetId });
  const verdict = { id: 'late-verdict', type: 'task_verdict', expectedVersion: h.state.version, actorSessionId: 'verify-1', payload: {
    taskId: 'T1', packetId: packet.packetId, verdict: 'pass', findings: [],
  } };
  assertRejectedWithoutMutation(h, verdict, 'LEASE_EXPIRED', attempt.leaseUntil);
});

test('uses attempt fences, cumulative command budgets, and stop proof before reallocating a failed scope', () => {
  const { h, attempt } = prepareHandoff(harness(makePlan(), { maxCommandRuns: 1 }));
  const secondCommand = { id: 'command-2', type: 'command_reserved', expectedVersion: h.state.version, actorSessionId: 'coord', payload: {
    taskId: 'T1', attemptId: attempt.attemptId, fence: attempt.fence, commandRunId: 'run-check-2', commandId: 'check',
  } };
  assertRejectedWithoutMutation(h, secondCommand, 'COMMAND_BUDGET_EXHAUSTED');
  h.emit('fail', 'worker-1', { taskId: 'T1', attemptId: attempt.attemptId, category: 'process-loss', fingerprint: 'WF-AC-08/process-loss/pilot/input.txt' });
  assert.equal(h.state.locks['pilot/input.txt'].quarantined, true);
  const blockedDispatch = { id: 'dispatch-quarantined', type: 'dispatch', expectedVersion: h.state.version, actorSessionId: 'coord', payload: { taskId: 'T1' } };
  assertRejectedWithoutMutation(h, blockedDispatch, 'ATTEMPT_ALREADY_ACTIVE');
  h.emit('stop_confirmed', 'coord', { taskId: 'T1', attemptId: attempt.attemptId, stopReceiptId: 'provider-stopped-1' });
  assert.equal(Object.hasOwn(h.state.locks, 'pilot/input.txt'), false);
  h.emit('dispatch', 'coord', { taskId: 'T1' });
  assert.equal(h.state.tasks.T1.infraRetries, 1);
  h.register('worker-2', 'worker');
  const replacement = h.state.tasks.T1.attempts.at(-1);
  h.emit('bind_worker', 'coord', { taskId: 'T1', attemptId: replacement.attemptId, sessionId: 'worker-2' });
  const staleHeartbeat = { id: 'old-fence-heartbeat', type: 'heartbeat', expectedVersion: h.state.version, actorSessionId: 'worker-1', payload: {
    taskId: 'T1', attemptId: attempt.attemptId, fence: attempt.fence,
  } };
  assertRejectedWithoutMutation(h, staleHeartbeat, 'STALE_ATTEMPT');
});

test('makes event replay idempotent and rejects changed IDs or stale versions', () => {
  const h = harness();
  const event = { id: 'register-1', type: 'register_session', expectedVersion: 0, actorSessionId: 'coord', payload: {
    sessionId: 'worker-1', role: 'worker', model: 'gpt-6-luna', effort: 'max', providerReceiptId: 'provider-worker-1',
  } };
  const next = applyEvent(h.plan, h.state, event, { now: 101 });
  assert.equal(applyEvent(h.plan, next, event, { now: 1 }), next);
  const changed = { ...event, payload: { ...event.payload, providerReceiptId: 'different' } };
  assert.throws(() => applyEvent(h.plan, next, changed, { now: 102 }), (error) => error.code === 'DUPLICATE_EVENT_ID');
  const stale = { ...event, id: 'register-stale', expectedVersion: 0 };
  assert.throws(() => applyEvent(h.plan, next, stale, { now: 102 }), (error) => error.code === 'VERSION_CONFLICT');
  assert.equal(next.version, 1);
});

test('requires independent candidate gates and reaches done only on a fresh full pass', () => {
  const { h } = fullHappyPath();
  assert.equal(h.state.status, 'done');
  assert.deepEqual(Object.keys(h.state.receipts.gates), ['verify', 'review', 'audit', 'final']);
  assert.equal(Object.isFrozen(h.state.tasks.T1.attempts[0]), true);
  assert.equal(h.state.receipts.gates.verify.reportDigest, sha('9'));
  assert.deepEqual(h.state.receipts.gates.audit.priorEvidenceDigests, [sha('9'), sha('8')]);
});

test('requires an explicit positive AC closure for every expected-red characterization', () => {
  const incomplete = characterizePlan();
  incomplete.acceptance[1].closesAcId = undefined;
  delete incomplete.acceptance[1].closesAcId;
  assert.equal(validatePlan(incomplete).ok, false);
  incomplete.acceptance[1].closesAcId = 'AC-RED';
  assert.equal(validatePlan(incomplete).ok, true);
});

test('dispatch enforces unresolved decisions and predecessor verification even against a forged ready status', () => {
  const decisionPlan = makePlan({ decisions: [{ id: 'D1', resolved: false, decisionId: 'owner-1' }] });
  decisionPlan.tasks[0].requiredDecisions = ['D1'];
  const decisionHarness = harness(decisionPlan);
  const unresolved = { id: 'dispatch-unresolved', type: 'dispatch', expectedVersion: decisionHarness.state.version, actorSessionId: 'coord', payload: { taskId: 'T1' } };
  assertRejectedWithoutMutation(decisionHarness, unresolved, 'UNRESOLVED_DECISION');

  const h = harness(characterizePlan());
  const forgedState = structuredClone(h.state);
  forgedState.tasks.C1.status = 'ready';
  const blocked = { id: 'dispatch-dependent', type: 'dispatch', expectedVersion: forgedState.version, actorSessionId: 'coord', payload: { taskId: 'C1' } };
  assert.throws(() => applyEvent(h.plan, forgedState, blocked, { now: forgedState.lastNow + 1 }), (error) => error.code === 'PREDECESSOR_UNVERIFIED');
  assert.equal(forgedState.version, 0);
});

test('required skips cannot pass, while optional absence and explicit skip stay non-blocking', () => {
  const requiredSkip = harness();
  assert.throws(() => completeTask(requiredSkip, 'T1', { outcomeOverrides: { 'AC-1': 'skip' } }), (error) => error.code === 'REQUIRED_AC_SKIPPED');

  const base = makePlan();
  const optionalPlan = structuredClone(base);
  optionalPlan.acceptance.push({ id: 'AC-OPTIONAL', requirementId: 'WF-OPTIONAL', taskId: 'T1', caseId: 'optional', expected: 'pass', required: false, commandId: 'check' });
  assert.equal(validatePlan(optionalPlan).ok, true);
  const absent = harness(optionalPlan);
  completeTask(absent, 'T1', { omitOptional: true });
  assert.equal(absent.state.tasks.T1.status, 'verified');
  const skipped = harness(optionalPlan);
  completeTask(skipped, 'T1', { outcomeOverrides: { 'AC-OPTIONAL': 'skip' } });
  assert.equal(skipped.state.tasks.T1.status, 'verified');
  assert.equal(skipped.state.tasks.T1.acceptedAcResults.find((row) => row.acId === 'AC-OPTIONAL').outcome, 'skip');
});

test('required unsupported and schema-invalid outcomes are distinct valid AC evidence', () => {
  for (const outcome of ['unsupported', 'schema-invalid']) {
    const plan = makePlan();
    plan.acceptance[0].expected = outcome;
    assert.equal(validatePlan(plan).ok, true);
    const h = harness(plan);
    completeTask(h, 'T1');
    assert.equal(h.state.tasks.T1.acceptedAcResults[0].outcome, outcome);
  }
  const invalid = makePlan();
  invalid.acceptance[0].expected = 'green-enough';
  assert.equal(validatePlan(invalid).ok, false);
});

test('expected-red requires a parsed RED marker and a normal nonzero process exit', () => {
  const plan = characterizePlan();
  const timeoutHarness = harness(plan);
  assert.throws(() => completeTask(timeoutHarness, 'C0', { runnerOverrides: { exitCode: null, timedOut: true } }), (error) => error.code === 'EXPECTED_RED_MISSING');
  const signalHarness = harness(plan);
  assert.throws(() => completeTask(signalHarness, 'C0', { runnerOverrides: { exitCode: null, signal: 'SIGTERM' } }), (error) => error.code === 'EXPECTED_RED_MISSING');
});

test('accepted C0 RED evidence must feed its linked C1 GREEN task and an atomic integration group', () => {
  const plan = characterizePlan();
  const h = harness(plan);
  const c0 = completeTask(h, 'C0');
  assert.equal(h.state.tasks.C0.status, 'verified');
  assert.equal(h.state.tasks.C1.status, 'ready');
  assert.equal(c0.packet.acResults[0].outcome, 'expected-red');
  const c1 = completeTask(h, 'C1');
  assert.equal(c1.packet.acResults[0].outcome, 'pass');
  const candidate = integrate(h, [{ tasks: ['C0', 'C1'], outcome: 'pass' }]);
  assert.equal(h.state.candidate.sha, candidate.sha);

  const missingGroup = harness(plan);
  completeTask(missingGroup, 'C0');
  completeTask(missingGroup, 'C1');
  missingGroup.register('integration-1', 'integration');
  const taskCommits = missingGroup.plan.tasks.map((task) => ({ taskId: task.id, sha: missingGroup.state.receipts.packets[missingGroup.state.tasks[task.id].acceptedPacketId].packet.candidate.sha }));
  const badIntegration = { id: 'integrate-no-group', type: 'integrate', expectedVersion: missingGroup.state.version, actorSessionId: 'integration-1', payload: {
    candidate: { sha: gitSha('e'), tree: gitSha('f'), sourceDigest: sha('a') }, taskCommits, groups: [], artifactDigests: [],
  } };
  assertRejectedWithoutMutation(missingGroup, badIntegration, 'INVALID_INTEGRATION');
});

test('one-shot gate reports preserve the report hash first and reject reused sessions, stale order, and stale candidates', () => {
  const { h } = fullHappyPath();
  assert.equal(h.state.receipts.gates.review.reportDigest, sha('8'));

  const partial = harness();
  completeTask(partial, 'T1');
  partial.register('review-1', 'review');
  partial.register('audit-1', 'audit');
  integrate(partial);
  const candidateDigest = digest(partial.state.candidate);
  const verifyReportId = 'verify-report-once';
  partial.emit('gate_report_reserved', 'verify-T1', { stage: 'verify', candidateDigest, reportRunId: verifyReportId });
  const duplicate = { id: 'verify-report-second', type: 'gate_report_reserved', expectedVersion: partial.state.version, actorSessionId: 'verify-T1', payload: { stage: 'verify', candidateDigest, reportRunId: 'verify-report-retry' } };
  assertRejectedWithoutMutation(partial, duplicate, 'GATE_REPORT_ALREADY_RESERVED');
  partial.emit('gate', 'verify-T1', {
    stage: 'verify', candidateDigest, reportRunId: verifyReportId, priorEvidenceDigests: [], verdict: 'pass',
    acResults: partial.state.tasks.T1.acceptedAcResults, findings: [], evidenceDigests: [sha('9')],
  });
  const reused = { id: 'review-reused-session', type: 'gate_report_reserved', expectedVersion: partial.state.version, actorSessionId: 'verify-T1', payload: { stage: 'review', candidateDigest, reportRunId: 'review-report' } };
  assertRejectedWithoutMutation(partial, reused, 'SESSION_REUSED');
  partial.emit('invalidate_candidate', 'coord', { reason: 'fixture mutation' });
  const stale = { id: 'stale-gate-reserve', type: 'gate_report_reserved', expectedVersion: partial.state.version, actorSessionId: 'verify-T1', payload: { stage: 'verify', candidateDigest, reportRunId: 'stale-report' } };
  assertRejectedWithoutMutation(partial, stale, 'STALE_CANDIDATE');
});

test('task authors cannot self-review and integration sessions cannot fill a Review role', () => {
  const authorPlan = makePlan();
  authorPlan.tasks[0].role = 'review';
  const author = harness(authorPlan);
  completeTask(author, 'T1', { workerId: 'review-author', verifyId: 'verify-task' });
  author.register('audit-1', 'audit');
  integrate(author);
  emitGate(author, 'verify', 'verify-task');
  const authorReview = { id: 'author-review', type: 'gate_report_reserved', expectedVersion: author.state.version, actorSessionId: 'review-author', payload: {
    stage: 'review', candidateDigest: digest(author.state.candidate), reportRunId: 'author-review-report',
  } };
  assertRejectedWithoutMutation(author, authorReview, 'NON_INDEPENDENT_GATE');
  const integratorReview = { id: 'integrator-review', type: 'gate_report_reserved', expectedVersion: author.state.version, actorSessionId: 'integration-1', payload: {
    stage: 'review', candidateDigest: digest(author.state.candidate), reportRunId: 'integrator-review-report',
  } };
  assertRejectedWithoutMutation(author, integratorReview, 'FORBIDDEN');
});

test('a failed gate retains partial AC results and a blocker, then blocks all further reports', () => {
  const h = harness();
  completeTask(h, 'T1');
  h.register('review-1', 'review');
  h.register('audit-1', 'audit');
  integrate(h);
  const finding = [{ fingerprint: 'WF-AC-05/test-failed', blocking: true, critical: false }];
  emitGate(h, 'verify', 'verify-T1', 'fail', finding, []);
  assert.equal(h.state.status, 'blocked');
  assert.deepEqual(h.state.receipts.gates.verify.acResults, []);
  const retry = { id: 'blocked-review-reserve', type: 'gate_report_reserved', expectedVersion: h.state.version, actorSessionId: 'review-1', payload: {
    stage: 'review', candidateDigest: digest(h.state.candidate), reportRunId: 'review-after-fail',
  } };
  assertRejectedWithoutMutation(h, retry, 'RUN_BLOCKED');
});

test('final gate cannot be reserved before Audit, and candidate invalidation rejects stale completion', () => {
  const h = harness();
  completeTask(h, 'T1');
  h.register('review-1', 'review');
  h.register('audit-1', 'audit');
  integrate(h);
  emitGate(h, 'verify', 'verify-T1');
  emitGate(h, 'review', 'review-1');
  const missingAudit = { id: 'early-final', type: 'gate_report_reserved', expectedVersion: h.state.version, actorSessionId: 'coord', payload: {
    stage: 'final', candidateDigest: digest(h.state.candidate), reportRunId: 'final-before-audit',
  } };
  assertRejectedWithoutMutation(h, missingAudit, 'GATE_ORDER');
  const oldDigest = digest(h.state.candidate);
  h.emit('invalidate_candidate', 'coord', { reason: 'fresh integration required' });
  const staleGate = { id: 'stale-final', type: 'gate', expectedVersion: h.state.version, actorSessionId: 'coord', payload: {
    stage: 'audit', candidateDigest: oldDigest, reportRunId: 'old-report', priorEvidenceDigests: [],
    verdict: 'pass', acResults: [], findings: [], evidenceDigests: [sha('7')],
  } };
  assertRejectedWithoutMutation(h, staleGate, 'STALE_CANDIDATE');
});

test('bounds three code attempts and one infrastructure retry, requiring stop receipts each time', () => {
  const code = harness();
  for (let attemptNo = 1; attemptNo <= 3; attemptNo += 1) {
    code.emit('dispatch', 'coord', { taskId: 'T1' });
    const attempt = code.state.tasks.T1.attempts.at(-1);
    code.register(`worker-code-${attemptNo}`, 'worker');
    code.emit('bind_worker', 'coord', { taskId: 'T1', attemptId: attempt.attemptId, sessionId: `worker-code-${attemptNo}` });
    code.emit('fail', `worker-code-${attemptNo}`, { taskId: 'T1', attemptId: attempt.attemptId, category: 'code', fingerprint: `WF-AC-05/code-${attemptNo}` });
    code.emit('stop_confirmed', 'coord', { taskId: 'T1', attemptId: attempt.attemptId, stopReceiptId: `stopped-code-${attemptNo}` });
  }
  assert.equal(code.state.tasks.T1.codeAttempts, 3);
  assert.equal(code.state.status, 'blocked');
  assert.equal(code.state.tasks.T1.status, 'blocked');
  const fourth = { id: 'attempt-four', type: 'dispatch', expectedVersion: code.state.version, actorSessionId: 'coord', payload: { taskId: 'T1' } };
  assertRejectedWithoutMutation(code, fourth, 'RUN_BLOCKED');

  const infra = harness();
  for (let attemptNo = 1; attemptNo <= 2; attemptNo += 1) {
    infra.emit('dispatch', 'coord', { taskId: 'T1' });
    const attempt = infra.state.tasks.T1.attempts.at(-1);
    infra.register(`worker-infra-${attemptNo}`, 'worker');
    infra.emit('bind_worker', 'coord', { taskId: 'T1', attemptId: attempt.attemptId, sessionId: `worker-infra-${attemptNo}` });
    infra.emit('fail', `worker-infra-${attemptNo}`, { taskId: 'T1', attemptId: attempt.attemptId, category: 'infra', fingerprint: `WF-AC-08/infra-${attemptNo}` });
    infra.emit('stop_confirmed', 'coord', { taskId: 'T1', attemptId: attempt.attemptId, stopReceiptId: `stopped-infra-${attemptNo}` });
  }
  assert.equal(infra.state.tasks.T1.infraRetries, 1);
  assert.equal(infra.state.status, 'blocked');
});

test('repeated same-source failures ignore cosmetic artifacts, and trusted time cannot roll back', () => {
  // A failed verifier records source identity only; changing logs or artifacts is not progress.
  const failedHarness = harness();
  const setup = prepareHandoff(failedHarness);
  failedHarness.emit('submit_handoff', 'worker-1', { packet: setup.packet });
  failedHarness.emit('ack_handoff', 'verify-1', { taskId: 'T1', packetId: setup.packet.packetId });
  failedHarness.emit('task_verdict', 'verify-1', { taskId: 'T1', packetId: setup.packet.packetId, verdict: 'fail', findings: [] });
  failedHarness.emit('stop_confirmed', 'coord', { taskId: 'T1', attemptId: setup.attempt.attemptId, stopReceiptId: 'stop-failed-1' });
  failedHarness.emit('dispatch', 'coord', { taskId: 'T1' });
  const attempt = failedHarness.state.tasks.T1.attempts.at(-1);
  failedHarness.register('worker-2', 'worker');
  failedHarness.register('verify-2', 'verify');
  failedHarness.emit('bind_worker', 'coord', { taskId: 'T1', attemptId: attempt.attemptId, sessionId: 'worker-2' });
  failedHarness.emit('command_reserved', 'coord', { taskId: 'T1', attemptId: attempt.attemptId, fence: attempt.fence, commandRunId: 'run-check-2', commandId: 'check' });
  const receipt = { ...setup.packet.commands[0], commandRunId: undefined };
  delete receipt.commandRunId;
  failedHarness.emit('command_completed', 'coord', { commandRunId: 'run-check-2', receipt });
  const packet2 = structuredClone(setup.packet);
  packet2.packetId = 'packet-2';
  packet2.attemptId = attempt.attemptId;
  packet2.fence = attempt.fence;
  packet2.senderSessionId = 'worker-2';
  packet2.receiverSessionId = 'verify-2';
  packet2.commands = [{ commandRunId: 'run-check-2', ...receipt }];
  packet2.artifacts = [{ path: 'log.txt', sha256: sha('8'), size: 5 }];
  failedHarness.emit('submit_handoff', 'worker-2', { packet: packet2 });
  failedHarness.emit('ack_handoff', 'verify-2', { taskId: 'T1', packetId: 'packet-2' });
  failedHarness.emit('task_verdict', 'verify-2', { taskId: 'T1', packetId: 'packet-2', verdict: 'fail', findings: [] });
  assert.equal(failedHarness.state.tasks.T1.noProgressCount, 2);
  assert.equal(failedHarness.state.status, 'blocked');

  const clock = harness();
  clock.emit('register_session', 'coord', { sessionId: 'worker-1', role: 'worker', model: 'gpt-6-luna', effort: 'max', providerReceiptId: 'provider-worker-1' }, 105);
  const rollback = { id: 'time-backward', type: 'dispatch', expectedVersion: clock.state.version, actorSessionId: 'coord', payload: { taskId: 'T1' } };
  assertRejectedWithoutMutation(clock, rollback, 'CLOCK_ROLLBACK', 104);
});

test('rejected candidates count source progress and keep handoff finding identity stable across reasons', () => {
  const h = harness();
  const first = prepareHandoff(h);
  h.emit('submit_handoff', first.packet.senderSessionId, { packet: first.packet });
  h.emit('reject_handoff', 'verify-1', { taskId: 'T1', packetId: first.packet.packetId, reason: 'missing evidence' });
  assert.equal(h.state.tasks.T1.noProgressCount, 1);
  assert.equal(h.state.tasks.T1.findingCounts['handoff/t1/rejected'], 1);
  assert.equal(h.state.receipts.packets[first.packet.packetId].rejectionReason, 'missing evidence');
  h.emit('stop_confirmed', 'coord', { taskId: 'T1', attemptId: first.attempt.attemptId, stopReceiptId: 'stop-rejected-1' });

  const second = prepareHandoff(h, {
    workerId: 'worker-2', verifyId: 'verify-2', packetId: 'packet-2',
    commandRunId: 'run-check-2', sourceDigest: first.packet.candidate.sourceDigest,
  });
  h.emit('submit_handoff', 'worker-2', { packet: second.packet });
  h.emit('reject_handoff', 'verify-2', { taskId: 'T1', packetId: second.packet.packetId, reason: 'please attach proof' });
  assert.equal(h.state.tasks.T1.noProgressCount, 2);
  assert.equal(h.state.tasks.T1.findingCounts['handoff/t1/rejected'], 2);
  assert.equal(h.state.receipts.packets[second.packet.packetId].rejectionReason, 'please attach proof');
  assert.equal(h.state.status, 'blocked');
  assert.equal(h.state.tasks.T1.status, 'blocked');
  const third = { id: 'dispatch-after-rejected-no-progress', type: 'dispatch', expectedVersion: h.state.version, actorSessionId: 'coord', payload: { taskId: 'T1' } };
  assertRejectedWithoutMutation(h, third, 'RUN_BLOCKED');
});

test('same-source rejection followed by failed verdict reaches the two-candidate stop', () => {
  const h = harness();
  const first = prepareHandoff(h);
  h.emit('submit_handoff', 'worker-1', { packet: first.packet });
  h.emit('reject_handoff', 'verify-1', { taskId: 'T1', packetId: first.packet.packetId, reason: 'packet rejected' });
  h.emit('stop_confirmed', 'coord', { taskId: 'T1', attemptId: first.attempt.attemptId, stopReceiptId: 'stop-mixed-1' });

  const second = prepareHandoff(h, {
    workerId: 'worker-2', verifyId: 'verify-2', packetId: 'packet-2',
    commandRunId: 'run-check-2', sourceDigest: first.packet.candidate.sourceDigest,
  });
  h.emit('submit_handoff', 'worker-2', { packet: second.packet });
  h.emit('ack_handoff', 'verify-2', { taskId: 'T1', packetId: second.packet.packetId });
  h.emit('task_verdict', 'verify-2', { taskId: 'T1', packetId: second.packet.packetId, verdict: 'fail', findings: [] });
  assert.equal(h.state.tasks.T1.noProgressCount, 2);
  assert.equal(h.state.status, 'blocked');
  assert.equal(h.state.tasks.T1.findingCounts['handoff/t1/rejected'], 1);
});
