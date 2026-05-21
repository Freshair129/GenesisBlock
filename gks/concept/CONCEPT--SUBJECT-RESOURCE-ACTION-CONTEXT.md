---
id: CONCEPT--SUBJECT-RESOURCE-ACTION-CONTEXT
phase: 1
type: concept
status: draft
tier: process
source_type: axiomatic
vault_id: default
title: Subject / Resource / Action / Context — the universal request 4-tuple
attributes:
  id: CONCEPT--SUBJECT-RESOURCE-ACTION-CONTEXT
  phase: 1
  type: concept
  status: draft
  tier: process
  source_type: axiomatic
  vault_id: default
  title: Subject / Resource / Action / Context — the universal request 4-tuple
  attributes:
    id: CONCEPT--SUBJECT-RESOURCE-ACTION-CONTEXT
    phase: 1
    type: concept
    status: draft
    tier: process
    source_type: axiomatic
    vault_id: default
    title: Subject / Resource / Action / Context — the universal request 4-tuple
    attributes:
      id: CONCEPT--SUBJECT-RESOURCE-ACTION-CONTEXT
      phase: 1
      type: concept
      status: draft
      tier: process
      source_type: axiomatic
      vault_id: default
      title: Subject / Resource / Action / Context — the universal request 4-tuple
      attributes:
        domain:
          - ucf
          - msp
      tags: &a1
        - msp
        - ucf
        - concept
        - abac
        - request-shape
      crosslinks: &a2
        references:
          - FRAMEWORK--UNIVERSAL-CONTEXT-FRAMEWORK
          - CONCEPT--ATTRIBUTE-BAG-MODEL
      created_at: 2026-05-13T08:59:38.431+07:00
      aliases: &a3
        - CONCEPT
        - implementation_flow
        - Strategic intent / PRD
      cluster: implementation_flow
      role: Strategic intent / PRD
      domain: concept
      language: markdown
      is_test: false
      is_entrypoint: false
      has_secret: true
      secret_type: high_entropy_string
      leak_risk: high
      encryption_level: none
    tags: *a1
    crosslinks: *a2
    created_at: 2026-05-13T08:59:38.431+07:00
    aliases: *a3
    cluster: implementation_flow
    role: Strategic intent / PRD
    domain: concept
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: true
    secret_type: high_entropy_string
    leak_risk: high
    encryption_level: none
  tags: *a1
  crosslinks: *a2
  created_at: 2026-05-13T08:59:38.431+07:00
  aliases: *a3
  cluster: implementation_flow
  role: Strategic intent / PRD
  domain: concept
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: true
  secret_type: high_entropy_string
  leak_risk: high
  encryption_level: none
tags: *a1
crosslinks: *a2
created_at: 2026-05-13T08:59:38.431+07:00
aliases: *a3
cluster: implementation_flow
role: Strategic intent / PRD
---

# CONCEPT — Subject / Resource / Action / Context

> Defines the **request shape** that every PEP/PDP interaction uses across MSP. Adopts the XACML / Cedar / OPA four-tuple, proven over 20+ years in IAM systems.
>
> Spec section: `UNIVERSAL-CONTEXT-FRAMEWORK_spec.md` §3.

## Problem

MSP's entry points (Express routes, MCP tools, facade methods, GKS recall/retain) currently take heterogeneous, untyped request arguments. None of them carry **identity of caller**, **action being performed**, or **environmental context** in a unified way. As a result:

- Auth checks are scattered (or absent).
- Audit logs cannot reliably answer "who did what to what, when, and why was it allowed?"
- A policy engine cannot be plugged in without rewriting every entry point.
- Subagents inherit ambient credentials of the parent process — no per-task scoping.

A request shape that **every** entry point accepts is a precondition for the rest of UCF.

## Hypothesis

Adopting the XACML/Cedar **four-tuple** as the canonical request shape — Subject, Resource, Action, Context — gives MSP a single representation that:

1. Survives all transports (HTTP, MCP stdio, in-process facade, scheduled job).
2. Is the only input the Policy Decision Point (PDP) needs.
3. Carries enough metadata for full audit reconstruction.
4. Supports both human users **and** non-human subjects (subagents, services, jobs) without special-casing.

The decision returned by the PDP is also structured (effect + obligations + advice + reasoning), enabling permit-with-conditions and explainable deny.

## Scope

In:

- TypeScript types `Subject`, `Resource`, `Action`, `RequestContext`, `Decision`, `Obligation` in `packages/msp/src/policy/types.ts` (new module).
- `Subject.kind` enumeration: `user | subagent | service | scheduled-job | mcp-client`.
- `Action` enumeration: `read | recall | embed | expose-to-llm | summarize | write | modify | delete | cite`.
- `RequestContext` minimum fields: `time, origin, trace_id`; optional: `network, purpose, scale_level`.
- Helper constructors `makeSubject(...)`, `makeResource(...)` that fill defaults and validate shape.
- Threading the tuple through `recall()`, `retain()`, `runTask()`, `expose()`, and each MCP tool handler.

Out:

- The policy language itself — see `[[ADR--POLICY-AS-DATA-NOT-CODE]]`.
- Domain-specific attribute taxonomies — see `[[CONCEPT--ATTRIBUTE-BAG-MODEL]]`.
- Enforcement decisions (when to deny, when to redact) — see policy packs.

## Why these four and not three or five

- **Three (Subject/Resource/Action)** lacks environmental context (time-of-day rules, purpose markers, scale level). Auditors and operators repeatedly need that.
- **Five (adding "Purpose" as a peer)** over-promotes one specific attribute. "Purpose" is just one field in `Context.purpose`; it does not warrant a top-level slot.
- **Four** matches XACML/Cedar/OPA, so policies written for any of those languages map cleanly. Migration paths stay open.

## Why distinguish `recall` from `expose-to-llm`

A Resource may be **recall**-able (it can appear as a hit, with its id surfaced to the caller) but **not** `expose-to-llm`-able (its body cannot enter LLM context). This split is the foundation of the **citation-only pattern** used for high-sensitivity data:

- Caller (or LLM) receives `[citation: ATOM-ID]`.
- The body is fetched out-of-band at the UI or data layer, where row-level security can be enforced.
- The LLM never sees PHI / PCI / proprietary data directly.

A single `read` action would not capture this distinction.

## Verification

- Type-check the new types — `tsc --noEmit` passes.
- Every existing entry point in MSP accepts the four-tuple (with sensible defaults for backward compatibility).
- Decision returned by a stub PDP is shape-correct and consumable by a (future) PEP.
- Audit log records show the four-tuple per call.

## Out of scope

- Schema for `AttributeBag` contents (see `[[CONCEPT--ATTRIBUTE-BAG-MODEL]]`).
- PDP implementation (see `[[FEAT--POLICY-DECISION-POINT]]`).
- Per-transport PEP wiring (see `[[ADR--TRANSPORT-AGNOSTIC-ENFORCEMENT]]`).
- Step-up authentication (see `[[CONCEPT--STEP-UP-AUTH]]`).

## Source

- `docs/msp/UNIVERSAL-CONTEXT-FRAMEWORK_spec.md` §3 — full type definitions and rationale.
- XACML core specification (OASIS) — established the four-tuple in 2003.
- Cedar policy language (AWS, 2023) — modern ergonomic variant.
- Open Policy Agent (OPA / Rego) — production-deployed PDP architecture.

## Connections

- [[FRAMEWORK--UNIVERSAL-CONTEXT-FRAMEWORK]]
