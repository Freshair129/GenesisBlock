---
id: FEAT--CODEGEN-MICROTASK-RUNNER
phase: 2
type: feat
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: msp:run-task — execute one T*.task.yaml end to end
tags: &a1
  - msp
  - codegen
  - runner
  - cli
  - user-facing
crosslinks: &a2
  implements:
    - ADR--CODEGEN-MICROTASK-RUNNER
  references:
    - CONCEPT--CODEGEN-MICROTASK-RUNNER
    - ADR--CODEGEN-RETRY-POLICY
linked_symbols: &a3
  - file: packages/msp/src/codegen/runner.ts
  - file: packages/msp/src/codegen/post-process.ts
  - file: packages/msp/src/codegen/forbidden-patterns.ts
  - file: src/codegen/escalate.ts
created_at: 2026-05-03T14:16:37.122+07:00
aliases: &a4
  - FEAT
  - implementation_flow
  - Feature spec
cluster: implementation_flow
role: Feature spec
attributes:
  id: FEAT--CODEGEN-MICROTASK-RUNNER
  phase: 2
  type: feat
  status: stable
  vault_id: default
  tier: process
  source_type: axiomatic
  title: msp:run-task — execute one T*.task.yaml end to end
  tags: *a1
  crosslinks: *a2
  linked_symbols: *a3
  created_at: 2026-05-03T14:16:37.122+07:00
  aliases: *a4
  cluster: implementation_flow
  role: Feature spec
  attributes:
    id: FEAT--CODEGEN-MICROTASK-RUNNER
    phase: 2
    type: feat
    status: stable
    vault_id: default
    tier: process
    source_type: axiomatic
    title: msp:run-task — execute one T*.task.yaml end to end
    tags: *a1
    crosslinks: *a2
    linked_symbols: *a3
    created_at: 2026-05-03T14:16:37.122+07:00
    aliases: *a4
    cluster: implementation_flow
    role: Feature spec
    attributes:
      id: FEAT--CODEGEN-MICROTASK-RUNNER
      phase: 2
      type: feat
      status: stable
      vault_id: default
      tier: process
      source_type: axiomatic
      title: msp:run-task — execute one T*.task.yaml end to end
      tags: *a1
      crosslinks: *a2
      linked_symbols: *a3
      created_at: 2026-05-03T14:16:37.122+07:00
      aliases: *a4
      cluster: implementation_flow
      role: Feature spec
      attributes:
        domain: feat
      domain: feat
      language: markdown
      is_test: false
      is_entrypoint: false
      has_secret: true
      secret_type: high_entropy_string
      leak_risk: high
      encryption_level: none
    domain: feat
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: true
    secret_type: high_entropy_string
    leak_risk: high
    encryption_level: none
  domain: feat
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: true
  secret_type: high_entropy_string
  leak_risk: high
  encryption_level: none
---

# FEAT — msp:run-task

## User-facing behaviour

```sh
npm run msp:run-task -- .brain/default/tasks/msp-validator/T1_parser-frontmatter.task.yaml
```

Reads the YAML, runs the codegen pipeline (SLM → post-process → forbidden-pattern → acceptance), retries on fail per `[[ADR--CODEGEN-RETRY-POLICY]]`, and exits 0/1/2/3/4 per `[[ADR--CODEGEN-MICROTASK-RUNNER]]`.

Optional flags:

- `--model=<name>` — override SLM choice (default: configured Qwen 2.5 Coder)
- `--no-escalate` — fail at exit 1 instead of escalating to Gemini
- `--dry-run` — print the assembled prompt; don't call SLM
- `--json` — machine-readable output

## Acceptance criteria

- [ ] Loads any T*.task.yaml under `.brain/default/tasks/<feature>/` and validates schema (id, parent_blueprint, prompt, acceptance, geography)
- [ ] Refuses if `parent_blueprint` doesn't exist in `gks/blueprint/` or isn't `stable`
- [ ] Resolves blueprint geography → reads each file (if exists) → injects context into prompt
- [ ] On SLM call success, applies post-processing strip pipeline before any pattern check
- [ ] Forbidden-pattern hit → emits clear error message naming the rule + line
- [ ] Acceptance test fail → triggers retry with failure context; max 3 retries
- [ ] After 3 retries, escalates to Gemini once; logs the escalation to `MSP-ACT-` devlog
- [ ] On success, writes candidate code to geography paths in a sandbox; main commit is human-gated
- [ ] `--dry-run` exits 0 without calling SLM
- [ ] `--json` emits `{taskId, attempts, finalStatus, exitCode, escalation?}`

## Surfaces

| Surface | Form |
|---|---|
| TS API | `runTask(taskPath, opts): Promise<RunResult>` |
| CLI | `msp-run-task <path>` with above flags |
| MCP | `msp_run_task` tool (M4+) |

## Out of scope

- BLUEPRINT generation (separate feature in P3 toolchain).
- Multi-task batch orchestration (composable via `xargs`).
- Auto-commit on success (always human-gated).

## Connections

- [[CONCEPT--CODEGEN-MICROTASK-RUNNER]]
