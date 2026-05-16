---
id: AUDIT--PHASE-F4-EPISODE-GC
phase: 6
type: audit
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: Phase F4 — episode retention + GC (gcEpisodes + msp-episode-gc CLI)
tags:
  - msp
  - phase-f4
  - audit
  - agents
  - episode
  - retention
  - gc
crosslinks:
  references:
    - CONCEPT--EPISODE-RETENTION
    - ADR--EPISODE-GC-POLICY
    - SPEC--EPISODE-ATOM
linked_symbols: []
created_at: 2026-05-14T05:08:00.000+07:00
---

# AUDIT — Phase F4 episode retention + GC

## Scope

Phase F4 of the post-monorepo-pivot ROADMAP. Closes the long-standing TODO in `[[SPEC--EPISODE-ATOM]]` §9 ("Retention policy / garbage collection") for the project-local episode directory `<root>/gks/episode/`.

The write side (`result-recorder.ts`, `dispatch.ts`) is **untouched** — GC is strictly out-of-band.

## What shipped

### New atoms

| Atom | Purpose |
|---|---|
| `gks/concept/[[CONCEPT--EPISODE-RETENTION]].md` | Goals, default policy, inference rules for `ok` and `severity`, archive-vs-delete rationale. |
| `gks/adr/[[ADR--EPISODE-GC-POLICY]].md` | Locks numeric defaults (30-day window) and archive layout (`<root>/gks/episode/_archive/<YYYY-MM>/`). |
| `gks/audit/[[AUDIT--PHASE-F4-EPISODE-GC]].md` | This atom. |

### New code

| File | Purpose |
|---|---|
| `packages/msp/src/agents/episode-gc.ts` | `gcEpisodes(root, opts?)` library entry point. Reads each `EPISODE--*.md` under `<root>/gks/episode/` (top-level only), parses YAML frontmatter via the `yaml` package, infers `ok` + `severity` per CONCEPT §"Inferring `ok`", and archives or deletes per ADR §3. Returns `GcReport`. |
| `packages/msp/src/agents/episode-gc-cli.ts` | `msp-episode-gc` CLI: `--keep-days=N`, `--delete`, `--dry-run`, `--apply`, `--root=PATH`, `--json`, `--help`. Default is implicit dry-run — `--apply` required to mutate. |

### New tests

| File | Purpose |
|---|---|
| `packages/msp/test/agents/episode-gc.test.ts` | tmpdir-fixture tests: 5 fake episodes (2 recent, 2 old-ok-non-critical, 1 old-error). Verifies dry-run plans without mutating, default archive, `--delete` semantics, `keep_days: 0` edge case, idempotency, and inference rule from body-only metadata. |
| `packages/msp/test/agents/episode-gc-cli.test.ts` | CLI smoke: `--help`, implicit-dry-run, `--apply`, `--apply --delete`, `--json`, bad-flag error path. |

### Package wiring

- `packages/msp/package.json` — added bin entry: `"msp-episode-gc": "./dist/agents/episode-gc-cli.js"`.

## Default policy (locked by [[ADR--EPISODE-GC-POLICY]])

- `keep_days = 30`
- Keep recent (`created_at ≥ now − keep_days`) episodes.
- For older episodes, keep if `ok === false` OR `severity === 'critical'`.
- Eligible old episodes are **archived** by default to `<root>/gks/episode/_archive/<YYYY-MM>/<filename>.md` (month from episode's own `created_at`).
- `opts.delete = true` swaps archive for `unlink()`.
- `opts.dry_run = true` returns the same `GcReport` without touching disk.
- CLI requires explicit `--apply` to mutate (implicit dry-run for human safety).

## Inference rules (locked by [[CONCEPT--EPISODE-RETENTION]])

`ok` (in priority order; first match wins):

1. Frontmatter `ok: false` → false.
2. Frontmatter `exit_code: <n>` with `n !== 0` → false.
3. No `- tier_used:` body bullet → false (malformed; keep).
4. Empty `## Output` body block → false.
5. Otherwise true.

`severity`:

1. Frontmatter `severity:` if present.
2. Else body bullet `- task.severity: <x>`.
3. Else default `regular`.

## Verification

| Command | Result |
|---|---|
| `npm test --workspace=packages/msp -- test/agents/episode-gc` | Green (see PR for exact count). |
| `npm run typecheck` | Green. |
| `npm run msp:index && tsx packages/msp/src/validator/cli.ts --root=. gks/concept/[[CONCEPT--EPISODE-RETENTION]].md gks/adr/[[ADR--EPISODE-GC-POLICY]].md gks/audit/[[AUDIT--PHASE-F4-EPISODE-GC]].md` | All three atoms validate. |

## What this does NOT change

- `result-recorder.ts` — untouched.
- `dispatch.ts` — untouched.
- Validator contract (`msp/LLM_Contract/atomic_contract.yaml`) — untouched.
- `~/.brain/episodic/` migration (still per `[[ADR--BRAIN-PATH-RESOLUTION]]` open question §7) — out of scope.

## Follow-ups (not blocking)

- Wire GC into the future "consolidator" loop so steady-state housekeeping is automatic.
- Once `~/.brain/episodic/` migration lands, point GC at that location too.
- Consider adding `--keep-count=N` as a complementary policy if age-only proves coarse for high-volume agents.
- Consider compressing `_archive/<YYYY-MM>/` to tarballs after some grace period (cron-driven, not GC-driven).

## Source

User direction "Phase F4 — Episode Retention + GC" in agent task brief 2026-05-14.
