#!/usr/bin/env node

import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { randomUUID } from 'node:crypto';
import { RunStore, RuntimeError, parseJsonNoDuplicates } from './workflow-guard/store.mjs';

const HELP = `Workflow Guard V1 (local durable adapter; no provider or merge calls)

Usage:
  node scripts/workflow-guard.mjs init --plan <plan.json> --run-dir <absolute-dir> --workspace <git-worktree> --run-id <id> --coordinator-session-id <id> [--task-workspaces <mapping.json>]
  node scripts/workflow-guard.mjs show --run-dir <absolute-dir> [--ack-stale-lock <nonce>]
  node scripts/workflow-guard.mjs event --run-dir <absolute-dir> --event <event.json> [--ack-stale-lock <nonce>]
  node scripts/workflow-guard.mjs command --run-dir <absolute-dir> --task <task-id> --attempt <attempt-id> --command <command-id> [--ack-stale-lock <nonce>]
  node scripts/workflow-guard.mjs gate-report --run-dir <absolute-dir> --report <closed-report-input.json> [--ack-stale-lock <nonce>]
  node scripts/workflow-guard.mjs recover --run-dir <absolute-dir> [--ack-stale-lock <nonce>]

gate-report input has exactly {stage,sessionId,verdict,findings,assessment,commandIds}.
Verify commandIds must exactly cover required non-expected-red AC commands; other stages use [].
The Verify reservation is one-shot. A committed FAIL report prints JSON and exits nonzero.

`;

function parseArgs(argv) {
  const options = Object.create(null);
  const positional = [];
  for (let index = 0; index < argv.length; index += 1) {
    const item = argv[index];
    if (!item.startsWith('--')) { positional.push(item); continue; }
    if (Object.hasOwn(options, item)) throw new RuntimeError('CLI_DUPLICATE_FLAG', `Flag ${item} was supplied more than once.`);
    const value = argv[index + 1];
    if (value === undefined || value.startsWith('--')) throw new RuntimeError('CLI_FLAG_VALUE', `Flag ${item} requires a value.`);
    options[item] = value;
    index += 1;
  }
  return { options, positional };
}

function requireOption(options, name) {
  const value = options[name];
  if (typeof value !== 'string' || value.length === 0) throw new RuntimeError('CLI_REQUIRED', `Missing required ${name}.`);
  return value;
}

function rejectUnknownOptions(options, allowed) {
  for (const key of Object.keys(options)) if (!allowed.has(key)) throw new RuntimeError('CLI_UNKNOWN_FLAG', `Unknown flag ${key}.`);
}

async function readInputJson(file, label) {
  const absolute = path.resolve(file);
  const source = await (await import('node:fs/promises')).readFile(absolute, 'utf8').catch((error) => {
    throw new RuntimeError('CLI_READ', `Could not read ${label} ${absolute}.`, { cause: error });
  });
  return parseJsonNoDuplicates(source, label);
}

function emit(value) {
  process.stdout.write(`${JSON.stringify(value)}\n`);
}

function usageError() {
  throw new RuntimeError('CLI_USAGE', `Unknown operation.\n${HELP.trim()}`);
}

async function main(argv) {
  const operation = argv[0];
  if (!operation || operation === 'help' || operation === '--help' || operation === '-h') {
    process.stdout.write(HELP);
    return;
  }
  const { options, positional } = parseArgs(argv.slice(1));
  if (positional.length > 0) throw new RuntimeError('CLI_POSITIONAL', `Unexpected argument ${positional[0]}.`);

  if (operation === 'init') {
    rejectUnknownOptions(options, new Set(['--plan', '--run-dir', '--workspace', '--run-id', '--coordinator-session-id', '--task-workspaces']));
    const planPath = path.resolve(requireOption(options, '--plan'));
    const plan = await readInputJson(planPath, 'plan');
    let taskWorkspaces = Object.create(null);
    if (options['--task-workspaces']) taskWorkspaces = await readInputJson(options['--task-workspaces'], 'task workspace mapping');
    if (!taskWorkspaces || typeof taskWorkspaces !== 'object' || Array.isArray(taskWorkspaces)) throw new RuntimeError('CLI_TASK_WORKSPACES', 'Task workspace mapping must be a JSON object of task IDs to absolute Git worktree paths.');
    const store = await RunStore.initialize({
      plan,
      planPath,
      runDir: requireOption(options, '--run-dir'),
      workspace: path.resolve(requireOption(options, '--workspace')),
      taskWorkspaces,
      runId: requireOption(options, '--run-id'),
      coordinatorSessionId: requireOption(options, '--coordinator-session-id'),
    });
    const result = await store.status();
    emit({ ok: true, operation, runDir: store.runDir, planDigest: store.manifest.planDigest, baseline: store.manifest.baseline, state: result.state });
    return;
  }

  const runDir = requireOption(options, '--run-dir');
  const store = await RunStore.open(runDir);
  const ackStaleNonce = options['--ack-stale-lock'];

  if (operation === 'show') {
    rejectUnknownOptions(options, new Set(['--run-dir', '--ack-stale-lock']));
    const result = await store.status({ ackStaleNonce });
    emit({ ok: true, operation, runDir: store.runDir, planDigest: store.manifest.planDigest, headDigest: result.headDigest, sequence: result.sequence, candidate: result.candidate, state: result.state, orphanTemps: result.orphanTemps });
    return;
  }

  if (operation === 'event') {
    rejectUnknownOptions(options, new Set(['--run-dir', '--event', '--ack-stale-lock']));
    const eventPath = requireOption(options, '--event');
    const event = await readInputJson(eventPath, 'event');
    const result = await store.processEvent(event, { ackStaleNonce });
    emit({ ok: true, operation, version: result.state.version, sequence: result.sequence, idempotent: result.idempotent, dispatchRequest: result.dispatchRequest, state: result.state });
    return;
  }

  if (operation === 'command') {
    rejectUnknownOptions(options, new Set(['--run-dir', '--task', '--attempt', '--command', '--ack-stale-lock']));
    const result = await store.executeApprovedCommand({
      taskId: requireOption(options, '--task'),
      attemptId: requireOption(options, '--attempt'),
      commandId: requireOption(options, '--command'),
      ackStaleNonce,
    });
    emit({ ok: true, operation, ...result });
    return;
  }

  if (operation === 'gate-report') {
    rejectUnknownOptions(options, new Set(['--run-dir', '--report', '--ack-stale-lock']));
    const report = await readInputJson(requireOption(options, '--report'), 'gate report input');
    const result = await store.executeGateReport({ input: report, ackStaleNonce });
    emit({ ok: true, operation, ...result });
    if (result.verdict === 'fail') process.exitCode = 1;
    return;
  }

  if (operation === 'recover') {
    rejectUnknownOptions(options, new Set(['--run-dir', '--ack-stale-lock']));
    const result = await store.recover({ ackStaleNonce });
    emit({ ok: true, operation, version: result.state.version, sequence: result.sequence, state: result.state });
    return;
  }

  usageError();
}

if (process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href) {
  main(process.argv.slice(2)).catch((error) => {
    const code = error instanceof RuntimeError ? error.code : error?.code ?? 'RUNTIME_ERROR';
    const message = error instanceof Error ? error.message : String(error);
    emit({ ok: false, code, message });
    process.exitCode = 1;
  });
}
