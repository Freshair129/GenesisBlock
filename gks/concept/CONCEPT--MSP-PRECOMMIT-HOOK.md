---
id: CONCEPT--MSP-PRECOMMIT-HOOK
phase: 1
type: concept
status: stable
vault_id: default
title: MSP pre-commit hook — block commits that break the validator
tags:
  - msp
  - precommit
  - hook
  - validator
  - automation
crosslinks: {"references":["FEAT--MSP-VALIDATOR","ADR--PROMOTION-WORKFLOW"]}
created_at: 2026-05-03T07:39:04.340Z
---

# CONCEPT — MSP pre-commit hook

## Problem

The validator (`FEAT--MSP-VALIDATOR`) catches schema, ID, wikilink and anti-hallucination violations — but only when someone explicitly invokes `npm run msp:validate`. In practice, agents and humans forget. Broken atoms make it into commits, surface only during PR review, and waste round-trips: reviewer flags → agent re-submits → re-reviews.

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
- Auto-fixing — validator is read-only by design (`ADR--PROMOTION-WORKFLOW`).
- Running tests, typecheck, or lint — those are separate hooks composed by the user.
- Replacing CI — local hook is best-effort; CI is the source of truth.

## Source

Closes the M3 backlog item recorded in `AUDIT--MSP-VALIDATOR` and `AUDIT--KNOWLEDGE-BASE`.
