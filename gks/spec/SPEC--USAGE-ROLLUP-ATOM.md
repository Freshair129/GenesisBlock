---
id: SPEC--USAGE-ROLLUP-ATOM
phase: 2
type: spec
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: SPEC — Usage Roll-up Atom — weekly + monthly cost-aggregation contract
tags: &a1
  - msp
  - agents
  - usage
  - cost
  - rollup
  - spec
crosslinks: &a2
  references:
    - SPEC--USAGE-ATOM
    - CONCEPT--USAGE-ROLLUPS
    - CONCEPT--COST-TRACKING
    - ADR--AGENT-TIER-COST-POLICY
created_at: 2026-05-14T05:01:00.000+07:00
aliases: &a3
  - SPEC
  - implementation_flow
  - Technical specification
cluster: implementation_flow
role: Technical specification
attributes:
  id: SPEC--USAGE-ROLLUP-ATOM
  phase: 2
  type: spec
  status: stable
  vault_id: default
  tier: process
  source_type: axiomatic
  title: SPEC — Usage Roll-up Atom — weekly + monthly cost-aggregation contract
  tags: *a1
  crosslinks: *a2
  created_at: 2026-05-14T05:01:00.000+07:00
  aliases: *a3
  cluster: implementation_flow
  role: Technical specification
  attributes:
    id: SPEC--USAGE-ROLLUP-ATOM
    phase: 2
    type: spec
    status: stable
    vault_id: default
    tier: process
    source_type: axiomatic
    title: SPEC — Usage Roll-up Atom — weekly + monthly cost-aggregation contract
    tags: *a1
    crosslinks: *a2
    created_at: 2026-05-14T05:01:00.000+07:00
    aliases: *a3
    cluster: implementation_flow
    role: Technical specification
    attributes:
      domain: spec
    domain: spec
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: true
    secret_type: high_entropy_string
    leak_risk: high
    encryption_level: none
  domain: spec
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: true
  secret_type: high_entropy_string
  leak_risk: high
  encryption_level: none
---

# SPEC — Usage Roll-up Atom

## 1. What is a usage roll-up atom

A **usage roll-up atom** is a derived artefact written by `packages/msp/src/usage/rollup-writer.ts` that aggregates one or more `[[USAGE--DAILY]]-*` daily atoms (per `[[SPEC--USAGE-ATOM]]`) into a single weekly or monthly summary. Roll-ups are:

- **Derived** — every value is reconstructible by re-running the aggregator over the same date range.
- **Opt-in** — never written automatically by `dispatch()`; only when explicitly requested via `msp-usage rollup-{week,month} --write`.
- **Per-bucket-unique** — at most one weekly atom per ISO week, one monthly atom per calendar month.
- **Best-effort immutable** — like daily atoms, the writer rewrites the JSON block in place if the file exists. No enforcement; humans editing roll-up atoms is unusual but supported.

Where `[[USAGE--DAILY]]-*` is the **source of truth** for cost telemetry, roll-ups are the **readout layer** for week- and month-scale operator queries.

## 2. Id pattern

```
USAGE--WEEKLY-<isoWeek>
USAGE--MONTHLY-<YYYY-MM>
```

This SPEC extends the bucket-namespace reserved in `[[SPEC--USAGE-ATOM]]` §2 (`HOURLY` / `DAILY` / `WEEKLY` / `MONTHLY`).

- **`<isoWeek>`** uses ISO 8601 week date notation: `<YYYY>-W<WW>` where `WW` is the zero-padded ISO week number (`01`–`53`). ISO weeks start on Monday; week 01 is the week containing the first Thursday of the calendar year. Example: `2026-W19` = the week of 2026-05-04 to 2026-05-10.
- **`<YYYY-MM>`** is the calendar month: 4-digit year, hyphen, zero-padded month. Example: `2026-05`.

Examples:

```
USAGE--WEEKLY-2026-W19
USAGE--MONTHLY-2026-05
```

The id is unique per (bucket, period) — exactly one atom per ISO week, one per month.

## 3. Required frontmatter fields

