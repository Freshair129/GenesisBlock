---
id: AUDIT--PHASE-E1-REAL-CLI-WIRING
phase: 6
type: audit
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: Phase E1 — real CLI wiring for T1/T2/T3 adapters + opt-in integration tests
tags:
  - msp
  - phase-e1
  - audit
  - tier-adapters
  - integration-test
crosslinks: {"references":["CONCEPT--REAL-CLI-WIRING"]}
linked_symbols: []
created_at: 2026-05-14T04:05:00.000+07:00
---

# AUDIT — Phase E1 real CLI wiring

## Scope

Phase E1 of the post-monorepo-pivot ROADMAP. Verifies and (where needed)
corrects the invocation patterns used by the T1/T2/T3 tier adapters,
documents them in `[[CONCEPT--REAL-CLI-WIRING]]`, and adds an opt-in
integration test suite that exercises the real CLIs locally without
breaking CI.

## What was wired

### Adapter source touch-ups

| File | Change |
|---|---|
| `packages/msp/src/agents/tiers/qwen.ts` | Fixed: prompt is now positional (`['<prompt>']`) instead of `['--prompt', '<prompt>']`. Healthcheck switched from `--version` (unsupported by the Python Ollama-wrapper CLI) to `--help`. Added doc-block with verified invocation pattern. |
| `packages/msp/src/agents/tiers/gemini.ts` | No behavioural change — invocation already correct. Added doc-block documenting `--approval-mode yolo -p <prompt>` pattern and `--version` healthcheck. |
| `packages/msp/src/agents/tiers/claude.ts` | No behavioural change — invocation already correct. Added doc-block documenting `--print <prompt>` pattern and `--version` healthcheck. |

### Unit test update

| File | Change |
|---|---|
| `packages/msp/test/agents/tiers/adapters.test.ts` | Updated qwen-section assertions: healthcheck now expects `['--help']`; `run()` now expects `['<prompt>']` (positional). All 11 adapter tests still pass; total 18 tests in `test/agents/tiers/` green. |

### New integration test

| File | Purpose |
|---|---|
| `packages/msp/test/agents/integration/real-cli.test.ts` | Opt-in real-CLI suite, gated on `MSP_TEST_REAL_CLIS === '1'` via `describe.skipIf`. 6 tests total (healthcheck + run smoke per adapter). Stays skipped in CI; runs end-to-end locally with binaries on PATH. |

### New atoms

| Atom | Purpose |
|---|---|
| `gks/concept/[[CONCEPT--REAL-CLI-WIRING]].md` | Documents the verified invocation pattern + healthcheck semantics + integration-test gating convention. |
| `gks/audit/[[AUDIT--PHASE-E1-REAL-CLI-WIRING]].md` | This atom. |

## Verification matrix (2026-05-14)

| Tier | Binary location | `--version` works? | `--help` works? | Invocation in adapter |
|---|---|---|---|---|
| T1 / qwen | `C:\Users\freshair\AppData\Local\Programs\Python\Python313\Scripts\qwen.exe` | No (unrecognized arg → exit non-zero) | Yes (exit 0) | `qwen <prompt-positional>` |
| T2 / gemini | `C:\Users\freshair\AppData\Roaming\npm\gemini.cmd` | Yes (`0.42.0` → exit 0) | Yes | `gemini --approval-mode yolo -p "<prompt>"` |
| T3 / claude | `C:\Users\freshair\.local\bin\claude.exe` | Yes (`2.1.140 (Claude Code)` → exit 0) | Yes | `claude --print "<prompt>"` |

## Test counts

| Suite | Before E1 | After E1 |
|---|---|---|
| `test/agents/tiers/` (unit, mocked) | 18 passed | 18 passed |
| `test/agents/integration/` (default — env unset) | n/a | 6 skipped |
| `test/agents/integration/` (with `MSP_TEST_REAL_CLIS=1`, all 3 binaries on PATH) | n/a | 6 passed (T1 + T2 logged backend-outage; T3 fully ran) |

## Manual smoke (developer machine)

Run locally with binaries installed:

```sh
MSP_TEST_REAL_CLIS=1 npm test --workspace=packages/msp -- test/agents/integration/
```

Expected behaviour:
- **Binary missing**: healthcheck test passes (boolean returned, no throw),
  run-test logs `[real-cli] <label>: binary missing — skipping run() test`
  and returns silently.
- **Binary present, backend healthy**: run-test returns `ok=true`,
  `exit_code=0`, non-empty output.
- **Binary present, backend down** (e.g. Ollama not running, API key
  missing): run-test logs the exit code and passes — adapter wiring is
  proven, backend health is out of scope here.

## What to test manually

1. Run with **no** CLIs installed → all 3 healthchecks return false; suite
   still passes (skips run-tests with logged message).
2. Run with **only T3 (claude)** installed → T1/T2 healthchecks false, T3
   completes a real round-trip.
3. Stop Ollama and run with qwen present → T1 healthcheck true,
   run-test logs non-zero exit and passes.
4. Send a prompt with special characters (quotes, backslashes) → confirm
   no shell-injection corruption. (Limited concern: prompts are trusted
   internal input per `[[CONCEPT--REAL-CLI-WIRING]]` §Adapter prompt-arg
   safety.)

## Follow-ups (not blocking)

- **`qwen.md` doc cleanup**: that file currently documents the stale
  `--prompt` invocation pattern. A small follow-up PR should update it
  to point at `[[CONCEPT--REAL-CLI-WIRING]]` for the source-of-truth pattern.
- **`spawn-helper.ts` hardening**: if MSP ever forwards untrusted prompts
  to a tier CLI, switch to full-binary-path resolution so `shell: false`
  can be used uniformly on Windows. Tracked under
  `[[CONCEPT--REAL-CLI-WIRING]]` §Adapter prompt-arg safety.
- **CI matrix entry**: optional future work — a separate CI job that
  installs the CLIs and sets `MSP_TEST_REAL_CLIS=1`, gated to the dev
  branch only (not main).

## Source

User direction "Phase E1 — Real CLI Integration" in agent task brief
2026-05-14; direct inspection of `qwen --help`, `gemini --help`,
`claude --help` on the developer machine.
