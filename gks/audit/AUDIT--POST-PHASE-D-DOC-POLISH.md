---
id: AUDIT--POST-PHASE-D-DOC-POLISH
phase: 6
type: audit
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: Post-Phase-D doc polish — sync FRAME-V2 / ROADMAP / README / msp_spec to
  current architecture
tags:
  - msp
  - audit
  - documentation
  - polish
crosslinks:
  references:
    - FRAMEWORK--MSP-ARCHITECTURE-V2
    - CONCEPT--AGENT-AGNOSTIC
    - CONCEPT--AGENT-INTEGRATION-PATTERNS
    - ADR--GLOBAL-VS-WORKSPACE
    - AUDIT--ARCH-DOC-CLEANUP
    - AUDIT--PHASE-B-IMPL-COMPLETE
    - AUDIT--PHASE-C-AGENT-INTEGRATION-DOCS
linked_symbols: []
created_at: 2026-05-10T07:00:00.000+07:00
aliases:
  - AUDIT
  - implementation_flow
  - Test results / quality report
cluster: implementation_flow
role: Test results / quality report
attributes:
  domain: audit
---

# AUDIT — Post-Phase-D doc polish

## Scope

After Phases A–D shipped (PRs #65/#66/#67/#68), several "outer" documents still described pre-Phase-A reality: gatekeeper-only framing, 11 / 16 MCP tools, no global/workspace split, no agent-integration story. This audit records the polish PR that synced them with the merged-state architecture.

## Files updated

| File | Change | Why |
|---|---|---|
| `gks/frame/[[FRAMEWORK--MSP-ARCHITECTURE-V2]].md` | Added 3-layer ecosystem diagram (cognitive / memory / knowledge); flipped 7 modules from `⏳` → `✅`; added storage layout section; updated Smart Connections wording (no longer "MSP never embeds" — clarified GKS embeds via `createNomicEmbedder`); added crosslinks to AGENT-AGNOSTIC + AGENT-INTEGRATION-PATTERNS + GLOBAL-VS-WORKSPACE | All shipped in Phase A–D but FRAME-V2 was untouched since 2026-05-03 |
| `ROADMAP.md` | New "Status — post-Phase-D" section at top with PR table; refreshed MCP tool surface (19 tools across 4 groups); inbound→candidates section flipped from "in progress" to DONE; counts updated (159→202 atoms, 535→663 tests, 11→19 MCP tools, 5→6 upstream proposals); MCP client config snippet now includes `MSP_HOME` + `MSP_PROJECT`; added pointer to `docs/AGENT-INTEGRATION.md` | ROADMAP claimed v0.4.0 still in progress for inbound migration; counts off by ~6 weeks |
| `README.md` | Full rewrite — agent-agnostic header; 3-layer diagram; "What this repo is" enumerates passport surfaces correctly; layout tree includes `~/.msp/`, `src/projects/`, `docs/`; MCP tool table (19, grouped); CLI bins table (6); workflow updated to current scripts; status section trimmed (was M0–M6 checklist; now points at ROADMAP); added Authoritative-docs section linking FRAME-V2 + msp_spec + AGENT-AGNOSTIC + AGENT-INTEGRATION + ROADMAP | README still described MSP as "Gatekeeper layer" with M6-era tool list; mentioned `msp_propose` (removed Phase 3); test counts off |
| `msp_spec.md` | §2.1 Layers table — relabeled "Inbound" row → "Candidates" with note about Phase 3 inbound removal; added Identity (global) + Identity (override) rows referencing `[[ADR--GLOBAL-VS-WORKSPACE]]`; replaced all hardcoded `.brain/msp/projects/evaAI/` paths with `.brain/msp/projects/<ns>/` to match the post-Phase-B namespace-generic story | Spec still presented `evaAI` as canonical path everywhere despite namespace becoming pluggable |

## Files NOT changed (intentionally)

- `gks/frame/[[FRAMEWORK--MSP-ARCHITECTURE]].md` (v1, superseded) — banner already strong; body stays verbatim per supersession discipline
- `gks/concept/[[CONCEPT--MSP-ROADMAP]].md` — atom version of the roadmap; intentionally not edited in this PR (would require atom contradiction policy review). ROADMAP.md is the user-facing mirror.
- `msp_spec.md` deeper sections (§3 inbound flow, §4 atomic write contract, §6 phase governance) — pre-existing accurate; no edits needed
- Any `src/` code — this is pure doc polish

## Atom contradiction policy

No supersession needed. All edits are **clarifications** of existing stable atoms, not contradictions:

- FRAME-V2 stays `status: stable`. The 3-layer ecosystem diagram is additive (it sits above the existing 2-layer MSP-internal diagram). Module status flips from `⏳` to `✅` reflect what shipped, not policy reversal.
- New crosslinks added (AGENT-AGNOSTIC, AGENT-INTEGRATION-PATTERNS, GLOBAL-VS-WORKSPACE) are forward references to atoms that already exist on `main`.

## Verification

- `npm run msp:index` — atoms re-indexed, count unchanged net (only AUDIT atom added: +1)
- `npx tsx src/validator/cli.ts --all` — 0 new failures (PROTO failures `PHASE-GATES`, `SCALING-LEVEL-GATE` remain, pre-existing)
- `npm run msp:check-links` — OK

## Counts (post-polish)

- ~203 atoms in `gks/`
- 19 MCP tools, 6 CLI bins
- 663 passing tests on Node 20 + 22

## Source

Recommended option 1+4 from architecture-cleanup follow-up planning. Drove from user-facing accuracy concerns ("README still says gatekeeper", "ROADMAP says 11 tools") rather than from any atom-level contradiction.

## Connections
- [[CONCEPT--AGENT-AGNOSTIC]]
- [[CONCEPT--AGENT-INTEGRATION-PATTERNS]]
- [[AUDIT--ARCH-DOC-CLEANUP]]
- [[AUDIT--PHASE-B-IMPL-COMPLETE]]
- [[AUDIT--PHASE-C-AGENT-INTEGRATION-DOCS]]

