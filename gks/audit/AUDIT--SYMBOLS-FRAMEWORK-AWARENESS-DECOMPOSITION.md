---
id: AUDIT--SYMBOLS-FRAMEWORK-AWARENESS-DECOMPOSITION
phase: 6
type: audit
status: stable
tier: process
source_type: axiomatic
vault_id: default
title: AUDIT — Decompose FEAT--SYMBOLS-FRAMEWORK-AWARENESS into CONCEPT + ADR + ALGO + PROTO; harden FEAT→ADR validator rule
tags:
  - msp
  - symbol-graph
  - audit
  - decomposition
  - atom-policy
  - validator
crosslinks: {"references":["ADR--SYMBOLS-FRAMEWORK-AWARENESS","CONCEPT--SYMBOLS-FRAMEWORK-AWARENESS","ALGO--SYMBOLS-FRAMEWORK-RECOGNITION","PROTO--SYMBOLS-FRAMEWORK-INVARIANTS","FEAT--SYMBOLS-FRAMEWORK-AWARENESS","PROTO--SCALING-LEVEL-GATE"]}
linked_symbols:
  - {"file":"packages/msp/src/validator/proto/scaling-level-gate.ts"}
phase_override:
  skip_blueprint: true
  reason: "The one touched file is a PROTO predicate on the already-blueprinted loader scaffold (BLUEPRINT--PROTO-LOADER); the decomposition work itself is governed by ADR--SYMBOLS-FRAMEWORK-AWARENESS + CONCEPT--SYMBOLS-FRAMEWORK-AWARENESS."
created_at: 2026-05-12T05:42:00.000+07:00
aliases:
  - AUDIT
  - implementation_flow
  - Test results / quality report
cluster: implementation_flow
role: "Test results / quality report"
---

# AUDIT — FEAT decomposition + validator hardening

## Scope verified

This PR delivers two coupled outcomes per `[[ADR--SYMBOLS-FRAMEWORK-AWARENESS]]`:

### 1. Atom decomposition (per ADR §1)
- `[[CONCEPT--SYMBOLS-FRAMEWORK-AWARENESS]]` created (P1)
- `[[ADR--SYMBOLS-FRAMEWORK-AWARENESS]]` created (P2)
- `[[ALGO--SYMBOLS-FRAMEWORK-RECOGNITION]]` created (P2)
- `[[PROTO--SYMBOLS-FRAMEWORK-INVARIANTS]]` created (P2, tier: safety, severity: error)
- `[[FEAT--SYMBOLS-FRAMEWORK-AWARENESS]]` superseded: status → `superseded`, reciprocal `superseded_by` → 4 new atoms, deprecation banner added at top of body, original content preserved verbatim below banner

### 2. Validator hardening (per ADR §5)
- `src/validator/proto/scaling-level-gate.ts` modified:
  - `HARD_ENFORCE_CUTOFF = 2026-05-12T00:00:00Z` constant introduced
  - FEAT atoms with `created_at >= cutoff` now emit `severity: 'error'` for missing CONCEPT/ADR/BLUEPRINT linkage
  - Older atoms grandfathered (`severity: 'warning'` as before) until retrofit per Phase-2 handoff PR-D
  - `status: superseded | deprecated` FEATs skipped (historical — no chain expectation)
  - `predicate.ok` now reflects only `error`-level violations (warnings don't block)

### 3. Type-coverage fix (downstream of `SymbolKind` extension)
- `KIND_SHORTHAND: Record<SymbolKind, string>` in all 3 parser files (`typescript.ts`, `python.ts`, `cobol.ts`) now exhaustively covers all 20 `SymbolKind` values including framework-aware kinds (page/layout/loading/error_boundary/route/template/middleware/not_found/entity/tool/data_loader/metadata_loader)
- Required because PR #77's `SymbolKind` type extension broke type-coverage in this branch; fixed forward

## Validator + crosslink integrity

| Check | Result |
|---|---|
| `npm run msp:index` | ✅ 222 atoms indexed (was 218 in PR #77; +4 new, −0 deletions; superseded counts in same total) |
| `npm run msp:check-links` | ✅ All crosslinks resolve |
| Validate 4 new atoms individually | ✅ 4/4 pass (warning on ADR tier 'architecture' matches existing convention) |
| `npm run typecheck` | ✅ Pass (after KIND_SHORTHAND fix) |

## Deviations from plan

| Plan item | Deviation | Reason |
|---|---|---|
| Author all 4 atoms from scratch | PROTO already existed (Antigravity created it in earlier turn); merged-into rather than overwrote | Antigravity's content was substantially correct; minor edits would have been duplicative |
| `[[ADR--SYMBOLS-FRAMEWORK-AWARENESS]]` tier: 'architecture' | Validator warns (allowed set: `safety/master/genesis/process`) | Match convention of other ADRs in `gks/adr/` (warning-only, not blocking) |
| `predicate.ok` change | Now `violations.every((v) => v.severity !== 'error')` instead of `violations.length === 0` | Required so warnings don't fail CI; preserves grandfather behavior |

## Anti-hallucination check

- All 4 new atoms cross-reference each other consistently (CONCEPT ← ADR ← ALGO/PROTO)
- Superseded FEAT's `superseded_by` lists exactly the 4 new atoms — no orphans, no extras
- Validator predicate code references the `linked_symbols` declared in `[[PROTO--SCALING-LEVEL-GATE]]` (no fabricated file paths)
- KIND_SHORTHAND fix verified: every `SymbolKind` literal in `types.ts` has a matching entry in all 3 parser maps

## Follow-ups (handled in Phase-2 handoff)

- **PR-A** (`HANDOFF-SYMBOLS-EXPANSION-PHASE-2.md`): implementation of recognizers per ALGO + validator predicate per PROTO
- **PR-B**: apply same decomposition pattern to `[[FEAT--SYMBOLS-PROCESS-TRACING]]`
- **PR-C**: atom workflow scripts (`atom-date`, `scaffold-atom`, `supersede`) — prevents the date-format / reciprocal-link bugs seen in Phase 1
- **PR-D**: retrofit existing FEATs that lack ADR (grandfather clause expires when 0 warnings remain); then remove `HARD_ENFORCE_CUTOFF`

## Source

- `[[ADR--SYMBOLS-FRAMEWORK-AWARENESS]]` (decision)
- `[[CONCEPT--SYMBOLS-FRAMEWORK-AWARENESS]]` (motivation)
- `[[ALGO--SYMBOLS-FRAMEWORK-RECOGNITION]]` (implementation contract)
- `[[PROTO--SYMBOLS-FRAMEWORK-INVARIANTS]]` (invariants)
- `[[FEAT--SYMBOLS-FRAMEWORK-AWARENESS]]` (superseded)
- `[[PROTO--SCALING-LEVEL-GATE]]` (validator rule hardened)
- `[[MASTER--ATOM-CONTRADICTION-POLICY]]` (decomposition justification)
- `HANDOFF-SYMBOLS-EXPANSION-PHASE-2.md` (continued work routed to Antigravity)

## Connections
- [[BLUEPRINT--PROTO-LOADER]]

