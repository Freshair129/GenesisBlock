---
id: ADR--MASTER-PROMOTION-DOC-TO-CODE
phase: 2
type: adr
status: stable
tier: genesis
source_type: axiomatic
vault_id: default
title: Promote the doc-to-code workflow to a Master atom (MASTER--MSP-DOC-TO-CODE)
tags:
  - msp
  - master
  - promotion
  - doc-to-code
  - 3-tier
  - decision
crosslinks: {"references":["FRAMEWORK--KNOWLEDGE-3-TIER","CONCEPT--CODEGEN-MICROTASK-CONTRACT","MASTER--MSP-DOC-TO-CODE"]}
created_at: 2026-05-09T15:02:00.000+07:00
aliases:
  - ADR
  - implementation_flow
  - Architecture decision record
cluster: implementation_flow
role: "Architecture decision record"
---

# ADR — promote doc-to-code to Master

## Context

`[[FRAMEWORK--KNOWLEDGE-3-TIER]]` introduced a Master tier for stable cross-cutting
knowledge that an agent should carry as instinct (loaded as a system-prompt
preamble) rather than re-discover per session. The frame requires a `tier:
master` atom to declare `promoted_from`, `promoted_at`, `promotion_adr`, and
that promotion proceed via an evidence ADR — Master atoms are not authored
directly. PR-5 of the rollout plan ships the first two promotions.

The doc-to-code workflow (FRAME → CONCEPT → ADR/FEAT → BLUEPRINT → CODE →
AUDIT) is the strongest candidate for the first Master promotion. Cross-context
stability evidence:

- The workflow is described in `CLAUDE.md` § "Doc-to-code workflow" as
  mandatory and has applied unchanged to every milestone since the repo's
  inception.
- All 175 atoms in the canon today were authored under this discipline.
  `[[CONCEPT--CODEGEN-MICROTASK-CONTRACT]]` (status: stable) is one of the
  earliest expressions of the contract that microtasks land via this chain.
- Recent milestones M7b (PR #16), M8 PROTO migration (PRs #20–#33), and the
  3-tier rollout itself (PRs #48–#51 + this PR) all opened atoms before
  code; their AUDIT atoms (`[[AUDIT--TWO-REPO-VALIDATION]]`, `AUDIT--*` series)
  attest the order held under load.
- No ADR has ever proposed an exception. Where Phase 4 (TASK) was skipped
  for single-developer slices, that exemption is itself encoded in
  `[[ADR--PROMOTION-LEVELS]]`.

These three lines of evidence — written discipline (`CLAUDE.md`), a stable
genesis CONCEPT, and a multi-milestone audit trail — meet the bar for
"true regardless of session, project, or context."

## Decision

Promote the doc-to-code workflow to `[[MASTER--MSP-DOC-TO-CODE]]`, with
`promoted_from: [[CONCEPT--CODEGEN-MICROTASK-CONTRACT]]` and
`promotion_adr: [[ADR--MASTER-PROMOTION-DOC-TO-CODE]]` (this atom).

The Master body is a terse 5-section distillation (Intent / Why /
Directives / Apply when / Conflicts with) targeted at agent system-prompt
injection. The pre-promotion Genesis atom remains stable — the Master is
an additive distillation, not a supersession. `CLAUDE.md` continues as the
human-readable contract; `[[MASTER--MSP-DOC-TO-CODE]]` is the SSOT for the
agent-facing rule.

## Consequences

- Future agent sessions that load Master atoms (PR-6 ships the loader)
  receive the doc-to-code rule as instinct rather than re-deriving it.
- Drift between `CLAUDE.md` and the Master body is mitigated by
  `[[MASTER--ATOM-CONTRADICTION-POLICY]]` (companion promotion in this PR).
  If `CLAUDE.md` updates this rule, the Master must be updated in the
  same PR (or an explicit supersession ADR opened).
- The Master body sits within the 400-token warn threshold enforced by
  `[[PROTO--MASTER-TOKEN-CAP]]`. If future edits expand it past 400, the
  validator will warn; past 600 it errors.
- Master tier carries no `learned_from` (per
  `src/validator/rules/master-requires-promotion.ts`); origin tracking
  lives on this ADR and the pre-promotion Genesis atom.

## Alternatives considered

- **Leave the rule in `CLAUDE.md` only.** Rejected: agents that don't load
  `CLAUDE.md` (per-tool variation in MCP / non-Claude agents) miss it.
- **Promote `[[FRAMEWORK--MSP-ARCHITECTURE-V2]]` instead.** Rejected: that frame
  is too broad to fit the 400-token budget; doc-to-code is a single
  enforceable directive that distills cleanly.

## Source

- `[[FRAMEWORK--KNOWLEDGE-3-TIER]]` (Master Block § "How they get created")
- `CLAUDE.md` § "Doc-to-code workflow"
- `[[CONCEPT--CODEGEN-MICROTASK-CONTRACT]]`
- 3-tier rollout plan (PR-5)
