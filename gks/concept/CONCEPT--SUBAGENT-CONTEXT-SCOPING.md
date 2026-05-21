---
id: CONCEPT--SUBAGENT-CONTEXT-SCOPING
phase: 1
type: concept
status: stable
tier: process
source_type: axiomatic
vault_id: default
title: Subagent context scoping — POLA for agents via task scope + escalation
tags: &a1
  - msp
  - ucf
  - concept
  - subagent
  - scope
  - pola
  - least-privilege
crosslinks: &a2
  references:
    - FRAMEWORK--UNIVERSAL-CONTEXT-FRAMEWORK
    - CONCEPT--ABAC-POLICY-ENGINE
    - CONCEPT--SUBJECT-RESOURCE-ACTION-CONTEXT
    - CONCEPT--RESOLUTION-GRADIENT
created_at: 2026-05-13T17:22:03.289+07:00
aliases: &a3
  - CONCEPT
  - implementation_flow
  - Strategic intent / PRD
cluster: implementation_flow
role: Strategic intent / PRD
attributes:
  id: CONCEPT--SUBAGENT-CONTEXT-SCOPING
  phase: 1
  type: concept
  status: stable
  tier: process
  source_type: axiomatic
  vault_id: default
  title: Subagent context scoping — POLA for agents via task scope + escalation
  tags: *a1
  crosslinks: *a2
  created_at: 2026-05-13T17:22:03.289+07:00
  aliases: *a3
  cluster: implementation_flow
  role: Strategic intent / PRD
  attributes:
    id: CONCEPT--SUBAGENT-CONTEXT-SCOPING
    phase: 1
    type: concept
    status: stable
    tier: process
    source_type: axiomatic
    vault_id: default
    title: Subagent context scoping — POLA for agents via task scope + escalation
    tags: *a1
    crosslinks: *a2
    created_at: 2026-05-13T17:22:03.289+07:00
    aliases: *a3
    cluster: implementation_flow
    role: Strategic intent / PRD
    attributes:
      id: CONCEPT--SUBAGENT-CONTEXT-SCOPING
      phase: 1
      type: concept
      status: stable
      tier: process
      source_type: axiomatic
      vault_id: default
      title: Subagent context scoping — POLA for agents via task scope + escalation
      tags: *a1
      crosslinks: *a2
      created_at: 2026-05-13T17:22:03.289+07:00
      aliases: *a3
      cluster: implementation_flow
      role: Strategic intent / PRD
      attributes:
        domain: concept
      domain: concept
      language: markdown
      is_test: false
      is_entrypoint: false
      has_secret: true
      secret_type: aws_secret
      leak_risk: high
      encryption_level: none
    domain: concept
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: true
    secret_type: aws_secret
    leak_risk: high
    encryption_level: none
  domain: concept
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: true
  secret_type: aws_secret
  leak_risk: high
  encryption_level: none
---

# CONCEPT — Subagent context scoping

> Applies the **principle of least privilege (POLA)** to subagents: each subagent receives only the context its task requires, declared by the **parent** (not the subagent) as a task scope, enforced by the same ABAC machinery as user-level access, with an **escalation pattern** for missing context that surfaces — never silent.
>
> Spec section: `UNIVERSAL-CONTEXT-FRAMEWORK_spec.md` §9.

## Problem

When a parent agent spawns subagents (codegen, retrieval, validation, summarisation), each subagent is a small LLM session with its own context window. Today's standard practice is to give subagents **broad** context "just in case" — the parent dumps the relevant atoms, plus neighbours, plus task description, plus a few defensively-included extras.

This is both **wasteful** and **dangerous**:

- **Wasteful** — every subagent pays the token cost of context it never uses.
- **Dangerous** — a prompt-injection vector in any single atom of the loaded context can cause the subagent to exfiltrate or misuse **everything else** in its context.

A subagent fixing the login flow does not need patient records, database schemas, marketing copy, or the deploy runbook. Loading them is wrong on both axes: it bloats the context window **and** it expands the blast radius of any successful prompt injection.

POLA — the same principle Unix uses for process privileges — applies. Subagents should see only what their task requires.

## Hypothesis

A **task scope descriptor** authored by the parent, enforced by the same ABAC PDP as user-level access, with **mandatory escalation** when the scope is insufficient, gives MSP fine-grained subagent isolation without breaking the parent-subagent collaboration model.

Critically: the **subagent does not set its own scope**. If it could, it would (rationally) always declare maximum scope to be safe. The parent constrains; the subagent operates within the constraint or escalates.

## Scope

In:

- `SubagentScope` type carried in `Subject.attributes` when `Subject.kind = 'subagent'`:

  ```ts
  interface SubagentScope {
    needs: string[]           // domain attributes required
    nice_to_have: string[]    // load if budget permits
    excludes: string[]        // explicit forbid list
    budget_tokens: number
    allow_expand: boolean
    expand_limit?: number     // max expansions per task
  }
  ```

