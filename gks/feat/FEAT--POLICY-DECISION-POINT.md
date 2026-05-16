---
id: FEAT--POLICY-DECISION-POINT
phase: 2
type: feat
status: active
tier: process
source_type: axiomatic
vault_id: default
title: Policy Decision Point — the pure-function PDP and its API
tags:
  - msp
  - ucf
  - feat
  - abac
  - pdp
crosslinks: {"references":["CONCEPT--ABAC-POLICY-ENGINE","ADR--POLICY-AS-DATA-NOT-CODE","ADR--DEFAULT-POLICY-POSTURE","ADR--TRANSPORT-AGNOSTIC-ENFORCEMENT","CONCEPT--SUBJECT-RESOURCE-ACTION-CONTEXT"]}
created_at: 2026-05-14T19:42:01.331+07:00
---

# FEAT — Policy Decision Point

> The user-facing API contract for the PDP described in `[[CONCEPT--ABAC-POLICY-ENGINE]]`, shaped by `[[ADR--POLICY-AS-DATA-NOT-CODE]]` (YAML v1) and `[[ADR--DEFAULT-POLICY-POSTURE]]` (default-permit + shadow).

## User-facing behaviour

A new module `packages/msp/src/policy/` exports:

```ts
// The PDP — a pure function. No I/O, no side effects.
function evaluatePolicy(
  subject: Subject,
  resource: Resource,
  action: Action,
  context: RequestContext,
): Decision

// Policy set lifecycle
function loadPolicies(dir: string): PolicySet           // parse policies/*.yaml
function watchPolicies(dir: string, onReload: () => void): Disposable

interface Decision {
  effect: 'permit' | 'deny' | 'indeterminate'
  obligations: Obligation[]
  advice: Advice[]
  reasoning: ReasonTrace[]      // which rules matched, which conditions held
  ttl_seconds?: number
}
```

Behaviour contract:

- **Pure**: `evaluatePolicy` given the same 4-tuple and the same `PolicySet` always returns the same `Decision`. No clock reads except `context.time` (passed in), no file I/O, no network.
- **Default posture is configurable per endpoint**: with no matching rule, the effect is `policySet.defaultEffect[endpoint]` — `permit` in Phase 1 per `[[ADR--DEFAULT-POLICY-POSTURE]]`.
- **YAML operator set** (per `[[ADR--POLICY-AS-DATA-NOT-CODE]]`): equality, `in` / `not_in`, set `∩ ∪ ∖`, arithmetic, time comparison. Unknown operators are a load-time error.
- **Hot reload**: `watchPolicies` bumps a monotonic `policy_version`; callers include it in cache keys.
- **Explainability**: every `Decision` carries a non-empty `reasoning` array — even a default-effect decision records "no rule matched, applied default".
- **Linting on load**: unknown attribute names (cross-checked against declared schemas) and trivially contradictory rules produce warnings.

A CLI `msp-policy` is provided for operators:

```sh
msp-policy lint policies/                    # validate the policy set
msp-policy explain --subject=… --resource=… --action=recall    # dry-run one decision
msp-policy shadow-report                     # would-have-denied summary from the shadow log
```

## Verification

- Unit: `evaluatePolicy` called twice with identical inputs returns identical `Decision` (purity).
- Unit: a 4-tuple matching no rule returns the endpoint's `defaultEffect` with a `reasoning` entry naming the default.
- Unit: each operator (`in`, `not_in`, set ops, arithmetic, time) evaluated against fixtures.
- Integration: `watchPolicies` picks up a new YAML file without restart; `policy_version` increments.
- Integration: `msp-policy explain` reproduces the same `Decision` the runtime PDP would return.
- Lint: a policy referencing an undeclared attribute emits a warning; a rule that both permits and denies the same match emits a contradiction warning.

## Out of scope

- PEP wiring at the transports — see `[[ADR--TRANSPORT-AGNOSTIC-ENFORCEMENT]]` and the per-transport PEP FEATs.
- The Cedar / OPA migration — separate ADR when triggered.
- Domain policy packs (`pack-pii`, `pack-medical`, …) — these are *content* loaded by `loadPolicies`, not part of this FEAT.
- Decision cache implementation — the PDP returns `ttl_seconds`; the cache is a PEP-side concern.
- Step-up mechanics — the PDP only emits `advice: ['request-step-up-auth']`; fulfilment is `[[FEAT--STEP-UP-AUTH-PIN]]`.

## Source

- `docs/msp/UNIVERSAL-CONTEXT-FRAMEWORK_spec.md` §7 — PDP architecture, decision shape.
- `[[CONCEPT--ABAC-POLICY-ENGINE]]` — the concept this FEAT implements.
- `[[ADR--POLICY-AS-DATA-NOT-CODE]]` — YAML format + operator set.
- `[[ADR--DEFAULT-POLICY-POSTURE]]` — default-permit + shadow log.
- `[[ADR--TRANSPORT-AGNOSTIC-ENFORCEMENT]]` — how PEPs consume this PDP.
- `[[CONCEPT--SUBJECT-RESOURCE-ACTION-CONTEXT]]` — the 4-tuple input shape.
