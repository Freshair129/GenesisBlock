---
id: ADR--CODEGEN-MICROTASK-RUNNER
phase: 2
type: adr
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: Codegen runner is a single-task CLI, not a daemon
tags:
  - msp
  - codegen
  - runner
  - cli
  - decision
crosslinks: {"references":["CONCEPT--CODEGEN-MICROTASK-RUNNER","ADR--CODEGEN-RETRY-POLICY","ADR--CODEGEN-FORBIDDEN-PATTERNS","ADR--CODEGEN-POST-PROCESSING"]}
created_at: 2026-05-03T15:16:36.663+07:00
---

# ADR — codegen runner shape

## Context

The runner could be (a) a long-running daemon that watches `.brain/<ns>/tasks/` and processes new YAMLs as they appear, or (b) a one-shot CLI that processes exactly one task per invocation. The orchestrator (a higher layer) needs different shapes for each:

- (a) requires queue management, restart logic, leader election if HA.
- (b) is dumb but composable — `xargs -n1 npm run msp:run-task <T>` parallelises trivially.

## Decision

**Single-task CLI**: `npm run msp:run-task -- <path/to/T*.task.yaml>`. One YAML per invocation. Exit codes:

| Code | Meaning |
|---|---|
| 0 | Acceptance passed; code written to geography paths |
| 1 | All 3 SLM retries failed; escalation to Gemini also failed |
| 2 | Internal error (YAML malformed, missing parent BLUEPRINT, etc.) |
| 3 | Escalated to Gemini and **Gemini** succeeded (audit-flagged) |
| 4 | Escalated to Opus review (human action required) |

### Per-invocation pipeline (CLI Implementation)

1. Load + validate `T*.task.yaml`.
2. Resolve `parent_blueprint` from `gks/blueprint/`; refuse if missing or non-stable.
3. Build SLM prompt from: blueprint geography + task prompt + acceptance criteria.
4. **Call Primary SLM**: Execute `python G:\qwen-cli\qwen.py --code --no-stream` with the assembled prompt.
5. Apply `ADR--CODEGEN-POST-PROCESSING` strip pipeline.
6. Apply `ADR--CODEGEN-FORBIDDEN-PATTERNS` checks.
7. Write candidate to sandbox; run acceptance test.
8. On fail, prepare retry prompt per `ADR--CODEGEN-RETRY-POLICY`; jump to 4.
9. **Escalation (v0.4.0)**: After 3 retries, invoke **Gemini CLI Subagent** via `gemini -p "<escalation_prompt>" -y`.
    - If Gemini passes acceptance → exit 3.
    - If Gemini fails → exit 4 (requires human review / Opus layer).
10. Emit `MSP-ACT-` devlog row.

## Consequences

**Positive**
- Composable with any orchestrator (cron, `xargs`, GitHub Actions matrix).
- Stateless — crash = re-run with the same input.
- Per-task cost trackable (one invocation = one budget envelope).

**Negative**
- No cross-task batching optimisation (sometimes useful for prompt cache reuse). Acceptable trade-off.
- Orchestrator must implement queue ordering. Out of scope for the runner itself.

## Alternatives considered

1. **Daemon mode.** Rejected per composability concerns.
2. **Multi-task per invocation (`--all`).** Considered. Defer until we need it; trivially added later.
3. **Embed inside the validator CLI.** Rejected — different concern, different failure modes.

## Source

`CONCEPT--CODEGEN-MICROTASK-RUNNER` + `msp_spec.md` §11.
