---
id: ADR--EPISODE-GC-POLICY
phase: 2
type: adr
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: ADR — Episode GC policy — 30-day window + error preservation +
  archive-by-default
tags:
  - msp
  - phase-f4
  - agents
  - episode
  - retention
  - gc
  - decision
crosslinks:
  references:
    - CONCEPT--EPISODE-RETENTION
    - SPEC--EPISODE-ATOM
    - AUDIT--PHASE-F4-EPISODE-GC
created_at: 2026-05-14T05:05:00.000+07:00
aliases:
  - ADR
  - implementation_flow
  - Architecture decision record
cluster: implementation_flow
role: Architecture decision record
attributes:
  domain: adr
---

# ADR — Episode GC policy

## Context

`[[CONCEPT--EPISODE-RETENTION]]` motivates the need for bounded growth of `<root>/gks/episode/`. This ADR locks the **numeric defaults** and the **archive directory layout** so multiple consumers (CLI, future cron job, future consolidator loop) agree on one policy. Implementation: `packages/msp/src/agents/episode-gc.ts` + CLI `packages/msp/src/agents/episode-gc-cli.ts`.

## Decision

### 1. Retention window — 30 days

The default `keep_days = 30`. Rationale:

- Long enough to cover a typical sprint cycle, weekly retro, and most incident post-mortems.
- Short enough that on a developer machine the steady-state episode directory stays under a few thousand atoms.
- Configurable per-invocation via `--keep-days=<N>`; `--keep-days=0` archives everything that isn't an error (useful for one-shot cleanup or test fixtures).

### 2. Error preservation rule

Older-than-keep_days episodes are **kept** iff:

- inferred `ok === false` (any signal — see CONCEPT §"Inferring `ok`"), **or**
- `severity === 'critical'`.

The `critical` carve-out exists because a *successful critical* dispatch is still evidence we want forever (e.g., a recovery action that worked). The `ok === false` carve-out is the audit trail of failures.

### 3. Archive-by-default; delete is opt-in

Eligible old episodes are **moved** (renamed) to:

```
<root>/gks/episode/_archive/<YYYY-MM>/<original-filename>.md
```

`<YYYY-MM>` is derived from the *episode's* `created_at`, not from now. This:

- Keeps each archive subdirectory small (one month of episodes).
- Makes "delete everything older than 2025" a trivial `rm -rf` on a handful of subdirectories.
- Avoids re-archiving on a second GC pass (already-archived files are out of the scan path because `_archive/` is skipped at the top level).

`--delete` (CLI) / `opts.delete = true` (lib) skips the move and `unlink()`s the file instead.

### 4. Conservative default — implicit dry-run

The CLI requires `--apply` to mutate the filesystem. Without `--apply`, the run is functionally a dry-run: it prints what it *would* do and exits.

The library function `gcEpisodes()` defaults `dry_run: false` so callers in code (consolidator, cron) get the normal behaviour, but the CLI wrapper inverts this so a human typing `msp-episode-gc` cannot accidentally delete anything.

### 5. Scan scope

GC scans only the **top level** of `<root>/gks/episode/`. The `_archive/` subtree is explicitly skipped. Other subdirectories (none expected) are also skipped. This makes the operation idempotent: running GC twice in a row on the same input produces identical second-pass behaviour (the second pass sees zero eligible files).

### 6. `GcReport` shape

The library function returns and the CLI emits (with `--json`):

```ts
interface GcReport {
  total_scanned: number
  archived: number
  deleted: number
  kept: number
  errors: string[]
}
```

- `archived + deleted + kept === total_scanned` (less any read errors, which appear in `errors[]`).
- `errors` is non-fatal: a malformed episode whose frontmatter cannot be parsed is counted as a read error and **kept** (conservative — when in doubt, don't destroy).

## Consequences

### Positive

- Episode directory size is bounded in steady state.
- Failures and critical successes are never silently lost.
- Default behaviour is impossible to accidentally weaponise — the worst case of `msp-episode-gc` with no flags is "nothing happened, here's a report".
- One simple flag (`--delete`) covers the high-volume / sensitive-prompt case.

### Negative

- Two-step UX: a human running cleanup must run twice (once to inspect, once with `--apply`). This is intentional friction.
- Archive directory still grows over time (one subdirectory per month). Bulk cleanup of `_archive/` is a manual operator step; we intentionally do not "GC the GC archive."

### Neutral

- The default 30-day window is conservative and may be tuned via empirical usage. No automatic policy adjustment is in scope.
- We do *not* delete the `_archive/` directory even when empty. A `mkdir -p` cost per GC pass is trivial.

## Alternatives considered

1. **Hard delete by default** — rejected. Episodes are evidence; irreversibility is too high a price for default behaviour.
2. **Single tarball archive (`_archive/<YYYY-MM>.tar.gz`)** — rejected for v1. Adds a tarball dependency and complicates idempotency. Filesystem-native is enough; the operator can tar/zip on their own.
3. **Age-only policy (drop the error-preservation carve-out)** — rejected. Failures are the most valuable subset of episodes for MLL and post-mortems.
4. **Size-based policy (e.g., keep last 1000 episodes)** — rejected as primary policy. Age is a more natural unit for an evidence trail. The CLI's `--keep-days=0` is the "purge non-errors" escape hatch; a future `--keep-count=N` could be added if needed.

## Implementation pointers

- `gcEpisodes(root: string, opts?: GcOpts): Promise<GcReport>` — library entry point.
- `msp-episode-gc` — CLI wrapper (`packages/msp/src/agents/episode-gc-cli.ts`).
- Frontmatter parsing: `yaml` package.
- Filesystem ops: `node:fs/promises` (`readdir`, `readFile`, `mkdir`, `rename`, `unlink`).
- Tests: `packages/msp/test/agents/episode-gc.test.ts` (+ CLI smoke).

## Connections
- [[SPEC--EPISODE-ATOM]]
- [[AUDIT--PHASE-F4-EPISODE-GC]]

