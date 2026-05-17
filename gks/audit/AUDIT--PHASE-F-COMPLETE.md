---
id: AUDIT--PHASE-F-COMPLETE
phase: 6
type: audit
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: AUDIT — Phase F (refinements) — implementation complete
tags:
  - msp
  - audit
  - phase-f
  - master
  - usage
  - episode
  - gc
crosslinks:
  references:
    - AUDIT--PHASE-F1-MASTER-GENESIS-WIRING
    - AUDIT--PHASE-F2-COST-DASHBOARD
    - AUDIT--PHASE-E1-REAL-CLI-WIRING
    - AUDIT--PHASE-D-AGENTIC-RUNTIME-COMPLETE
    - CONCEPT--PROMOTED-BLOCK-REGISTRY
    - CONCEPT--USAGE-ROLLUPS
    - CONCEPT--EPISODE-RETENTION
    - ADR--EPISODE-GC-POLICY
    - SPEC--USAGE-ROLLUP-ATOM
    - BLUEPRINT--MASTER-RUNTIME-INTEGRATION
created_at: 2026-05-14T05:30:00.000+07:00
aliases:
  - AUDIT
  - implementation_flow
  - Test results / quality report
cluster: implementation_flow
role: Test results / quality report
attributes:
  domain: audit
---

# AUDIT — Phase F (refinements) — implementation complete

## 1. Summary

Phase F of `[[ULTRAPLAN--AGENTIC-MONOREPO-PIVOT]]` is complete. It closes
loose ends from Phase E by wiring promotion ↔ runtime, surfacing the
cost data already captured in Phase E3, and adding a retention story
for the episode atoms that started piling up after Phase D.

Three independent streams shipped in parallel: F1, F2, F4. F3 (master
recall ergonomics) was rolled into F1's `findActiveMaster` helper and
did not need its own PR.

## 2. What shipped

### 2.1 PRs

| PR | Stream | Subject |
|---|---|---|
| #124 | F1 | Master Promotion ↔ Genesis Runtime wiring — `registry.ts` + `promote-apply.ts` + `executor.from_master` |
| #125 | F2 | Cost dashboard + USAGE weekly/monthly roll-ups — `aggregator.ts` + `rollup-writer.ts` + `msp-usage` CLI |
| #126 | F4 | Episode retention + GC policy — `episode-gc.ts` + `msp-episode-gc` CLI |
| this | closeout | This audit + ROADMAP update |

### 2.2 Working features

- **`msp-master-propose apply <path>`** — moves a Master proposal from
  `gks/inbound/` to `gks/master/` and appends a `registry.jsonl` entry,
  making the promoted Block first-class to the executor.
- **`ExecuteResult.from_master?: boolean`** — runtime now flags when a
  Block resolved through the active-master registry vs. the on-disk
  manifest, letting callers branch on provenance.
- **`msp-usage today | week | month | rollup-week | rollup-month`** —
  daily aggregation over the `USAGE--*` atoms emitted by Phase E3,
  with `--write` to persist `[[USAGE--WEEKLY]]-*` / `[[USAGE--MONTHLY]]-*`
  roll-up atoms.
- **`msp-episode-gc`** — applies the retention policy declared in
  `[[ADR--EPISODE-GC-POLICY]]`: keep the last 30 days; for older episodes,
  keep `ok === false` and `severity === 'critical'`; archive the rest
  to `gks/episode/_archive/<YYYY-MM>/`. `--delete` swaps archive for
  unlink. CLI defaults to dry-run; `--apply` required to mutate.

### 2.3 Atom additions

- `[[CONCEPT--PROMOTED-BLOCK-REGISTRY]]`, `[[BLUEPRINT--MASTER-RUNTIME-INTEGRATION]]` (F1)
- `[[CONCEPT--USAGE-ROLLUPS]]`, `[[SPEC--USAGE-ROLLUP-ATOM]]` (F2)
- `[[CONCEPT--EPISODE-RETENTION]]`, `[[ADR--EPISODE-GC-POLICY]]` (F4)
- Per-stream audits: `[[AUDIT--PHASE-F1-MASTER-GENESIS-WIRING]]`,
  `[[AUDIT--PHASE-F2-COST-DASHBOARD]]`, `[[AUDIT--PHASE-F4-EPISODE-GC]]`
- This roll-up audit.

### 2.4 Test coverage delta

- F1: 85 tests passing (registry + apply + executor)
- F2: 38 tests passing (aggregator + rollup-writer + CLI)
- F4: 22 new tests (13 lib + 9 CLI); full agents suite 104 tests still green

### 2.5 Bin entries

`packages/msp/package.json` `bin` grew by two:

- `msp-usage` → `./dist/usage/cli.js` (F2)
- `msp-episode-gc` → `./dist/agents/episode-gc-cli.js` (F4)

## 3. Known follow-ups

- The episode-storage contradiction surfaced in Phase D (`[[ADR--BRAIN-PATH-RESOLUTION]]`
  vs. `result-recorder.ts`) is **still open**. F4 retention runs against
  whatever path is in use; it does not resolve where episodes *should*
  live. Tracked in `[[SPEC--EPISODE-ATOM]]` §7.
- `[[USAGE--WEEKLY]]-*` and `[[USAGE--MONTHLY]]-*` are emitted by F2 but the
  validator does not have dedicated `required_fields` rules for them
  yet — they pass via the default fallback. A `[[SPEC--USAGE-ROLLUP-ATOM]]`
  upgrade to strict required-fields is deferred.
- The `from_master` flag is exposed on `ExecuteResult` but the
  `msp-genesis-exec` CLI does not yet surface it in human-readable
  output (only structured JSON). Cosmetic; deferred.

## 4. Verification

- `npm run test` — green on Node 20 + 22 for all three F-stream PRs.
- `npm run typecheck` — green.
- `npm run msp:validate` — green for the new atoms at audit time.
- `npm run msp:check-links` — green; crosslinks resolve.

## 5. Closeout sign-off

Phase F is closed. The agentic monorepo pivot's three implementation
phases (D — runtime, E — features, F — refinements) are now all
complete. The repo can dispatch tasks (`msp-dispatch`), record what
happened (`result-recorder`), aggregate cost (`msp-usage`), promote
blocks (`msp-master-propose`), execute them (`msp-genesis-exec`), and
garbage-collect old episodes (`msp-episode-gc`) — all from a single
workspace.

## Connections
- [[AUDIT--PHASE-E1-REAL-CLI-WIRING]]
- [[AUDIT--PHASE-D-AGENTIC-RUNTIME-COMPLETE]]

