---
id: BLUEPRINT--PHASE-4-USER-ABAC
phase: 3
type: blueprint
status: draft
tier: process
source_type: axiomatic
vault_id: default
scale_level: L2
title: "BLUEPRINT — Phase 4 user-level ABAC: authenticated Subject, policy
  packs, all read entry points enforce"
aliases: &a1
  - BLUEPRINT
  - implementation_flow
  - Implementation plan
cluster: implementation_flow
role: Implementation plan
tags: &a2
  - msp
  - ucf
  - blueprint
  - phase-4
  - abac
crosslinks: &a3
  references:
    - CONCEPT--ABAC-POLICY-ENGINE
    - CONCEPT--ATTRIBUTE-BAG-MODEL
    - FEAT--POLICY-DECISION-POINT
    - FEAT--IDENTITY-LAYER
    - ADR--DEFAULT-POLICY-POSTURE
    - BLUEPRINT--PHASE-3-VAULT-AND-RESOLUTION
linked_symbols: &a4
  - file: packages/msp/src/policy/subject.ts
  - file: policies/30-multi-tenant.yaml
  - file: policies/40-pii-block-from-llm.yaml
created_at: 2026-05-14T23:46:17.000+07:00
attributes:
  id: BLUEPRINT--PHASE-4-USER-ABAC
  phase: 3
  type: blueprint
  status: draft
  tier: process
  source_type: axiomatic
  vault_id: default
  scale_level: L2
  title: "BLUEPRINT — Phase 4 user-level ABAC: authenticated Subject, policy
    packs, all read entry points enforce"
  aliases: *a1
  cluster: implementation_flow
  role: Implementation plan
  tags: *a2
  crosslinks: *a3
  linked_symbols: *a4
  created_at: 2026-05-14T23:46:17.000+07:00
  attributes:
    id: BLUEPRINT--PHASE-4-USER-ABAC
    phase: 3
    type: blueprint
    status: draft
    tier: process
    source_type: axiomatic
    vault_id: default
    scale_level: L2
    title: "BLUEPRINT — Phase 4 user-level ABAC: authenticated Subject, policy
      packs, all read entry points enforce"
    aliases: *a1
    cluster: implementation_flow
    role: Implementation plan
    tags: *a2
    crosslinks: *a3
    linked_symbols: *a4
    created_at: 2026-05-14T23:46:17.000+07:00
    attributes:
      domain: blueprint
    domain: blueprint
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: true
    secret_type: high_entropy_string
    leak_risk: high
    encryption_level: none
  domain: blueprint
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: true
  secret_type: high_entropy_string
  leak_risk: high
  encryption_level: none
---

# BLUEPRINT — Phase 4: user-level ABAC

> Implementation plan for spec §11 Phase 4. The `Subject` stops being an anonymous default and is **hydrated from the authenticated user**. Every read entry point graduates to an enforced PEP for `Subject.kind === 'user'`, and the first domain policy packs ship. This is the phase where two users sharing a deployment start seeing different things.

## Geography

New:

- `packages/msp/src/policy/subject.ts` — `hydrateSubject(identity): Subject` — maps an authenticated identity (`roles`, `clearance`, `mfa_status`, `tenant_id`) into the `Subject` `AttributeBag`. Reads the attribute store; does not own it.
- `policies/30-multi-tenant.yaml` — the `pack-multi-tenant` policy: a user only sees Resources whose `tenant_id` is in `S.tenant_ids`.
- `policies/40-pii-block-from-llm.yaml` — the `pack-pii-block-from-llm` policy: `expose-to-llm` is denied for any Resource whose body matches an SSN-like regex.

Touched:

- `packages/msp/src/identity/` — **reused, not rebuilt.** `store.ts` / `profile.ts` / `types.ts` already provide the per-`MSP_HOME` identity store; Phase 4 extends the identity record with `roles` / `clearance` / `mfa_status` / `tenant_ids` and reads it from `subject.ts`.
- `packages/msp/src/index.ts` — Express middleware upgrades the Phase 0 anonymous `Subject` to an authenticated one: in-house identity, or `X-Forwarded-User` / `X-Forwarded-Groups` reverse-proxy headers (spec §14 OQ-3 working assumption).
- `packages/msp/src/mcp/tools/*.ts` — each tool handler attaches per-call identity, replacing the Phase 0 static `Subject{ kind: 'mcp-client' }`.
- composer + `memory.ts` recall — the PEPs that were shadow-only for non-subagent subjects flip to **enforce** for `Subject.kind === 'user'`.

