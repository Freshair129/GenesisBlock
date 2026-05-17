---
id: AUDIT--PHASE-F2-COST-DASHBOARD
phase: 6
type: audit
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: Phase F2 — Cost Dashboard + USAGE roll-ups — what shipped
tags: &a1
  - msp
  - phase-f2
  - cost
  - usage
  - rollup
  - dashboard
  - audit
crosslinks: &a2
  references:
    - CONCEPT--USAGE-ROLLUPS
    - SPEC--USAGE-ROLLUP-ATOM
    - SPEC--USAGE-ATOM
    - CONCEPT--COST-TRACKING
    - BLUEPRINT--COST-TRACKING
linked_symbols: &a3
  - file: packages/msp/src/usage/aggregator.ts
  - file: packages/msp/src/usage/rollup-writer.ts
  - file: packages/msp/src/usage/cli.ts
phase_override: &a4
  skip_blueprint: true
  reason: Phase F2 refinement stream — planned via CONCEPT--USAGE-ROLLUPS +
    SPEC--USAGE-ROLLUP-ATOM (the doc-to-code chain) rather than a separate
    phase-3 blueprint.
created_at: 2026-05-14T05:10:00.000+07:00
aliases: &a5
  - AUDIT
  - implementation_flow
  - Test results / quality report
cluster: implementation_flow
role: Test results / quality report
attributes:
  id: AUDIT--PHASE-F2-COST-DASHBOARD
  phase: 6
  type: audit
  status: stable
  vault_id: default
  tier: process
  source_type: axiomatic
  title: Phase F2 — Cost Dashboard + USAGE roll-ups — what shipped
  tags: *a1
  crosslinks: *a2
  linked_symbols: *a3
  phase_override: *a4
  created_at: 2026-05-14T05:10:00.000+07:00
  aliases: *a5
  cluster: implementation_flow
  role: Test results / quality report
  attributes:
    id: AUDIT--PHASE-F2-COST-DASHBOARD
    phase: 6
    type: audit
    status: stable
    vault_id: default
    tier: process
    source_type: axiomatic
    title: Phase F2 — Cost Dashboard + USAGE roll-ups — what shipped
    tags: *a1
    crosslinks: *a2
    linked_symbols: *a3
    phase_override: *a4
    created_at: 2026-05-14T05:10:00.000+07:00
    aliases: *a5
    cluster: implementation_flow
    role: Test results / quality report
    attributes:
      domain: audit
    domain: audit
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: true
    secret_type: high_entropy_string
    leak_risk: high
    encryption_level: none
  domain: audit
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: true
  secret_type: high_entropy_string
  leak_risk: high
  encryption_level: none
---

# AUDIT — Phase F2 Cost Dashboard + USAGE Roll-ups

## Scope

