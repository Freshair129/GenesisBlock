---
id: ADR--DEFAULT-POLICY-POSTURE
phase: 2
type: adr
status: stable
tier: process
source_type: axiomatic
vault_id: default
title: Default policy posture — default-permit + shadow log, tighten per-endpoint
tags: &a1
  - msp
  - ucf
  - adr
  - abac
  - policy
  - rollout
crosslinks: &a2
  references:
    - CONCEPT--ABAC-POLICY-ENGINE
    - ADR--POLICY-AS-DATA-NOT-CODE
    - FRAMEWORK--UNIVERSAL-CONTEXT-FRAMEWORK
created_at: 2026-05-14T18:37:54.914+07:00
aliases: &a3
  - ADR
  - implementation_flow
  - Architecture decision record
cluster: implementation_flow
role: Architecture decision record
attributes:
  id: ADR--DEFAULT-POLICY-POSTURE
  phase: 2
  type: adr
  status: stable
  tier: process
  source_type: axiomatic
  vault_id: default
  title: Default policy posture — default-permit + shadow log, tighten per-endpoint
  tags: *a1
  crosslinks: *a2
  created_at: 2026-05-14T18:37:54.914+07:00
  aliases: *a3
  cluster: implementation_flow
  role: Architecture decision record
  attributes:
    id: ADR--DEFAULT-POLICY-POSTURE
    phase: 2
    type: adr
    status: stable
    tier: process
    source_type: axiomatic
    vault_id: default
    title: Default policy posture — default-permit + shadow log, tighten per-endpoint
    tags: *a1
    crosslinks: *a2
    created_at: 2026-05-14T18:37:54.914+07:00
    aliases: *a3
    cluster: implementation_flow
    role: Architecture decision record
    attributes:
      id: ADR--DEFAULT-POLICY-POSTURE
      phase: 2
      type: adr
      status: stable
      tier: process
      source_type: axiomatic
      vault_id: default
      title: Default policy posture — default-permit + shadow log, tighten
        per-endpoint
      tags: *a1
      crosslinks: *a2
      created_at: 2026-05-14T18:37:54.914+07:00
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

# ADR — Default policy posture

> Resolves decision **D-7** in `UNIVERSAL-CONTEXT-FRAMEWORK_spec.md` §0.

## Context

When the PDP (`[[CONCEPT--ABAC-POLICY-ENGINE]]`) goes live, every entry point starts consulting it. The question is the **default effect** when no rule explicitly matches a request:

1. **`default-deny`** — secure by default; nothing is permitted unless a rule explicitly allows it.
2. **`default-permit`** — permissive by default; only explicit deny rules block anything.

`default-deny` is the textbook-correct security posture. But MSP already has working flows (recall, retain, runTask, the 20 MCP tools, the Express API) that today run with no policy layer at all. Flipping to `default-deny` on day one means **every existing flow breaks** until a permit rule is written for it. History shows that is how policy-engine rollouts get reverted and lose team trust.

## Decision

**Phased posture: `default-permit` + shadow log first, then tighten to `default-deny` per-endpoint from Phase 3 onward.**

- **Phase 1 — `default-permit` + shadow mode.** The PDP runs and logs a full reasoning trace for every request, but enforcement is off — every action proceeds regardless of the decision. The shadow log answers: *"if we flipped this endpoint to enforce, what would have been denied?"*
- **Phase 2 — author rules against the shadow log.** Operators read the would-have-denied report, write the permit rules that legitimate flows need, and fix the atoms / attributes that were mis-tagged.
- **Phase 3 onward — flip to `default-deny` per-endpoint.** One entry point at a time, starting with the lowest-risk surface (`runTask`), graduating each only when its shadow log is clean. The `expose-to-llm` action on `restricted`-tier Resources is an early target.
- **End state — `default-deny` everywhere.** The permissive default is a rollout scaffold, not the destination.

The posture is itself policy data (per `[[ADR--POLICY-AS-DATA-NOT-CODE]]`): a per-endpoint `default_effect` field, flippable without a deploy.

## Consequences

Positive:

- Zero breakage of existing flows on day one — Phase 1 changes nothing observable except the appearance of a shadow log.
- The shadow log is a **concrete artifact** for the Phase 2 conversation — operators tighten policy against real traffic, not guesses.
- Per-endpoint graduation means a bug in one endpoint's rules cannot take down the others.
- Team trust is built incrementally: each endpoint flips only after its shadow log proves the rules are right.

Negative / accepted costs:

- There is a window (Phase 1 → Phase 3 per endpoint) where the system is **not actually enforcing** — `default-permit` means an attacker is not blocked yet. Accepted and explicitly bounded: shadow mode is a rollout phase, not a resting state, and the highest-sensitivity action (`expose-to-llm` on `restricted`) is among the first to flip.
- Risk that "tighten later" never happens and the system sits in `default-permit` forever. Mitigated: each phase in spec §11 names the per-endpoint flips as explicit deliverables with acceptance criteria; `default-deny` everywhere is the stated end state, not optional.
- Two posture states to test (permit + shadow, deny + enforce) per endpoint. Accepted — both are needed regardless; the phasing just sequences them.

## Alternatives considered

**`default-deny` from Phase 1.** Rejected. Textbook-correct, operationally fatal here: every existing flow breaks simultaneously, the team cannot write all permit rules atomically, the rollout gets reverted, and the policy engine acquires a reputation as "the thing that broke prod." Correct posture, wrong moment.

**`default-permit` forever (only ever write deny rules).** Rejected. That is not access control — it is a blocklist, and blocklists fail open on everything the author did not anticipate. Acceptable as a *phase*, unacceptable as an *end state*.

**Global flip (all endpoints at once) when shadow logs are clean.** Rejected: couples the rollout of unrelated endpoints. A bad rule on the symbol API should not block tightening the recall API. Per-endpoint graduation isolates risk.

## Source

- `docs/msp/UNIVERSAL-CONTEXT-FRAMEWORK_spec.md` §0 (D-7), §11 Phase 1 / Phase 3.
- `[[CONCEPT--ABAC-POLICY-ENGINE]]` — shadow mode and the PDP this posture configures.
- `[[ADR--POLICY-AS-DATA-NOT-CODE]]` — the posture is itself a policy-data field, flippable per endpoint.

## Connections

- [[FRAMEWORK--UNIVERSAL-CONTEXT-FRAMEWORK]]
