---
id: CONCEPT--MSP-PRECOMMIT-HOOK
phase: 1
type: concept
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: MSP pre-commit hook — block commits that break the validator
tags: &a1
  - msp
  - precommit
  - hook
  - validator
  - automation
crosslinks: &a2
  references:
    - FEAT--MSP-VALIDATOR
    - ADR--AGENT-WRITE-BOUNDARIES
created_at: 2026-05-03T14:39:04.340+07:00
aliases: &a3
  - CONCEPT
  - implementation_flow
  - Strategic intent / PRD
cluster: implementation_flow
role: Strategic intent / PRD
attributes:
  id: CONCEPT--MSP-PRECOMMIT-HOOK
  phase: 1
  type: concept
  status: stable
  vault_id: default
  tier: genesis
  source_type: axiomatic
  title: MSP pre-commit hook — block commits that break the validator
  tags: *a1
  crosslinks: *a2
  created_at: 2026-05-03T14:39:04.340+07:00
  aliases: *a3
  cluster: implementation_flow
  role: Strategic intent / PRD
  attributes:
    id: CONCEPT--MSP-PRECOMMIT-HOOK
    phase: 1
    type: concept
    status: stable
    vault_id: default
    tier: genesis
    source_type: axiomatic
    title: MSP pre-commit hook — block commits that break the validator
    tags: *a1
    crosslinks: *a2
    created_at: 2026-05-03T14:39:04.340+07:00
    aliases: *a3
    cluster: implementation_flow
    role: Strategic intent / PRD
    attributes:
      domain: concept
    domain: concept
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: false
    leak_risk: low
    encryption_level: none
  domain: concept
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: false
  leak_risk: low
  encryption_level: none
---

# CONCEPT — MSP pre-commit hook

## Problem

The validator (`[[FEAT--MSP-VALIDATOR]]`) catches schema, ID, wikilink and anti-hallucination violations — but only when someone explicitly invokes `npm run msp:validate`. In practice, agents and humans forget. Broken atoms make it into commits, surface only during PR review, and waste round-trips: reviewer flags → agent re-submits → re-reviews.

A pre-commit hook closes that latency by running the validator at the cheapest possible point: before the commit lands.

## Hypothesis

If a portable bash hook detects staged `.md` files under `gks/` and `.brain/msp/projects/<ns>/inbound/` and runs the validator on each, then commits that violate the contract are blocked locally with the same rule-keyed error messages the agent will see in CI. The agent fixes the file, re-stages, and the next `git commit` either passes or fails for the same reason. No PR-round-trip latency for trivially-broken submissions.

## Scope

In:
- Detect staged `.md` files matching `gks/**` or `.brain/msp/projects/*/inbound/**`.
- For each, invoke `npm run msp:validate -- <file>`.
- Block the commit if any validator run exits non-zero; print which file + which rule.
- Skip the hook if no relevant files are staged (zero-cost happy path).
- Standard escape: `git commit --no-verify` (we don't invent a custom flag).

Out:
- Auto-fixing — validator is read-only by design (`[[ADR--PROMOTION-WORKFLOW]]`).
- Running tests, typecheck, or lint — those are separate hooks composed by the user.
- Replacing CI — local hook is best-effort; CI is the source of truth.

## Source

Closes the M3 backlog item recorded in `[[AUDIT--MSP-VALIDATOR]]` and `[[AUDIT--KNOWLEDGE-BASE]]`.

## Connections
- [[ADR--AGENT-WRITE-BOUNDARIES]]