- Task descriptor schema in `.brain/tasks/<TASK-ID>/T*.task.yaml` carrying the scope.
- PEP in the composer (`packages/msp/src/codegen/master/composer.ts`) consults the PDP per candidate Resource using the scope as part of Subject.attributes.
- The effective-context formula (see §10 of spec):

  ```
  include(R, subagent) =
      namespace_filter(R, vault)
    ∩ user_abac_filter(R, parent_user)
    ∩ task_scope_filter(R, subagent.scope)
    ∩ resolution_tier(R, score, budget) > omitted
  ```

- **Escalation API** so the subagent can request a scope extension mid-task:

  ```ts
  await layer.escalate({
    request_scope_extension: ['session-management'],
    reason: 'login fix requires understanding session expiry path'
  })
  ```

  Parent receives, decides; subagent retries with widened scope or fails loudly.
- Audit log entries for: scope assignment, filter drops, escalations, parent decisions on escalation.

Out:

- Auto-scope generation from task description. Initial parents declare scope manually; later phases may learn typical scopes per task class.
- Cross-subagent scope sharing. Each subagent gets its own scope; siblings can have disjoint scopes.
- Adaptive scope tightening ("subagent never expanded these → drop them next time"). Phase 5+ enhancement.

## Why subagent doesn't set its own scope

An LLM asked to declare its own task scope will default to **maximum** scope ("I might need anything"). This is rational under uncertainty but defeats POLA. Mirror Unix's `setuid`: declared elevation is allowed; arbitrary self-promotion is not.

The parent — which has full context and a defined goal — is the right declarer. If it gets the scope wrong (too narrow), the **escalation pattern** surfaces the problem loudly rather than silently degrading subagent output.

## Why escalation is load-bearing

Without escalation, a too-narrow scope produces a **silent quality drop**: the subagent answers the question but answers it badly because it missed crucial context. The parent never knows.

With escalation, the subagent **must** surface "I might be missing context X" rather than guess. The parent decides — approve, deny, or widen task scope and retry. This makes scope errors **observable** instead of **invisible**, which is the only way to iteratively tighten scope without compromising quality.

The contract is enforced by:

1. Composer logs every filter-drop so retrospective analysis can spot under-included Resources.
2. Subagents instructed (via system prompt) to escalate when uncertain rather than guess.
3. Per-task escalation count is a quality signal — zero escalations on a hard task is suspicious.

## Defense-in-depth with user ABAC

The chain `user_abac ∩ task_scope` is the highest-value property of the entire framework:

> Dr. Alice is logged in and authorized to view patient records. She launches a subagent to fix the login bug. The subagent's `excludes` includes `patient-records`.
>
> Even though Alice's session would permit access to patient records, the task scope forbids them. The subagent **cannot** see them — eliminating the risk that a prompt injection in any atom causes the subagent to exfiltrate patient records.

User-level access control alone cannot achieve this. Many real-world incidents (Twitter 2020 admin tool, internal-IT helpdesk supply chain) trace to over-broad ambient privileges. Task scoping at the subagent layer is the systemic answer.

## Composition with Resolution Gradient

Task scope **filters which Resources are eligible**; resolution gradient **chooses the tier of those eligible**. They compose in order: scope first, tier second.

A Resource that fails the scope filter has no tier; it does not appear in the subagent context at any resolution. This is the correct ordering because the alternative — assigning a SKELETON tier to scope-failing Resources — would still leak their existence and title.

## Verification

- A subagent task with `excludes: [patient-records]` cannot recall any atom tagged `domain: patient-records`, even if user-level ABAC would permit.
- `escalate()` calls round-trip: subagent → parent → updated scope → subagent retry.
- A run-task descriptor with scope produces audit-log entries showing both included and filter-dropped Resources.
- Quality A/B in Phase 2: scoped subagent on a representative coding task matches or exceeds unscoped subagent at ≥30% lower token cost.

## Out of scope

- Auto-generation of scope from task descriptors (Phase 5+).
- Per-subagent step-up authentication (separate concept, mostly applies to user actions not subagent actions).
- Inter-subagent communication scoping (multi-subagent orchestration is a separate problem).

## Source

- `packages/msp/docs/UNIVERSAL-CONTEXT-FRAMEWORK_spec.md` §9 — task descriptor, escalation flow, effective-context formula.
- `[[FRAMEWORK--UNIVERSAL-CONTEXT-FRAMEWORK]]` §10 Layer 2 — where task scope fits in the pipeline.
- `[[CONCEPT--ABAC-POLICY-ENGINE]]` — same PDP enforces both user-ABAC and task-scope.
- `[[CONCEPT--SUBJECT-RESOURCE-ACTION-CONTEXT]]` — Subject.kind = 'subagent' carries scope in attributes.
- `[[CONCEPT--RESOLUTION-GRADIENT]]` — composes after scope filter.
- Saltzer & Schroeder (1975) "The Protection of Information in Computer Systems" — POLA origin.
