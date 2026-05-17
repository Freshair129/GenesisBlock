---
id: CONCEPT--USAGE-ROLLUPS
phase: 1
type: concept
status: draft
vault_id: default
tier: genesis
source_type: axiomatic
title: Usage Roll-ups — weekly + monthly aggregates over daily USAGE atoms
tags:
  - msp
  - agents
  - cost
  - usage
  - rollup
  - observability
crosslinks:
  references:
    - CONCEPT--COST-TRACKING
    - SPEC--USAGE-ATOM
    - SPEC--USAGE-ROLLUP-ATOM
    - BLUEPRINT--COST-TRACKING
created_at: 2026-05-14T05:00:00.000+07:00
aliases:
  - CONCEPT
  - implementation_flow
  - Strategic intent / PRD
cluster: implementation_flow
role: Strategic intent / PRD
attributes:
  domain: concept
---

# CONCEPT — Usage Roll-ups

## Problem

`[[USAGE--DAILY]]-<isoDate>` atoms (per `[[SPEC--USAGE-ATOM]]`) capture per-day cost telemetry written by `usage-recorder.ts`. Daily granularity is the right unit for the **writer** (one file per UTC date, idempotent rewrites) but it is the **wrong unit for the reader**. Operators routinely ask:

1. "What did this week's dispatch cost?"
2. "Was last month over our T3 budget?"
3. "Which week had the noisiest tier-3 usage spike?"

Answering any of these by hand-scanning a directory of daily atoms is busywork — and gets worse linearly as the system runs longer. The dispatcher already writes the source-of-truth dailies; what is missing is an **aggregation + readout layer**.

## Approach

Two layers, deliberately decoupled — same shape as Cost-Tracking but **read-only consumer** of the daily writer:

### Layer 1 — pure aggregator (`packages/msp/src/usage/aggregator.ts`)

A dependency-free module that scans `<root>/gks/usage/[[USAGE--DAILY]]-*.md`, parses the JSON summary block from each (the same fenced block the recorder writes per `[[SPEC--USAGE-ATOM]]` §4), and produces a `UsageSummary` shape collapsing dailies across a range:

```typescript
interface UsageSummary {
  total_cost_usd: number
  calls_by_tier: { T1: number; T2: number; T3: number }
  days_covered: number
  top_episodes: { id: string; cost_usd: number }[]
}
```

Three entry points:
- `aggregateDaily(root, since?, until?)` — full scan, optional date filter.
- `aggregateWeek(root, weekIso)` — restricts to a specific ISO week (e.g. `2026-W19`).
- `aggregateMonth(root, monthIso)` — restricts to a specific month (e.g. `2026-05`).

The aggregator never writes — it is pure read. Aggregation math is identical to what the recorder does in-process for one day; the difference is that the aggregator unions N dailies.

### Layer 2 — roll-up atom writer (`packages/msp/src/usage/rollup-writer.ts`)

Runs an aggregator over a week or month and writes the result as a `[[USAGE--WEEKLY]]-<isoWeek>.md` or `[[USAGE--MONTHLY]]-<YYYY-MM>.md` atom (contract: `[[SPEC--USAGE-ROLLUP-ATOM]]`). Same body layout as the daily atom — frontmatter + a fenced JSON summary block between HTML comment markers — so any tool that already parses daily atoms also parses roll-ups.

Roll-ups are **opt-in writes**, gated behind the `--write` flag in the CLI. The reason: nightly cron or CI may want to commit the previous week's roll-up to git as an immutable artefact, but the default `msp-usage week` invocation should be a no-side-effect dashboard query.

### Layer 3 — CLI dashboard (`packages/msp/src/usage/cli.ts`)

The `msp-usage` binary surfaces the aggregator's output:

| Sub-command | Behaviour |
|---|---|
| `msp-usage today` | Today's daily totals (one-shot read of today's [[USAGE--DAILY]] atom). |
| `msp-usage week [--iso=...]` | Aggregate of the ISO week (default: current week). |
| `msp-usage month [--iso=...]` | Aggregate of the calendar month (default: current month). |
| `msp-usage rollup-week --iso=... --write` | Aggregate **and** write the weekly atom. |
| `msp-usage rollup-month --iso=... --write` | Aggregate **and** write the monthly atom. |

`--json` flips output from a human-readable table to a `UsageSummary` JSON document.

## What this is NOT

- **Not a new writer for daily atoms.** The daily atom contract is owned by `[[SPEC--USAGE-ATOM]]`; this concept layers on top.
- **Not a budget enforcer.** Same boundary as `[[CONCEPT--COST-TRACKING]]` — observation, not control.
- **Not real-time.** The aggregator reads files; if the dispatcher just wrote a daily-atom edit, you may need to re-run the query.
- **Not cross-vault.** Roll-ups are scoped to one repo / one `gks/usage/` directory. Multi-vault rollup is out of scope.
- **Not a retention policy.** Daily atoms are not deleted by roll-up; both coexist. Retention is a separate concern (see [[SPEC--USAGE-ATOM]] §5 "out of scope" carveout).

## Related

- `[[SPEC--USAGE-ATOM]]` — the daily atom contract this aggregator consumes
- `[[SPEC--USAGE-ROLLUP-ATOM]]` — frontmatter contract for the weekly/monthly atoms this concept introduces
- `[[BLUEPRINT--COST-TRACKING]]` — the Phase E3 plan that shipped dailies
- `[[CONCEPT--COST-TRACKING]]` — sibling concept; this one extends it from "record" to "read"
