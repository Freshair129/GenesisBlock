---
id: AUDIT--ATOM-WORKFLOW-SCRIPTS
phase: 6
type: audit
status: stable
tier: process
source_type: axiomatic
vault_id: default
title: AUDIT — atom workflow scripts (atom-date, scaffold-atom, supersede)
  verification
tags:
  - msp
  - audit
  - tooling
  - scripts
  - dx
crosslinks:
  references:
    - MASTER--ATOM-CONTRADICTION-POLICY
    - PROTO--SCALING-LEVEL-GATE
linked_symbols:
  - file: scripts/msp/atom-date.ts
  - file: scripts/msp/scaffold-atom.ts
  - file: scripts/msp/supersede.ts
  - file: packages/msp/test/scripts/workflow-scripts.test.ts
phase_override:
  skip_blueprint: true
  reason: Small atom-authoring CLI helper scripts (atom-date / scaffold-atom /
    supersede) — developer tooling, not a feature surface; a per-script phase-3
    blueprint would be doc theater.
created_at: 2026-05-12T22:05:00.000+07:00
aliases:
  - AUDIT
  - implementation_flow
  - Test results / quality report
cluster: implementation_flow
role: Test results / quality report
attributes:
  domain: audit
---

# AUDIT — atom workflow scripts (PR-C closure)

## Scope verified

Closes PR-C of `HANDOFF-SYMBOLS-EXPANSION-PHASE-2.md` — atom workflow scripts that eliminate recurring agent errors observed across PR #73 through #81.

### Scripts delivered

| Script | npm command | Purpose |
|---|---|---|
| `atom-date.ts` | `npm run msp:atom-date [-- --utc]` | Print current TH-time ISO with `+07:00` offset (or UTC `Z` with `--utc`). Stops the recurring "future-date" validator error from agents typing local wall-clock as `Z`. |
| `scaffold-atom.ts` | `npm run msp:scaffold-atom -- --type=<t> --slug=<S> [--title=<T>] [--root=<dir>]` | Generate a new atom with canonical frontmatter (correct status enum, phase per type, +07:00 timestamp, body skeleton). Generated atom validates clean immediately. |
| `supersede.ts` | `npm run msp:supersede -- --old=<ID> --new=<ID,...> [--root=<dir>]` | Atomically supersede an old atom with one or more replacements. Updates both sides reciprocally — flips old `status: superseded` + sets `superseded_by`; sets `supersedes` on each new atom. Refuses already-superseded atoms (idempotency safety). |

### Recurring bugs these fix

| Bug pattern (observed in PR #73-#81) | Fixed by |
|---|---|
| Agent types TH local wall-clock 17:35 with `Z` suffix → validator rejects as future | `msp:atom-date` outputs correctly-offset ISO |
| Agent uses wrong status enum (`proposed`, `accepted`) → validator warns/errors | `scaffold-atom` writes `status: draft` (canonical) |
| Agent uses wrong tier/source_type → validator warns | `scaffold-atom` writes `tier: process`, `source_type: axiomatic` (canonical) |
| Agent forgets reciprocal supersession crosslink → `[[MASTER--ATOM-CONTRADICTION-POLICY]]` violation | `msp:supersede` updates both sides atomically; refuses partial updates |
| Agent overwrites superseded atom by mistake | `msp:supersede` refuses if target already superseded |

## Test results

10 / 10 tests passing locally:
- `msp:atom-date` — 3 tests (default format, `--utc` flag, instant-equivalence)
- `msp:scaffold-atom` — 4 tests (creates valid atom, refuses overwrite, rejects bad slug, rejects unknown type)
- `msp:supersede` — 3 tests (reciprocal update, idempotency refusal, missing-atom rejection)

```
npx vitest run packages/msp/test/scripts/workflow-scripts.test.ts
Test Files  1 passed (1)
     Tests  10 passed (10)
  Duration  ~12s (most spent in tsx spawn cold-start)
```

## Validator integrity

- `npm run msp:index` → atomic_index regen clean
- `npm run msp:check-links` → PASS
- `npm run typecheck` → PASS
- All 3 scripts compile + run via tsx and via `npm run` (both code paths verified)

## Deviations from plan

| Plan item (HANDOFF Phase-2 §6) | Deviation | Reason |
|---|---|---|
| `--out=<path>` flag on scaffold-atom | omitted | `--root=<dir>` covers the same use case; reduces API surface |
| `--reason=<text>` flag on supersede | omitted | not strictly needed — supersession note belongs in the new atom's body, not a script flag. Easy to add later if requested. |
| README section documenting the 3 scripts | included separately | added to `packages/msp/README.md` (see PR diff) |

## Anti-hallucination check

- Tests use `mkdtempSync` fixtures (no dependency on real repo state)
- Each script is exercised via spawned `tsx` (mirrors real `npm run msp:*` invocation path — not import-based mock)
- Both happy and edge cases covered per script
- Time-sensitive test (`atom-date` instant equivalence) uses 5-second tolerance to absorb tsx cold-start variance on slow CI

## Follow-ups (out of scope here)

- **Migrating older agents' workflow** to use these scripts — handoff in HANDOFF-SYMBOLS-EXPANSION-PHASE-2.md and HANDOFF-PR-A-CLOSURE.md reference them so future Antigravity runs benefit
- **Pre-commit hook integration** — optional: hook could auto-call `msp:atom-date` to suggest timestamp if frontmatter missing. Defer; not blocking
- **PR-D retrofit** — independent next step. Now easier to retrofit FEAT→ADR chains because `scaffold-atom` produces compliant ADRs

## Source

- `packages/msp/HANDOFF-SYMBOLS-EXPANSION-PHASE-2.md` §6 (PR-C spec — non-atom doc, hence not in crosslinks)
- `[[PROTO--SCALING-LEVEL-GATE]]` (rule the FEAT→ADR retrofit will satisfy faster with these scripts)
- `[[MASTER--ATOM-CONTRADICTION-POLICY]]` (rule that `supersede.ts` enforces mechanically)
