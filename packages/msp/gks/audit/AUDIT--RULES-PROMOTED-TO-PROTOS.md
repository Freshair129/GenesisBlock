---
id: AUDIT--RULES-PROMOTED-TO-PROTOS
phase: 6
type: audit
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: M8f — promote 3 existing core rules to PROTO atoms (draft, overlap)
tags:
  - msp
  - proto
  - validator
  - audit
  - m8f
crosslinks: {"references":["CONCEPT--PROTO-AUDIT-EXISTING-RULES","ADR--ANTI-HALLUCINATION-RULES","FEAT--PROTO-LOADER","CONCEPT--PROTO-PATTERN"]}
linked_symbols:
  - {"file":"src/validator/proto/rule-adapter.ts"}
  - {"file":"src/validator/proto/summary-min.ts"}
  - {"file":"src/validator/proto/adr-monotonic.ts"}
  - {"file":"src/validator/proto/evidence-for-decisions.ts"}
created_at: 2026-05-05T13:00:00.000Z
---

# M8f — promote 3 existing rules → PROTO atoms

## Scope

Per `CONCEPT--PROTO-AUDIT-EXISTING-RULES`, 3 governance-flavoured rules from `src/validator/rules/` get PROTO atoms documenting them as governance contracts (instead of hardcoded structural rules). The original rule code stays in core for backward-compat; the PROTO ships `status: draft` so it runs but does not fail-exit. Cutover (PROTO promoted to stable + original removed) is M8f-2 follow-up.

## What shipped

| File | Purpose |
|---|---|
| `src/validator/proto/rule-adapter.ts` | Generic helper: `Rule → Predicate` adapter |
| `src/validator/proto/summary-min.ts` | wraps `summaryMin` |
| `src/validator/proto/adr-monotonic.ts` | wraps `adrMonotonic` (filter: type==='adr') |
| `src/validator/proto/evidence-for-decisions.ts` | wraps `evidenceForDecisions` (filter: type==='adr') |
| `gks/proto/PROTO--SUMMARY-MIN.md` | atom |
| `gks/proto/PROTO--ADR-MONOTONIC.md` | atom |
| `gks/proto/PROTO--EVIDENCE-FOR-DECISIONS.md` | atom |

## Decisions during impl

1. **Wrapper, not duplicate** — each PROTO predicate calls the existing core rule via `ruleAdapter`. No re-implementation, no logic drift between the two paths.
2. **Original rules stay in core** — overlap is intentional during draft phase. Once a PROTO is promoted to stable, the structural-rule entry can be removed. Stages the migration safely.
3. **`ruleAdapter` reads atom files from disk** — PROTO context only has frontmatter via `AtomicIndexEntry`. The adapter parses YAML + body to produce `ParsedAtom` (matching the core rule's input shape), then maps `ValidationError[] → PredicateViolation[]`.
4. **`filter` opt** — adr-monotonic + evidence-for-decisions only run on `type==='adr'`. Avoids per-atom file reads that would no-op anyway.

## Cutover plan (M8f-2, deferred)

When ready to remove the duplicate run:

1. Bump each PROTO from `draft` → `stable` (one PR per PROTO, with observation logs)
2. Remove the rule from `src/validator/index.ts`'s rule list
3. Update tests that exercised the old rule path to use the PROTO instead
4. Remove `src/validator/rules/<name>.ts` (or keep as a private helper that the PROTO imports)

Not urgent — the duplicate run is essentially free (same atom is read once for the structural pass, once for the PROTO pass). Real-world signal will tell us whether the PROTO surface is preferable.

## Verification

- `npm run typecheck` → clean
- `npm test` → 498 → **535 passed** (+37 — ALSO includes loader + sample tests merged earlier from M8a; not all from M8f)
- `npx tsx src/validator/cli.ts --all` → 9 PROTOs run; 7 pass + 2 surface real warnings against existing atoms (PROTO--PHASE-GATES + PROTO--SCALING-LEVEL-GATE flag stuff in the live tree). All draft → CI exit 0.
- `npm run msp:check-links` → OK

## Counts

- Atoms 150 → 154 (+ this AUDIT + 3 PROTOs)
- New impl files: 4 (`rule-adapter.ts` + 3 wrappers)
- New PROTO atoms: 3 (SUMMARY-MIN, ADR-MONOTONIC, EVIDENCE-FOR-DECISIONS)
- Test count: stable (no new tests; existing rule tests already cover the logic)

## What this AUDIT does NOT cover

- Cutover M8f-2 (remove duplicate run)
- Promotion of `cite-or-mark-inferred` to PROTO (was 4th candidate in CONCEPT — deferred since soft-warning rule, less urgent)

## Source

`CONCEPT--PROTO-AUDIT-EXISTING-RULES`, `FEAT--PROTO-LOADER`, M8a foundation.
