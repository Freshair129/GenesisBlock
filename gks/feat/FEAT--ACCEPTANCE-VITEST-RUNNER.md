---
id: FEAT--ACCEPTANCE-VITEST-RUNNER
phase: 2
type: feat
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: createVitestAcceptance(opts) — pluggable real test gate
tags:
  - msp
  - codegen
  - acceptance
  - vitest
  - user-facing
crosslinks:
  implements:
    - ADR--ACCEPTANCE-VITEST-RUNNER
  references:
    - CONCEPT--ACCEPTANCE-VITEST-RUNNER
    - FEAT--CODEGEN-MICROTASK-RUNNER
linked_symbols:
  - file: packages/msp/src/codegen/acceptance/vitest.ts
  - file: packages/msp/src/codegen/acceptance/sandbox.ts
  - file: packages/msp/src/codegen/acceptance/types.ts
created_at: 2026-05-03T16:27:18.409+07:00
aliases:
  - FEAT
  - implementation_flow
  - Feature spec
cluster: implementation_flow
role: Feature spec
attributes:
  domain: feat
---

# FEAT — vitest acceptance runner

## User-facing behaviour

```ts
import { runTask } from '@/codegen/runner'
import { createVitestAcceptance } from '@/codegen/acceptance/vitest'

const acceptance = createVitestAcceptance({
  repoRoot: process.cwd(),
  verificationFiles: [
    { src: 'test/foo/expected.test.ts', dest: 'test/foo/expected.test.ts' },
  ],
  timeoutMs: 60_000,
})

await runTask(taskPath, {
  slmClient: createOllamaClient(),
  acceptanceRunner: acceptance,
})
```

When acceptance fails, the runner sees error strings like:

```
test/foo/expected.test.ts > foo() returns 42: expected 41 to be 42
```

…and feeds them into the next SLM retry per `[[ADR--CODEGEN-RETRY-POLICY]]`.

## Acceptance criteria

- [ ] `createVitestAcceptance(opts)` returns an `AcceptanceRunner`
- [ ] On call: creates a tmp sandbox, symlinks `<repoRoot>/node_modules`, writes candidate at task.geography paths, copies verificationFiles
- [ ] Spawns `npx vitest run --reporter=json` with `cwd=sandbox`
- [ ] Sandbox cleaned up after every invocation (success or failure)
- [ ] vitest exit 0 + non-empty testResults all passing → returns `[]`
- [ ] vitest exit non-zero → returns one string per failed assertion
- [ ] Each failure string contains: file > test name + first line of failure message
- [ ] Malformed vitest JSON → falls back to last 50 lines of stderr
- [ ] vitest binary missing → throws `AcceptanceError('vitest-not-found')`
- [ ] vitest hangs past `timeoutMs` → kills child, returns `['acceptance: timeout']`
- [ ] Symlink failure → falls back to copying `node_modules` with a warning
- [ ] Integration test: known-good candidate against a known-passing test → exit 0, errors=[]
- [ ] Integration test: known-bad candidate against same test → exit 1, errors[0] matches the assertion

## Surfaces

| Surface | Form |
|---|---|
| TS API | `createVitestAcceptance(opts: VitestOpts): AcceptanceRunner` |
| Types | `AcceptanceError` class; `VitestOpts` interface |

## Out of scope

- Reading verification files from BLUEPRINT YAML's `verification_plan` field.
- vitest watch mode.
- Docker isolation.
- Caching across runs.

## Connections
- [[ADR--ACCEPTANCE-VITEST-RUNNER]]
- [[CONCEPT--ACCEPTANCE-VITEST-RUNNER]]
- [[FEAT--CODEGEN-MICROTASK-RUNNER]]

