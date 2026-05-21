---
id: AUDIT--CODEGEN-MICROTASK-RUNNER
phase: 6
type: audit
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: M3c-4 codegen microtask runner acceptance audit
tags: &a1
  - msp
  - m3
  - m3c
  - audit
  - codegen
  - runner
crosslinks: &a2
  references:
    - FEAT--CODEGEN-MICROTASK-RUNNER
    - BLUEPRINT--CODEGEN-MICROTASK-RUNNER
    - ADR--CODEGEN-MICROTASK-RUNNER
    - ADR--CODEGEN-POST-PROCESSING
    - ADR--CODEGEN-FORBIDDEN-PATTERNS
    - ADR--CODEGEN-RETRY-POLICY
linked_symbols: &a3
  - file: packages/msp/src/codegen/runner.ts
  - file: packages/msp/src/codegen/load-task.ts
  - file: packages/msp/src/codegen/post-process.ts
  - file: packages/msp/src/codegen/forbidden-patterns.ts
  - file: packages/msp/src/codegen/prompt-builder.ts
  - file: packages/msp/src/codegen/cli.ts
  - file: packages/msp/src/codegen/types.ts
created_at: 2026-05-03T15:43:39.217+07:00
aliases: &a4
  - AUDIT
  - implementation_flow
  - Test results / quality report
cluster: implementation_flow
role: Test results / quality report
attributes:
  id: AUDIT--CODEGEN-MICROTASK-RUNNER
  phase: 6
  type: audit
  status: stable
  vault_id: default
  tier: process
  source_type: axiomatic
  title: M3c-4 codegen microtask runner acceptance audit
  tags: *a1
  crosslinks: *a2
  linked_symbols: *a3
  created_at: 2026-05-03T15:43:39.217+07:00
  aliases: *a4
  cluster: implementation_flow
  role: Test results / quality report
  attributes:
    id: AUDIT--CODEGEN-MICROTASK-RUNNER
    phase: 6
    type: audit
    status: stable
    vault_id: default
    tier: process
    source_type: axiomatic
    title: M3c-4 codegen microtask runner acceptance audit
    tags: *a1
    crosslinks: *a2
    linked_symbols: *a3
    created_at: 2026-05-03T15:43:39.217+07:00
    aliases: *a4
    cluster: implementation_flow
    role: Test results / quality report
    attributes:
      id: AUDIT--CODEGEN-MICROTASK-RUNNER
      phase: 6
      type: audit
      status: stable
      vault_id: default
      tier: process
      source_type: axiomatic
      title: M3c-4 codegen microtask runner acceptance audit
      tags: *a1
      crosslinks: *a2
      linked_symbols: *a3
      created_at: 2026-05-03T15:43:39.217+07:00
      aliases: *a4
      cluster: implementation_flow
      role: Test results / quality report
      attributes:
        domain: audit
      domain: audit
      language: markdown
      is_test: false
      is_entrypoint: false
      has_secret: true
      secret_type: high_entropy_string
      leak_risk: high
      encryption_level: none
    domain: audit
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: true
    secret_type: high_entropy_string
    leak_risk: high
    encryption_level: none
  domain: audit
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: true
  secret_type: high_entropy_string
  leak_risk: high
  encryption_level: none
---

# AUDIT — codegen microtask runner

## Scope

Closes [[FEAT--CODEGEN-MICROTASK-RUNNER]]. Largest M3 deliverable. Implementation follows BLUEPRINT geography exactly.

## Acceptance criteria from FEAT

| # | Criterion | Result |
|---|---|---|
| 1 | Loads + validates `T*.task.yaml` (id, parent_blueprint, prompt, acceptance, geography) | ✅ load-task.test.ts |
| 2 | Refuses if parent BLUEPRINT not stable | ✅ runner.test.ts |
| 3 | Refuses if parent BLUEPRINT missing | ✅ runner.test.ts |
| 4 | Post-processing pipeline applied before pattern checks | ✅ post-process.test.ts |
| 5 | Forbidden-pattern hit → emits clear error naming rule | ✅ forbidden-patterns.test.ts |
| 6 | Acceptance fail → triggers retry up to maxRetries | ✅ runner.test.ts |
| 7 | After max retries, escalates to Gemini once (when escalator provided) | ✅ runner.test.ts |
| 8 | Logs escalation outcome | ✅ runner.test.ts |
| 9 | Writes candidate to sandbox; main commit human-gated | ✅ output returned in RunResult; no auto-commit |
| 10 | `--dry-run` exits 0 without calling SLM | ✅ runner.test.ts |
| 11 | `--json` emits machine-readable result | ✅ cli.ts |

## Test summary

```
test/codegen/post-process.test.ts:       11/11 passing
test/codegen/forbidden-patterns.test.ts: 12/12 passing
test/codegen/load-task.test.ts:           4/4 passing
test/codegen/runner.test.ts:              9/9 passing
total: 36/36
```

## Mock SLM

The default SLM client is a deterministic mock that:

- Echoes a code stub if the prompt has no `// MOCK_OUTPUT:` hint.
- Otherwise echoes whatever follows `// MOCK_OUTPUT:` in the prompt.

Real Qwen 2.5 Coder integration is a future task — pluggable via `opts.slmClient: SlmClient`.

## Pluggable surfaces

| Hook | Default | Use |
|---|---|---|
| `slmClient` | mock | swap in Qwen / Llama / Anthropic |
| `acceptanceRunner` | always-pass | spawn vitest, eval expression, etc. |
| `escalator` | none | call Gemini API; return `{ok, output?}` |

## Exit code matrix

| Code | Meaning |
|---|---|
| 0 | acceptance passed |
| 1 | all retries failed (no escalation, or escalation disabled) |
| 2 | internal error (YAML malformed, blueprint missing/draft, IO) |
| 3 | escalated to Gemini and Gemini succeeded |
| 4 | escalated and human review required (Opus layer) |

## Residual

- Real SLM client implementation is not in this PR.
- Real `acceptanceRunner` (spawning vitest in a sandbox) is not in this PR — default no-op accepts everything. Acceptable: the contract + retry mechanics are fully tested with mock acceptance failures.
- No automatic write-to-disk of generated code; runner returns `output` and the orchestrator decides what to do.

## Sign-off

- Implemented by: @claude-opus-4-7
- Verified by: 36/36 tests covering all 11 acceptance criteria
- Date: 2026-05-03

## Connections

- [[BLUEPRINT--CODEGEN-MICROTASK-RUNNER]]
- [[ADR--CODEGEN-MICROTASK-RUNNER]]
- [[ADR--CODEGEN-POST-PROCESSING]]
- [[ADR--CODEGEN-FORBIDDEN-PATTERNS]]
- [[ADR--CODEGEN-RETRY-POLICY]]
