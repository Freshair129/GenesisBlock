---
id: CONCEPT--ABAC-POLICY-ENGINE
phase: 1
type: concept
status: stable
vault_id: default
tier: process
source_type:
title: ABAC policy engine — PDP / PEP separation, policy as data
tags:
  - msp
  - ucf
  - concept
  - abac
  - pdp
  - pep
  - policy
crosslinks: 
  references:
    - FRAMEWORK--UNIVERSAL-CONTEXT-FRAMEWORK
    - CONCEPT--SUBJECT-RESOURCE-ACTION-CONTEXT
    - CONCEPT--ATTRIBUTE-BAG-MODEL
created_at: 2026-05-13T17:22:01.640+07:00
aliases:
  - CONCEPT
  - implementation_flow
  - Strategic intent
  - PRD
cluster: implementation_flow
role: Strategic_intent_PRD
attributes:
  domain:
    - ucf
    - msp
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: true
  secret_type: high_entropy_string
  leak_risk: high
  encryption_level: none
---

# CONCEPT — ABAC policy engine

> The **WHO** axis of UCF. Decisions flow from a **Policy Decision Point** (pure function) to **Policy Enforcement Points** (one per transport surface). Policy is expressed as **data**, not code, and the engine is **domain-agnostic** — domain knowledge lives in policy files and classifier output.
>
> Spec section: `UNIVERSAL-CONTEXT-FRAMEWORK_spec.md` §7. Related decisions: D-1 (YAML language), D-7 (default-permit + shadow log).

## Problem️

MSP today does not have a uniform access control mechanism. Checks (when present) are scattered across modules: HTTP routes, MCP tools, the facade. Each location interprets caller identity differently, audits inconsistently, and bypasses easily. Adding a new check requires editing N call sites; removing a check requires finding them all.

Worse, **domain-specific** rules (HIPAA, PCI, internal-only) get hardcoded into the framework if no clean separation exists. A medical-agent maintainer ends up patching the same files a finance-agent maintainer would, and the two diverge.

We need a **single decision authority** consulted at **every** entry point, where the rules are **data files** that domain teams can author, audit, and hot-reload independently of code.

## Hypothesis

The classic XACML / OPA / Cedar architecture — **PDP / PEP separation** with **policy-as-data** — solves this:

- **Policy Decision Point (PDP)** is a pure function `(subject, resource, action, context) → decision`. No I/O, no side effects. Deterministic, cacheable, testable in isolation.
- **Policy Enforcement Points (PEPs)** are per-transport interceptors that consult the PDP and enforce the result. One PEP per entry surface — HTTP, MCP, facade, composer, embedder.
- **Policy** is a set of YAML files (per D-1) loaded at startup with hot-reload. Domain teams add policies by dropping files; the framework runtime is untouched.

The decision returned is **richer than allow/deny** — it carries obligations (mandatory follow-ups like redaction or logging) and advice (suggestions like "request step-up"), plus a reasoning trace for explainable audit.

## Scope

In:

- **PDP module** `packages/msp/src/policy/pdp.ts` (new) exporting `evaluatePolicy(subject, resource, action, context): Decision`.
- **Policy loader** that reads `policies/*.yaml`, parses with minimal operator set (`equals`, `in`, `not_in`, set ∩/∪/∖, arithmetic), hot-reloads on file change.
- **Decision type** = `{ effect: 'permit' | 'deny' | 'indeterminate', obligations: Obligation[], advice: Advice[], reasoning: ReasonTrace[], ttl_seconds?: number }`.
- **PEP interfaces** for HTTP middleware, MCP tool wrapper, recall interceptor, compose interceptor, embed interceptor — installed per `[[ADR--TRANSPORT-AGNOSTIC-ENFORCEMENT]]`.
- **Decision caching** keyed by `(subject, resource, action)` hash; invalidated on policy-version bump.
- **Default posture** per D-7: `default-permit` + shadow log in Phase 1; tighten per-endpoint to `default-deny` from Phase 3.
- **Policy linter** that warns on unknown attributes and contradictory rules at load time.

