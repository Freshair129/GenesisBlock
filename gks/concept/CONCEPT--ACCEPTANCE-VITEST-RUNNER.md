---
id: CONCEPT--ACCEPTANCE-VITEST-RUNNER
phase: 1
type: concept
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: Vitest acceptance runner — sandbox candidate + spawn vitest
tags:
  - msp
  - codegen
  - acceptance
  - vitest
  - runtime
crosslinks: {"references":["FEAT--CODEGEN-MICROTASK-RUNNER","ADR--CODEGEN-RETRY-POLICY"]}
created_at: 2026-05-03T16:27:17.430+07:00
---

# CONCEPT — vitest acceptance runner

## Problem

The runner currently uses `defaultAcceptance` that returns `[]` always (always passes). Every SLM output that gets past pattern checks is treated as correct, even if it doesn't compile or fails the parent BLUEPRINT's tests. Without a real acceptance runner the codegen pipeline can never reject a hallucinated solution that *looks* clean.

## Hypothesis

If the runner spawns vitest in a sandbox containing the candidate code at the BLUEPRINT's geography paths plus the parent BLUEPRINT's verification tests, then vitest's pass/fail output (parsed) becomes the acceptance signal. SLM gets real test errors as retry context — same shape as a human iterating against `vitest --watch`.

## Scope

In:
- Write candidate code to `<sandbox>/<geography-path>` (sandbox is a tmp dir).
- Symlink `node_modules` from repo root to sandbox.
- Spawn `vitest run --reporter=json` against the sandbox; capture exit code + JSON.
- Parse failed test names + error messages → `string[]` matching the runner's `AcceptanceRunner` contract.
- Cleanup sandbox after each invocation.

Out:
- vitest config inheritance (sandbox uses a minimal config).
- Watch mode — single shot per call.
- Test file selection logic — vitest config picks up tests; we just spawn it.
- Caching / incremental — every call is fresh.

## Source

Closes M3c-4's residual ("real `acceptanceRunner` (spawning vitest in a sandbox) is not in this PR — default no-op"). P0 item #2 from production-readiness backlog.

## Connections
- [[FEAT--CODEGEN-MICROTASK-RUNNER]]
- [[ADR--CODEGEN-RETRY-POLICY]]

