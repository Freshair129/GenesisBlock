---
id: BLUEPRINT--MASTER-PROMOTION-PIPELINE
phase: 3
type: blueprint
status: draft
tier: process
source_type: axiomatic
vault_id: default
title: BLUEPRINT — Master Block promotion pipeline (scan → analyze → propose,
  never auto-write)
tags:
  - msp
  - master
  - promotion
  - genesis-block
  - blueprint
  - phase-e4
crosslinks:
  references:
    - CONCEPT--MASTER-PROMOTION
    - SPEC--GENESIS-BLOCK-MANIFEST
    - ADR--MASTER-PROMOTION-DOC-TO-CODE
    - ADR--HUMAN-REVIEW-GATES
linked_symbols:
  - file: packages/msp/src/master/dimensions.ts
  - file: packages/msp/src/master/scanner.ts
  - file: packages/msp/src/master/promote.ts
  - file: packages/msp/src/master/cli.ts
  - file: packages/msp/test/master/dimensions.test.ts
  - file: packages/msp/test/master/scanner.test.ts
  - file: packages/msp/test/master/promote.test.ts
  - file: packages/msp/test/master/cli.test.ts
  - file: packages/msp/package.json
  - file: gks/inbound/
created_at: 2026-05-13T10:05:00.000+07:00
aliases:
  - BLUEPRINT
  - implementation_flow
  - Implementation plan
cluster: implementation_flow
role: Implementation plan
attributes:
  domain: blueprint
---

# BLUEPRINT — Master Block promotion pipeline

## Goal

Implement the analysis + proposal half of the Master Block promotion workflow described by `[[CONCEPT--MASTER-PROMOTION]]`. Phase E4 ships the pipeline that **scans** the vault for Genesis Block manifests, **analyzes** dimension coverage per the 4-of-5 rule, and **proposes** Master atoms by writing scaffolded `.proposal.md` files to `gks/inbound/`. Phase E4 does NOT ship the human-review tooling that promotes a proposal to `gks/master/` — that remains a manual editorial step per `[[ADR--MASTER-PROMOTION-DOC-TO-CODE]]`.

## Non-goals

- Writing to `gks/master/` programmatically (explicit ADR rejection — `[[ADR--HUMAN-REVIEW-GATES]]`).
- Authoring the evidence ADR for each proposed Master (each promotion still needs a human-written `[[ADR--MASTER-PROMOTION]]-<topic>`).
- Validating that proposed Master bodies fit the 400-token cap (the `[[PROTO--MASTER-TOKEN-CAP]]` rule already runs against `gks/master/`; proposals live in `inbound/` and bypass that rule by design).
- Mechanical contradiction detection across Master atoms (deferred to `[[BLUEPRINT--CONTRADICTION-DETECTION-IMPL]]` Layer 1+).
- Updating the validator's per-type required-fields contract for `type: genesis` — deferred to `[[PROTO--GENESIS-BLOCK-MEMBERSHIP]]`.

## Deliverables

### 1. `packages/msp/src/master/dimensions.ts`

Pure function `analyzeDimensions(memberIds, lookup) → DimensionCoverage`. Given a flat list of member-atom ids (the union of `members.core.*` from a manifest) and a `lookup: (id) => AtomRecord | null` callback that returns parsed atom records, classify each id into one of the 5 roles by prefix:

- `COGNITIVE--*` → `cognitive`
- `ALGO--*`      → `algo`
- `RUNBOOK--*`   → `runbook`
- `CONCEPT--*`   → `concept`
- `PARAMS--*`    → `params`

Ids whose prefix matches no role are filtered out (the SPEC §3.1 closes the core to these five). Ids whose lookup returns `null` are still classified by prefix but flagged as unresolved. An id only contributes to `filled_count` when (a) the prefix matches a role, (b) lookup resolves, and (c) `status === 'stable'`. `promotable = filled_count >= 4`.

The function is **pure** — no I/O. Tests mock `lookup` to construct adversarial cases.

### 2. `packages/msp/src/master/scanner.ts`

`findGenesisBlocks(root) → Promise<GenesisBlock[]>`. Walks the atom vault for `*.md` files whose frontmatter `type === 'genesis'`. For each match:

- Parses YAML frontmatter (via the `yaml` dep, already vendored).
- Reads `members.core.{cognitive,algo,runbook,concept,params}` and `members.optional.*` if present.
- Returns `{ genesisId, manifestPath, members: string[] }` where `members` is the union of all listed ids across `members.core.*` (flattened).

Search roots (in priority order, deduplicated by canonical path):
1. `<root>/gks/genesis/`
2. `<root>/gks/` (recursive, to catch any GENESIS atom mis-filed during the FRAME→GENESIS rename — see `[[ADR--TAXONOMY-V2-3-MIGRATION]]`)
3. `<root>/packages/*/gks/genesis/` (post-migration per-package vaults)

Files whose frontmatter is unparseable or whose `type` is not `genesis` are silently skipped.

### 3. `packages/msp/src/master/promote.ts`

