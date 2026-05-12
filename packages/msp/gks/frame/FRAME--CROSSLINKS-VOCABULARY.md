---
id: FRAME--CROSSLINKS-VOCABULARY
phase: 0
type: frame
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: Crosslinks vocabulary — predicates between atoms
tags:
  - msp
  - crosslinks
  - vocabulary
  - foundation
crosslinks: {"references":["CONCEPT--ATOMIC-WRITE-CONTRACT"]}
created_at: 2026-05-03T15:01:49.773+07:00
---

# FRAME — crosslinks vocabulary

Atoms relate through a small fixed vocabulary of predicates under `crosslinks.*`. The validator checks every value resolves to an existing atom (`dangling-wikilink` rule) and `gks verify-flow` walks them to compute chain integrity.

## Predicates

| Key | Direction | Meaning |
|---|---|---|
| `implements` | this → upstream | "this atom is the realisation of upstream X" (FEAT→ADR, BLUEPRINT→FEAT) |
| `references` | this → context | "this atom depends on context from X" (weaker than `implements`) |
| `used_by` | this → downstream | "X depends on this atom" (rare; prefer letting downstream declare via `references`) |
| `contradicts` | this ↔ peer | "X says the opposite — explicit conflict for human resolution" |
| `supersedes` | this → predecessor | "this atom replaces X entirely; X is now `superseded`" |
| `partially_supersedes` | this → predecessor | "this atom replaces parts of X; X stays `stable` with edits" |
| `superseded_by` | this → successor | derived; set on the *old* atom when `supersedes` is set on the new |
| `partially_superseded_by` | this → successor | derived; same |
| `resolves` | this → debt | "this atom closes the open debt X" (typically `HOTFIX--`) |

## Usage rules

1. **Upstream-first.** Prefer declaring `implements` / `references` on the atom that needs the link, not the upstream atom. Backlinks are derived by the indexer.
2. **No self-link in `implements`.** An atom cannot implement itself; the validator may not catch this in M2 — flagged as M3 work.
3. **`contradicts` requires Boss approval.** Filing a contradiction is a governance event, not a frontmatter edit. The reviewer must reconcile the two atoms.
4. **`supersedes` triggers a status change.** The promote workflow flips the predecessor to `superseded` and adds `superseded_by` automatically.

## Validator behaviour

`dangling-wikilinks` rule walks every key in the table above plus body `[[X]]` references. Inline-code spans (`` `[[X]]` ``) and fenced code blocks are ignored.

## Source

`msp_spec.md` §4.6 (Epistemic & Crosslinks Block).