| Field | Required | Value |
|---|---|---|
| `id` | yes | `[[USAGE--WEEKLY]]-<isoWeek>` or `[[USAGE--MONTHLY]]-<YYYY-MM>` |
| `phase` | yes | `5` (runtime / code phase per `gks/concept/[[CONCEPT--TAXONOMY-V2-3]].md`) |
| `type` | yes | `usage` (same type as the daily atom — distinction is in the id prefix) |
| `status` | yes | `stable` |
| `vault_id` | yes | `default` (or project-specific) |
| `tier` | yes | `genesis` |
| `source_type` | yes | `episodic` |
| `title` | yes | Human-readable summary, e.g. `USAGE — Weekly roll-up 2026-W19` or `USAGE — Monthly roll-up 2026-05` |
| `tags` | yes | `[agents, usage, cost, rollup, <weekly|monthly>]` |
| `created_at` | yes | ISO 8601 UTC of the write moment |

### Optional fields

| Field | When | Value |
|---|---|---|
| `updated_at` | best practice | ISO 8601 UTC of most recent rewrite |
| `period_start` | best practice | ISO date (`YYYY-MM-DD`) of the first day in the bucket — for weekly: the Monday; for monthly: day 1 |
| `period_end` | best practice | ISO date of the last day in the bucket — for weekly: the Sunday; for monthly: the last day of the month |
| `days_covered` | best practice | integer count of daily atoms that actually contributed (≤ bucket length; lower if the dispatcher had quiet days) |
| `total_cost_usd` | best practice | sum across the bucket; also appears in the body JSON |

## 4. Body contract

The body MUST contain a fenced `json` summary block delimited by HTML comment markers, identical to `[[SPEC--USAGE-ATOM]]` §4 but with a roll-up-specific payload shape:

```
<!-- USAGE-SUMMARY-START -->
```json
{
  "bucket": "weekly",
  "iso_period": "2026-W19",
  "period_start": "2026-05-04",
  "period_end": "2026-05-10",
  "total_cost_usd": 0.1234,
  "calls_by_tier": { "T1": 120, "T2": 47, "T3": 3 },
  "days_covered": 5,
  "top_episodes": [
    { "id": "[[EPISODE--AGENT-RUN-2026-05-07T14-22-00-000Z]]", "cost_usd": 0.045 }
  ],
  "generated_at": "2026-05-14T08:00:00.000Z"
}
```
<!-- USAGE-SUMMARY-END -->
```

For monthly atoms, `bucket` is `"monthly"` and `iso_period` is `"2026-05"`; the rest of the shape is identical.

The markers MUST appear exactly once. The aggregator/writer parses everything between them as JSON.

## 5. Lifecycle

Roll-up atoms have the same **two-phase lifecycle** as daily atoms:

1. **Open**: first `rollup-{week,month} --write` for a given bucket creates the file with full frontmatter + body.
2. **Rewrite**: subsequent invocations (re-aggregation after more dailies land, or after a hand-edited daily) parse + rewrite the JSON block in place; frontmatter is preserved except optional `updated_at`.

Unlike daily atoms, roll-ups are **never** written by the dispatcher's runtime path. They are write-on-demand artefacts. A nightly cron or CI job is the expected caller for archival commits.

## 6. Storage location

```
<root>/gks/usage/USAGE--WEEKLY-<isoWeek>.md
<root>/gks/usage/USAGE--MONTHLY-<YYYY-MM>.md
```

Roll-ups coexist with daily atoms in the same directory. The id-prefix distinguishes them; tooling that scans `[[USAGE--DAILY]]-*` will not accidentally pick up roll-ups, and vice versa.

## 7. Validation

`msp/LLM_Contract/atomic_contract.yaml`'s `required_fields.by_type.usage` already covers the required-frontmatter checks for both daily and roll-up atoms (same `type: usage`). No new contract entries needed.

## 8. Out of scope

- Hourly buckets (still reserved by name).
- Cross-vault aggregation.
- Garbage collection of stale roll-ups (deferred to a future `[[SPEC--USAGE-RETENTION]]`).
- Per-vault budget alerts.
- Streaming / push-based aggregation. The writer always re-reads dailies from disk.

## Connections
- [[CONCEPT--USAGE-ROLLUPS]]
- [[CONCEPT--COST-TRACKING]]
- [[ADR--AGENT-TIER-COST-POLICY]]

