---
id: AUDIT--CORE-FRAMEWORK-RECONCILE-V1
phase: 6
type: audit
status: stable
tier: process
source_type: axiomatic
vault_id: default
title: AUDIT — CORE_FRAMEWORK_MASTER_SPEC.md reconciled with actual codebase (W1+W2 wave)
tags:
  - msp
  - audit
  - core-framework
  - documentation
  - reconciliation
crosslinks: {"references":["FRAMEWORK--MSP-ARCHITECTURE-V2","FRAMEWORK--KNOWLEDGE-3-TIER","ADR--AGENT-WRITE-BOUNDARIES"]}
created_at: 2026-05-09T16:30:00.000+07:00
aliases:
  - AUDIT
  - implementation_flow
  - Test results / quality report
cluster: implementation_flow
role: "Test results / quality report"
---

# AUDIT — CORE_FRAMEWORK reconciliation v1

## What was wrong

Audit conducted 2026-05-09 (post-PR-6 merge) compared `CORE_FRAMEWORK_MASTER_SPEC.md` (the project's authoritative master spec) against the actual MSP codebase. Found **9 discrepancies**: 3 critical, 4 medium, 2 minor.

The spec was originally written as a **generic boilerplate** for forking into other projects (per `License intent` line at the top). MSP itself uses only a subset of the layout the spec describes, and several items the spec mentions never existed in MSP (e.g. `gks/00_MASTER_DASHBOARD.md`, `gks/devlog/`, `npm run msp:check`).

## Critical fixes shipped (W1 + W2)

| # | Section(s) | Was | Now |
|---|---|---|---|
| 1 | §4.2 directory tree, §4.4 type table, §0, §5.3, §10.5, §17, §18 | Plural folder names: `concepts/`, `adrs/`, `features/`, `frameworks/`, `blueprints/`, `microtasks/`, `audits/` | Singular per actual MSP layout: `concept/`, `adr/`, `feat/`, `frame/`, `blueprint/`, `task/`, `audit/`. Added `master/`, `proto/` to type table (PR-5/PR-6 territory). |
| 2 | §15.1 commands, §16.4 pre-commit, §8.6, §9.6 | `npm run msp:check`, `msp:codegen`, `msp:compose` (don't exist) | Real scripts: `msp:validate -- --all`, `msp:check-links`, `msp:verify`, `msp:backlinks`, `msp:run-task`, `msp:master`, `msp:hotfix:*`. §15.1 now enumerates all real scripts. |
| 3 | §16.5 Slash Commands | Mentioned only `msp_candidate` (1 of 11) plus 4 aspirational `/slash` commands | Renamed to "MCP tool surface" with full 11-tool table (validate, candidate, run-task, session-append, episode-append, backlinks-rebuild, recall, remember, compress, identity-get, identity-set). Aspirational slash commands flagged as not implemented. |

## Medium fixes shipped

| # | Section | Change |
|---|---|---|
| 4 | §15.1 | Bin entries (5 bins) added as a paragraph after the script table |
| 5 | §4.2.1 / §4.10 / §4.4 footer | Generic-boilerplate folders (ideas/, algorithms/, entities/, flows/, modules/, parameters/, ops/, devlog/) and `00_MASTER_DASHBOARD.md` flagged as aspirational with explicit notes |
| 6 | §4.2 directory tree | Added `master/`, `proto/`, `task/`, `issues/`, `audit/` (the 3-tier-model + governance artefacts that the audit confirmed exist) |
| 7 | §10.5 ownership table | Replaced legacy `gks/algorithms/* ฯลฯ` row with explicit "every `gks/<type>/*` goes via `msp_candidate` per `[[ADR--AGENT-WRITE-BOUNDARIES]]`"; dropped `devlog/` row |
| 8 | §18 bootstrap | Rewrote `mkdir -p` command to MSP-actual layout |
| 9 | EOF note | Updated example folder names |

## Deferred (separate PRs)

- **W3 — phase/status vocab** (§4.1, §7.3): spec uses P0–P7 + `stub/raw/verified`; reality uses `draft/stable/superseded/deprecated`. Not done in this PR — needs a thoughtful rewrite of §4.1 to align with `[[FRAMEWORK--PHASE-GOVERNANCE]]` semantics.
- **W3 — forbidden/required fields** (§7.3): point to `.brain/msp/LLM_Contract/atomic_contract.yaml` as SSOT and update example to show actual 17 forbidden fields. Not urgent — the example values shown happen to be a valid (if incomplete) subset.
- **W4 — `[[FRAMEWORK--AUTHORITY-MATRIX]].md` atom edit**: still references inbound legacy `/submit-memory → inbound queue`. Atom edit is out-of-scope for spec audit but is the next reconciliation target.

## Verification

- `npx tsx src/validator/cli.ts --all` exit 0
- `npm run msp:check-links` OK on 183 atoms (was 182; +1 this AUDIT)
- `npm run msp:index` regen `atomic_index.jsonl`
- Doc renders cleanly (markdown lint not enforced; spot-check passes)
- No code/test changes in this PR — purely a documentation reconciliation

## Atom contradiction checklist

- ✅ This AUDIT atom is additive; no supersession
- ✅ References `[[FRAMEWORK--MSP-ARCHITECTURE-V2]]`, `[[FRAMEWORK--KNOWLEDGE-3-TIER]]`, `[[ADR--AGENT-WRITE-BOUNDARIES]]` — all stable
- ✅ No claims about anything that contradicts another stable atom

## Lesson

The "boilerplate-vs-actual" gap accumulates silently when the spec gets editorial touchups (e.g. §3.6 added in PR-6) without revisiting earlier sections. A future task: add a `[[PROTO--CORE-FRAMEWORK-DRIFT-CHECK]]` predicate that grep's the spec for path strings and verifies they exist in the repo. Logged for later — not blocking.

## Source

- Audit subagent run 2026-05-09 (~600-word report; findings preserved as a comment thread in this PR)
- This reconciliation as a single PR (W1+W2 wave; W3+ deferred)
