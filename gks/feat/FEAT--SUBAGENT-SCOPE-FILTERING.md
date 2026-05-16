---
id: FEAT--SUBAGENT-SCOPE-FILTERING
phase: 2
type: feat
status: active
tier: process
source_type: axiomatic
vault_id: default
title: Subagent scope filtering — task-scoped context with escalation
tags:
  - msp
  - ucf
  - feat
  - subagent
  - scope
  - codegen
crosslinks: {"references":["CONCEPT--SUBAGENT-CONTEXT-SCOPING","CONCEPT--ABAC-POLICY-ENGINE","ADR--TRANSPORT-AGNOSTIC-ENFORCEMENT","CONCEPT--RESOLUTION-GRADIENT"]}
created_at: 2026-05-14T19:42:03.565+07:00
---

# FEAT — Subagent scope filtering

> The user-facing API contract for `[[CONCEPT--SUBAGENT-CONTEXT-SCOPING]]`. The composer becomes a PEP (per `[[ADR--TRANSPORT-AGNOSTIC-ENFORCEMENT]]`) that filters subagent context by the parent-declared task scope.

## User-facing behaviour

`runTask` accepts a task descriptor whose `scope` block is declared by the **parent**:

```yaml
# .brain/tasks/FIX-LOGIN-MFA/T1.task.yaml
task:
  id: fix-login-mfa
  goal: "Add passkey support to login flow"
scope:
  needs: [auth, session, middleware]
  nice_to_have: [identity, telemetry]
  excludes: [schema-design, db-migration, patient-records]
budget:
  tokens: 8000
  allow_expand: true
  expand_limit: 5
```

The facade:

```ts
await layer.runTask('./.brain/tasks/FIX-LOGIN-MFA/T1.task.yaml', {
  scale: 'L2',
  // scope is read from the descriptor; the subagent cannot override it
})

// Escalation — the subagent's only scope autonomy
await layer.escalate({
  request_scope_extension: ['session-management'],
  reason: 'login fix requires the session-expiry path',
}): EscalationResult   // parent decides: approve | deny | widen
```

Behaviour contract:

- **Scope becomes part of `Subject.attributes`** when `Subject.kind = 'subagent'`. The composer PEP passes the full 4-tuple to the PDP for every candidate Resource.
- **`needs` / `excludes` are domain-attribute set tests**: a Resource is eligible iff `R.domain ∩ scope.needs ≠ ∅` **and** `R.domain ∩ scope.excludes = ∅`. `excludes` wins ties.
- **The subagent cannot set or widen its own scope.** Scope is read from the descriptor (authored by the parent). The only subagent autonomy is `escalate()`.
- **`escalate()` round-trips to the parent**: parent approves (scope widened, subagent retries), denies (subagent proceeds without, or fails loudly), or widens differently.
- **Effective context** = `namespace_filter ∩ user_abac ∩ task_scope ∩ resolution_tier` — scope filter runs before resolution tiering (per `[[CONCEPT--RESOLUTION-GRADIENT]]`).
- **Every filter-drop is audit-logged** — the log is the retrospective signal for whether a scope was too narrow.
- **`nice_to_have`** Resources are included only if budget remains after `needs`.

## Verification

- A task with `excludes: [patient-records]` produces a subagent context with zero atoms tagged `domain: patient-records`, **even when** the parent user's ABAC would permit them (the defense-in-depth property).
- The subagent has no API to mutate its own `scope` — attempting to returns a type error / runtime rejection.
- `escalate()` round-trips: subagent → parent decision → (on approve) widened scope → subagent retry succeeds.
- Composer audit log lists both included and filter-dropped Resources per task.
- Phase 2 quality A/B: scoped vs unscoped subagent on a representative coding task — scoped matches or beats unscoped at ≥30% lower token cost.

## Out of scope

- Auto-generation of `scope` from the task `goal` text — Phase 5+; MVP parents declare scope manually.
- Inter-subagent scope sharing / multi-subagent orchestration.
- The PDP itself — `[[FEAT--POLICY-DECISION-POINT]]`. This FEAT only wires the composer as a PEP.
- Resolution tiering — `[[FEAT--RESOLUTION-EXPAND-ON-DEMAND]]`. Scope filter runs *before* it; this FEAT does not own tiering.
- Domain tagging of atoms — classifier scope (Phase 6); MVP uses manually-tagged `domain` attributes.

## Source

- `docs/msp/UNIVERSAL-CONTEXT-FRAMEWORK_spec.md` §9 — task descriptor, escalation flow, effective-context formula.
- `[[CONCEPT--SUBAGENT-CONTEXT-SCOPING]]` — the concept this FEAT implements.
- `[[CONCEPT--ABAC-POLICY-ENGINE]]` — same PDP enforces task scope and user ABAC.
- `[[ADR--TRANSPORT-AGNOSTIC-ENFORCEMENT]]` — the composer is a PEP.
- `[[CONCEPT--RESOLUTION-GRADIENT]]` — composes after the scope filter.