Out:

- The **policy language v2** — Cedar / OPA migration is a separate ADR once policy count or expressiveness exceeds YAML v1.
- **Domain-specific policy packs** — those ship separately (`pack-pii`, `pack-multi-tenant`, `pack-medical`).
- **Step-up authentication** — triggered by a policy obligation, but the mechanism lives in `[[CONCEPT--STEP-UP-AUTH]]`.
- **The 4-tuple shape** — see `[[CONCEPT--SUBJECT-RESOURCE-ACTION-CONTEXT]]`.
- **Attribute schema** — see `[[CONCEPT--ATTRIBUTE-BAG-MODEL]]`.

## Why policy-as-data, not code

| Code-based rules | Data-based rules |
|---|---|
| Each change requires deploy | Hot-reload |
| Rules scattered across modules | One source of truth |
| Hard to audit / explain | Decision trace is structural |
| Hard to test in isolation | Fixture-driven tests |
| Domain teams need engineers | Domain teams can author rules |

The cost is a small DSL with bounded expressive power. We accept that ceiling deliberately — if policy expressiveness becomes the constraint, the migration path to Cedar is mechanical translation (each YAML rule maps to one Cedar permission).

## Why PDP / PEP separation matters

A PDP that is **pure** can be:

- Tested with fixtures alone — no transport mocks needed.
- Replayed against historical decisions to verify a new policy is safe (shadow mode).
- Cached aggressively without correctness risk.
- Replaced (e.g. YAML → Cedar) without touching any PEP.

A PEP that is **per-transport** can be:

- Optimised for that transport's cost model (HTTP middleware is cheap; LLM-context filtering is expensive).
- Bypassed for admin paths without weakening other PEPs.
- Audited per-surface (which entry point denied this call?).

Folding them together (a PDP that does its own I/O for resource lookup) destroys both properties.

## Decision + obligations + advice

Returning more than allow/deny is **necessary**, not a nicety:

- An LLM-context filter may **permit** a Resource but require it be redacted first → obligation `redact-fields: [ssn]`.
- A deny may carry advice `["request-step-up-auth"]` so the caller knows the action would succeed after re-authentication.
- A permit may attach a TTL so the next identical call within 5 minutes does not re-evaluate.

The reasoning trace records which rules matched and why. **Opaque deny decisions are useless** to operators and break compliance audits. Explainability is non-negotiable.

## Shadow mode (Phase 1)

Per D-7, the first deployment of the PDP runs in **shadow mode**: every decision is computed and **logged with reasoning** but **not enforced**. Operators read the shadow log to see "what would have been denied if we flipped to enforce" before authorizing per-endpoint enforcement in Phase 3.

This is the single most important defense against policy bugs taking down production flows.

## Verification

- `evaluatePolicy` is a pure function (no global state, no I/O).
- Policy hot-reload picks up new YAML files without restart.
- Decision contains `reasoning` array with rule ids and matched conditions.
- Cache hit ratio observable in metrics; invalidates on policy-version bump.
- Shadow log records would-have-denied counts per rule; operators can flip a specific endpoint to enforce mode.

## Out of scope

- Policy language migration (Cedar / OPA) — separate ADR when triggered.
- The catalogue of policy packs — see Phase 6 in spec §11.
- UI for policy authoring — operators write YAML at v1.
- Distributed PDP / sidecar — single-process is sufficient at MSP scale.

## Source

- `packages/msp/docs/UNIVERSAL-CONTEXT-FRAMEWORK_spec.md` §7 — PDP/PEP architecture, YAML schema, decision shape.
- Decision §0 D-1 — YAML + minimal operators for v1.
- Decision §0 D-7 — default-permit + shadow log in Phase 1.
- XACML (OASIS, 2003) — established PDP/PEP terminology.
- OPA / Rego — production-deployed pure-function PDP.
- Cedar (AWS) — modern ergonomic ABAC language; future migration target.

## Connections

- [[FRAMEWORK--UNIVERSAL-CONTEXT-FRAMEWORK]]
