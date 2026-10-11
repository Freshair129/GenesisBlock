import assert from 'node:assert/strict';
import { spawn, spawnSync } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import { mkdtemp, mkdir, readFile, readdir, rm, stat, symlink, utimes, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { RunStore, assertTrackedWorktreeBytes, captureCandidate, decodeTrackedPath, resolveExistingParents } from '../scripts/workflow-guard/store.mjs';

const ROOT = path.resolve(import.meta.dirname, '..');
const CLI = path.join(ROOT, 'scripts', 'workflow-guard.mjs');

function git(workspace, args) {
  const result = spawnSync('git', ['-C', workspace, ...args], { encoding: 'utf8', windowsHide: true, shell: false });
  if (result.status !== 0) throw new Error(`git ${args.join(' ')} failed: ${result.stderr}`);
  return result.stdout.trim();
}

function cliSync(args, extraEnv = {}) {
  const result = spawnSync(process.execPath, [CLI, ...args], {
    encoding: 'utf8',
    windowsHide: true,
    shell: false,
    env: { ...process.env, ...extraEnv },
    timeout: 30_000,
  });
  let json;
  try { json = JSON.parse(result.stdout.trim()); }
  catch { throw new Error(`CLI returned no JSON (status ${result.status}): ${result.stdout}\n${result.stderr}`); }
  return { ...result, json };
}

function cliAsync(args) {
  return new Promise((resolve, reject) => {
    const child = spawn(process.execPath, [CLI, ...args], { windowsHide: true, shell: false });
    let stdout = '';
    let stderr = '';
    child.stdout.setEncoding('utf8').on('data', (chunk) => { stdout += chunk; });
    child.stderr.setEncoding('utf8').on('data', (chunk) => { stderr += chunk; });
    child.once('error', reject);
    child.once('close', (status) => {
      try { resolve({ status, json: JSON.parse(stdout.trim()), stderr }); }
      catch { reject(new Error(`CLI returned no JSON (status ${status}): ${stdout}\n${stderr}`)); }
    });
  });
}

function sha256(bytes) {
  return createHash('sha256').update(bytes).digest('hex');
}

function planFor(baselineSha, command = { executable: process.execPath, args: ['check.mjs'], cwd: '.', timeoutMs: 5_000 }) {
  return {
    schemaVersion: 1,
    id: 'runtime-pilot',
    revision: 1,
    baselineSha,
    authorization: { accepted: true, dispatchAuthorized: true, scope: 'synthetic-pilot', decisionId: 'runtime-test-owner' },
    roles: {
      worker: { model: 'gpt-6-luna', effort: 'max' },
      verify: { model: 'gpt-6-luna', effort: 'max' },
      review: { model: 'gpt-6-luna', effort: 'max' },
      audit: { model: 'gpt-6.1-sol', effort: 'high' },
      integration: { model: 'gpt-6.1-sol', effort: 'high' },
      orchestrator: { model: 'gpt-6.1-sol', effort: 'high' },
      final: { model: 'gpt-6.1-sol', effort: 'high' },
    },
    decisions: [],
    tasks: [{
      id: 'T1', role: 'worker', dependsOn: [], requiredDecisions: [],
      writePaths: ['pilot/input.txt'], requiredAc: ['AC-1'], timeoutMs: 30_000,
      leaseMs: 20_000, maxCommandRuns: 4, kind: 'normal',
    }],
    graph: { nodes: ['T1'], edges: [] },
    acceptance: [{
      id: 'AC-1', requirementId: 'WF-AC-05', taskId: 'T1', caseId: 'positive',
      expected: 'pass', required: true, commandId: 'check',
    }],
    commands: { check: command },
    policy: { maxAttempts: 3, infraRetries: 1, repeatLimit: 2 },
  };
}

function twoTaskPlanFor(baselineSha, command) {
  const plan = planFor(baselineSha, command);
  plan.id = 'runtime-parallel-pilot';
  plan.tasks = [
    { ...plan.tasks[0], id: 'T1', writePaths: ['pilot/a.txt'], requiredAc: ['AC-1'] },
    { ...plan.tasks[0], id: 'T2', writePaths: ['pilot/b.txt'], requiredAc: ['AC-2'] },
  ];
  plan.graph = { nodes: ['T1', 'T2'], edges: [] };
  plan.acceptance = [
    { ...plan.acceptance[0], id: 'AC-1', taskId: 'T1' },
    { ...plan.acceptance[0], id: 'AC-2', taskId: 'T2', caseId: 'positive-2' },
  ];
  return plan;
}

async function fixture({ command, checkSource, planFactory, separateTaskWorkspaces = false, crlfCheckout = false, crlfNormalized = false, baselineFiles = {} } = {}) {
  const temporary = await mkdtemp(path.join(os.tmpdir(), 'workflow-guard-runtime-'));
  const workspace = path.join(temporary, 'candidate');
  const runDir = path.join(temporary, 'ledger');
  const requests = path.join(temporary, 'requests');
  await mkdir(workspace);
  await mkdir(requests);
  await mkdir(path.join(workspace, 'pilot'));
  await writeFile(path.join(workspace, 'pilot', 'input.txt'), crlfCheckout ? 'baseline\r\n' : 'baseline\n');
  if (crlfCheckout) await writeFile(path.join(workspace, '.gitattributes'), 'pilot/*.txt -text\n');
  if (crlfNormalized) await writeFile(path.join(workspace, '.gitattributes'), 'pilot/*.txt text eol=crlf\n');
  await writeFile(path.join(workspace, 'check.mjs'), checkSource ?? "console.log('WORKFLOW_GUARD_RESULT='+JSON.stringify({acResults:[{acId:'AC-1',caseId:'positive',outcome:'pass',testCount:1}]}));\n");
  for (const [relativePath, contents] of Object.entries(baselineFiles)) {
    await writeFile(path.join(workspace, relativePath), contents);
  }
  git(workspace, ['init', '-b', 'main']);
  git(workspace, ['config', 'user.name', 'Workflow Guard Pilot']);
  git(workspace, ['config', 'user.email', 'workflow-guard@example.invalid']);
  git(workspace, ['add', '--all']);
  git(workspace, ['commit', '-m', 'baseline']);
  const baselineSha = git(workspace, ['rev-parse', 'HEAD']);
  if (crlfNormalized) {
    const trackedFile = path.join(workspace, 'pilot', 'input.txt');
    await writeFile(trackedFile, (await readFile(trackedFile, 'utf8')).replaceAll('\n', '\r\n'));
    // Refresh the index stat after Git's built-in CRLF clean conversion. The
    // committed blob stays LF, while this is now a genuine clean CRLF checkout.
    git(workspace, ['add', '--renormalize', '.']);
    assert.equal(git(workspace, ['status', '--porcelain']).length, 0, 'normalized CRLF checkout must be clean');
    assert.ok((await readFile(trackedFile)).includes(Buffer.from('\r\n')));
  }
  const plan = planFactory ? planFactory(baselineSha, command) : planFor(baselineSha, command);
  let taskWorkspaces = Object.create(null);
  if (separateTaskWorkspaces) {
    for (const task of plan.tasks) {
      const taskWorkspace = path.join(temporary, `worktree-${task.id}`);
      git(workspace, ['worktree', 'add', '--detach', taskWorkspace, baselineSha]);
      taskWorkspaces[task.id] = taskWorkspace;
    }
  }
  const planPath = path.join(temporary, 'plan.json');
  await writeFile(planPath, `${JSON.stringify(plan, null, 2)}\n`);
  const initArgs = [
    'init', '--plan', planPath, '--run-dir', runDir, '--workspace', workspace,
    '--run-id', 'runtime-run', '--coordinator-session-id', 'coord',
  ];
  if (separateTaskWorkspaces) {
    const mappingFile = path.join(temporary, 'task-workspaces.json');
    await writeFile(mappingFile, JSON.stringify(taskWorkspaces));
    initArgs.push('--task-workspaces', mappingFile);
  }
  const initialized = cliSync(initArgs);
  assert.equal(initialized.status, 0, initialized.json.message);
  return { temporary, workspace, runDir, requests, planPath, plan, baselineSha, taskWorkspaces, state: initialized.json.state, sequence: 0 };
}

async function event(project, type, actorSessionId, payload, extra = {}) {
  const input = {
    id: extra.id ?? randomUUID(),
    type,
    expectedVersion: extra.expectedVersion ?? project.state.version,
    actorSessionId,
    payload,
  };
  const file = path.join(project.requests, `${randomUUID()}.json`);
  await writeFile(file, JSON.stringify(input));
  const result = cliSync(['event', '--run-dir', project.runDir, '--event', file], extra.env ?? {});
  if (result.status === 0) project.state = result.json.state;
  return { ...result, event: input };
}

async function register(project, sessionId, role, model, effort) {
  return event(project, 'register_session', 'coord', {
    sessionId,
    role,
    model,
    effort,
    providerReceiptId: `provider-${sessionId}`,
  });
}

async function startAttempt(project, taskId = 'T1', workerSession = 'worker-1', verifySession = 'verify-1') {
  const worker = await register(project, workerSession, 'worker', 'gpt-6-luna', 'max');
  assert.equal(worker.status, 0, worker.json.message);
  const verify = await register(project, verifySession, 'verify', 'gpt-6-luna', 'max');
  assert.equal(verify.status, 0, verify.json.message);
  const dispatched = await event(project, 'dispatch', 'coord', { taskId });
  assert.equal(dispatched.status, 0, dispatched.json.message);
  const request = dispatched.json.dispatchRequest;
  assert.equal(request.taskId, taskId);
  const bound = await event(project, 'bind_worker', 'coord', {
    taskId, attemptId: request.attemptId, sessionId: workerSession,
  });
  assert.equal(bound.status, 0, bound.json.message);
  return request;
}

async function commitTaskChange(project) {
  await writeFile(path.join(project.workspace, 'pilot', 'input.txt'), 'task candidate\n');
  git(project.workspace, ['add', '--all']);
  git(project.workspace, ['commit', '-m', 'task candidate']);
}

async function runCommand(project, request, taskId = 'T1') {
  const result = cliSync([
    'command', '--run-dir', project.runDir, '--task', taskId,
    '--attempt', request.attemptId, '--command', 'check',
  ], { WORKFLOW_TASK_ID: taskId });
  if (result.status === 0) project.state = result.json.state;
  return result;
}

async function runGateReport(project, report, extraEnv = {}) {
  const file = path.join(project.requests, `${randomUUID()}.gate.json`);
  await writeFile(file, JSON.stringify(report));
  const result = cliSync(['gate-report', '--run-dir', project.runDir, '--report', file], extraEnv);
  if (result.json.state) project.state = result.json.state;
  return result;
}

async function prepareOneTask(project) {
  const request = await startAttempt(project);
  await commitTaskChange(project);
  const commandResult = await runCommand(project, request);
  assert.equal(commandResult.status, 0, commandResult.json.message);
  const packet = packetFrom(project, request, commandResult);
  assert.equal((await event(project, 'submit_handoff', 'worker-1', { packet })).status, 0);
  assert.equal((await event(project, 'ack_handoff', 'verify-1', { taskId: 'T1', packetId: packet.packetId })).status, 0);
  assert.equal((await event(project, 'task_verdict', 'verify-1', { taskId: 'T1', packetId: packet.packetId, verdict: 'pass', findings: [] })).status, 0);
  for (const [sessionId, role, model, effort] of [
    ['integration-1', 'integration', 'gpt-6.1-sol', 'high'],
    ['review-1', 'review', 'gpt-6-luna', 'max'],
    ['audit-1', 'audit', 'gpt-6.1-sol', 'high'],
    ['final-1', 'final', 'gpt-6.1-sol', 'high'],
  ]) {
    const registered = await register(project, sessionId, role, model, effort);
    assert.equal(registered.status, 0, `${sessionId}: ${registered.json.message}`);
  }
  return { request, packet };
}

async function integrateOneTask(project) {
  const { request, packet } = await prepareOneTask(project);
  const candidate = cliSync(['show', '--run-dir', project.runDir]).json.candidate;
  const integrated = await event(project, 'integrate', 'integration-1', {
    candidate,
    taskCommits: [{ taskId: 'T1', sha: packet.candidate.sha }],
    groups: [], artifactDigests: [],
  });
  assert.equal(integrated.status, 0, integrated.json.message);
  return { request, packet, candidate };
}

function packetFrom(project, request, commandResult, {
  taskId = 'T1', senderSessionId = 'worker-1', receiverSessionId = 'verify-1', changedPaths = ['pilot/input.txt'],
} = {}) {
  const command = { commandRunId: commandResult.json.commandRunId, ...commandResult.json.receipt };
  const current = cliSync(['show', '--run-dir', project.runDir]);
  assert.equal(current.status, 0, current.json.message);
  const candidate = current.json.candidate;
  return {
    packetId: `packet-${randomUUID()}`,
    runId: 'runtime-run',
    planDigest: project.state.planDigest,
    taskId,
    attemptId: request.attemptId,
    fence: request.fence,
    senderSessionId,
    receiverSessionId,
    candidate,
    changedPaths,
    artifacts: [],
    commands: [command],
    acResults: command.caseResults.map((row) => ({ ...row, evidenceDigest: command.stdoutDigest })),
    findings: [],
  };
}

test('durable CLI completes the synthetic handoff and replays accepted state after restart', async (t) => {
  const project = await fixture();
  t.after(() => rm(project.temporary, { recursive: true, force: true }));
  const request = await startAttempt(project);
  await commitTaskChange(project);
  const commandResult = await runCommand(project, request);
  assert.equal(commandResult.status, 0, commandResult.json.message);
  assert.equal(commandResult.json.receipt.testCount, 1);
  assert.equal(commandResult.json.receipt.caseResults[0].acId, 'AC-1');
  const packet = packetFrom(project, request, commandResult);
  const submitted = await event(project, 'submit_handoff', 'worker-1', { packet });
  assert.equal(submitted.status, 0, submitted.json.message);
  const ack = await event(project, 'ack_handoff', 'verify-1', { taskId: 'T1', packetId: packet.packetId });
  assert.equal(ack.status, 0, ack.json.message);
  const verdict = await event(project, 'task_verdict', 'verify-1', { taskId: 'T1', packetId: packet.packetId, verdict: 'pass', findings: [] });
  assert.equal(verdict.status, 0, verdict.json.message);
  assert.equal(verdict.json.state.tasks.T1.status, 'verified');

  const replayed = cliSync(['show', '--run-dir', project.runDir]);
  assert.equal(replayed.status, 0, replayed.json.message);
  assert.equal(replayed.json.state.version, project.state.version);
  assert.equal(replayed.json.state.tasks.T1.status, 'verified');
  assert.equal(replayed.json.sequence, 9);
});

test('concurrent dispatch writers serialize to one attempt', async (t) => {
  const project = await fixture();
  t.after(() => rm(project.temporary, { recursive: true, force: true }));
  const firstPath = path.join(project.requests, 'dispatch-a.json');
  const secondPath = path.join(project.requests, 'dispatch-b.json');
  const baseEvent = { type: 'dispatch', expectedVersion: 0, actorSessionId: 'coord', payload: { taskId: 'T1' } };
  await writeFile(firstPath, JSON.stringify({ ...baseEvent, id: 'dispatch-a' }));
  await writeFile(secondPath, JSON.stringify({ ...baseEvent, id: 'dispatch-b' }));
  const [first, second] = await Promise.all([
    cliAsync(['event', '--run-dir', project.runDir, '--event', firstPath]),
    cliAsync(['event', '--run-dir', project.runDir, '--event', secondPath]),
  ]);
  assert.equal([first, second].filter((result) => result.status === 0).length, 1);
  const current = cliSync(['show', '--run-dir', project.runDir]);
  assert.equal(current.status, 0, current.json.message);
  assert.equal(current.json.state.tasks.T1.attemptCount, 1);
  assert.equal(current.json.state.version, 1);
});

test('generic event input cannot reserve or forge gate reports and help lists bounded operations', async (t) => {
  const project = await fixture();
  t.after(() => rm(project.temporary, { recursive: true, force: true }));
  const help = spawnSync(process.execPath, [CLI, '--help'], { encoding: 'utf8', windowsHide: true, shell: false });
  assert.equal(help.status, 0);
  assert.match(help.stdout, /gate-report --run-dir/);
  for (const type of ['gate_report_reserved', 'gate']) {
    const result = await event(project, type, 'verify-1', {});
    assert.equal(result.status, 1);
    assert.equal(result.json.code, 'GATE_REPORT_REQUIRED');
    assert.equal(project.state.version, 0);
  }
});

test('parallel tasks dispatch into distinct pinned worktrees and shared-root writers serialize fail-closed', async (t) => {
  const separate = await fixture({
    planFactory: twoTaskPlanFor,
    separateTaskWorkspaces: true,
  });
  t.after(() => rm(separate.temporary, { recursive: true, force: true }));
  assert.notEqual(separate.taskWorkspaces.T1, separate.taskWorkspaces.T2);
  for (const sessionId of ['worker-1', 'worker-2']) {
    const registered = await register(separate, sessionId, 'worker', 'gpt-6-luna', 'max');
    assert.equal(registered.status, 0, registered.json.message);
  }
  for (const [taskId, sessionId] of [['T1', 'worker-1'], ['T2', 'worker-2']]) {
    const dispatched = await event(separate, 'dispatch', 'coord', { taskId });
    assert.equal(dispatched.status, 0, dispatched.json.message);
    assert.equal((await event(separate, 'bind_worker', 'coord', {
      taskId, attemptId: dispatched.json.dispatchRequest.attemptId, sessionId,
    })).status, 0);
  }
  assert.equal(Object.keys(separate.state.dispatchRequests).length, 2);
  assert.equal(separate.state.tasks.T1.status, 'running');
  assert.equal(separate.state.tasks.T2.status, 'running');

  const shared = await fixture({ planFactory: twoTaskPlanFor });
  t.after(() => rm(shared.temporary, { recursive: true, force: true }));
  for (const sessionId of ['worker-1', 'worker-2']) {
    const registered = await register(shared, sessionId, 'worker', 'gpt-6-luna', 'max');
    assert.equal(registered.status, 0, registered.json.message);
  }
  const first = await event(shared, 'dispatch', 'coord', { taskId: 'T1' });
  assert.equal(first.status, 0, first.json.message);
  assert.equal((await event(shared, 'bind_worker', 'coord', {
    taskId: 'T1', attemptId: first.json.dispatchRequest.attemptId, sessionId: 'worker-1',
  })).status, 0);
  const second = await event(shared, 'dispatch', 'coord', { taskId: 'T2' });
  assert.equal(second.status, 1);
  assert.equal(second.json.code, 'WORKSPACE_BUSY');
  assert.equal(shared.state.tasks.T2.status, 'ready');
  assert.equal(shared.state.tasks.T2.currentAttemptId, null);
  assert.equal(Object.values(shared.state.dispatchRequests).filter((request) => request.taskId === 'T2').length, 0);
});

test('sequential tasks reuse the default shared workspace through integration and all four gates', async (t) => {
  const checkSource = [
    "const rows = process.env.GATE_MODE",
    "  ? [{acId:'AC-1',caseId:'positive',outcome:'pass',testCount:1},{acId:'AC-2',caseId:'positive-2',outcome:'pass',testCount:1}]",
    "  : process.env.WORKFLOW_TASK_ID === 'T2' ? [{acId:'AC-2',caseId:'positive-2',outcome:'pass',testCount:1}] : [{acId:'AC-1',caseId:'positive',outcome:'pass',testCount:1}];",
    "console.log('WORKFLOW_GUARD_RESULT='+JSON.stringify({acResults:rows}));",
    '',
  ].join('\n');
  const project = await fixture({ planFactory: twoTaskPlanFor, checkSource });
  t.after(() => rm(project.temporary, { recursive: true, force: true }));

  const request1 = await startAttempt(project, 'T1', 'worker-1', 'verify-1');
  await writeFile(path.join(project.workspace, 'pilot', 'a.txt'), 'task one candidate\n');
  git(project.workspace, ['add', '--all']);
  git(project.workspace, ['commit', '-m', 'task T1 candidate']);
  const command1 = await runCommand(project, request1, 'T1');
  assert.equal(command1.status, 0, command1.json.message);
  assert.equal(command1.json.receipt.caseResults[0].acId, 'AC-1');
  const packet1 = packetFrom(project, request1, command1, {
    taskId: 'T1', senderSessionId: 'worker-1', receiverSessionId: 'verify-1', changedPaths: ['pilot/a.txt'],
  });
  assert.equal((await event(project, 'submit_handoff', 'worker-1', { packet: packet1 })).status, 0);
  assert.equal((await event(project, 'ack_handoff', 'verify-1', { taskId: 'T1', packetId: packet1.packetId })).status, 0);
  assert.equal((await event(project, 'task_verdict', 'verify-1', { taskId: 'T1', packetId: packet1.packetId, verdict: 'pass', findings: [] })).status, 0);
  const commitA = packet1.candidate.sha;

  const request2 = await startAttempt(project, 'T2', 'worker-2', 'verify-2');
  await writeFile(path.join(project.workspace, 'pilot', 'b.txt'), 'task two candidate\n');
  git(project.workspace, ['add', '--all']);
  git(project.workspace, ['commit', '-m', 'task T2 candidate']);
  const command2 = await runCommand(project, request2, 'T2');
  assert.equal(command2.status, 0, command2.json.message);
  assert.equal(command2.json.receipt.caseResults[0].acId, 'AC-2');
  const packet2 = packetFrom(project, request2, command2, {
    taskId: 'T2', senderSessionId: 'worker-2', receiverSessionId: 'verify-2', changedPaths: ['pilot/b.txt'],
  });
  assert.equal((await event(project, 'submit_handoff', 'worker-2', { packet: packet2 })).status, 0);
  assert.equal((await event(project, 'ack_handoff', 'verify-2', { taskId: 'T2', packetId: packet2.packetId })).status, 0);
  assert.equal((await event(project, 'task_verdict', 'verify-2', { taskId: 'T2', packetId: packet2.packetId, verdict: 'pass', findings: [] })).status, 0);
  const commitB = packet2.candidate.sha;
  assert.notEqual(commitA, commitB);
  assert.equal(spawnSync('git', ['-C', project.workspace, 'merge-base', '--is-ancestor', commitA, commitB], {
    encoding: 'utf8', windowsHide: true, shell: false,
  }).status, 0);

  for (const [sessionId, role, model, effort] of [
    ['integration-1', 'integration', 'gpt-6.1-sol', 'high'],
    ['verify-gate', 'verify', 'gpt-6-luna', 'max'],
    ['review-1', 'review', 'gpt-6-luna', 'max'],
    ['audit-1', 'audit', 'gpt-6.1-sol', 'high'],
    ['final-1', 'final', 'gpt-6.1-sol', 'high'],
  ]) {
    const registered = await register(project, sessionId, role, model, effort);
    assert.equal(registered.status, 0, `${sessionId}: ${registered.json.message}`);
  }
  const candidate = cliSync(['show', '--run-dir', project.runDir]).json.candidate;
  assert.equal(candidate.sha, commitB);
  const integrated = await event(project, 'integrate', 'integration-1', {
    candidate,
    taskCommits: [{ taskId: 'T1', sha: commitA }, { taskId: 'T2', sha: commitB }],
    groups: [], artifactDigests: [],
  });
  assert.equal(integrated.status, 0, integrated.json.message);

  const verify = await runGateReport(project, {
    stage: 'verify', sessionId: 'verify-gate', verdict: 'pass', findings: [],
    assessment: 'Both sequential task acceptance commands passed on the integrated candidate.', commandIds: ['check'],
  }, { GATE_MODE: 'all' });
  assert.equal(verify.status, 0, verify.json.message);
  assert.equal(verify.json.state.receipts.gates.verify.candidateDigest, verify.json.state.receipts.integration.candidateDigest);
  const review = await runGateReport(project, {
    stage: 'review', sessionId: 'review-1', verdict: 'pass', findings: [],
    assessment: 'The integrated descendant retains both accepted task commits.', commandIds: [],
  });
  assert.equal(review.status, 0, review.json.message);
  const audit = await runGateReport(project, {
    stage: 'audit', sessionId: 'audit-1', verdict: 'pass', findings: [],
    assessment: 'Historical packet hashes and stored command evidence are complete.', commandIds: [],
  });
  assert.equal(audit.status, 0, audit.json.message);
  const final = await runGateReport(project, {
    stage: 'final', sessionId: 'final-1', verdict: 'pass', findings: [],
    assessment: 'All independent gates passed on the frozen integrated candidate.', commandIds: [],
  });
  assert.equal(final.status, 0, final.json.message);
  assert.equal(final.json.state.status, 'done');
  assert.equal(final.json.state.tasks.T1.acceptedPacketId, packet1.packetId);
  assert.equal(final.json.state.tasks.T2.acceptedPacketId, packet2.packetId);
  assert.equal(final.json.state.candidate.sha, commitB);
});

test('expired retry rejection durably records watchdog closure after stop proof', async (t) => {
  const project = await fixture({
    planFactory: (baselineSha, command) => {
      const plan = planFor(baselineSha, command);
      plan.tasks[0].timeoutMs = 60_000;
      return plan;
    },
  });
  t.after(() => rm(project.temporary, { recursive: true, force: true }));
  const request = await startAttempt(project);
  const original = project.state.tasks.T1;
  const deadline = original.deadline;
  const dispatchedCount = Object.keys(project.state.dispatchRequests).length;
  const failed = await event(project, 'fail', 'worker-1', {
    taskId: 'T1', attemptId: request.attemptId, category: 'process-loss', fingerprint: 'WF-AC-08/stopped-worker',
  });
  assert.equal(failed.status, 0, failed.json.message);
  const stopped = await event(project, 'stop_confirmed', 'coord', {
    taskId: 'T1', attemptId: request.attemptId, stopReceiptId: 'provider-stop-confirmed',
  });
  assert.equal(stopped.status, 0, stopped.json.message);
  assert.equal(project.state.status, 'active');
  assert.equal(project.state.tasks.T1.status, 'rework');
  assert.equal(project.state.tasks.T1.currentAttemptId, null);
  assert.equal(project.state.tasks.T1.attempts[0].status, 'stopped');
  assert.equal(project.state.tasks.T1.attempts[0].stopReceiptId, 'provider-stop-confirmed');
  assert.equal(Object.keys(project.state.locks).length, 0);

  const store = await RunStore.open(project.runDir);
  const expiredDispatch = {
    id: 'expired-retry-dispatch',
    type: 'dispatch',
    expectedVersion: project.state.version,
    actorSessionId: 'coord',
    payload: { taskId: 'T1' },
  };
  await assert.rejects(
    store.processEvent(expiredDispatch, { now: deadline }),
    (error) => error.code === 'TASK_DEADLINE_EXHAUSTED',
  );
  const replayed = cliSync(['show', '--run-dir', project.runDir]);
  assert.equal(replayed.status, 0, replayed.json.message);
  assert.equal(replayed.json.state.status, 'blocked');
  assert.equal(replayed.json.state.tasks.T1.status, 'blocked');
  assert.equal(replayed.json.state.tasks.T1.deadline, deadline);
  assert.equal(replayed.json.state.tasks.T1.currentAttemptId, null);
  assert.equal(replayed.json.state.tasks.T1.attempts.length, 1);
  assert.equal(Object.keys(replayed.json.state.dispatchRequests).length, dispatchedCount);
  assert.equal(Object.keys(replayed.json.state.locks).length, 0);

  const eventNames = (await readdir(path.join(project.runDir, 'events'))).filter((name) => /^\d{12}\.json$/.test(name)).sort();
  const lastRecord = JSON.parse(await readFile(path.join(project.runDir, 'events', eventNames.at(-1)), 'utf8'));
  assert.equal(lastRecord.event.type, 'watchdog');
  const blockedDispatch = { ...expiredDispatch, id: 'blocked-retry-dispatch', expectedVersion: replayed.json.state.version };
  await assert.rejects(store.processEvent(blockedDispatch, { now: deadline + 1 }), (error) => error.code === 'RUN_BLOCKED');
  const unchanged = cliSync(['show', '--run-dir', project.runDir]);
  assert.equal(unchanged.status, 0, unchanged.json.message);
  assert.equal(unchanged.json.state.version, replayed.json.state.version);
  assert.equal(unchanged.json.state.tasks.T1.codeAttempts, original.codeAttempts);
});

test('watchdog and stop-proof replay preserve owner-aborted run identity', async (t) => {
  const project = await fixture({
    planFactory: (baselineSha, command) => {
      const plan = planFor(baselineSha, command);
      plan.tasks[0].timeoutMs = 60_000;
      return plan;
    },
  });
  t.after(() => rm(project.temporary, { recursive: true, force: true }));
  const request = await startAttempt(project);
  const deadline = project.state.tasks.T1.deadline;
  const aborted = await event(project, 'abort', 'coord', { reason: 'owner cancelled synthetic run' });
  assert.equal(aborted.status, 0, aborted.json.message);
  const abortReceipt = structuredClone(project.state.receipts.abort);
  assert.equal(project.state.status, 'aborted');
  assert.equal(project.state.tasks.T1.attempts[0].status, 'quarantined');
  assert.equal(project.state.locks['pilot/input.txt'].quarantined, true);

  const store = await RunStore.open(project.runDir);
  const watchdog = await store.processEvent({
    id: 'watchdog-after-owner-abort', type: 'watchdog', expectedVersion: project.state.version,
    actorSessionId: 'coord', payload: {},
  }, { now: deadline });
  assert.equal(watchdog.state.status, 'aborted');
  assert.deepEqual(watchdog.state.receipts.abort, abortReceipt);
  assert.equal(watchdog.state.tasks.T1.attempts[0].status, 'quarantined');
  assert.equal(watchdog.state.locks['pilot/input.txt'].quarantined, true);
  const replayed = cliSync(['show', '--run-dir', project.runDir]);
  assert.equal(replayed.status, 0, replayed.json.message);
  assert.equal(replayed.json.state.status, 'aborted');
  assert.deepEqual(replayed.json.state.receipts.abort, abortReceipt);

  const stopped = await store.processEvent({
    id: 'stop-proof-after-owner-abort', type: 'stop_confirmed', expectedVersion: replayed.json.state.version,
    actorSessionId: 'coord', payload: { taskId: 'T1', attemptId: request.attemptId, stopReceiptId: 'provider-stop-after-abort' },
  }, { now: deadline + 1 });
  assert.equal(stopped.state.status, 'aborted');
  assert.equal(stopped.state.tasks.T1.attempts[0].status, 'stopped');
  assert.equal(stopped.state.tasks.T1.attempts[0].stopReceiptId, 'provider-stop-after-abort');
  assert.equal(Object.hasOwn(stopped.state.locks, 'pilot/input.txt'), false);
  const afterStop = cliSync(['show', '--run-dir', project.runDir]);
  assert.equal(afterStop.status, 0, afterStop.json.message);
  assert.equal(afterStop.json.state.status, 'aborted');
  assert.deepEqual(afterStop.json.state.receipts.abort, abortReceipt);

  await assert.rejects(store.processEvent({
    id: 'dispatch-after-owner-abort', type: 'dispatch', expectedVersion: afterStop.json.state.version,
    actorSessionId: 'coord', payload: { taskId: 'T1' },
  }, { now: deadline + 2 }), (error) => error.code === 'RUN_ABORTED');
  const unchanged = cliSync(['show', '--run-dir', project.runDir]);
  assert.equal(unchanged.status, 0, unchanged.json.message);
  assert.equal(unchanged.json.state.status, 'aborted');
  assert.equal(unchanged.json.state.tasks.T1.attempts.length, 1);
});

test('integration rejects an out-of-scope commit after the accepted task commit without recording integration', async (t) => {
  const project = await fixture();
  t.after(() => rm(project.temporary, { recursive: true, force: true }));
  const { packet } = await prepareOneTask(project);
  const versionBeforeIntegration = project.state.version;

  await writeFile(path.join(project.workspace, 'unauthorized.txt'), 'unapproved change\n');
  git(project.workspace, ['add', '--all']);
  git(project.workspace, ['commit', '-m', 'unrelated out-of-scope change']);
  const candidate = cliSync(['show', '--run-dir', project.runDir]).json.candidate;
  const acceptedCommitIsAncestor = spawnSync('git', ['-C', project.workspace, 'merge-base', '--is-ancestor', packet.candidate.sha, candidate.sha], {
    encoding: 'utf8', windowsHide: true, shell: false,
  });
  assert.equal(acceptedCommitIsAncestor.status, 0, 'the accepted task commit must remain an ancestor of the integration candidate');

  const rejected = await event(project, 'integrate', 'integration-1', {
    candidate,
    taskCommits: [{ taskId: 'T1', sha: packet.candidate.sha }],
    groups: [], artifactDigests: [],
  });
  assert.equal(rejected.status, 1);
  assert.equal(rejected.json.code, 'INTEGRATION_SCOPE');
  assert.equal(project.state.version, versionBeforeIntegration);
  const replayed = cliSync(['show', '--run-dir', project.runDir]);
  assert.equal(replayed.status, 0, replayed.json.message);
  assert.equal(replayed.json.state.version, versionBeforeIntegration);
  assert.equal(replayed.json.state.receipts.integration, null);
});

test('integration scope diff rejects a rename from an out-of-scope baseline path into an allowed directory', async (t) => {
  const project = await fixture({
    baselineFiles: { 'outside.txt': 'rename source content\n' },
    planFactory: (baselineSha, command) => {
      const plan = planFor(baselineSha, command);
      plan.tasks[0].writePaths = ['pilot'];
      return plan;
    },
  });
  t.after(() => rm(project.temporary, { recursive: true, force: true }));
  const { packet } = await prepareOneTask(project);

  git(project.workspace, ['mv', 'outside.txt', 'pilot/renamed.txt']);
  git(project.workspace, ['commit', '-m', 'rename an out-of-scope baseline file']);
  const candidate = cliSync(['show', '--run-dir', project.runDir]).json.candidate;
  const renameStatus = git(project.workspace, ['diff', '--find-renames', '--name-status', project.baselineSha, candidate.sha]);
  assert.match(renameStatus, /R\d+\s+outside\.txt\s+pilot\/renamed\.txt/);
  const renameCollapsedPaths = git(project.workspace, ['diff', '--name-only', project.baselineSha, candidate.sha]).split(/\r?\n/);
  assert.equal(renameCollapsedPaths.includes('outside.txt'), false, 'Git name-only with rename detection must hide the old path in this fixture');

  const versionBeforeIntegration = project.state.version;
  const rejected = await event(project, 'integrate', 'integration-1', {
    candidate,
    taskCommits: [{ taskId: 'T1', sha: packet.candidate.sha }],
    groups: [], artifactDigests: [],
  });
  assert.equal(rejected.status, 1);
  assert.equal(rejected.json.code, 'INTEGRATION_SCOPE');
  assert.equal(project.state.version, versionBeforeIntegration);
  const replayed = cliSync(['show', '--run-dir', project.runDir]);
  assert.equal(replayed.status, 0, replayed.json.message);
  assert.equal(replayed.json.state.version, versionBeforeIntegration);
  assert.equal(replayed.json.state.receipts.integration, null);
});

test('handoff scope validation sees both sides of a rename and rejects the packet without accepting it', async (t) => {
  const project = await fixture({
    baselineFiles: { 'outside.txt': 'rename source content\n' },
    planFactory: (baselineSha, command) => {
      const plan = planFor(baselineSha, command);
      plan.tasks[0].writePaths = ['pilot'];
      return plan;
    },
  });
  t.after(() => rm(project.temporary, { recursive: true, force: true }));
  const request = await startAttempt(project);
  git(project.workspace, ['mv', 'outside.txt', 'pilot/renamed.txt']);
  git(project.workspace, ['commit', '-m', 'rename an out-of-scope baseline file']);
  const commandResult = await runCommand(project, request);
  assert.equal(commandResult.status, 0, commandResult.json.message);
  const packet = packetFrom(project, request, commandResult);
  const renameStatus = git(project.workspace, ['diff', '--find-renames', '--name-status', project.baselineSha, packet.candidate.sha]);
  assert.match(renameStatus, /R\d+\s+outside\.txt\s+pilot\/renamed\.txt/);

  const versionBeforeSubmit = project.state.version;
  packet.changedPaths = ['pilot/renamed.txt'];
  const hiddenDeletion = await event(project, 'submit_handoff', 'worker-1', { packet });
  assert.equal(hiddenDeletion.status, 1);
  assert.equal(hiddenDeletion.json.code, 'CHANGED_PATHS_MISMATCH');
  assert.equal(project.state.version, versionBeforeSubmit);

  packet.changedPaths = ['outside.txt', 'pilot/renamed.txt'];
  const outsideScope = await event(project, 'submit_handoff', 'worker-1', { packet });
  assert.equal(outsideScope.status, 1);
  assert.equal(outsideScope.json.code, 'OUT_OF_SCOPE');
  assert.equal(project.state.version, versionBeforeSubmit);
  const replayed = cliSync(['show', '--run-dir', project.runDir]);
  assert.equal(replayed.status, 0, replayed.json.message);
  assert.equal(replayed.json.state.version, versionBeforeSubmit);
  assert.equal(replayed.json.state.tasks.T1.acceptedPacketId, null);
});

test('event file crashes before and after atomic rename replay the correct authority', async (t) => {
  const project = await fixture();
  t.after(() => rm(project.temporary, { recursive: true, force: true }));
  const eventPath = path.join(project.requests, 'register.json');
  await writeFile(eventPath, JSON.stringify({
    id: 'crash-register', type: 'register_session', expectedVersion: 0, actorSessionId: 'coord',
    payload: { sessionId: 'worker-crash', role: 'worker', model: 'gpt-6-luna', effort: 'max', providerReceiptId: 'provider-crash' },
  }));
  const before = spawnSync(process.execPath, [CLI, 'event', '--run-dir', project.runDir, '--event', eventPath], {
    encoding: 'utf8', windowsHide: true, shell: false, timeout: 30_000,
    env: { ...process.env, NODE_ENV: 'test', WORKFLOW_GUARD_TEST_CRASH: 'before-event-rename' },
  });
  assert.equal(before.status, 91);
  const lock = JSON.parse(await readFile(path.join(project.runDir, 'run.lock'), 'utf8'));
  const beforeReplay = cliSync(['show', '--run-dir', project.runDir, '--ack-stale-lock', lock.nonce]);
  assert.equal(beforeReplay.status, 0, beforeReplay.json.message);
  assert.equal(beforeReplay.json.state.version, 0);

  const after = spawnSync(process.execPath, [CLI, 'event', '--run-dir', project.runDir, '--event', eventPath], {
    encoding: 'utf8', windowsHide: true, shell: false, timeout: 30_000,
    env: { ...process.env, NODE_ENV: 'test', WORKFLOW_GUARD_TEST_CRASH: 'after-event-rename' },
  });
  assert.equal(after.status, 92);
  const afterLock = JSON.parse(await readFile(path.join(project.runDir, 'run.lock'), 'utf8'));
  const afterReplay = cliSync(['show', '--run-dir', project.runDir, '--ack-stale-lock', afterLock.nonce]);
  assert.equal(afterReplay.status, 0, afterReplay.json.message);
  assert.equal(afterReplay.json.state.version, 1);
  assert.equal(afterReplay.json.state.sessions['worker-crash'].role, 'worker');
});

test('ledger digest corruption is rejected without truncating or rewriting history', async (t) => {
  const project = await fixture();
  t.after(() => rm(project.temporary, { recursive: true, force: true }));
  const registered = await register(project, 'worker-1', 'worker', 'gpt-6-luna', 'max');
  assert.equal(registered.status, 0, registered.json.message);
  const recordPath = path.join(project.runDir, 'events', '000000000001.json');
  const original = await readFile(recordPath, 'utf8');
  const corrupted = original.replace(/("digest":"[a-f0-9])/, (prefix) => `${prefix.slice(0, -1)}${prefix.at(-1) === 'a' ? 'b' : 'a'}`);
  await writeFile(recordPath, corrupted);
  const result = cliSync(['show', '--run-dir', project.runDir]);
  assert.equal(result.status, 1);
  assert.equal(result.json.code, 'LEDGER_RECORD_DIGEST');
  assert.equal(await readFile(recordPath, 'utf8'), corrupted);
});

test('ledger cannot be initialized inside the source tree and cwd symlinks cannot escape it', async (t) => {
  const project = await fixture();
  t.after(() => rm(project.temporary, { recursive: true, force: true }));
  const nestedRunDir = path.join(project.workspace, '.workflow-guard-ledger');
  const rejected = cliSync([
    'init', '--plan', project.planPath, '--run-dir', nestedRunDir, '--workspace', project.workspace,
    '--run-id', 'nested-run', '--coordinator-session-id', 'coord',
  ]);
  assert.equal(rejected.status, 1);
  assert.equal(rejected.json.code, 'RUN_DIR_IN_SOURCE_TREE');
  await assert.rejects(readFile(path.join(nestedRunDir, 'manifest.json')));

  const outside = path.join(project.temporary, 'outside');
  await mkdir(outside);
  const linkPath = path.join(project.workspace, 'escape');
  await symlink(outside, linkPath, process.platform === 'win32' ? 'junction' : 'dir');
  await assert.rejects(resolveExistingParents(project.workspace, 'escape/outside.txt'), (error) => error.code === 'PATH_ESCAPE');
});

test('candidate capture rejects hidden index flags, catches metadata-preserving byte changes, and accepts CRLF normalization', async (t) => {
  for (const flag of ['--assume-unchanged', '--skip-worktree']) {
    const project = await fixture();
    t.after(() => rm(project.temporary, { recursive: true, force: true }));
    const trackedFile = path.join(project.workspace, 'pilot', 'input.txt');
    git(project.workspace, ['update-index', flag, '--', 'pilot/input.txt']);
    await writeFile(trackedFile, 'changed!\n');
    assert.equal((await readFile(trackedFile)).length, Buffer.byteLength('baseline\n'));
    assert.equal(git(project.workspace, ['status', '--porcelain']).length, 0, `${flag} fixture should hide the worktree edit from status`);
    await assert.rejects(captureCandidate(project.workspace), (error) => error.code === 'INDEX_HIDDEN_ENTRY');
  }

  const project = await fixture();
  t.after(() => rm(project.temporary, { recursive: true, force: true }));
  const trackedFile = path.join(project.workspace, 'pilot', 'input.txt');
  git(project.workspace, ['config', 'core.trustctime', 'false']);
  const roundedTime = Math.floor((await stat(trackedFile)).mtimeMs);
  await utimes(trackedFile, roundedTime / 1000, roundedTime / 1000);
  git(project.workspace, ['update-index', '--refresh']);
  const originalStat = await stat(trackedFile);
  await writeFile(trackedFile, 'changed!\n');
  await utimes(trackedFile, originalStat.atime, originalStat.mtime);
  const replacementStat = await stat(trackedFile);
  assert.equal(replacementStat.size, originalStat.size);
  assert.equal(replacementStat.mtimeMs, originalStat.mtimeMs);
  await assert.rejects(captureCandidate(project.workspace), (error) => error.code === 'TRACKED_BYTES_MISMATCH');

  const crlf = await fixture({ crlfCheckout: true });
  t.after(() => rm(crlf.temporary, { recursive: true, force: true }));
  const crlfBytes = await readFile(path.join(crlf.workspace, 'pilot', 'input.txt'));
  assert.ok(crlfBytes.includes(Buffer.from('\r\n')));
  assert.equal(git(crlf.workspace, ['status', '--porcelain']).length, 0);
  const candidate = await captureCandidate(crlf.workspace);
  assert.equal(candidate.sha, crlf.baselineSha);

  const normalized = await fixture({ crlfNormalized: true });
  t.after(() => rm(normalized.temporary, { recursive: true, force: true }));
  const normalizedBytes = await readFile(path.join(normalized.workspace, 'pilot', 'input.txt'));
  assert.ok(normalizedBytes.includes(Buffer.from('\r\n')));
  assert.equal(git(normalized.workspace, ['status', '--porcelain']).length, 0);
  const normalizedCandidate = await captureCandidate(normalized.workspace);
  assert.equal(normalizedCandidate.sha, normalized.baselineSha);
  await assertTrackedWorktreeBytes(normalized.workspace, normalized.baselineSha);
});

test('tracked path validation rejects C-quoted stdin hash ambiguity', () => {
  assert.throws(
    () => decodeTrackedPath(Buffer.from('"pilot/input.txt"', 'utf8')),
    (error) => error.code === 'TRACKED_PATH_UNSUPPORTED',
  );
});

test('handoff rejects wrong artifact hashes and changed candidates before event commit', async (t) => {
  const project = await fixture();
  t.after(() => rm(project.temporary, { recursive: true, force: true }));
  const request = await startAttempt(project);
  await commitTaskChange(project);
  const commandResult = await runCommand(project, request);
  assert.equal(commandResult.status, 0, commandResult.json.message);
  const artifactPath = path.join(project.temporary, 'evidence.bin');
  const bytes = Buffer.from('immutable evidence');
  await writeFile(artifactPath, bytes);
  const packet = packetFrom(project, request, commandResult);
  packet.artifacts = [{ path: artifactPath, sha256: '0'.repeat(64), size: bytes.length }];
  const wrongHash = await event(project, 'submit_handoff', 'worker-1', { packet });
  assert.equal(wrongHash.status, 1);
  assert.equal(wrongHash.json.code, 'ARTIFACT_HASH_MISMATCH');
  assert.equal(project.state.version, 6);

  packet.artifacts = [];
  await writeFile(path.join(project.workspace, 'pilot', 'input.txt'), 'changed after command\n');
  git(project.workspace, ['add', '--all']);
  git(project.workspace, ['commit', '-m', 'candidate changed after command']);
  const changed = await event(project, 'submit_handoff', 'worker-1', { packet });
  assert.equal(changed.status, 1);
  assert.equal(changed.json.code, 'CANDIDATE_HASH_MISMATCH');
  const replayed = cliSync(['show', '--run-dir', project.runDir]);
  assert.equal(replayed.status, 0, replayed.json.message);
  assert.equal(replayed.json.state.version, 6);
});

test('timed-out commands finish within a bound and quarantine the task writer', async (t) => {
  const project = await fixture({ command: { executable: process.execPath, args: ['-e', 'setTimeout(()=>{}, 10000)'], cwd: '.', timeoutMs: 250 } });
  t.after(() => rm(project.temporary, { recursive: true, force: true }));
  const request = await startAttempt(project);
  const start = Date.now();
  const result = await runCommand(project, request);
  assert.equal(result.status, 0, result.json.message);
  assert.equal(result.json.receipt.timedOut, true);
  assert.ok(Date.now() - start < 5_000, 'timed-out child must not hang the CLI');
  assert.equal(result.json.state.status, 'blocked');
  assert.equal(Object.values(result.json.state.locks)[0].quarantined, true);
});

test('tracked and untracked subprocess mutations fail closed', async (t) => {
  for (const [label, code] of [
    ['tracked', "require('node:fs').writeFileSync('pilot/input.txt','mutated\\n')"],
    ['untracked', "require('node:fs').writeFileSync('rogue.txt','mutated\\n')"],
  ]) {
    const project = await fixture({ command: { executable: process.execPath, args: ['-e', code], cwd: '.', timeoutMs: 3_000 } });
    t.after(() => rm(project.temporary, { recursive: true, force: true }));
    const request = await startAttempt(project);
    const result = await runCommand(project, request);
    assert.equal(result.status, 0, `${label}: ${result.json.message}`);
    assert.equal(result.json.candidateMutation, true);
    assert.equal(result.json.state.tasks.T1.status, 'rework');
    assert.equal(Object.values(result.json.state.locks)[0].quarantined, true);
  }
});

test('gate-report executes fresh integrated Verify and stores a hash-bound four-stage happy path', async (t) => {
  const project = await fixture();
  t.after(() => rm(project.temporary, { recursive: true, force: true }));
  const { candidate } = await integrateOneTask(project);

  const verify = await runGateReport(project, {
    stage: 'verify', sessionId: 'verify-1', verdict: 'pass', findings: [],
    assessment: 'Integrated command check passed on the frozen candidate.', commandIds: ['check'],
  });
  assert.equal(verify.status, 0, verify.json.message);
  assert.equal(verify.json.verdict, 'pass');
  assert.equal(project.state.receipts.gates.verify.reportDigest, verify.json.reportDigest);
  assert.equal(verify.json.evidenceDigests[0], verify.json.reportDigest);
  const reportBytes = await readFile(path.join(project.runDir, 'objects', verify.json.reportDigest));
  assert.equal(sha256(reportBytes), verify.json.reportDigest);
  assert.equal(JSON.parse(reportBytes.toString('utf8')).commandReceipts[0].candidateAfter.sha, candidate.sha);

  const review = await runGateReport(project, {
    stage: 'review', sessionId: 'review-1', verdict: 'pass', findings: [],
    assessment: 'Review found no blocking issue.', commandIds: [],
  });
  assert.equal(review.status, 0, review.json.message);
  assert.deepEqual(review.json.state.receipts.gates.review.priorEvidenceDigests, [verify.json.reportDigest]);
  const audit = await runGateReport(project, {
    stage: 'audit', sessionId: 'audit-1', verdict: 'pass', findings: [],
    assessment: 'Provenance and required evidence are complete.', commandIds: [],
  });
  assert.equal(audit.status, 0, audit.json.message);
  assert.deepEqual(audit.json.state.receipts.gates.audit.priorEvidenceDigests, [verify.json.reportDigest, review.json.reportDigest]);
  const final = await runGateReport(project, {
    stage: 'final', sessionId: 'final-1', verdict: 'pass', findings: [],
    assessment: 'Final gate passed.', commandIds: [],
  });
  assert.equal(final.status, 0, final.json.message);
  assert.equal(final.json.state.status, 'done');
  assert.deepEqual(final.json.state.receipts.gates.final.priorEvidenceDigests, [verify.json.reportDigest, review.json.reportDigest, audit.json.reportDigest]);
});

test('malformed integrated Verify is committed as a blocking FAIL and exits nonzero', async (t) => {
  const project = await fixture({ checkSource: "if (process.env.GATE_MODE) console.log('WORKFLOW_GUARD_RESULT={bad-json}'); else console.log('WORKFLOW_GUARD_RESULT='+JSON.stringify({acResults:[{acId:'AC-1',caseId:'positive',outcome:'pass',testCount:1}]}));\n" });
  t.after(() => rm(project.temporary, { recursive: true, force: true }));
  await integrateOneTask(project);
  const result = await runGateReport(project, {
    stage: 'verify', sessionId: 'verify-1', verdict: 'pass', findings: [],
    assessment: 'Verify result marker was malformed.', commandIds: ['check'],
  }, { GATE_MODE: 'malformed' });
  assert.equal(result.status, 1);
  assert.equal(result.json.ok, true);
  assert.equal(result.json.verdict, 'fail');
  assert.equal(result.json.state.status, 'blocked');
  assert.equal(result.json.state.receipts.gates.verify.verdict, 'fail');
  assert.equal(result.json.state.receipts.gates.verify.findings.some((finding) => finding.blocking), true);
  assert.equal(result.json.state.receipts.gates.verify.reportDigest, result.json.reportDigest);
});

test('crashed Verify reservation replays as consumed and cannot spawn again', async (t) => {
  const project = await fixture();
  t.after(() => rm(project.temporary, { recursive: true, force: true }));
  await integrateOneTask(project);
  const reportPath = path.join(project.requests, 'verify-report.json');
  await writeFile(reportPath, JSON.stringify({
    stage: 'verify', sessionId: 'verify-1', verdict: 'pass', findings: [],
    assessment: 'This report should stop immediately after durable reservation.', commandIds: ['check'],
  }));
  const crashed = spawnSync(process.execPath, [CLI, 'gate-report', '--run-dir', project.runDir, '--report', reportPath], {
    encoding: 'utf8', windowsHide: true, shell: false, timeout: 30_000,
    env: { ...process.env, NODE_ENV: 'test', WORKFLOW_GUARD_TEST_CRASH: 'after-event-rename' },
  });
  assert.equal(crashed.status, 92);
  const lock = JSON.parse(await readFile(path.join(project.runDir, 'run.lock'), 'utf8'));
  const recovered = cliSync(['show', '--run-dir', project.runDir, '--ack-stale-lock', lock.nonce]);
  assert.equal(recovered.status, 0, recovered.json.message);
  assert.equal(Object.keys(recovered.json.state.receipts.gateReports).length, 1);
  const sequence = recovered.json.sequence;
  const retry = await runGateReport(project, JSON.parse(await readFile(reportPath, 'utf8')));
  assert.equal(retry.status, 1);
  assert.equal(retry.json.code, 'GATE_REPORT_ALREADY_RESERVED');
  const after = cliSync(['show', '--run-dir', project.runDir]);
  assert.equal(after.status, 0, after.json.message);
  assert.equal(after.json.sequence, sequence);
  assert.equal(Object.keys(after.json.state.receipts.gates).length, 0);
});
