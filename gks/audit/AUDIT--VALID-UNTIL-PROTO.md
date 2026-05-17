---
id: AUDIT--VALID-UNTIL-PROTO
phase: 6
type: audit
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: M9a — PROTO--VALID-UNTIL decision atrophy guard (draft)
tags:
  - msp
  - proto
  - lifecycle
  - valid-until
  - atrophy
  - audit
  - m9a
crosslinks:
  references:
    - PROTO--VALID-UNTIL
    - CONCEPT--DECISION-ATROPHY-GUARDS
    - FEAT--PROTO-LOADER
    - FRAMEWORK--MSP-ARCHITECTURE-V2
linked_symbols:
  - file: packages/msp/src/validator/proto/valid-until.ts
  - file: gks/proto/PROTO--VALID-UNTIL.md
  - file: packages/msp/test/validator/proto/valid-until.test.ts
phase_override:
  skip_blueprint: true
  reason: "Incremental PROTO rule plugged into the already-blueprinted loader
    scaffold (BLUEPRINT--PROTO-LOADER). Doc-to-code chain:
    CONCEPT--DECISION-ATROPHY-GUARDS -> PROTO--VALID-UNTIL -> predicate -> this
    audit; per-rule predicates do not each warrant a separate phase-3
    blueprint."
created_at: 2026-05-05T18:11:00.000+07:00
aliases:
  - AUDIT
  - implementation_flow
  - Test results / quality report
cluster: implementation_flow
role: Test results / quality report
attributes:
  domain: audit
---

# M9a — [[PROTO--VALID-UNTIL]] (draft)

## Scope

Implements Guard 1 from `[[CONCEPT--DECISION-ATROPHY-GUARDS]]`: a scanning predicate that flags atoms whose `valid_until:` frontmatter date has passed (or is approaching). Layered on top of the M8a PROTO loader; no new infrastructure.

## What shipped

| File | Purpose |
|---|---|
| `src/validator/proto/valid-until.ts` | Predicate impl. Reads each atom file, parses frontmatter, compares `valid_until` to `now`. Warning if expired, info if within 30 days. |
| `gks/proto/[[PROTO--VALID-UNTIL]].md` | PROTO atom (status: draft, severity: warning). Enforces `[[FRAMEWORK--MSP-ARCHITECTURE-V2]]`. |
| `test/validator/proto/valid-until.test.ts` | 6 tests covering vacuous pass, expired, near-expiry, far-future, superseded skip, and `MSP_NOW` env override. |

## Behaviour

- For every atom in `ctx.atomicIndex` (excluding `status: superseded`), open `gks/<atom.path>` and parse YAML frontmatter.
- If frontmatter has a `valid_until:` ISO 8601 date:
  - `now > valid_until` → warning (`expired N days ago`).
  - `0 <= valid_until - now < 30 days` → info (`expires in N days`).
  - Otherwise no violation.
- Atoms without `valid_until` are silently skipped.

## Decisions during impl

1. **Read atom files directly, not from `AtomicIndexEntry`.** `valid_until` is not part of the `AtomicIndexEntry` shape today, so the predicate opens `gks/<path>` and parses frontmatter on the fly. Cost is one read per atom; acceptable for a periodic / on-demand scan. Avoids touching the index schema (no shared-infra mutation).

2. **`MSP_NOW` env var for time injection.** The `Predicate` signature is `(ctx) => PredicateResult` — no opts. To make tests deterministic, the impl reads `MSP_NOW` (any ISO 8601 string) before falling back to `new Date()`. Tests that exercise expiry / near-expiry set `MSP_NOW` and assert against a known frame of reference.

3. **`status: draft` at ship.** Real atoms in this repo (notably `[[CONCEPT--MSP-ROADMAP]]`) reference a `valid_until` in their body text, and other atoms may add real `valid_until` frontmatter going forward. Shipping draft means the loader reports findings without ever fail-exiting CI — operators can review, decide, and only then promote to stable.

4. **No `error` severity emitted.** Even on expiry, the violation is `warning`. The CONCEPT explicitly classifies atrophy as a process concern, not a CI fail. `result.ok` therefore stays `true` even when violations are present (the loader's `shouldFailExit` only flips on stable + error).

5. **Date wrapped in quotes in test fixtures.** `parse-yaml` would otherwise turn a bare `valid_until: 2026-01-01` into a JS `Date`, not a string. Predicate handles both (string and Date), but tests deliberately serialise as quoted strings to mirror typical hand-written frontmatter.

## Verification

- `npx vitest run test/validator/proto/valid-until.test.ts` → **6 passed**
- `npm run typecheck` → clean
- `npm test` → full suite green (vitest)
- `npx tsx src/validator/cli.ts --all` → 145/145 atoms pass; 2 PROTOs run (`SAMPLE-RULE`, `VALID-UNTIL`); both pass; **no `valid_until` frontmatter in this repo today** so no atrophy violations emit yet.
- `npm run msp:check-links` → OK

## Counts

- Atoms 144 → 145 (+1: [[PROTO--VALID-UNTIL]]; +1 audit doesn't count toward live atoms — same gks/audit pattern)
- Tests: +6 (`valid-until.test.ts`)
- New impl file: `src/validator/proto/valid-until.ts`

## Hard constraints respected

- No shared-infra modifications (loader / types / CLI all unchanged)
- Shipped as `status: draft` — never fail-exits even when promoted to stable later (severity is warning)
- No new dependencies (`yaml` already a dep)

## Follow-ups

- M9a-2 (separate atom) — required-fields contract update for atom types that should always carry `valid_until` (e.g. roadmap concepts).
- M9a-3 — process atom describing how to triage stale atoms (review / supersede / update / re-up `valid_until`).
- Optional: weekly CI cron that runs `msp:validate --all` and posts the `info`/`warning` set to a `STALE_ATOMS` issue.

## Source

`[[CONCEPT--DECISION-ATROPHY-GUARDS]]` (Guard 1), `[[FEAT--PROTO-LOADER]]`, `[[FRAMEWORK--MSP-ARCHITECTURE-V2]]`.

## Connections
- [[BLUEPRINT--PROTO-LOADER]]

