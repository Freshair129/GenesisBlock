---
id: ADR--TAXONOMY-V2-3-MIGRATION
phase: 2
type: adr
status: stable
tier: genesis
source_type: axiomatic
vault_id: default
title: Taxonomy v2.3 migration — FRAME redefined, FRAMEWORK added, GUARDRAIL renamed
tags:
  - msp
  - taxonomy
  - migration
  - genesis-block
  - decision
crosslinks: {"references":["CONCEPT--TAXONOMY-V2-3","FRAMEWORK--CROSSLINKS-VOCABULARY","CONCEPT--KNOWLEDGE-LAYERS-V2"]}
created_at: 2026-05-13T12:21:49+07:00
---

# ADR — Taxonomy v2.3 migration

## Context

`[[CONCEPT--TAXONOMY-V2-3]]` introduces the new prefix layer for the Genesis Block era. This ADR records the **migration decisions** — what gets renamed, what gets added, what stays, and the order of operations — so the change is reviewable, reversible, and validated by tooling.

Current state (counted on `main` at session start):

- **9 `FRAME--*` atom files** under `packages/{gks,msp}/gks/frame/` — all describe architecture or governance, none describe an executable engine manifest
- **270 references to `FRAME--`** across 133 markdown files (crosslinks, body prose, wikilinks)
- **0 `GUARDRAIL--*` atom files** in the repo (the prefix exists only in taxonomy tables and template files under `packages/gks/examples/atom-templates/`)
- **5 source files** hardcode `'frame'` as an atom-type literal: `packages/gks/src/memory/types.ts`, `packages/msp/src/{obsidian/filesystem.ts, cognitive/ssot.ts, cognitive/scale-gate.ts, mcp/tools/candidate.ts}`, `packages/msp/src/validator/proto/sample.ts`

## Decision

### 1. Redefine `FRAME--` as Block Manifest

`FRAME--<NAME>.md` is reserved for the runtime entry-point of a Genesis Block — a file that aggregates other atoms into an executable knowledge engine. No existing atom matches this definition.

### 2. Introduce `FRAMEWORK--` for the old `FRAME--` meaning

`FRAMEWORK--<NAME>.md` carries the meaning the repo currently uses `FRAME--` for: architectural blueprints, governance frameworks, methodology, code standards.

### 3. Rename all 9 existing `FRAME--*` atoms to `FRAMEWORK--*`

| Old id (pre-migration) | New id (post-migration) | Reason |
|---|---|---|
| `FRAME-`​`-FOUR-LAYERS` (gks) | `[[FRAMEWORK--FOUR-LAYERS]]` | four-layer storage model — architecture |
| `FRAME-`​`-AUTHORITY-MATRIX` | `[[FRAMEWORK--AUTHORITY-MATRIX]]` | governance — write authority per path |
| `FRAME-`​`-CROSSLINKS-VOCABULARY` | `[[FRAMEWORK--CROSSLINKS-VOCABULARY]]` | vocabulary spec |
| `FRAME-`​`-KNOWLEDGE-3-TIER` | `[[FRAMEWORK--KNOWLEDGE-3-TIER]]` | tier architecture |
| `FRAME-`​`-MSP-ARCHITECTURE` | `[[FRAMEWORK--MSP-ARCHITECTURE]]` | architecture (superseded; rename for chain integrity) |
| `FRAME-`​`-MSP-ARCHITECTURE-V2` | `[[FRAMEWORK--MSP-ARCHITECTURE-V2]]` | architecture |
| `FRAME-`​`-PHASE-GOVERNANCE` | `[[FRAMEWORK--PHASE-GOVERNANCE]]` | P0..P6 governance |
| `FRAME-`​`-SCALING-LEVELS` | `[[FRAMEWORK--SCALING-LEVELS]]` | L1/L2/L3 governance |
| `FRAME-`​`-SYMBOL-GRAPH` | `[[FRAMEWORK--SYMBOL-GRAPH]]` | structural-knowledge axis |

> Note on rendering: the `FRAME-`​`-X` notation uses a zero-width-space between the two hyphens so future runs of `migrate-frame-to-framework.mjs` don't re-rewrite this historical reference table. Readers see plain `[[FRAME--X]]`.

Directory move: `gks/frame/` → `gks/framework/` in both `packages/gks` and `packages/msp`.

Frontmatter update inside each renamed file:
- `id:` field — old `FRAME-`​`-X` prefix becomes `[[FRAMEWORK--X]]`
- `type: frame` → `type: framework`

### 4. Rename `GUARDRAIL--` to `GUARD--` (doc-only migration)

