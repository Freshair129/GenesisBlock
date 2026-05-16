---
id: BLUEPRINT--CODEGEN-MICROTASK-RUNNER
phase: 3
type: blueprint
scale_level: L2
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: BLUEPRINT — codegen microtask runner implementation plan
tags:
  - msp
  - codegen
  - runner
  - blueprint
  - implementation
crosslinks: {"implements":["FEAT--CODEGEN-MICROTASK-RUNNER"],"references":["ADR--CODEGEN-MICROTASK-RUNNER"]}
linked_symbols:
  - {"file":"packages/msp/src/codegen/runner.ts"}
  - {"file":"packages/msp/src/codegen/load-task.ts"}
  - {"file":"packages/msp/src/codegen/prompt-builder.ts"}
  - {"file":"src/codegen/slm-client.ts"}
  - {"file":"packages/msp/src/codegen/post-process.ts"}
  - {"file":"packages/msp/src/codegen/forbidden-patterns.ts"}
  - {"file":"src/codegen/acceptance.ts"}
  - {"file":"src/codegen/escalate.ts"}
  - {"file":"packages/msp/src/codegen/cli.ts"}
created_at: 2026-05-03T14:16:37.606+07:00
---

# BLUEPRINT — codegen microtask runner

```yaml
metadata:
  title: "Codegen microtask runner"
  parent_feat: FEAT--CODEGEN-MICROTASK-RUNNER

architectural_pattern: |
  Pipeline of small async functions over a TaskRun value object.
  Pure modules (post-process, forbidden-patterns, prompt-builder) +
  one IO-bearing module per external dependency (slm-client, acceptance).
  CLI is a thin wrapper that loads the task and invokes runTask().

data_logic: |
  Input: path to T*.task.yaml
  Pipeline:
    1. loadTask(path) → Task
    2. resolveBlueprint(task.parent_blueprint) → Blueprint
    3. for attempt in 1..3:
         prompt = buildPrompt(task, blueprint, lastFailure?)
         raw    = await callSlm(prompt, ctx)
         clean  = postProcess(raw)
         pErrs  = checkForbiddenPatterns(clean)
         if pErrs.length: lastFailure = { kind: 'pattern', errors: pErrs }; continue
         writeSandbox(clean, task.geography)
         tErrs  = await runAcceptance(task)
         if tErrs.length: lastFailure = { kind: 'test', errors: tErrs }; continue
         return success(attempt)
    4. escalateToGemini(task, blueprint, allFailures) → either success(escalated) or escalateOpus()

geography:
  - "src/codegen/runner.ts"            # public TS API: runTask(path, opts)
  - "src/codegen/load-task.ts"         # loadTask(path): Promise<Task>
  - "src/codegen/prompt-builder.ts"    # buildPrompt(task, blueprint, lastFailure?)
  - "src/codegen/slm-client.ts"        # callSlm(prompt, ctx) — pluggable
  - "src/codegen/post-process.ts"      # ports the strip pipeline from ADR--CODEGEN-POST-PROCESSING
  - "src/codegen/forbidden-patterns.ts"# ports the regex/import checks from ADR--CODEGEN-FORBIDDEN-PATTERNS
  - "src/codegen/acceptance.ts"        # runAcceptance(task) — vitest spawn or vm.runInContext
  - "src/codegen/escalate.ts"          # escalateToGemini, escalateOpus
  - "src/codegen/cli.ts"               # bin entry
  - "test/codegen/runner.test.ts"
  - "test/codegen/post-process.test.ts"
  - "test/codegen/forbidden-patterns.test.ts"

api_contracts:
  - name: runTask
    signature: |
      async function runTask(
        taskPath: string,
        opts?: RunOptions,
      ): Promise<RunResult>
    types: |
      interface RunOptions {
        model?: string                    // SLM identifier
        maxRetries?: number               // default 3 per ADR--CODEGEN-RETRY-POLICY
        escalate?: boolean                // default true; --no-escalate to disable
        dryRun?: boolean                  // print prompt; skip SLM call
        sandbox?: string                  // dir for candidate writes; default tmp
      }
      interface RunResult {
        taskId: string
        attempts: AttemptRecord[]
        finalStatus: 'success' | 'pattern-fail' | 'acceptance-fail' | 'escalated-success' | 'escalated-fail'
        exitCode: 0 | 1 | 2 | 3 | 4
        escalation?: { layer: 'gemini' | 'opus'; outcome: 'pass' | 'fail' }
      }
      interface AttemptRecord {
        attempt: number
        promptHash: string
        slmModel: string
        rawOutput: string
        cleanedOutput: string
        patternErrors: string[]
        acceptanceErrors: string[]
      }

verification_plan:
  - vitest unit on post-process (porting test cases from ADR--CODEGEN-POST-PROCESSING)
  - vitest unit on forbidden-patterns (one test per rule in ADR--CODEGEN-FORBIDDEN-PATTERNS)
  - integration test with mock SLM that emits known-bad output → asserts retry then escalation
  - integration test with mock SLM that emits known-good output → asserts exit 0 + sandbox contents
  - CLI test (spawn) with --dry-run → exits 0 without SLM call
```

## Implementation order (TASK chain)

T1 LOAD-TASK-YAML
T2 INVOKE-SLM (with mock client first; real Qwen integration last)
T3 APPLY-CHECKS (post-process + forbidden-patterns)
T4 RUN-ACCEPTANCE
T5 ESCALATE

## Connections
- [[FEAT--CODEGEN-MICROTASK-RUNNER]]
- [[ADR--CODEGEN-MICROTASK-RUNNER]]

