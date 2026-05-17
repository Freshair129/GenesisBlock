---
id: ADR--MASTER-PROMOTION-CONTRADICTION-POLICY
phase: 2
type: adr
status: stable
tier: genesis
source_type: axiomatic
vault_id: default
title: Promote the Layer 0 supersession rule to a Master atom
  (MASTER--ATOM-CONTRADICTION-POLICY)
tags: &a1
  - msp
  - master
  - promotion
  - contradiction
  - supersession
  - 3-tier
  - decision
crosslinks: &a2
  references:
    - FRAMEWORK--KNOWLEDGE-3-TIER
    - BLUEPRINT--CONTRADICTION-DETECTION-IMPL
    - MASTER--ATOM-CONTRADICTION-POLICY
    - ADR--CONTRADICTION-DETECTION-STACK
    - CONCEPT--ATOM-CONTRADICTION-DETECTION
created_at: 2026-05-09T15:03:00.000+07:00
aliases: &a3
  - ADR
  - implementation_flow
  - Architecture decision record
cluster: implementation_flow
role: Architecture decision record
attributes:
  id: ADR--MASTER-PROMOTION-CONTRADICTION-POLICY
  phase: 2
  type: adr
  status: stable
  tier: genesis
  source_type: axiomatic
  vault_id: default
  title: Promote the Layer 0 supersession rule to a Master atom
    (MASTER--ATOM-CONTRADICTION-POLICY)
  tags: *a1
  crosslinks: *a2
  created_at: 2026-05-09T15:03:00.000+07:00
  aliases: *a3
  cluster: implementation_flow
  role: Architecture decision record
  attributes:
    id: ADR--MASTER-PROMOTION-CONTRADICTION-POLICY
    phase: 2
    type: adr
    status: stable
    tier: genesis
    source_type: axiomatic
    vault_id: default
    title: Promote the Layer 0 supersession rule to a Master atom
      (MASTER--ATOM-CONTRADICTION-POLICY)
    tags: *a1
    crosslinks: *a2
    created_at: 2026-05-09T15:03:00.000+07:00
    aliases: *a3
    cluster: implementation_flow
    role: Architecture decision record
    attributes:
      domain: adr
    domain: adr
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: false
    leak_risk: low
    encryption_level: none
  domain: adr
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: false
  leak_risk: low
  encryption_level: none
---

# ADR — promote contradiction policy to Master

## Context

`[[FRAMEWORK--KNOWLEDGE-3-TIER]]` requires Master atoms be created via an
evidence ADR (Master is not authored directly). PR-5 promotes the second
of two foundational rules into the Master tier; this ADR is the evidence
record for `[[MASTER--ATOM-CONTRADICTION-POLICY]]`.

The Layer 0 rule of supersession discipline lives in two places today:

1. `[[BLUEPRINT--CONTRADICTION-DETECTION-IMPL]]` — the implementation plan
   for the full contradiction-detection stack. Layer 0 is the
   write-time author rule that mechanical layers wrap, not replace.
2. `CLAUDE.md` § "Atom contradiction policy" — the human-facing
   contract. The `.github/pull_request_template.md` carries the
   reviewer checklist that enforces the rule pre-merge.

Cross-context stability evidence:

- Every supersession in the repo to date has followed the reciprocal
  pattern: `crosslinks.supersedes` + `crosslinks.superseded_by` + status
  flip. Examples: `[[ADR--AGENT-WRITE-BOUNDARIES]]` supersedes
  `[[ADR--PROMOTION-WORKFLOW]]` and `[[ADR--PROMOTION-LEVELS]]` (PR #51); the
  Phase 4 supersession round in PR #51 closed the inbound queue's
  obsolete atoms with the same shape.
- `[[ADR--CONTRADICTION-DETECTION-STACK]]` explicitly names Layer 0 as the
  cheapest, highest-value layer — mechanical layers
  (`[[PROTO--RECIPROCAL-SUPERSESSION]]`, embedding similarity, optional LLM
  judge) are scaffolding, not replacements.
- The PR template's atom contradiction checklist has been required on
  every gks/-touching PR since its introduction; no merged PR has
  bypassed it.

The rule is invariant across atom types, milestones, and PRs.

## Decision

Promote the Layer 0 supersession rule to
`[[MASTER--ATOM-CONTRADICTION-POLICY]]`, with
`promoted_from: [[BLUEPRINT--CONTRADICTION-DETECTION-IMPL]]` and
`promotion_adr: [[ADR--MASTER-PROMOTION-CONTRADICTION-POLICY]]` (this atom).

The Master body distills the rule into the canonical 5-section schema
(Intent / Why / Directives / Apply when / Conflicts with). The
pre-promotion BLUEPRINT remains stable — Master is an additive
distillation, not a supersession. `CLAUDE.md` continues as the human
contract; `[[MASTER--ATOM-CONTRADICTION-POLICY]]` is the agent-facing SSOT.

## Consequences

- Agents loading Master atoms (PR-6 ships the loader) receive the
  supersession rule as instinct on every relevant session and apply it
  before authoring an atom that overlaps an existing scope.
- The Master body fits the 400-token warn threshold enforced by
  `[[PROTO--MASTER-TOKEN-CAP]]`. Edits that expand it must respect the cap.
- Drift between `CLAUDE.md` § "Atom contradiction policy" and the
  Master body is governed by `[[MASTER--ATOM-CONTRADICTION-POLICY]]` itself
  (the rule applies recursively — any future Master that contradicts
  this one must be supersession-tagged).
- Layer 0 staying author-enforced is preserved; the Master codifies the
  author rule, while `[[BLUEPRINT--CONTRADICTION-DETECTION-IMPL]]` continues
  to track the mechanical layers (PROTO predicates, embedding hints,
  LLM judge) that wrap it.

## Alternatives considered

- **Promote the full contradiction stack (Layers 0–N) to one Master.**
  Rejected: too long for the 400-token budget, and Layers 1+ are
  scaffolding that change as predicates ship — Master is for the
  invariant.
- **Wait for `[[PROTO--RECIPROCAL-SUPERSESSION]]` to ship before
  promotion.** Rejected: the author rule predates and outlasts the
  predicate; Master should encode the human-enforced floor regardless
  of mechanical coverage.

## Source

- `[[FRAMEWORK--KNOWLEDGE-3-TIER]]`
- `[[BLUEPRINT--CONTRADICTION-DETECTION-IMPL]]` § Layer 0
- `CLAUDE.md` § "Atom contradiction policy"
- `.github/pull_request_template.md` (atom contradiction checklist)
- `[[ADR--CONTRADICTION-DETECTION-STACK]]`

## Connections
- [[CONCEPT--ATOM-CONTRADICTION-DETECTION]]

