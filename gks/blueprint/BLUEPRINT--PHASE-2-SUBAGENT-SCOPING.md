---
id: BLUEPRINT--PHASE-2-SUBAGENT-SCOPING
phase: 3
type: blueprint
status: active
tier: process
source_type: axiomatic
vault_id: default
scale_level: L2
title: "BLUEPRINT — Phase 2 subagent scope filtering: first enforced PEP, with escalation"
tags:
  - msp
  - ucf
  - blueprint
  - phase-2
  - subagent
crosslinks: {"implements":["FEAT--SUBAGENT-SCOPE-FILTERING"],"references":["CONCEPT--SUBAGENT-CONTEXT-SCOPING","ADR--TRANSPORT-AGNOSTIC-ENFORCEMENT","BLUEPRINT--PHASE-1-PDP-SHADOW"]}
linked_symbols:
  - {"file":"packages/msp/src/policy/task-scope.ts"}
  - {"file":"packages/msp/src/policy/escalation.ts"}
created_at: 2026-05-14T22:21:53.742+07:00
---

# BLUEPRINT — Phase 2: subagent scope filtering

> Implementation plan for spec §11 Phase 2. The **first enforced PEP**: the composer flips from shadow to enforce, filtering subagent context by parent-declared task scope. Lowest-risk surface to graduate first because it has a human in the loop (the parent).

## Geography

New:

- `packages/msp/src/policy/task-scope.ts` — `SubagentScope` type + task-descriptor `scope` block parser.
- `packages/msp/src/policy/escalation.ts` — `escalate()` round-trip: subagent request → parent decision → widened scope or loud failure.
- `policies/10-subagent-scope.yaml` — the policy that evaluates `R.domain ∩ scope.needs` / `∩ scope.excludes`.

Touched:

- `packages/msp/src/codegen/master/composer.ts` — the shadow PEP from Phase 1 flips to **enforce** for `Subject.kind === 'subagent'`: candidate Resources failing the scope filter are dropped from the subagent context.
- `packages/msp/src/cognitive/index.ts` — `runTask` reads the `scope` block; exposes `escalate()` on the facade.
- `.brain/tasks/*/T*.task.yaml` schema — `scope: { needs, nice_to_have, excludes }` + `budget: { tokens, allow_expand, expand_limit }`.
- Subagent system prompt — instruction to `escalate()` on uncertainty rather than guess.

## Acceptance

- A task descriptor with `excludes: [patient-records]` produces a subagent context with **zero** atoms tagged `domain: patient-records` — verified even when the parent user's ABAC would permit them (the `user_abac ∩ task_scope` defense-in-depth property).
- The subagent has no API to set or widen its own `scope` — a test asserts the type/runtime rejection.
- `escalate()` round-trips: subagent request → parent `approve` widens scope and the retry succeeds; parent `deny` leaves scope unchanged.
- Composer audit log lists both included and filter-dropped Resources per task.
- **Quality A/B gate**: on a representative coding task, the scoped subagent matches or beats the unscoped baseline at ≥ 30% lower token cost. If it regresses task success, the phase does **not** ship — the scope rules are wrong.
- The composer PEP for non-subagent subjects stays in shadow mode (only `subagent` enforces in Phase 2).

## Dependencies

- `BLUEPRINT--PHASE-1-PDP-SHADOW` — the PDP + composer-as-PEP must exist in shadow mode first.
- `FEAT--SUBAGENT-SCOPE-FILTERING` — the API contract implemented.
- `CONCEPT--SUBAGENT-CONTEXT-SCOPING` — scope model + escalation pattern.
- `ADR--TRANSPORT-AGNOSTIC-ENFORCEMENT` — the composer is a registered PEP.
- Manually-tagged `domain` attributes on the test-bed atoms (classifiers are Phase 6).

## Tasks

1. **T2.1** — `task-scope.ts`: `SubagentScope` type + parse the descriptor `scope` block; reject subagent-side mutation.
2. **T2.2** — `policies/10-subagent-scope.yaml`: the set-intersection rule (`needs` eligible, `excludes` wins ties).
3. **T2.3** — Flip the composer PEP to enforce for `Subject.kind === 'subagent'`; drop scope-failing candidates before tiering.
4. **T2.4** — `escalation.ts`: `escalate()` round-trip + parent decision handling; audit-log each escalation.
5. **T2.5** — Subagent system-prompt update: escalate-on-uncertainty instruction.
6. **T2.6** — Manually tag `domain` on a top-20 test-bed atom set; use as the A/B fixture.
7. **T2.7** — Quality A/B harness: scoped vs unscoped on the fixture task; assert ≥30% token reduction without success regression. This is the **ship gate**.

## Source

- `docs/msp/UNIVERSAL-CONTEXT-FRAMEWORK_spec.md` §11 Phase 2, §9.
- `FEAT--SUBAGENT-SCOPE-FILTERING` — the contract implemented.
- `CONCEPT--SUBAGENT-CONTEXT-SCOPING` — scope + escalation model.
- `ADR--TRANSPORT-AGNOSTIC-ENFORCEMENT` — composer as PEP.
- `BLUEPRINT--PHASE-1-PDP-SHADOW` — predecessor phase.