Phase F2 of the post-monorepo-pivot ROADMAP. Lands the **reader** half of the cost-tracking system: aggregator over daily USAGE atoms, weekly + monthly roll-up atom writer, and `msp-usage` dashboard CLI. Phase E3 (PR #120) already shipped the writer half (`usage-recorder.ts` + `cost-tracker.ts`); F2 layers consumption on top without touching the writer.

## What shipped

### Atoms (3)

| Atom | Purpose |
|---|---|
| `gks/concept/[[CONCEPT--USAGE-ROLLUPS]].md` | What + why for weekly/monthly aggregation over the existing daily atoms. Establishes the read-only consumer model. |
| `gks/spec/[[SPEC--USAGE-ROLLUP-ATOM]].md` | Frontmatter contract for `[[USAGE--WEEKLY]]-<isoWeek>` and `[[USAGE--MONTHLY]]-<YYYY-MM>` atoms. Extends `[[SPEC--USAGE-ATOM]]` §2's reserved bucket name-space. |
| `gks/audit/[[AUDIT--PHASE-F2-COST-DASHBOARD]].md` | This atom. |

### Code (3 modules)

| File | Lines (approx) | Purpose |
|---|---|---|
| `packages/msp/src/usage/aggregator.ts` | ~280 | Read-only scan of `gks/usage/[[USAGE--DAILY]]-*.md`, parses JSON summary blocks, reduces to `UsageSummary { total_cost_usd, calls_by_tier, days_covered, top_episodes }`. Exports `aggregateDaily`, `aggregateWeek`, `aggregateMonth`, `aggregateSingleDay`. Includes pure ISO-week helpers (`isoWeekOf`, `isoWeekStart`, `isoWeekEnd`, `formatIsoWeek`, `parseIsoWeek`, `parseIsoMonth`, `formatIsoMonth`) — no external date deps. |
| `packages/msp/src/usage/rollup-writer.ts` | ~170 | Aggregates and writes `[[USAGE--WEEKLY]]-<iso>.md` / `[[USAGE--MONTHLY]]-<iso>.md` per `[[SPEC--USAGE-ROLLUP-ATOM]]`. Same body layout as the daily atom (fenced JSON between HTML comment markers); preserves frontmatter on re-runs. |
| `packages/msp/src/usage/cli.ts` | ~210 | `msp-usage` CLI with 5 subcommands: `today`, `week`, `month`, `rollup-week`, `rollup-month`. `--json` flag emits raw `UsageSummary`; `--write` is required to persist roll-ups. ANSI colours gated on `process.stdout.isTTY && !process.env.NO_COLOR`. |

### Tests (3 suites, 38 tests)

| File | Tests | Coverage |
|---|---|---|
| `packages/msp/test/usage/aggregator.test.ts` | 18 | ISO week/month helpers (round-trip, boundary cases, malformed-input rejection); file scanning with tmpdir fakes; week + month filtering; top-episode sort + cap-at-5; ignores non-daily files. |
| `packages/msp/test/usage/rollup-writer.test.ts` | 5 | Weekly + monthly atom creation; frontmatter preservation on re-run; empty-week handling; invalid-month rejection. |
| `packages/msp/test/usage/cli.test.ts` | 15 | Help + arg-parse errors; read subcommands (`today`/`week`/`month`); `--json` and `--iso` flags; current-week / current-month defaults; rollup dry-run vs. `--write` persistence. |

### Bin entry

Added `"msp-usage": "./dist/usage/cli.js"` to `packages/msp/package.json`.

## CLI subcommands

| Sub-command | Default behaviour | With `--write` | With `--json` |
|---|---|---|---|
| `msp-usage today` | print today's daily totals as human table | n/a | emit `UsageSummary` JSON for today |
| `msp-usage week [--iso=YYYY-Www]` | print weekly aggregate | n/a | emit JSON |
| `msp-usage month [--iso=YYYY-MM]` | print monthly aggregate | n/a | emit JSON |
| `msp-usage rollup-week --iso=YYYY-Www` | dry-run (aggregate + print) | write `[[USAGE--WEEKLY]]-<iso>.md` | emit JSON |
| `msp-usage rollup-month --iso=YYYY-MM` | dry-run (aggregate + print) | write `[[USAGE--MONTHLY]]-<iso>.md` | emit JSON |

## Verification matrix

| Check | Result |
|---|---|
| `npm test --workspace=packages/msp -- test/usage/` | 38 / 38 passing |
| `npm run typecheck` (root, all workspaces) | clean, no errors |
| Atoms validate via `msp-validate` | passes (see commit) |

## What this is NOT

- **Not a budget enforcer.** Read-only consumer of dailies; cost capping remains the policy layer's job (`[[ADR--AGENT-TIER-COST-POLICY]]`).
- **Not a hot-path writer.** `dispatch()` continues to call `recordUsage()` (daily atom only). Roll-ups are write-on-demand.
- **Not an automated rollup.** No cron / scheduled job ships in this phase; CI or operators must invoke `msp-usage rollup-* --write` explicitly.

## Out of scope / follow-ups

- Hourly buckets (still reserved by `[[SPEC--USAGE-ROLLUP-ATOM]]`).
- Cross-vault aggregation.
- Retention / GC of stale roll-up atoms (separate `[[SPEC--USAGE-RETENTION]]` proposal).
- Cost-by-tier breakdown in `UsageSummary` (currently only counts-by-tier; cost-by-tier could be added when readers actually need it).
- Optional CI job that auto-writes the previous week's roll-up on Monday — viable future enhancement; not blocking.

## Source

User direction "Phase F2 — Cost Dashboard + USAGE Roll-ups" agent task brief 2026-05-14; builds directly on Phase E3 (PR #120) `usage-recorder.ts` output contract.

## Connections
- [[CONCEPT--COST-TRACKING]]
- [[BLUEPRINT--COST-TRACKING]]

