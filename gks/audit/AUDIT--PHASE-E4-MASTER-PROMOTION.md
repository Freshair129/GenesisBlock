---
id: AUDIT--PHASE-E4-MASTER-PROMOTION
phase: 6
type: audit
status: draft
tier: process
source_type: axiomatic
vault_id: default
title: AUDIT — Phase E4 Master Block promotion pipeline (4-of-5 analyzer + proposal generator)
tags:
  - msp
  - master
  - promotion
  - phase-e4
  - audit
crosslinks: {"references":["CONCEPT--MASTER-PROMOTION","BLUEPRINT--MASTER-PROMOTION-PIPELINE","SPEC--GENESIS-BLOCK-MANIFEST","ADR--MASTER-PROMOTION-DOC-TO-CODE","ADR--HUMAN-REVIEW-GATES"]}
linked_symbols:
  - {"file":"packages/msp/src/master/dimensions.ts"}
  - {"file":"packages/msp/src/master/scanner.ts"}
  - {"file":"packages/msp/src/master/promote.ts"}
  - {"file":"packages/msp/src/master/cli.ts"}
  - {"file":"packages/msp/test/master/dimensions.test.ts"}
  - {"file":"packages/msp/test/master/scanner.test.ts"}
  - {"file":"packages/msp/test/master/promote.test.ts"}
  - {"file":"packages/msp/test/master/cli.test.ts"}
  - {"file":"packages/msp/package.json"}
created_at: 2026-05-13T11:00:00.000+07:00
aliases:
  - AUDIT
  - implementation_flow
  - Test results / quality report
cluster: implementation_flow
role: "Test results / quality report"
---

# AUDIT — Phase E4 Master Block promotion pipeline

## Scope

Phase E4 of the agentic monorepo roadmap. Shipped the deterministic
analysis + proposal half of the Master Block promotion workflow:
scan the vault for `GENESIS--*` manifests, count dimension coverage
against the 4-of-5 criterion (per `[[SPEC--GENESIS-BLOCK-MANIFEST]]` § 5),
emit per-block tables, and — under `--write` — drop scaffolded
`MASTER--<id>.proposal.md` files to `gks/inbound/` for human review.

Out of scope (deferred):
- `gks/master/` writes — explicit human gate per `[[ADR--MASTER-PROMOTION-DOC-TO-CODE]]`
  and `[[ADR--HUMAN-REVIEW-GATES]]`.
- `[[PROTO--GENESIS-BLOCK-MEMBERSHIP]]` — the validator rule that enforces the
  `members.core` shape on `type: genesis` atoms.
- Token-cap enforcement on proposals (the existing `[[PROTO--MASTER-TOKEN-CAP]]`
  runs against `gks/master/` only; proposals bypass by design).
- Mechanical cross-Master contradiction detection (Layer 1+ of
  `[[BLUEPRINT--CONTRADICTION-DETECTION-IMPL]]`).

## What shipped vs the BLUEPRINT

`[[BLUEPRINT--MASTER-PROMOTION-PIPELINE]]` listed 4 deliverables + 1 test
suite; all 5 are present.

| Deliverable | File | Notes |
|---|---|---|
| 1. `analyzeDimensions` (pure) | `packages/msp/src/master/dimensions.ts` | Adds `unresolved`, `not_stable`, `unknown_prefix` diagnostic lists beyond the BLUEPRINT shape so the CLI table can render `yes/no/stub` per cell. |
| 2. `findGenesisBlocks` | `packages/msp/src/master/scanner.ts` | Searches `gks/genesis/`, `gks/`, and `packages/*/gks/genesis/` + `packages/*/gks/`. De-dupes by canonical path. Sorts blocks by id for deterministic output. |
| 3. `proposePromotion` | `packages/msp/src/master/promote.ts` | Read-only filesystem I/O for atom lookup; tests inject `lookupOverride` to stay pure. Exports `renderProposalDocument` for the CLI. |
| 4. `msp-master-propose` CLI | `packages/msp/src/master/cli.ts` | Bin entry added to `packages/msp/package.json`. (E5 adds `msp-genesis-exec` on a parallel branch — both bin entries can coexist.) |
| 5. Tests | `packages/msp/test/master/{dimensions,scanner,promote,cli}.test.ts` | 30+ test cases total — well above the BLUEPRINT's "≥18" target. |

### Role-to-prefix mapping (per `[[SPEC--GENESIS-BLOCK-MANIFEST]]` § 3.1)

The implementation classifies member atoms by id prefix, not by parsed
`type:` field. The mapping is closed to the five core prefixes:

- `COGNITIVE--`  → cognitive
- `ALGO--`       → algo
- `RUNBOOK--`    → runbook
- `CONCEPT--`    → concept
- `PARAMS--`     → params

Any other prefix (e.g. `FRAMEWORK--`, `GENESIS--`, `GUARD--`, `STACK--`,
`SAFETY--`, `MOD--`, `SPEC--`, `PROTOCOL--`) is filtered into the
`unknown_prefix` diagnostic list and excluded from the dimension counts.

## Live run

`msp-master-propose --root=. --write` against the live repo produces
zero proposals — no Genesis Block currently has 4/5 stable members
(only `[[SPEC--GENESIS-BLOCK-MANIFEST]]` exists; no `GENESIS--*` manifests
have been authored yet). This is expected per the BLUEPRINT — Phase E4
ships the machinery, not the first promotion. The pipeline becomes
load-bearing once the first `GENESIS--<NAME>.md` lands with a populated
`members.core` block.

## Test counts

- `dimensions.test.ts` — 14 cases (classifier + analyzer)
- `scanner.test.ts` — 8 cases (tmpdir fixtures)
- `promote.test.ts` — 7 cases (with `lookupOverride`)
- `cli.test.ts` — 5 cases (spawned via tsx)

All green under `npm test --workspace=packages/msp -- test/master/`.

## Boundaries respected

- Stayed within `packages/msp/src/master/` + `packages/msp/test/master/` +
  the 3 new atoms — no cross-cutting touches to E1 (`test/agents/integration/`)
  or E5 (`src/genesis/`).
- Added one new bin entry (`msp-master-propose`); does not collide with
  the parallel `msp-genesis-exec` entry being added by Phase E5.

## Follow-ups (not in this PR)

1. Author the first real `GENESIS--<NAME>.md` Block Manifest under
   `gks/genesis/` (likely `[[GENESIS--IDENTITY-ENGINE]]` per the SPEC's
   worked example) so the pipeline produces a real proposal.
2. `[[PROTO--GENESIS-BLOCK-MEMBERSHIP]]` — the validator rule that enforces
   `members.core` shape + the 4-of-5 status cascade at validation time.
3. The human-review tooling: a `msp-master-review` command that reads
   `gks/inbound/MASTER--*.proposal.md`, opens it for editing, and on
   confirmation moves it to `gks/master/` after the human writes the
   evidence ADR.
4. CI: a scheduled GitHub Action that runs `msp-master-propose` on `main`
   weekly and opens a draft PR with any new proposals.

## Connections
- [[CONCEPT--MASTER-PROMOTION]]

