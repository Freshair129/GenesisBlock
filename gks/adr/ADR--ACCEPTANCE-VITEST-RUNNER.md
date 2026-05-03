---
id: ADR--ACCEPTANCE-VITEST-RUNNER
phase: 2
type: adr
status: stable
vault_id: default
title: Spawn vitest in tmp sandbox; parse JSON reporter output
tags:
  - msp
  - codegen
  - acceptance
  - vitest
  - decision
crosslinks: {"references":["CONCEPT--ACCEPTANCE-VITEST-RUNNER","ADR--CODEGEN-RETRY-POLICY"]}
created_at: 2026-05-03T09:27:17.914Z
---

# ADR — vitest acceptance runner shape

## Context

Three live designs for running tests against candidate code:

1. **In-process VM** — `vm.Script` evaluates candidate; we hook a tiny test framework. Fastest, no spawn, but reproducing vitest semantics in 200 LOC is fragile.
2. **Spawn vitest against the repo working tree** — overwrite paths in place, run `vitest run`, revert. Risk: parallel runs corrupt each other; aborted runs leave the repo dirty.
3. **Spawn vitest against a tmp sandbox** — mkdtemp → write candidate → symlink `node_modules` → run vitest with cwd=sandbox → cleanup. Isolation by construction; parallel-safe.

## Decision

**Option 3 — tmp sandbox per invocation.**

### Sandbox layout

```
<sandbox>/                     mkdtemp output, cleaned on exit
├── node_modules        →      symlink to <repoRoot>/node_modules
├── package.json               minimal {"type":"module"}
├── vitest.config.ts           single-line config; tests via include
├── src/...                    candidate code at BLUEPRINT geography paths
└── test/...                   verification tests copied from BLUEPRINT
```

### Pipeline

```ts
async function runAcceptanceVitest(task, code, opts): Promise<string[]> {
  const sandbox = await mkdtemp('msp-accept-')
  try {
    await scaffoldSandbox(sandbox, opts.repoRoot)        // node_modules symlink, package.json, vitest.config.ts
    await writeCandidate(sandbox, task.geography, code)
    await copyVerification(sandbox, opts.verificationFiles)
    const r = spawnSync('npx', ['vitest', 'run', '--reporter=json'], {
      cwd: sandbox, encoding: 'utf8', env: { ...process.env, NO_COLOR: '1' },
    })
    return parseFailures(r.stdout, r.stderr, r.status)
  } finally {
    await rm(sandbox, { recursive: true, force: true })
  }
}
```

### Parsing

vitest `--reporter=json` emits `{ testResults: [{ assertionResults: [{ status, fullName, failureMessages }] }] }`. We extract every assertion with `status: 'failed'` into a single error line:
`"<file> > <fullName>: <first line of failureMessages>"`.

If JSON is malformed or empty, fall back to including the last 50 lines of stderr.

### Where verification tests come from

Two options:
- `opts.verificationFiles: { src: string, dest: string }[]` — caller supplies the test files. Default in M4c.
- BLUEPRINT YAML `verification_plan` field — parse + locate test files. **Future**: too much coupling for M4c.

For now, the runner accepts an explicit `verificationFiles` array. The M4c integration test wires this manually; future caller (orchestrator) reads the BLUEPRINT.

### Failure modes

| Symptom | Action |
|---|---|
| `node_modules` symlink fails (Windows / cross-fs) | Fall back to copying — log a warning; runs ~3× slower |
| vitest binary missing in sandbox `node_modules` | Throw `AcceptanceError('vitest-not-found')` |
| vitest spawn hangs | `timeoutMs` (default 60_000) → kill child → `[acceptance: timeout]` |
| vitest exits 0 with no testResults | Treated as pass (no test files matched — orchestrator's responsibility) |

## Consequences

**Positive**
- Isolation by construction. No risk of corrupting the working tree.
- Parallel-safe: each invocation gets its own tmp dir.
- Reuses the repo's existing vitest install via symlink — no reinstall cost.
- Failure messages flow back through the runner's `lastFailure` channel, which already feeds them into the next prompt.

**Negative**
- Tmp dir + symlink overhead: ~50–200 ms per invocation. Acceptable for an interactive flow.
- Symlink unavailable on some Windows setups → fallback path exists but is slower.
- `node_modules` shared across runs — a candidate that mutates `node_modules` could affect later runs. Acceptable risk (and would fail real-world too).

## Alternatives considered

1. **In-process VM.** Rejected — re-implementing vitest is out of scope.
2. **In-place mutation.** Rejected — parallel runs corrupt each other; aborted runs leave repo dirty.
3. **Docker container per run.** Rejected — overhead too high (1–5 s per invocation) and adds a new dependency.
4. **vitest's programmatic API (`createVitest`).** Considered. Doable but the API surface is unstable across vitest minor versions and harder to capture pass/fail JSON cleanly. Spawn is more robust.

## Source

`CONCEPT--ACCEPTANCE-VITEST-RUNNER` + vitest JSON reporter docs.
