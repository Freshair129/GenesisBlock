---
id: ADR--MONOREPO-STRUCTURE
phase: 2
type: adr
status: draft
vault_id: default
tier: genesis
source_type: axiomatic
title: Monorepo structure — npm workspaces with `packages/gks/` +
  `packages/msp/`, GKS still publishes to npm
tags: &a1
  - monorepo
  - workspace
  - npm
  - boundary
  - decision
  - msp
  - gks
crosslinks: &a2
  references:
    - CONCEPT--MONOREPO-MIGRATION
    - FRAMEWORK--MSP-ARCHITECTURE-V2
    - ADR--GRAPH-IS-GKS-DOMAIN
created_at: 2026-05-08T13:21:00.000+07:00
aliases: &a3
  - ADR
  - implementation_flow
  - Architecture decision record
cluster: implementation_flow
role: Architecture decision record
attributes:
  id: ADR--MONOREPO-STRUCTURE
  phase: 2
  type: adr
  status: draft
  vault_id: default
  tier: genesis
  source_type: axiomatic
  title: Monorepo structure — npm workspaces with `packages/gks/` +
    `packages/msp/`, GKS still publishes to npm
  tags: *a1
  crosslinks: *a2
  created_at: 2026-05-08T13:21:00.000+07:00
  aliases: *a3
  cluster: implementation_flow
  role: Architecture decision record
  attributes:
    id: ADR--MONOREPO-STRUCTURE
    phase: 2
    type: adr
    status: draft
    vault_id: default
    tier: genesis
    source_type: axiomatic
    title: Monorepo structure — npm workspaces with `packages/gks/` +
      `packages/msp/`, GKS still publishes to npm
    tags: *a1
    crosslinks: *a2
    created_at: 2026-05-08T13:21:00.000+07:00
    aliases: *a3
    cluster: implementation_flow
    role: Architecture decision record
    attributes:
      domain: adr
    domain: adr
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: true
    secret_type: high_entropy_string
    leak_risk: high
    encryption_level: none
  domain: adr
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: true
  secret_type: high_entropy_string
  leak_risk: high
  encryption_level: none
---

# ADR — monorepo structure

## Context

`[[CONCEPT--MONOREPO-MIGRATION]]` argues for collapsing two repos into one workspace. This ADR fixes the **how**: workspace tool, package layout, boundary rules, and the publish path that keeps `@freshair129/gks` standalone-installable.

## Decision

### Structure

```
freshair129/msp/                    (this repo, renamed-or-kept; archive GksV3 separately)
├── package.json                    (workspace root: `"workspaces": ["packages/*"]`, `"private": true`)
├── package-lock.json               (single lockfile across both packages)
├── packages/
│   ├── gks/                        ← contents of Freshair129/GksV3 today
│   │   ├── package.json            ("name": "@freshair129/gks", "publishConfig": {"access": "public"})
│   │   ├── src/
│   │   ├── test/
│   │   ├── dist/                   (gitignored; built per-publish)
│   │   ├── README.md               (standalone library doc)
│   │   ├── CHANGELOG.md
│   │   └── tsconfig.json
│   └── msp/                        ← contents of this repo's src/, test/, scripts/, web/
│       ├── package.json            ("name": "msp", "private": true)
│       ├── src/
│       ├── test/
│       ├── scripts/msp/
│       ├── web/
│       ├── gks/                    (atom store — KEEPS NAME; this is MSP's content store, not the GKS package)
│       └── tsconfig.json
├── .github/workflows/
│   ├── ci.yml                      (runs both packages' tests + lint + boundary check)
│   └── publish-gks.yml             (publishes packages/gks/ on tag push)
├── eslint.config.js                (boundary rule: GKS package can't import from MSP package)
└── CLAUDE.md
```

**Naming note**: MSP's content store stays `gks/<type>/*.md` even though the GKS *package* lives at `packages/gks/`. They are different things — the directory name `gks/` for content is a convention from `msp_spec.md` and renaming it is out of scope for this migration. Document the distinction in the unified `CLAUDE.md`.

### Workspace tool: npm workspaces (not pnpm, not yarn)

| Criterion | npm workspaces | pnpm workspaces | yarn workspaces |
|---|---|---|---|
| Already in repo | ✅ npm | ❌ adds tool | ❌ adds tool |
| Lockfile parity with current `package-lock.json` | ✅ | ❌ separate `pnpm-lock.yaml` | ❌ separate `yarn.lock` |
| CI cache compatibility | ✅ no change | ❌ requires `pnpm/action-setup` | ❌ requires yarn cache config |
| Disk efficiency (symlinked node_modules) | ⚠️ flat hoist | ✅ content-addressed | ⚠️ flat hoist |

**Decision**: npm workspaces. Disk efficiency loss is negligible for a 2-package monorepo and the migration cost stays minimal.

### Boundary (refined from `[[FRAMEWORK--MSP-ARCHITECTURE-V2]]` + `[[ADR--GRAPH-IS-GKS-DOMAIN]]`)

| Rule | Enforcement |
|---|---|
| `packages/gks/**` must not import from `packages/msp/**` | ESLint `no-restricted-imports` rule + CI |
| `packages/msp/**` may import from `@freshair129/gks` (workspace-resolved during dev) | npm workspace symlink |
| `packages/gks/test/**` must run green without `packages/msp/` present | CI matrix runs `npm test --workspace=packages/gks` from a fresh checkout-without-msp |
| `packages/gks/README.md` must be a standalone library doc | CI lint: README mentions no MSP-specific concept |
| `packages/gks/package.json#dependencies` must not name `msp` or workspace-internal | npm publish dry-run on every PR |

