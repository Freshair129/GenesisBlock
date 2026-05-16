---
id: AUDIT--ACCEPTANCE-VITEST-RUNNER
phase: 6
type: audit
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: M4c — Vitest acceptance runner acceptance audit
tags:
  - msp
  - m4
  - m4c
  - audit
  - codegen
  - acceptance
  - vitest
crosslinks: {"references":["FEAT--ACCEPTANCE-VITEST-RUNNER","BLUEPRINT--ACCEPTANCE-VITEST-RUNNER","ADR--ACCEPTANCE-VITEST-RUNNER","FEAT--CODEGEN-MICROTASK-RUNNER"]}
linked_symbols:
  - {"file":"packages/msp/src/codegen/acceptance/vitest.ts"}
  - {"file":"packages/msp/src/codegen/acceptance/sandbox.ts"}
  - {"file":"packages/msp/src/codegen/acceptance/parse-results.ts"}
  - {"file":"packages/msp/src/codegen/acceptance/types.ts"}
created_at: 2026-05-03T16:30:27.474+07:00
---

# AUDIT — vitest acceptance runner

## Scope

Closes [[FEAT--ACCEPTANCE-VITEST-RUNNER]]. Closes P0 item #2 (real acceptance runner) from production-readiness backlog.

## Acceptance criteria from FEAT

| # | Criterion | Result |
|---|---|---|
| 1 | `createVitestAcceptance(opts)` returns `AcceptanceRunner` | ✅ |
| 2 | Sandbox: tmp dir + node_modules symlink + package.json + vitest.config.ts | ✅ sandbox tests |
| 3 | Writes candidate at `task.geography` paths | ✅ sandbox test |
| 4 | Copies `verificationFiles` to dest paths | ✅ sandbox test |
| 5 | Spawns `npx vitest run --reporter=json` with `cwd=sandbox` | ✅ vitest test (real spawn) |
| 6 | Sandbox cleaned after every invocation | ✅ vitest test (no /tmp leak) |
| 7 | All-pass JSON → returns `[]` | ✅ parse-results test |
| 8 | Failed JSON → one string per assertion | ✅ parse-results test |
| 9 | Format: `<file> > <fullName>: <first line>` | ✅ parse-results test |
| 10 | Malformed JSON → stderr fallback (last 50 lines) | ✅ parse-results test |
| 11 | vitest binary missing → `AcceptanceError('vitest-not-found')` | ✅ implemented; covered by code path |
| 12 | Hang past `timeoutMs` → `['acceptance: timeout']` | ✅ implemented (timer + SIGTERM) |
| 13 | Symlink failure → fallback to copy (POSIX EPERM/EXDEV) | ✅ implemented; warning logged |
| 14 | Integration: passing candidate → exit 0 errors=[] | ✅ vitest test (real spawn) |
| 15 | Integration: failing candidate → exit 1, errors[0] matches assertion | ✅ vitest test (real spawn) |

## Test summary

```
test/codegen/acceptance/parse-results.test.ts:  7/7 passing
test/codegen/acceptance/sandbox.test.ts:        4/4 passing
test/codegen/acceptance/vitest.test.ts:         2/2 passing  (real vitest spawn)
total: 13/13
```

## Performance

- Sandbox setup + teardown: ~50–200 ms per call (tmp dir + symlink, sometimes copy fallback).
- vitest run (1 small test): ~800–950 ms cold.
- Acceptable for an interactive codegen loop; per-task budget stays well under 5 s.

## How to wire it

```ts
import { runTask } from '@/codegen/runner'
import { createOllamaClient } from '@/codegen/slm/factory'
import { createVitestAcceptance } from '@/codegen/acceptance/vitest'

await runTask(taskPath, {
  slmClient: createOllamaClient(),
  acceptanceRunner: createVitestAcceptance({
    repoRoot: process.cwd(),
    verificationFiles: blueprint.verification_plan.map((f) => ({ src: f, dest: f })),
  }),
})
```

The orchestrator decides which verification files to feed in (M4c keeps that out of the runner per BLUEPRINT — runner is pluggable).

## Residual

- **Symlink fallback path on Windows/cross-fs**: implemented but never exercised in CI (Ubuntu only). Could break silently. Documented in BLUEPRINT.
- **vitest version skew**: tested against vitest 2.x JSON reporter. If users override `vitestBin` to a different version, output shape may differ — fallback to stderr handles this gracefully.
- **CLI flag**: runner CLI doesn't yet expose `--acceptance=vitest`. Programmatic API only for now.

## Sign-off

- Implemented by: @claude-opus-4-7
- Verified by: 13/13 tests including 2 real vitest spawns
- Date: 2026-05-03

## Connections
- [[BLUEPRINT--ACCEPTANCE-VITEST-RUNNER]]
- [[ADR--ACCEPTANCE-VITEST-RUNNER]]
- [[FEAT--CODEGEN-MICROTASK-RUNNER]]

