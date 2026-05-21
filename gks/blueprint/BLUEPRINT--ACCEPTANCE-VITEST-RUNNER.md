---
id: BLUEPRINT--ACCEPTANCE-VITEST-RUNNER
phase: 3
type: blueprint
scale_level: L2
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: BLUEPRINT — vitest acceptance runner implementation plan
tags: &a1
  - msp
  - codegen
  - acceptance
  - vitest
  - blueprint
  - implementation
crosslinks: &a2
  implements:
    - FEAT--ACCEPTANCE-VITEST-RUNNER
  references:
    - ADR--ACCEPTANCE-VITEST-RUNNER
linked_symbols: &a3
  - file: packages/msp/src/codegen/acceptance/vitest.ts
  - file: packages/msp/src/codegen/acceptance/sandbox.ts
  - file: packages/msp/src/codegen/acceptance/parse-results.ts
  - file: packages/msp/src/codegen/acceptance/types.ts
created_at: 2026-05-03T16:27:18.868+07:00
aliases: &a4
  - BLUEPRINT
  - implementation_flow
  - Implementation plan
cluster: implementation_flow
role: Implementation plan
attributes:
  id: BLUEPRINT--ACCEPTANCE-VITEST-RUNNER
  phase: 3
  type: blueprint
  scale_level: L2
  status: stable
  vault_id: default
  tier: process
  source_type: axiomatic
  title: BLUEPRINT — vitest acceptance runner implementation plan
  tags: *a1
  crosslinks: *a2
  linked_symbols: *a3
  created_at: 2026-05-03T16:27:18.868+07:00
  aliases: *a4
  cluster: implementation_flow
  role: Implementation plan
  attributes:
    id: BLUEPRINT--ACCEPTANCE-VITEST-RUNNER
    phase: 3
    type: blueprint
    scale_level: L2
    status: stable
    vault_id: default
    tier: process
    source_type: axiomatic
    title: BLUEPRINT — vitest acceptance runner implementation plan
    tags: *a1
    crosslinks: *a2
    linked_symbols: *a3
    created_at: 2026-05-03T16:27:18.868+07:00
    aliases: *a4
    cluster: implementation_flow
    role: Implementation plan
    attributes:
      id: BLUEPRINT--ACCEPTANCE-VITEST-RUNNER
      phase: 3
      type: blueprint
      scale_level: L2
      status: stable
      vault_id: default
      tier: process
      source_type: axiomatic
      title: BLUEPRINT — vitest acceptance runner implementation plan
      tags: *a1
      crosslinks: *a2
      linked_symbols: *a3
      created_at: 2026-05-03T16:27:18.868+07:00
      aliases: *a4
      cluster: implementation_flow
      role: Implementation plan
      attributes:
        domain: blueprint
      domain: blueprint
      language: markdown
      is_test: false
      is_entrypoint: false
      has_secret: true
      secret_type: high_entropy_string
      leak_risk: high
      encryption_level: none
    domain: blueprint
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: true
    secret_type: high_entropy_string
    leak_risk: high
    encryption_level: none
  domain: blueprint
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: true
  secret_type: high_entropy_string
  leak_risk: high
  encryption_level: none
---

# BLUEPRINT — vitest acceptance runner

```yaml
metadata:
  title: "Vitest acceptance runner"
  parent_feat: FEAT--ACCEPTANCE-VITEST-RUNNER

architectural_pattern: |
  Three small modules:
    sandbox.ts        scaffoldSandbox + writeCandidate + cleanupSandbox
    parse-results.ts  parseVitestJson + fallbackToStderr
    vitest.ts         createVitestAcceptance(opts) returns an AcceptanceRunner
                      that composes the above + spawns vitest

data_logic: |
  createVitestAcceptance({ repoRoot, verificationFiles, timeoutMs?, vitestBin? }):
    return async (task, code) => {
      const sandbox = await mkdtemp('msp-accept-')
      try {
        await scaffoldSandbox(sandbox, repoRoot)
        await writeCandidate(sandbox, task.geography, code)
        for (const v of verificationFiles)
          await copyFile(resolve(repoRoot, v.src), resolve(sandbox, v.dest))
        const result = await spawnVitest(sandbox, vitestBin, timeoutMs)
        return parseVitestJson(result.stdout, result.stderr, result.code)
      } finally {
        await rm(sandbox, { recursive: true, force: true })
      }
    }

geography:
  - "packages/msp/src/codegen/acceptance/types.ts"
  - "packages/msp/src/codegen/acceptance/sandbox.ts"
  - "packages/msp/src/codegen/acceptance/parse-results.ts"
  - "packages/msp/src/codegen/acceptance/vitest.ts"
  - "packages/msp/test/codegen/acceptance/sandbox.test.ts"
  - "packages/msp/test/codegen/acceptance/parse-results.test.ts"
  - "packages/msp/test/codegen/acceptance/vitest.test.ts"

api_contracts:
  - name: createVitestAcceptance
    signature: |
      function createVitestAcceptance(opts: VitestOpts): AcceptanceRunner
    types: |
      interface VitestOpts {
        repoRoot: string
        verificationFiles: { src: string; dest: string }[]
        timeoutMs?: number               // default 60_000
        vitestBin?: string                // default 'npx vitest'
      }
      class AcceptanceError extends Error {
        constructor(message: string, kind: 'vitest-not-found' | 'spawn-failed' | 'sandbox')
      }

  - name: scaffoldSandbox
    signature: |
      function scaffoldSandbox(dir: string, repoRoot: string): Promise<void>
    types: |
      // creates package.json, vitest.config.ts, symlinks node_modules

  - name: parseVitestJson
    signature: |
      function parseVitestJson(stdout: string, stderr: string, code: number): string[]
    types: |
      // walks testResults[].assertionResults[] for status:'failed'
      // emits "<file> > <fullName>: <first line of failureMessages>" each
      // falls back to last 50 lines of stderr on parse failure

verification_plan:
  - vitest unit on parse-results: golden vitest JSON fixtures (pass + fail) → expected error strings
  - vitest unit on sandbox: scaffolds dir with expected files, symlinks present, cleanup removes
  - integration: real createVitestAcceptance against a known-good fixture (1 test asserting 1+1===2) with code='export const sum=(a,b)=>a+b' → returns []
  - integration: same fixture with bad code → returns 1 error string referencing the failed assertion
  - timeout: vitest invocation that hangs → killed; returns ['acceptance: timeout']
```

## Implementation order

T1 WRITE-SANDBOX (scaffoldSandbox + writeCandidate)
T2 SPAWN-VITEST (vitest.ts top-level façade — reuses T1 + T3)
T3 PARSE-FAILURES (parseVitestJson + stderr fallback)

## Connections

- [[FEAT--ACCEPTANCE-VITEST-RUNNER]]
- [[ADR--ACCEPTANCE-VITEST-RUNNER]]
