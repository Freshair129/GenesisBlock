---
id: ADR--PATH-ENCODING
phase: 2
type: adr
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: Project path encoding — bare name (per script) over D-- prefix (per spec)
tags:
  - msp
  - path-encoding
  - convention
  - decision
crosslinks: {"references":["FRAME--MSP-ARCHITECTURE-V2"]}
created_at: 2026-05-03T15:08:44.405+07:00
---

# ADR — project path encoding

## Context

`msp_spec.md` §12 says project paths under `~/.brain/msp/projects/` use the convention `D--<name>` (e.g. `D--ProA`). But the actual `gks init` scaffolder and `scripts/migration/standardizer.mjs` (referenced in the spec) use a *bare* project name (e.g. `evaAI`). The spec and the tooling disagree.

We have to pick one and harmonise.

## Decision

**Use the bare name** (`.brain/msp/projects/evaAI/`), per the actual `@freshair129/gks` package behaviour. Update the spec on the next revision (M3 doc PR) so wording matches reality.

### Why bare name wins

1. **`gks init` is authoritative.** The published GKS package creates `.brain/msp/projects/evaAI/inbound/...` automatically. Forcing `D--evaAI` would mean either patching GKS upstream OR running a renaming step after every `gks init`. Both are friction.
2. **`D--` adds no information.** The path `~/.brain/msp/projects/<X>` already communicates "X is a project". A type prefix on filenames (atomic IDs) is useful because filenames live next to siblings of other types; project directories live alone in their parent.
3. **Migration cost is one direction.** Updating the spec is one paragraph. Migrating every project's `.brain/` tree is hundreds of files per project.

### What "harmonise" means here

- This ADR records the decision.
- The spec gets updated in a separate doc-PR (referencing this ADR).
- All MSP tooling (validator, future scripts) treats `.brain/msp/projects/<bare>/` as canonical.
- If a project has both `D--evaAI/` and `evaAI/` from a confused migration, the migration script picks `evaAI/` and archives the other.

## Consequences

**Positive**
- Tooling and spec align.
- Existing repos using `evaAI/` stay valid without renaming.
- The decision is recorded so future readers know it's deliberate, not an oversight.

**Negative**
- Anyone who already standardised on `D--<name>` per the spec must rename. Mitigated by the migration script.
- Drift between the *master* spec (`FRAMEWORK_MASTER_SPEC.md` §7) and our local `msp_spec.md` until upstream picks one. Tracked as an upstream coordination task.

## Alternatives considered

1. **Use `D--<name>` per the spec.** Rejected per cost analysis above.
2. **Support both at the validator boundary.** Considered. Adds complexity for no gain — pick one canonical form.
3. **Defer the decision.** Considered. The open issue has been open for months; deferring further means the ambiguity bleeds into more tooling.

## Source

`msp_spec.md` §12 (Project Path Encoding) — explicitly flagged as an open issue in §15.
