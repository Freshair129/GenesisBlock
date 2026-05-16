---
id: AUDIT--V0-4-0
phase: 6
type: audit
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: v0.4.0 close-out — Tier 2 PROTOs (M8a–f) + M9a + Tier 3 explicit defer
tags:
  - msp
  - audit
  - v0.4.0
  - tier-2
  - proto
  - close-out
crosslinks: {"references":["AUDIT--ALL-M-MILESTONES","AUDIT--PROTO-LOADER","AUDIT--RULES-PROMOTED-TO-PROTOS","CONCEPT--TIER-3-DEFERRED","CONCEPT--MSP-ROADMAP"]}
linked_symbols: []
created_at: 2026-05-05T20:25:00.000+07:00
---

# v0.4.0 close-out — Tier 2 PROTOs + M9a + Tier 3 defer

## Scope

Records the Tier 2 → impl jump from v0.3.0 to v0.4.0 driven by user direction "วางแผนและทำให้จบ ทุก M" + "ทำที่เหลือทั้งหมด". v0.3.0 had Tier 2 atoms scoped only; v0.4.0 ships them as draft PROTOs (predicates running but no fail-exit) plus the M8a foundation, M8f rule promotion, and an explicit Tier 3 defer atom.

## What shipped (since v0.3.0)

### Foundation

- **M8a — PROTO loader** (PR #31, merged): `src/validator/proto/{types,loader,sample}.ts` + `gks/proto/[[PROTO--SAMPLE-RULE]].md` + 19 tests + [[AUDIT--PROTO-LOADER]]. Generic governance-rule infrastructure; PROTO atoms `crosslinks.enforces` a FRAME and link a TS predicate.

### 5 Tier 2 governance PROTOs (parallel fan-out)

| Milestone | PR | Status | Severity | What it enforces |
|---|---|---|---|---|
| M8b | #35 | draft | error | Phase ordering: P5/P6 atoms need backing P3 BLUEPRINT |
| M8c | #34 | draft | warning | L1/L2/L3 chain consistency from atomic crosslinks |
| M8d | #33 | draft | error | Bi-directional `tunes ↔ tunable_by` reciprocity |
| M8e | #36 | draft | error | `.brain/msp/authority.yaml` shape validation |
| M9a | #37 | draft | warning | `valid_until` expiry guard (warning at expired, info at <30d) |

All ship `status: draft` so even error-severity violations don't fail-exit CI. Real atoms in this repo today flag a few real warnings (3 phase-6 AUDITs missing backing BLUEPRINT, 5 ADRs without CONCEPT, 1 FEAT missing ADR linkage) — visible as signal in `--all` output, non-blocking.

### M8f — promote 3 existing rules to PROTOs (this PR)

- **`[[PROTO--SUMMARY-MIN]]`** wraps `summaryMin`
- **`[[PROTO--ADR-MONOTONIC]]`** wraps `adrMonotonic`
- **`[[PROTO--EVIDENCE-FOR-DECISIONS]]`** wraps `evidenceForDecisions`

Via `src/validator/proto/rule-adapter.ts` — a generic Rule→Predicate adapter. Original rules continue running in core (overlap during draft phase); cutover is M8f-2 follow-up. [[AUDIT--RULES-PROMOTED-TO-PROTOS]] records the decision.

### Tier 3 explicit defer

- **`[[CONCEPT--TIER-3-DEFERRED]]`** (PR #32, merged): records explicit defer rationale + revisit triggers + effort estimates for M9c (cross-repo verify-flow), M9d (Notion migration), M9e (auto-ADR generator), M10a (msp-bridge plugin), M10b (Kuzu/Neo4j backend), M10c (RRF tuning).

## Counts at v0.4.0

| Metric | Value |
|---|---|
| Atoms in `gks/` | **159** |
| Tests | **535** passed, 0 failed |
| Validator | 159/159 atoms pass + 9 PROTOs (7 pass, 2 surface real warnings — all draft → exit 0) |
| MCP tools | 11 (unchanged from v0.3.0) |
| AUDIT atoms | 27 |
| PROTO atoms | 9 (1 sample + 8 governance) |
| FEATs implemented | 17 (PROTO loader added) |

## What v0.4.0 enables vs v0.3.0

**v0.3.0** = passport works (recall/remember/compress/identity over GKS). Governance was descriptive (FRAME atoms).

**v0.4.0** = governance is also **mechanical**. PROTO loader runs predicates as part of every `msp:validate --all`. Each PROTO atom encodes a rule + links its TS impl. Status `draft` = observation phase; `stable` = hard gate; `superseded` = retired. Gradual rollout without CI breakage.

Real-world signal during draft phase determines when each PROTO is promoted to stable. Until then, MSP has the **mechanism** for governance enforcement even if specific rules are still in observation.

## Tier 1 / 2 / 3 status snapshot

```
Tier 1 (shipped + impl)
  M0–M6 ............. ✅
  M7-prep + follow-up ✅
  M7a Obsidian client ✅
  M7b Consolidator ... ✅
  M7c Recall/RRF ..... ✅
  M7d Compressor ..... ✅
  M7e Identity ....... ✅
  M7f MCP tools (5/5) ✅
  M8a PROTO loader ... ✅
  M9f Windows lock ... ✅

Tier 2 (atoms + draft impl, awaiting real-world signal for stable)
  M8b PHASE-GATES ........ 🟡 draft
  M8c SCALING-LEVEL-GATE . 🟡 draft
  M8d ALGO-PARAM-COUPLING  🟡 draft
  M8e AUTHORITY-ENFORCEMENT🟡 draft
  M8f rule promotion ..... 🟡 draft (3 of 4 promoted)
  M9a VALID-UNTIL ........ 🟡 draft
  M9b Delegation policy .. ✅ ADR-only (pure policy, shipped at v0.3.0)

Tier 3 (deferred — explicit triggers)
  M9c cross-repo verify-flow .. ⏸ awaits GKS upstream
  M9d Notion migration ........ ⏸ awaits real workspace adoption
  M9e Auto-ADR generator ...... ⏸ awaits 30+ ADR corpus
  M10a msp-bridge plugin ...... ⏸ awaits vault > 5,000 atoms
  M10b Kuzu/Neo4j backend ..... ⏸ awaits crosslinks > 50,000
  M10c RRF tuning ............. ⏸ awaits labeled corpus
```

## Pending follow-ups (not blocking)

| Follow-up | Trigger |
|---|---|
| M8b–e PROTOs draft → stable | Each one observed against real workloads; once happy, single PR per PROTO bumps `status: stable` |
| M8f-2 cutover (remove duplicate rule run) | After all M8f PROTOs stable |
| `cite-or-mark-inferred` 4th promotion | Lower priority; soft warning, less urgent |
| Push `v0.4.0` tag to remote | HTTP 403 on auth scope — user pushes manually |
| Submit 5 GKS upstream proposals | `upstream/gks-proposals/SUBMISSION.md` ready for relay |

## Source

User direction "วางแผนและทำให้จบ ทุก M" → "ทำที่เหลือทั้งหมด". Closes Tier 2 to draft-impl + Tier 3 to explicit defer. v0.4.0 is the mechanical-governance milestone — the framework now enforces what it documents.

## Connections
- [[AUDIT--ALL-M-MILESTONES]]
- [[CONCEPT--MSP-ROADMAP]]