Not touched: GKS. The `Namespace` is unchanged; user attributes live in identity records + atom metadata (per `ADR--BRING-YOUR-OWN-ATTRIBUTES` / D-8).

## Acceptance

- Two users sharing one deployment, each in a different `tenant_id`, see only their own tenant's atoms on `recall()` — verified with a two-identity fixture.
- The PII pack blocks an atom containing SSN-like content from entering LLM context on `expose-to-llm`; the deny is visible in the audit log with reasoning.
- Every read entry point (HTTP routes, MCP read tools, facade `recall`) is a PEP that consults the PDP for `Subject.kind === 'user'` — a `grep` confirms no read path bypasses `evaluatePolicy`.
- An anonymous / unauthenticated request still resolves to the Phase 0 anonymous `Subject` and is handled by the per-endpoint default posture — no hard crash on missing identity.
- Reverse-proxy header auth (`X-Forwarded-User`) produces the same `Subject` shape as in-house identity — a test asserts parity.
- `expose-to-llm` on `restricted` Resources stays `default-deny` (the Phase 3 flip holds); all other endpoints honour `ADR--DEFAULT-POLICY-POSTURE`'s per-endpoint posture.

## Dependencies

- `BLUEPRINT--PHASE-3-VAULT-AND-RESOLUTION` — the resolution pipeline and the first `default-deny` flip must already exist.
- `FEAT--POLICY-DECISION-POINT` — the PDP this phase points all read PEPs at (Phase 4 has no dedicated FEAT; it wires user identity into the existing PDP contract).
- `FEAT--IDENTITY-LAYER` — the identity store reused for the user attribute store.
- `CONCEPT--ABAC-POLICY-ENGINE` — PDP/PEP architecture.
- `CONCEPT--ATTRIBUTE-BAG-MODEL` — `Subject` attribute shape.
- `ADR--DEFAULT-POLICY-POSTURE` — per-endpoint posture graduation continues here.

## Tasks

1. **T4.1** — `policy/subject.ts`: `hydrateSubject` mapping an authenticated identity into the `Subject` `AttributeBag`. Reuse `packages/msp/src/identity/{store,profile,types}.ts` for the attribute store — extend the identity record, do **not** build a new store.
2. **T4.2** — Express auth middleware in `index.ts`: resolve identity from in-house auth or reverse-proxy headers (`X-Forwarded-User` / `X-Forwarded-Groups`); fall back to the Phase 0 anonymous `Subject` on no identity.
3. **T4.3** — MCP per-call identity: each MCP tool handler attaches the caller's identity, replacing the Phase 0 static `mcp-client` subject.
4. **T4.4** — `policies/30-multi-tenant.yaml`: the `pack-multi-tenant` rule (`R.tenant_id ∈ S.tenant_ids`).
5. **T4.5** — `policies/40-pii-block-from-llm.yaml`: the `pack-pii-block-from-llm` rule — deny `expose-to-llm` on SSN-regex matches.
6. **T4.6** — Flip the recall + `expose-to-llm` PEPs from shadow to enforce for `Subject.kind === 'user'`; non-user subjects keep their Phase 1–3 posture.
7. **T4.7** — Acceptance harness: two-user tenant-isolation fixture + PII-block fixture; assert audit-log entries for every deny.

## Source

- `docs/msp/UNIVERSAL-CONTEXT-FRAMEWORK_spec.md` §11 Phase 4, §3, §4.
- `FEAT--POLICY-DECISION-POINT` — the PDP contract all read PEPs consume.
- `FEAT--IDENTITY-LAYER` — the identity store reused as the user attribute store.
- `CONCEPT--ABAC-POLICY-ENGINE`, `CONCEPT--ATTRIBUTE-BAG-MODEL` — governing concepts.
- `ADR--DEFAULT-POLICY-POSTURE` — per-endpoint posture graduation.
- `BLUEPRINT--PHASE-3-VAULT-AND-RESOLUTION` — predecessor phase.
