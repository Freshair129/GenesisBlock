---
id: BLUEPRINT--PHASE-1-PDP-SHADOW
phase: 3
type: blueprint
status: active
tier: process
source_type: axiomatic
vault_id: default
scale_level: L2
title: "BLUEPRINT — Phase 1 PDP in shadow mode: evaluate and log, never enforce"
tags:
  - msp
  - ucf
  - blueprint
  - phase-1
  - abac
crosslinks: {"implements":["FEAT--POLICY-DECISION-POINT"],"references":["CONCEPT--ABAC-POLICY-ENGINE","ADR--POLICY-AS-DATA-NOT-CODE","ADR--DEFAULT-POLICY-POSTURE","BLUEPRINT--PHASE-0-PLUMBING"]}
linked_symbols:
  - {"file":"packages/msp/src/policy/pdp.ts"}
  - {"file":"packages/msp/src/policy/loader.ts"}
  - {"file":"packages/msp/src/policy/operators.ts"}
  - {"file":"packages/msp/src/policy/shadow-log.ts"}
created_at: 2026-05-14T22:21:52.912+07:00
---

# BLUEPRINT — Phase 1: PDP in shadow mode

> Implementation plan for spec §11 Phase 1. Builds the Policy Decision Point and wires **one** PEP (at `runTask`) in **shadow mode** — decisions are computed and logged with full reasoning, but enforcement is off. Output is identical to Phase 0; the deliverable is the shadow log.

## Geography

New:

- `packages/msp/src/policy/pdp.ts` — `evaluatePolicy(subject, resource, action, context): Decision`. Pure function (~300 LOC).
- `packages/msp/src/policy/loader.ts` — YAML policy parser + `watchPolicies` hot-reload + load-time linter.
- `packages/msp/src/policy/operators.ts` — the minimal operator set (`equals`, `in`, `not_in`, set `∩ ∪ ∖`, arithmetic, time).
- `policies/` (repo root) — `00-default-permit.yaml` starter policy (`log-everything-permit-everything`).
- `packages/msp/src/policy/shadow-log.ts` — append-only shadow log writer + `shadow-report` aggregator.
- `apps/cli` or `packages/msp` bin — `msp-policy` CLI (`lint` / `explain` / `shadow-report`).

Touched:

- `packages/msp/src/codegen/master/composer.ts` (or `runTask` entry) — wrap as a PEP: build the 4-tuple, call `evaluatePolicy`, **log the Decision, do not act on it**.

## Acceptance

- `evaluatePolicy` is pure — a property test asserts identical inputs → identical `Decision`, and a `grep` confirms no I/O imports in `pdp.ts`.
- `loadPolicies` parses `policies/*.yaml`; `watchPolicies` bumps `policy_version` on file change without restart (integration test).
- The starter policy permits everything; every `Decision` carries a non-empty `reasoning` array (even "no rule matched → default-permit").
- `runTask` output is **byte-identical** to Phase 0 for a fixture task — shadow mode changes nothing observable.
- The shadow log accumulates one entry per `runTask` invocation; `msp-policy shadow-report` summarises would-have-denied counts (zero, with the permit-everything starter policy).
- `msp-policy lint` flags an unknown attribute and a trivially contradictory rule in a fixture policy file.
- Default posture is `default-permit` per `[[ADR--DEFAULT-POLICY-POSTURE]]`.

## Dependencies

- `[[BLUEPRINT--PHASE-0-PLUMBING]]` — the 4-tuple must already be threaded to `runTask`.
- `[[FEAT--POLICY-DECISION-POINT]]` — the API contract this phase implements.
- `[[ADR--POLICY-AS-DATA-NOT-CODE]]` — YAML format + operator set.
- `[[ADR--DEFAULT-POLICY-POSTURE]]` — shadow mode + default-permit.
- `[[CONCEPT--ABAC-POLICY-ENGINE]]` — PDP/PEP architecture.

## Tasks

1. **T1.1** — `operators.ts`: implement + unit-test each operator against fixtures.
2. **T1.2** — `loader.ts`: YAML parse → `PolicySet`; load-time linter (unknown attributes, contradictions); `watchPolicies` with monotonic `policy_version`.
3. **T1.3** — `pdp.ts`: `evaluatePolicy` — match rules, apply first/strongest effect, fall through to per-endpoint `defaultEffect`, always emit `reasoning`. Property-test purity.
4. **T1.4** — `policies/00-default-permit.yaml` — the starter policy.
5. **T1.5** — `shadow-log.ts`: append-only writer keyed by `trace_id`; `shadow-report` aggregator.
6. **T1.6** — Wrap `runTask` as a shadow PEP: build tuple, call PDP, log Decision, **do not enforce**. Assert output unchanged via fixture diff.
7. **T1.7** — `msp-policy` CLI: `lint`, `explain` (dry-run one decision), `shadow-report`.
8. **T1.8** — Docs: a short `policies/README.md` for operators authoring their first rule.

## Source

- `docs/msp/UNIVERSAL-CONTEXT-FRAMEWORK_spec.md` §11 Phase 1, §7.
- `[[FEAT--POLICY-DECISION-POINT]]` — the contract implemented.
- `[[ADR--POLICY-AS-DATA-NOT-CODE]]`, `[[ADR--DEFAULT-POLICY-POSTURE]]` — governing decisions.
- `[[BLUEPRINT--PHASE-0-PLUMBING]]` — predecessor phase.