### Dev → publish path

- **Dev**: `npm install` at repo root → workspace links `packages/msp/node_modules/@freshair129/gks` → `packages/gks/`. MSP imports `@freshair129/gks` and the symlink resolves to live workspace source. Edits to GKS source visible immediately to MSP.
- **CI**: Run `npm ci` then `npm test -w packages/gks` and `npm test -w packages/msp` independently. Plus a "GKS standalone" matrix job that checks out only `packages/gks/` into a clean workspace and runs its tests there — proves the package has no hidden MSP dependency.
- **Publish**: Tag push `gks-vX.Y.Z` triggers `publish-gks.yml`. Workflow runs `npm publish --workspace=packages/gks --access=public`. External consumers `npm install @freshair129/gks` as before.

### Versioning

GKS keeps its own semver in `packages/gks/package.json`. MSP depends on GKS via the workspace protocol in dev (`"@freshair129/gks": "workspace:*"`) so it always builds against live source. The published version range is decided at MSP release time, not on every commit.

## Consequences

### Positive

- **Single PR for cross-cutting changes**: phase-6 acceptance, backlinks API, etc. all become one-PR features
- **No publish lag for internal use**: MSP tests against live GKS source on every commit; no waiting for npm publish
- **Drift-impossible**: there is no "MSP atomic claims vs published GKS" gap — they're literally the same git tree
- **`upstream/gks-proposals/` directory deleted**: proposals become normal PRs in this repo
- **`scripts/msp/propose.mjs` phase-6 hack deleted**: GKS CLI fixed in same PR as MSP usage
- **`src/memory/backlinks/` deleted**: backlinks API added to GKS package directly per `[[ADR--GRAPH-IS-GKS-DOMAIN]]`'s long-term plan
- **`[[AUDIT--TWO-REPO-VALIDATION]]` workflow obsolete**: replaced by ESLint boundary rule
- **CI faster**: no more separate `Freshair129/GksV3` pipeline

### Negative

- **One-time migration cost**: ~1–2 days to restructure + verify no regressions
- **External consumers need to know GKS lives in a monorepo** (cosmetic — `npm install @freshair129/gks` still works, but anyone reading the GitHub repo URL sees a different layout)
- **Git history of GKS lives in two places**: archived `Freshair129/GksV3` retains pre-merge history; `packages/gks/` starts fresh in this repo. Acceptable since GKS HEAD = 3.6.0 was just published — clean cut point.
- **Initial workspace setup quirks**: npm workspaces' hoisting can break tooling that hardcodes `node_modules/` paths. Test pre-commit hook scripts carefully.

### Neutral

- Boundary discipline shifts from "physical" (two npm packages) to "machine-checked" (ESLint + CI). For an actively-developed boundary this is stronger; for a frozen library boundary it's roughly equivalent.

## Alternatives considered

### A. Keep two repos, automate the publish lag

Add a GitHub Action on `Freshair129/GksV3` that auto-publishes patch versions on every merge to main. Use Renovate on `Freshair129/msp` to auto-bump the dep.

**Rejected because**: still doesn't solve atomic cross-cutting PRs; still requires writing the GKS change as a published API before MSP can use it; preserves ceremony tax (proposal queue, drift audits) while paying for it with bot infrastructure.

### B. Merge GKS into MSP, stop publishing as separate package

Drop `@freshair129/gks` from npm; serve GKS's public API as a re-export from `msp`.

**Rejected because**: explicitly contradicts the user's stated goal of preserving reusability for hypothetical external consumers. Also strands existing `^3.6.0` consumers (currently zero, but the published package is a public commitment).

### C. Use git submodules instead of npm workspaces

Add `Freshair129/GksV3` as a git submodule under `packages/gks/`.

**Rejected because**: doesn't solve cross-cutting PR ergonomics (still needs commits in the submodule's repo); makes onboarding harder (`git clone --recursive`); IDE tooling support is worse than workspaces.

### D. pnpm workspaces

Slightly better disk usage and stricter dependency hoisting than npm workspaces.

**Rejected because**: adds a tool to the dev environment; existing CI caches and `npm ci` flow work today; benefits don't outweigh migration cost for a 2-package monorepo. Revisit if package count grows past ~5.

## What this ADR does NOT change

- **MSP↔GKS boundary semantics**: `[[FRAMEWORK--MSP-ARCHITECTURE-V2]]` and `[[ADR--GRAPH-IS-GKS-DOMAIN]]` still define what each package owns. The migration moves files; it doesn't redraw the line.
- **Public API of `@freshair129/gks`**: same exports, same CLI, same MCP server entrypoints
- **MSP atom store layout**: `gks/concept/`, `gks/adr/`, etc. inside `packages/msp/` keep their current paths

## Source

- `[[CONCEPT--MONOREPO-MIGRATION]]` — motivation
- npm workspaces docs (https://docs.npmjs.com/cli/v10/using-npm/workspaces)
- Prior art: Vite (`packages/vite`, `packages/create-vite`), tRPC (`packages/server`, `packages/client`, …), Turborepo

---
**Post-2026-05-13 amendment:** The project has pivoted to an agentic monorepo architecture per `[[ADR--AGENTIC-MONOREPO-PIVOT]]`. The canonical layout defined in `FRAMEWORK_MASTER_SPEC.md §4.2` will be materialized, centralizing knowledge, scripts, and documentation at the root level.
