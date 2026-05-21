---
id: ADR--POLICY-AS-DATA-NOT-CODE
phase: 2
type: adr
status: stable
tier: process
source_type: axiomatic
vault_id: default
title: Policy as data, not code — YAML + minimal operators for v1
tags: &a1
  - msp
  - ucf
  - adr
  - abac
  - policy
crosslinks: &a2
  references:
    - CONCEPT--ABAC-POLICY-ENGINE
    - FRAMEWORK--UNIVERSAL-CONTEXT-FRAMEWORK
created_at: 2026-05-14T18:37:51.127+07:00
aliases: &a3
  - ADR
  - implementation_flow
  - Architecture decision record
cluster: implementation_flow
role: Architecture decision record
attributes:
  id: ADR--POLICY-AS-DATA-NOT-CODE
  phase: 2
  type: adr
  status: stable
  tier: process
  source_type: axiomatic
  vault_id: default
  title: Policy as data, not code — YAML + minimal operators for v1
  tags: *a1
  crosslinks: *a2
  created_at: 2026-05-14T18:37:51.127+07:00
  aliases: *a3
  cluster: implementation_flow
  role: Architecture decision record
  attributes:
    id: ADR--POLICY-AS-DATA-NOT-CODE
    phase: 2
    type: adr
    status: stable
    tier: process
    source_type: axiomatic
    vault_id: default
    title: Policy as data, not code — YAML + minimal operators for v1
    tags: *a1
    crosslinks: *a2
    created_at: 2026-05-14T18:37:51.127+07:00
    aliases: *a3
    cluster: implementation_flow
    role: Architecture decision record
    attributes:
      id: ADR--POLICY-AS-DATA-NOT-CODE
      phase: 2
      type: adr
      status: stable
      tier: process
      source_type: axiomatic
      vault_id: default
      title: Policy as data, not code — YAML + minimal operators for v1
      tags: *a1
      crosslinks: *a2
      created_at: 2026-05-14T18:37:51.127+07:00
      aliases: *a3
      cluster: implementation_flow
      role: Architecture decision record
      attributes:
        domain: adr
      domain: adr
      language: markdown
      is_test: false
      is_entrypoint: false
      has_secret: true
      secret_type: high_entropy_string
      leak_risk: high
      encryption_level: none
    domain: adr
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: true
    secret_type: high_entropy_string
    leak_risk: high
    encryption_level: none
  domain: adr
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: true
  secret_type: high_entropy_string
  leak_risk: high
  encryption_level: none
---

# ADR — Policy as data, not code

> Resolves decision **D-1** in `UNIVERSAL-CONTEXT-FRAMEWORK_spec.md` §0.

## Context

`[[CONCEPT--ABAC-POLICY-ENGINE]]` requires a Policy Decision Point (PDP) consulted at every entry point. The rules the PDP evaluates can be expressed three ways:

1. **As code** — TypeScript functions, one per rule, compiled into MSP.
2. **As a custom DSL** — a small YAML / JSON dialect with a bounded operator set, interpreted at runtime.
3. **As an embedded policy engine** — Cedar (AWS) or OPA / Rego, run in-process or as a sidecar.

The choice is load-bearing: it determines who can author policy, how fast a policy change ships, how testable rules are, and whether domain teams need engineers. The decision must also keep a migration path open — a v1 choice that paints us into a corner is unacceptable.

Constraints from the spec:

- Domain teams (medical, finance, legal) must be able to author and audit policy **without editing framework code**.
- Policy changes must hot-reload — no deploy.
- The PDP must stay a pure function (testable with fixtures).
- v1 timeline is tight; the policy surface is initially small (single-digit rules).

## Decision

**Adopt option 2 — a minimal YAML dialect with a small built-in operator set — for v1.** Estimated ~200 LOC for parser + evaluator.

- **Operators**: equality, membership (`in` / `not_in`), set operations (`∩`, `∪`, `∖`), basic arithmetic and time comparison. No recursion, no user-defined functions.
- **File layout**: `policies/*.yaml`, each file a list of rules with `id`, `match`, `condition`, `effect`, `obligations`, `advice`.
- **Hot reload**: a file watcher bumps a monotonic policy-version counter; decision cache keys include the counter.
- **Linting**: at load time, warn on unknown attributes (cross-checked against declared attribute schemas) and detect trivially contradictory rules.

**Migration path to Cedar is explicit and mechanical.** Each YAML rule maps 1:1 to a Cedar permission; the trigger to migrate is policy count exceeding ~30 rules **or** any need for recursion / formal verification.

## Consequences

Positive:

- Domain teams author policy in YAML — no TypeScript, no deploy, no engineer in the loop.
- The PDP stays a pure function: parse YAML once, evaluate against the 4-tuple.
- Rules are fixture-testable — a policy file plus a set of `(subject, resource, action, context) → expected decision` cases.
- Hot reload makes shadow-mode iteration (per `[[ADR--DEFAULT-POLICY-POSTURE]]`) fast.
- ~200 LOC is small enough to audit fully.

Negative / accepted costs:

- Expressive ceiling: no recursion, no custom functions. Accepted deliberately — when a rule needs more than the operator set, that is the migration signal, not a reason to extend the DSL ad hoc.
- A second parser to maintain until the Cedar migration. Bounded by the ~200 LOC budget.
- Risk of DSL scope creep. Mitigated by a hard rule: **new operators require an ADR**; the default answer to "can we add operator X" is "that is the Cedar trigger."

## Alternatives considered

**Option 1 — policy as TypeScript code.** Rejected: every policy change is a deploy; rules scatter across modules; domain teams cannot contribute; auditing requires reading code. The only upside (full language power) is precisely what we do **not** want at the policy layer.

**Option 3 — Cedar / OPA from day one.** Rejected for v1, not forever. Cedar is the intended v2 backend. Rejected now because: embedding the runtime (Java for some OPA paths, or the Rust Cedar engine) adds dependency weight and a learning curve disproportionate to a single-digit-rule policy surface; and starting with the simpler DSL lets us learn what expressiveness we actually need before committing to Cedar's model.

**Option 2b — JSON instead of YAML.** Rejected: YAML's comments and readability matter for human-authored, human-audited policy files. The parsing cost difference is negligible.

## Source

- `docs/msp/UNIVERSAL-CONTEXT-FRAMEWORK_spec.md` §0 (D-1), §7.3 — policy file format and migration trigger.
- `[[CONCEPT--ABAC-POLICY-ENGINE]]` — the PDP this policy format feeds.
- Cedar (AWS) — designated v2 migration target.
- Open Policy Agent / Rego — alternative v2 backend.

## Connections

- [[FRAMEWORK--UNIVERSAL-CONTEXT-FRAMEWORK]]
