---
id: ADR--TRANSPORT-AGNOSTIC-ENFORCEMENT
phase: 2
type: adr
status: stable
tier: process
source_type: axiomatic
vault_id: default
title: Transport-agnostic enforcement — one PEP per entry surface, one shared PDP
tags:
  - msp
  - ucf
  - adr
  - abac
  - pep
  - enforcement
crosslinks:
  references:
    - CONCEPT--ABAC-POLICY-ENGINE
    - CONCEPT--SUBJECT-RESOURCE-ACTION-CONTEXT
    - FRAMEWORK--UNIVERSAL-CONTEXT-FRAMEWORK
created_at: 2026-05-14T18:37:52.572+07:00
aliases:
  - ADR
  - implementation_flow
  - Architecture decision record
cluster: implementation_flow
role: Architecture decision record
attributes:
  domain: adr
---

# ADR — Transport-agnostic enforcement

> Implements the PEP side of `[[CONCEPT--ABAC-POLICY-ENGINE]]`. No spec decision id — this is an architecture decision derived from spec §7.

## Context

MSP exposes the same logical operations (recall, retain, expose-to-llm, runTask, embed) over multiple transports:

- **HTTP** — the Express server (`POST /index`, `/recall`, candidates API, symbol API).
- **MCP stdio** — the 20-tool `msp-mcp-server`.
- **In-process facade** — `createCognitiveLayer(...)` methods called directly by embedding code.
- **Scheduled / internal** — re-indexers, consolidators, background jobs.

`[[CONCEPT--ABAC-POLICY-ENGINE]]` mandates that **every** entry point consult the PDP. The question is **how** enforcement is wired: one check shared across transports, or one per transport, and where in each transport's request path it sits.

Getting this wrong produces either gaps (a transport with no PEP — silent bypass) or duplication (each transport re-implements decision logic — drift).

## Decision

**One shared PDP; one thin PEP per entry surface; the PEP does only enforcement, never decision logic.**

- **PDP** — a single pure function `evaluatePolicy(subject, resource, action, context): Decision`, transport-unaware. It has no I/O and no knowledge of HTTP, MCP, or anything else.
- **PEP** — a per-transport interceptor that (a) constructs the 4-tuple from that transport's request, (b) calls the PDP, (c) enforces the result (block / allow / attach obligations). Each PEP is thin — tuple construction + a PDP call + result handling.
- **PEP placement** — as early as possible in each transport's request path, before any side effect:
  - HTTP → Express middleware, before route handlers.
  - MCP → a tool-handler wrapper applied uniformly to all registered tools.
  - Facade → an interceptor inside `recall` / `retain` / `runTask` / `expose`.
  - Internal jobs → an explicit PDP call at the job's entry; internal jobs may carry a `service` Subject with a documented policy.
- **PEP registry** — a single module enumerates every PEP. A test asserts that the set of entry points equals the set of PEP-wrapped functions, so a new entry point without a PEP fails CI.

## Consequences

Positive:

- Decision logic exists in exactly one place (the PDP). PEPs cannot drift because they contain no logic to drift.
- Adding a transport = adding one thin PEP, not re-deriving policy.
- The "every entry point is enforced" invariant is **mechanically checked** by the registry test — not a documentation promise.
- Each PEP can be tuned for its transport's cost model (HTTP middleware is cheap and runs always; the compose-time PEP that filters LLM context is expensive and runs once per task).
- Admin / migration bypass is explicit and localized — a specific PEP can be configured to pass `crossNamespace`-style overrides, without weakening other PEPs.

Negative / accepted costs:

- Every transport pays a PDP call on the hot path. Mitigated by decision caching (per `[[CONCEPT--ABAC-POLICY-ENGINE]]`) keyed on the 4-tuple hash.
- Four PEPs to maintain. Bounded — they are thin by construction, and the registry test prevents silent omission.
- The 4-tuple must be constructible from every transport. For transports with weak identity (MCP stdio is local-trust), the PEP constructs a best-effort Subject; this is a known limitation addressed by `[[CONCEPT--STEP-UP-AUTH]]` for high-risk MCP tools.

## Alternatives considered

**Single chokepoint PEP (enforce only in GKS recall/retain).** Rejected: not every operation passes through GKS recall/retain (runTask composes context, the facade has its own surface), so a single chokepoint leaves gaps. It also pushes MSP-policy concern into GKS, violating the storage-engine boundary.

**Per-transport decision logic (each transport implements its own checks).** Rejected: guarantees drift. Three transports, three subtly different interpretations of the same policy, three places to patch a vulnerability.

**Decorator-only, no registry test.** Rejected: relies on every future contributor remembering to wrap new entry points. The registry test converts a discipline problem into a CI failure.

## Source

- `docs/msp/UNIVERSAL-CONTEXT-FRAMEWORK_spec.md` §7 (PDP/PEP), §7.6 (PEP integration points), §10 (five-layer pipeline).
- `[[CONCEPT--ABAC-POLICY-ENGINE]]` — the PDP these PEPs consult.
- `[[CONCEPT--SUBJECT-RESOURCE-ACTION-CONTEXT]]` — the 4-tuple each PEP constructs.
- `[[CONCEPT--STEP-UP-AUTH]]` — handles the weak-identity case for MCP stdio.

## Connections
- [[FRAMEWORK--UNIVERSAL-CONTEXT-FRAMEWORK]]

