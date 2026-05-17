---
id: ADR--PROMOTION-WORKFLOW
phase: 2
type: adr
status: superseded
tier: genesis
source_type: axiomatic
vault_id: default
title: Promotion workflow — three gates between agent draft and gks/
tags: &a1
  - msp
  - promotion
  - workflow
  - governance
  - superseded
crosslinks: &a2
  references:
    - CONCEPT--INBOUND-QUEUE
    - CONCEPT--PROPOSAL-TYPES
    - FRAMEWORK--AUTHORITY-MATRIX
  superseded_by:
    - ADR--AGENT-WRITE-BOUNDARIES
created_at: 2026-05-03T14:08:40.359+07:00
aliases: &a3
  - ADR
  - implementation_flow
  - Architecture decision record
cluster: implementation_flow
role: Architecture decision record
attributes:
  id: ADR--PROMOTION-WORKFLOW
  phase: 2
  type: adr
  status: superseded
  tier: genesis
  source_type: axiomatic
  vault_id: default
  title: Promotion workflow — three gates between agent draft and gks/
  tags: *a1
  crosslinks: *a2
  created_at: 2026-05-03T14:08:40.359+07:00
  aliases: *a3
  cluster: implementation_flow
  role: Architecture decision record
  attributes:
    id: ADR--PROMOTION-WORKFLOW
    phase: 2
    type: adr
    status: superseded
    tier: genesis
    source_type: axiomatic
    vault_id: default
    title: Promotion workflow — three gates between agent draft and gks/
    tags: *a1
    crosslinks: *a2
    created_at: 2026-05-03T14:08:40.359+07:00
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
---

> ⚠️ **Superseded by [`[[ADR--AGENT-WRITE-BOUNDARIES]]`](./[[ADR--AGENT-WRITE-BOUNDARIES]].md)** (Phase 4 of `[[BLUEPRINT--INBOUND-TO-CANDIDATES-MIGRATION]]`, 2026-05-09). The three-gate inbound workflow has been replaced with: agents write to `.brain/.../candidates/` via `msp_candidate`, promotion to `gks/<type>/` is a human PR action gated by CI. The body below is preserved as historical context.

# ADR — promotion workflow

## Context

GKS exposes `proposeInbound()` as the only write-path to candidate atoms. Without a defined workflow above it, agents would either (a) skip validation, (b) self-promote without human review, or (c) re-submit endlessly without iteration. We need an explicit three-gate sequence.

## Decision

Three gates, in order, between agent draft and a `stable` atom under `gks/<type>/`:

```
draft.md ──▶ inbound queue ──▶ validator ──▶ human review ──▶ gks/<type>/
              (gate 1)         (gate 2)       (gate 3)
```

### Gate 1 — submission

- Agent calls `gks propose-inbound <ID> --title=... --body=...` (or wraps in MSP envelope per `[[CONCEPT--SUBMISSION-ENVELOPE]]`).
- File lands in `.brain/msp/projects/<ns>/inbound/<ID>.rev-<reviewId>.md`.
- No automatic check yet; the agent owns the submission.

### Gate 2 — automated validation

- `npm run msp:validate -- <inbound-file>` runs the validator from `[[FEAT--MSP-VALIDATOR]]`.
- All hard rules apply: forbidden-fields, ID-format, ID-filename-match, ADR-monotonic, dangling-wikilinks, future-date, summary-min, phase-status.
- Fail → file moves to `.brain/msp/projects/<ns>/rejected/<YYYY-MM-DD>/` with `rejection_reason.md`. Agent re-submits with fixes.
- Pass → file ready for human review.

### Gate 3 — human review

- Reviewer is determined by atom type — see `[[ADR--HUMAN-REVIEW-GATES]]` (TBD).
- Reviewer reads body, checks substance (the validator can't judge "is this a good ADR").
- Approve → `npx gks inbound promote <ID>` moves the file to `gks/<type>/<ID>.md` with `status: stable`.
- Reject → manual move to rejected/ with reviewer + reason.

## Consequences

**Positive**
- Each gate has one job; failures localised.
- Bypass requires actively breaking the workflow (e.g. direct git push to `gks/`) — caught by pre-commit + branch protection.
- Audit log captures every transition.

**Negative**
- Three gates add latency. Typical agent-to-stable: minutes if reviewer is online; hours otherwise.
- Hotfix scenarios need the escape hatch in `[[ADR--HOTFIX-ESCAPE-HATCH]]` to skip the human gate without skipping accountability.

## Alternatives considered

1. **Two-gate (validator + auto-promote).** Rejected. Removes substance review; allows well-formed but wrong atoms.
2. **One-gate (just human).** Rejected. Reviewers waste time on trivially-broken submissions (forbidden fields, dangling links).
3. **Allow agents to direct-write to `gks/` for low-tier atoms (idea, devlog).** Considered; deferred. ADR-014 in GksV3 has the strict-vs-light tier split — could borrow that, but for MSP M-series we keep one workflow for predictability.

## What this ADR does NOT decide

- The promotion *levels* (L0/L1/L2) — see `[[ADR--PROMOTION-LEVELS]]`.
- Which reviewer for which atom — see `[[ADR--HUMAN-REVIEW-GATES]]` (TBD).
- The hotfix bypass — see `[[ADR--HOTFIX-ESCAPE-HATCH]]`.

## Source

`msp_spec.md` §3 (Inbound Flow) + §8 (Promotion).

## Connections
- [[CONCEPT--INBOUND-QUEUE]]
- [[CONCEPT--PROPOSAL-TYPES]]
- [[FRAMEWORK--AUTHORITY-MATRIX]]

