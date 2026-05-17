---
id: AUDIT--AUTHORITY-ENFORCEMENT-PROTO
phase: 6
type: audit
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: M8e — PROTO--AUTHORITY-ENFORCEMENT (sanity-check authority.yaml)
tags:
  - msp
  - proto
  - authority
  - audit
  - m8e
crosslinks: {"references":["PROTO--AUTHORITY-ENFORCEMENT","CONCEPT--PROTO-AUTHORITY-ENFORCEMENT","ADR--DELEGATION-POLICY","FRAMEWORK--AUTHORITY-MATRIX","FEAT--PROTO-LOADER","AUDIT--PROTO-LOADER"]}
linked_symbols:
  - {"file":"packages/msp/src/validator/proto/authority-enforcement.ts"}
  - {"file":"gks/proto/PROTO--AUTHORITY-ENFORCEMENT.md"}
  - {"file":"packages/msp/test/validator/proto/authority-enforcement.test.ts"}
phase_override:
  skip_blueprint: true
  reason: "Incremental PROTO rule plugged into the already-blueprinted loader scaffold (BLUEPRINT--PROTO-LOADER). Doc-to-code chain: CONCEPT--PROTO-AUTHORITY-ENFORCEMENT -> PROTO--AUTHORITY-ENFORCEMENT -> predicate -> this audit; per-rule predicates do not each warrant a separate phase-3 blueprint."
created_at: 2026-05-05T18:12:00.000+07:00
aliases:
  - AUDIT
  - implementation_flow
  - Test results / quality report
cluster: implementation_flow
role: "Test results / quality report"
---

# M8e — [[PROTO--AUTHORITY-ENFORCEMENT]]

## Scope

Adds a draft-status PROTO that sanity-checks `.brain/msp/authority.yaml`
against the shape declared in `[[CONCEPT--PROTO-AUTHORITY-ENFORCEMENT]]`.
Companion to `[[ADR--DELEGATION-POLICY]]` (M9b). Foundation for the future
PR-time CI workflow that will match git author tier to touched paths.

## What shipped

| File | Purpose |
|---|---|
| `src/validator/proto/authority-enforcement.ts` | Predicate: read `.brain/msp/authority.yaml`, parse YAML, validate shape, return violations |
| `gks/proto/[[PROTO--AUTHORITY-ENFORCEMENT]].md` | `status: draft`, `severity: error` PROTO atom; enforces `[[FRAMEWORK--AUTHORITY-MATRIX]]` |
| `test/validator/proto/authority-enforcement.test.ts` | 8 tests covering vacuous pass, valid file, invalid YAML, multi-tier user, empty paths, missing inbound, missing top-level keys, non-string entries |
| `gks/audit/[[AUDIT--AUTHORITY-ENFORCEMENT-PROTO]].md` | this file |

## What this PROTO checks (and doesn't)

| Check | Severity | Scope |
|---|---|---|
| File missing | n/a (pass) | Projects can opt in later |
| File present but bad YAML | error | Block CI |
| Top-level missing `tiers` or `allowed_paths` | error | Block CI |
| `tiers.T<n>` not array of strings | error | Block CI |
| Empty / non-string entry inside any tier list | error | Block CI |
| User appears in two tiers (sets not disjoint) | error | Block CI |
| `allowed_paths.T<n>` lacks an `inbound` substring entry | error | Block CI |
| `allowed_paths.T<n>` is empty array | warning | Surface, don't block |

What it does **not** do (out of scope for a PROTO predicate):

- Match `git log` author email/login to a tier
- Diff `base..head` and check each touched path against the author's tier
- Enforce the L2/L3 approval matrix from `[[ADR--DELEGATION-POLICY]]`
- Read `.brain/msp/authority.yaml` from outside the repo root

PROTO predicates have access only to `atomicIndex: AtomicIndexEntry[]` and
`repoRoot: string`. The PR-time pieces require git plumbing that lives in
a CI workflow, not the validator pipeline.

## Why draft, not stable

The shape check is real but partial — without the PR-time matcher, the
matrix is documented intent, not enforced policy. Promoting to `stable`
without the workflow would oversell what the PROTO catches. Once the
companion `.github/workflows/authority-check.yml` lands (M9b follow-up
per `[[ADR--DELEGATION-POLICY]]`), this PROTO can flip to stable.

## Decisions during impl

1. **Vacuous pass when file is missing** — projects opt in by writing
   `authority.yaml`. Single-developer slices and bootstrap repos shouldn't
   be CI-blocked just because they haven't populated the tier map.

2. **`severity: error` despite `status: draft`** — under the loader's
   `shouldFailExit` rule, draft PROTOs never fail-exit. So the declared
   severity describes intent for when this promotes to stable; it doesn't
   gate CI today. Atoms ship with the right severity from day one.

3. **Inbound check is substring-based, not glob-based** — looking for
   `inbound` substring in any entry of `allowed_paths.T<n>` is loose enough
   to accept the variants in the wild (`.brain/msp/projects/*/inbound/**`,
   `inbound/`, `gks/inbound/**`) without depending on a glob library.

4. **Empty `allowed_paths` is a warning, not an error** — a tier with no
   writable paths is a config smell (the tier exists but can't do
   anything), but might be intentional during onboarding (T2 not staffed
   yet). Warning surfaces it without blocking.

5. **No `linked_symbols` enforcement on the predicate's exports** — the
   PROTO loader already verifies the impl file exists; the predicate
   itself uses `default export`. Same pattern as `sample.ts`.

## Verification

- `npm run typecheck` → clean (worktree has node_modules symlinked from
  parent per CLAUDE.md worktree caveat)
- `npm test -- test/validator/proto/authority-enforcement.test.ts` → 8/8
  pass
- `npm test` → full suite green
- `npx tsx src/validator/cli.ts --all` → atoms validate; new PROTO
  discovered + run; predicate passes (no `.brain/msp/authority.yaml` in
  this repo so vacuous pass)

## Counts

- Atoms +2 (`[[PROTO--AUTHORITY-ENFORCEMENT]]`, this AUDIT)
- Tests +8 (authority-enforcement.test.ts)
- New impl: 1 file (`src/validator/proto/authority-enforcement.ts`)

## Future work

Tracked here so the next milestone slice doesn't lose context:

1. **CI workflow for PR-time enforcement** —
   `.github/workflows/authority-check.yml` reads PR author + diff and
   applies the matrix per `[[ADR--DELEGATION-POLICY]]`. Predicate stays in
   the validator; workflow handles git plumbing.

2. **Promote PROTO to `stable`** once (1) is shipped and observed for at
   least one milestone.

3. **Multi-author handling** — current shape model treats one user → one
   tier. PRs with multiple committers need a rule (highest-tier wins?
   lowest-tier wins? Boss arbitrates?). Out of scope for M8e per the
   CONCEPT atom.

4. **Default-fallback tier** — `[[CONCEPT--PROTO-AUTHORITY-ENFORCEMENT]]`
   says T1 is the fallback for unknown authors. The shape check doesn't
   verify or enforce this; a future iteration could check that the union
   of all tier members is non-empty (i.e. someone is configured).

## Source

`[[CONCEPT--PROTO-AUTHORITY-ENFORCEMENT]]`, `[[ADR--DELEGATION-POLICY]]`,
`[[FRAMEWORK--AUTHORITY-MATRIX]]`, `[[FEAT--PROTO-LOADER]]`, `[[AUDIT--PROTO-LOADER]]`.

## Connections
- [[BLUEPRINT--PROTO-LOADER]]

