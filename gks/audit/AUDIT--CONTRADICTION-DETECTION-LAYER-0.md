---
id: AUDIT--CONTRADICTION-DETECTION-LAYER-0
phase: 6
type: audit
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: Contradiction detection Layer 0 shipped — CLAUDE.md policy + PR template checklist
tags:
  - msp
  - audit
  - contradiction
  - governance
  - layer-0
  - pr-template
crosslinks: {"references":["CONCEPT--ATOM-CONTRADICTION-DETECTION","ADR--CONTRADICTION-DETECTION-STACK","BLUEPRINT--CONTRADICTION-DETECTION-IMPL"]}
linked_symbols:
  - {"file":"CLAUDE.md"}
  - {"file":".github/pull_request_template.md"}
  - {"file":"ROADMAP.md"}
created_at: 2026-05-08T18:50:00.000+07:00
---

# Contradiction detection Layer 0 — shipped

## Scope

Records implementation of Layer 0 (the human rule) of `[[BLUEPRINT--CONTRADICTION-DETECTION-IMPL]]`. Phase 0 is doc-only by design — `CLAUDE.md` policy section + PR template checklist + ROADMAP note. No code. No tests. Mechanical layers (1–4) follow in subsequent PRs.

## What shipped

### CLAUDE.md — new section "Atom contradiction policy"

Inserted between `## Doc-to-code workflow` (gates between phases) and `## Useful commands`. States the rule:

> If a new atom claims something that conflicts with an existing `status: stable` atom of the same type, the conflicting atom MUST be explicitly superseded in the same PR:
>
> 1. Add the old atom's id to the new atom's `crosslinks.supersedes`
> 2. Add the new atom's id to the old atom's `crosslinks.superseded_by`
> 3. Flip the old atom's `status` to `superseded`

References `[[BLUEPRINT--CONTRADICTION-DETECTION-IMPL]]` (forthcoming mechanical layers) and `[[ADR--CONTRADICTION-DETECTION-STACK]]` (full 5-layer plan).

### `.github/pull_request_template.md` (NEW)

Standard PR sections (`## Summary`, `## Test plan`) plus a conditional `## Atom contradiction checklist` that authors and reviewers tick when a PR touches `gks/<type>/`:

- No conflict with existing stable atoms of the same type, **OR**
- Conflicts marked via `crosslinks.supersedes` + `crosslinks.superseded_by` + `status: superseded` flip in the same PR
- Reviewer has verified the above

### ROADMAP.md — "Post-v0.4.0 — contradiction detection"

One-paragraph note marking Layer 0 as shipped + pointer to the ADR for the full plan. Inserted after the v0.4.0 enables block, before the counts block.

## What this AUDIT does NOT cover

- Layer 1 (`[[PROTO--RECIPROCAL-SUPERSESSION]]`) — its own AUDIT atom when shipped
- Layer 2 (`domain:` field + `[[PROTO--DOMAIN-UNIQUENESS]]`) — its own AUDIT atom when shipped
- Layer 3 (embedding similarity PR-comment bot) — its own AUDIT atom when shipped
- Layer 4 (LLM judge, opt-in) — its own AUDIT atom when shipped

## Verification

- `npx tsx src/validator/cli.ts --all` — exit 0
- `npm run msp:check-links` — all crosslinks resolve
- `npm test` — existing tests pass (no code change)
- Manual: render `.github/pull_request_template.md` in a sandbox PR (verified before merge)
- Manual: read `CLAUDE.md` § "Atom contradiction policy" — content matches `[[ADR--CONTRADICTION-DETECTION-STACK]]` § "Layer 0"

## Counts

- Files added: 1 (`.github/pull_request_template.md`)
- Files modified: 2 (`CLAUDE.md`, `ROADMAP.md`)
- Code lines: 0
- Test lines: 0
- AUDIT atoms: +1 (this)

## Source

- `[[BLUEPRINT--CONTRADICTION-DETECTION-IMPL]]` § "phase_0_human_rule"
- `[[ADR--CONTRADICTION-DETECTION-STACK]]` § "Layer 0 — Human rule"
- `[[CONCEPT--ATOM-CONTRADICTION-DETECTION]]` § "Why detection alone isn't enough"