Since zero atoms use the `GUARDRAIL--` prefix, this is a documentation-only change:
- Taxonomy tables in `CLAUDE.md` (×3) and `docs/gks/KNOWLEDGE-TYPES.md`
- Template file `packages/gks/examples/atom-templates/GUARDRAIL.md` → `GUARD.md` (with content update)
- Two ADR references (`docs/adr/012-extended-taxonomy.md`, `docs/adr/013-flat-atom-layout.md`) noting the supersession

### 5. Add `framework` as a recognised atom type in source

Five source files enumerate atom-type literals. Each must add `'framework'` alongside `'frame'`:

| File | Change |
|---|---|
| `packages/gks/src/memory/types.ts` | Add `'framework'` to `AtomicType` union (line 38–50) |
| `packages/msp/src/obsidian/filesystem.ts` | Add `'framework'` to `SEARCH_DIRS` (line 6) |
| `packages/msp/src/cognitive/ssot.ts` | Add `'framework'` to the priority list (line 19) |
| `packages/msp/src/cognitive/scale-gate.ts` | Add `'framework'` to the L3 required-types array (line 40) |
| `packages/msp/src/mcp/tools/candidate.ts` | Update the zod description to include `framework` (line 17) |
| `packages/msp/src/validator/proto/sample.ts` | Broaden `type === 'frame'` check to also accept `'framework'` (line 17) |

Note: `'frame'` is **kept** in all five lists, not replaced. Both prefixes are valid post-migration — `'frame'` for Block Manifests, `'framework'` for governance/architecture.

### 6. Preserve legacy prefixes

These existing GKS prefixes are not part of v2.3's primary tables but remain valid: `IDEA--`, `MASTER--`, `POLICY--`, `PERSONA--`, `REQ--`, `CONSTRAINT--`, `API--`, `ENDPOINT--`, `ENTRYPOINT--`, `PARAMS--`, `INSIGHT--`, `FACT--`, `RULE--`.

No deprecation is implied. A future ADR may revisit them individually.

## Migration sequence

The migration is split into discrete commits on this branch (`claude/msp-taxonomy-v2.3-migration`) for reviewability:

1. **Source-code prep** — add `'framework'` to the five enumerations (additive, non-breaking; can be reviewed in isolation)
2. **New atoms** — write `[[CONCEPT--TAXONOMY-V2-3]]` and this ADR (no breaking change yet; the new taxonomy is documented but not enforced)
3. **Migration script** — Node.js script under `scripts/msp/migrate-frame-to-framework.mjs` that performs renames + ref updates with a `--dry-run` flag
4. **Apply migration** — run the script; 9 file renames + 270 ref updates + frontmatter rewrites
5. **Regen index** — `npm run msp:index` to rebuild `atomic_index.jsonl`
6. **Docs sync** — update `CLAUDE.md` (root + msp + gks) and `KNOWLEDGE-TYPES.md` taxonomy tables
7. **Verify** — `npm run typecheck`, `npm test`, `npx tsx packages/msp/src/validator/cli.ts --all` all green before opening the PR

## Consequences

- **Wikilinks that referenced old `[[FRAME--X]]` ids will need a one-time update** wherever they appear in *non-tracked* documents (issue templates, external Obsidian vaults, …). The validator catches dangling wikilinks inside the repo, but external consumers must be informed via the PR description.
- **`[[PROTO--PHASE-GATES]]`** does not currently special-case the `frame` type, so no validator change is required for phase logic. Block Manifest atoms (the new `FRAME--` meaning) inherit the existing P0 phase semantics.
- **`atomic_index.jsonl` regen is mandatory** — stale ids in the index would cause validator false-positives. The Phase-7 verify step covers this.
- **The repo's own atom tree under `gks/` does not currently contain a `frame/` subdirectory** beyond `[[FRAMEWORK--FOUR-LAYERS]].md`. The directory move there is a single file; in `gks/` it covers 8 files. Empty `frame/` directories are removed by the script.

## Rollback

The migration is mechanical and reversible: the script accepts an `--inverse` flag that reverses the renames + ref updates. If a downstream issue is found post-merge, rollback is a single command + `npm run msp:index`.

## Open questions (deferred)

- Authoring guide for *new* `FRAME--` (Block Manifest) atoms — what frontmatter fields, what aggregation grammar — belongs in a follow-up `[[SPEC--BLOCK-MANIFEST]]` atom
- Whether `MASTER--`, `POLICY--`, `PERSONA--` should be folded into the v2.3 layers in a future revision; left explicit-preserve for now

## Connections
- [[CONCEPT--KNOWLEDGE-LAYERS-V2]]

