---
id: CONCEPT--MONOREPO-MIGRATION
phase: 1
type: concept
status: draft
vault_id: default
tier: genesis
source_type: axiomatic
title: Monorepo migration — collapse `Freshair129/msp` + `Freshair129/GksV3` into one workspace while still publishing `@freshair129/gks` separately
tags:
  - monorepo
  - workspace
  - migration
  - boundary
  - governance
  - msp
  - gks
crosslinks: {"references":["FRAMEWORK--MSP-ARCHITECTURE-V2","ADR--GRAPH-IS-GKS-DOMAIN","AUDIT--TWO-REPO-VALIDATION","AUDIT--GKS-UPSTREAM-PROPOSALS-FILED","AUDIT--GKS-3-6-0-PUBLISHED"]}
created_at: 2026-05-08T13:20:00.000+07:00
aliases:
  - CONCEPT
  - implementation_flow
  - Strategic intent / PRD
cluster: implementation_flow
role: "Strategic intent / PRD"
---

# CONCEPT — monorepo migration

## Problem

MSP and GKS live in two GitHub repos (`Freshair129/msp`, `Freshair129/GksV3`) with GKS published to npm as `@freshair129/gks`. The split was justified by reusability — GKS is a generic atomic-markdown + vector + graph store that anyone could plug into their own memory system. But:

- **No external consumer exists today.** GKS has zero downstream users besides MSP.
- **The split costs are concrete and ongoing.** Five outstanding upstream proposals (`upstream/gks-proposals/01..04`) — each a workaround MSP carries while waiting for GKS to land an API. `upstream/gks-proposals/05` was a meta-proposal: "publish 3.6.0 so MSP can use the features the CHANGELOG already documents."
- **Lag is the dominant tax.** Fix flow: open issue on `GksV3` → wait for upstream merge → wait for npm publish → bump `@freshair129/gks` in `msp/package.json` → ship the MSP-side change. Days-to-weeks per cross-cutting change vs. a single PR in a monorepo.

The repo already documents the friction:

- `CLAUDE.md` "Two-repo sync rule": "Before claiming a GKS API exists: check `node_modules/@freshair129/gks/dist/...`" — a manual gate that exists because the boundary is *physical* (two npm-disjoint package versions), not just logical.
- `[[AUDIT--TWO-REPO-VALIDATION]]` (2026-05-04): an entire audit dedicated to drift detection between MSP atomic claims and what GKS actually shipped.
- `scripts/msp/propose.mjs`: phase-6 patching hack tracked at `upstream/gks-proposals/01` — exists *only* because GKS 3.5.6's CLI capped phase at 5.
- `src/memory/backlinks/`: ~200 LoC duplicating GKS scope per `[[ADR--GRAPH-IS-GKS-DOMAIN]]`, kept until upstream proposal #03 lands.

## Intent

Move both packages into a single npm/pnpm workspace monorepo while **preserving GKS as a separately-publishable npm package**. External consumers continue installing `@freshair129/gks` exactly as today; nothing about the public package surface changes. What changes:

| Today | After migration |
|---|---|
| Cross-cutting change = PR on `GksV3` + wait + publish + bump + PR on `msp` | One PR touches both `packages/gks/` and `packages/msp/` |
| MSP-side workarounds (`scripts/msp/propose.mjs` phase-6, `src/memory/backlinks/`) | Land features directly in GKS in same PR |
| `upstream/gks-proposals/` queue | Gone — handled inline |
| "aspirational pending GKS X.Y" markers in atoms | Gone — atomic landings |
| `[[AUDIT--TWO-REPO-VALIDATION]]` two-repo drift checks | Replaced by single CI run validating both packages |

## Why this preserves reusability

The thing that makes GKS reusable is the *published npm package shape*, not the *git layout*. Vite + Vitest, tRPC, Turborepo, Next.js + SWC, and dozens of other projects ship reusable libraries from monorepos. External consumers see only `@freshair129/gks` on npm — they don't know or care whether it sits next to MSP in git.

What enforces "GKS is standalone" is not the repo split; it's:

1. **Import direction**: GKS package must not import from MSP package (enforceable via ESLint boundary rule + CI guard)
2. **Independent test surface**: GKS's tests must pass without MSP fixtures or runtime
3. **Independent README + examples**: GKS's `packages/gks/README.md` must work as a standalone library doc
4. **Independent publish pipeline**: `npm publish` on the GKS package without touching MSP

These constraints are stronger inside a monorepo because they're machine-checked on every PR — vs today's two-repo setup where boundary violations are caught only at version-bump time.

## Scope

**In scope:**

- Repo restructure into `packages/gks/` + `packages/msp/` under a workspace-aware root (npm or pnpm workspaces)
- Migration of GKS source from `Freshair129/GksV3` → `packages/gks/` in the unified repo
- Atomic landing of the 5 upstream proposals (#01–#04 still open, #05 already landed) directly into the unified repo
- Removal of the MSP-side workarounds those proposals were tracking
- ESLint / CI rule enforcing GKS → MSP import direction
- A separate publish workflow for `@freshair129/gks` from the new repo
- One AUDIT atom recording the migration outcome

**Out of scope (deferred):**

- Renaming the unified repo (`msp` vs `gks-stack` vs new name) — bikeshed for later
- Migrating `Freshair129/GksV3`'s git history (acceptable to start fresh; tag the old repo as archived)
- Splitting MSP's `web/` into a separate package (current monolithic layout fine for now)
- Multi-org publish (still publishing under `@freshair129/`)
- Renaming MSP's npm package name (stays `msp` for now)

## Source

- `CLAUDE.md` "Two-repo sync rule"
- `[[FRAMEWORK--MSP-ARCHITECTURE-V2]]` — MSP↔GKS boundary table
- `[[ADR--GRAPH-IS-GKS-DOMAIN]]` — example of MSP carrying GKS-domain code as workaround
- `[[AUDIT--TWO-REPO-VALIDATION]]` — drift audit motivated by the split
- `[[AUDIT--GKS-UPSTREAM-PROPOSALS-FILED]]` — the proposal queue
- `[[AUDIT--GKS-3-6-0-PUBLISHED]]` — the most recent publish-lag pain point (~6 days from proposal to MSP unblock)
- `upstream/gks-proposals/README.md` — workflow that this migration would obsolete