`proposePromotion(block, root) → Promise<Proposal>`. Composes the dimensions analyzer + a frontmatter scaffolder. Steps:

1. Build a `lookup` callback that finds atom files under `<root>/gks/**/*.md` (and any `packages/*/gks/**/*.md`), parses frontmatter, returns `{ id, type, status, tags }`. Lookups are cached per call.
2. Run `analyzeDimensions(block.members, lookup)`.
3. If `!coverage.promotable`, return `{ promotable: false, reason: '<missing dimensions>' }`.
4. If `coverage.promotable`, build a `proposed_master_id = MASTER--<deslugged block id>` (e.g. `[[GENESIS--IDENTITY-ENGINE]]` → `[[MASTER--IDENTITY-ENGINE]]`).
5. Build proposed frontmatter:
   ```yaml
   id: MASTER--<NAME>
   phase: 0
   type: master
   status: draft
   tier: master
   source_type: axiomatic
   promoted_from: GENESIS--<NAME>
   promoted_at: <ISO UTC>
   promotion_adr: ADR--MASTER-PROMOTION-<NAME>   # not yet authored — proposal flags
   vault_id: default
   title: <derived from manifest title or block id>
   tags: [msp, master, promotion, <block-specific tags from manifest>]
   crosslinks: {"references":["FRAMEWORK--KNOWLEDGE-3-TIER","SPEC--GENESIS-BLOCK-MANIFEST","GENESIS--<NAME>"]}
   created_at: <ISO ICT>
   ```
6. Build proposed body in the canonical 5-section schema with placeholder content for `Intent / Why / Directives / Apply when / Conflicts with`. Each section is a heading + a TODO line; the human reviewer fills the content.
7. Return `{ promotable: true, proposed_master_id, proposed_frontmatter, proposed_body, coverage }`.

The function does NOT write to disk. Disk writes are the CLI's job.

### 4. `packages/msp/src/master/cli.ts`

CLI `msp-master-propose` with subcommands implicit (single command). Flags:

- `--root=<dir>`  default `process.cwd()`
- `--write`       when present, drop promotable proposals to `<root>/gks/inbound/MASTER--<id>.proposal.md`. Without `--write`, the CLI prints a table to stdout listing each block's filled / missing dimensions and the `promotable` verdict.
- `--help`

Output without `--write`: a table with columns `block | cognitive | algo | runbook | concept | params | filled | promotable`. The 5 dimension columns show "yes/no" (or "stub" for unresolved). A summary line lists how many blocks were scanned and how many were promotable.

Exit codes:
- `0` success (zero or more proposals written)
- `1` no GENESIS atoms found (likely wrong `--root`)
- `2` internal error (bad args, IO failure)

Bin registration in `packages/msp/package.json`:
```json
"msp-master-propose": "./dist/master/cli.js"
```

### 5. Tests — `packages/msp/test/master/`

- `dimensions.test.ts` — 10+ cases covering: each dimension in isolation, mixed cases, ids with unknown prefix (filtered), unresolved lookup, draft-not-stable members not counted, exactly-4 promotable, exactly-3 not promotable, all-5 promotable, empty input, duplicate ids.
- `scanner.test.ts` — tmpdir fixture with one `[[GENESIS--FOO]].md` (5-dim manifest), one unrelated CONCEPT, one malformed file, one `type: framework` file with `FRAMEWORK--` prefix (must NOT be picked up). Verify members parsed in order and across `core.*` keys.
- `promote.test.ts` — promotable block returns full proposal; under-promotable block returns `{ promotable: false, reason }`. Verify proposed frontmatter contains the canonical keys and the body has 5 section headings.
- `cli.test.ts` — spawn the CLI via `tsx src/master/cli.ts --root=<tmpdir>` against a tmpdir; verify the table is printed to stdout. Spawn again with `--write`; verify a `gks/inbound/MASTER--<id>.proposal.md` file appears.

## Acceptance

- `npm test --workspace=packages/msp -- test/master/` passes (all 4 files, ≥18 test cases total).
- `npm run typecheck` passes (strict, no `any`).
- `msp-master-propose --root=. --write` against the live repo writes 0 proposals (no Genesis Block currently has 4/5 stable dimensions — Phase E4 ships the machinery, not the first promotion).
- `npm run msp:validate` passes after the 3 atoms (this BLUEPRINT, the CONCEPT, the AUDIT) are added.

## Out of scope

Per CONCEPT §"Out of scope":
- `gks/master/` writes (human-only)
- Token-cap enforcement on proposals
- Cross-Master contradiction detection
- `[[PROTO--GENESIS-BLOCK-MEMBERSHIP]]` (the genesis frontmatter validator rule)

## Reference

- `[[CONCEPT--MASTER-PROMOTION]]` § "The 4-of-5 rule"
- `[[SPEC--GENESIS-BLOCK-MANIFEST]]` § 3.1 + § 5
- `[[ADR--MASTER-PROMOTION-DOC-TO-CODE]]` § Decision (human review)
- `[[ADR--HUMAN-REVIEW-GATES]]`
