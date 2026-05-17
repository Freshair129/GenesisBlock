---
id: AUDIT--SYMBOL-GRAPH-V1
phase: 6
type: audit
status: stable
tier: process
source_type: axiomatic
vault_id: default
title: AUDIT — Symbol Graph v1 shipped (PR-1..PR-6, prototype-in-MSP complete)
tags:
  - msp
  - symbol-graph
  - audit
  - phase-6
  - upstream-gks
crosslinks: {"references":["FRAMEWORK--SYMBOL-GRAPH","CONCEPT--SYMBOL-GRAPH","CONCEPT--PARSER-CHOICE","ADR--SYMBOL-GRAPH-PERSISTENCE","ADR--LEIDEN-COMMUNITY-DETECTION","FEAT--MSP-SYMBOL-MCP","FEAT--MSP-GRAPH-CLI","FEAT--SYMBOLS-WEB-TAB","BLUEPRINT--SYMBOL-GRAPH-CORE"]}
created_at: 2026-05-09T19:30:00.000+07:00
aliases:
  - AUDIT
  - implementation_flow
  - Test results / quality report
cluster: implementation_flow
role: "Test results / quality report"
---

# AUDIT — Symbol Graph v1 shipped

## Summary

The 6-PR Symbol Graph rollout completed 2026-05-09. MSP now has a queryable structural graph over its source code, orthogonal to the conceptual atom graph defined in `[[FRAMEWORK--KNOWLEDGE-3-TIER]]`.

Per `[[FRAMEWORK--SYMBOL-GRAPH]]`'s "path to GKS upstream": this PR also files `upstream/gks-proposals/05-symbol-graph.md` so GKS can absorb the layer and MSP can become a thin wrapper in a future major release.

## What shipped (6 PRs)

| PR | Title | What |
|---|---|---|
| #57 | PR-1: FRAME + 2 CONCEPTs | atoms only — `[[FRAMEWORK--SYMBOL-GRAPH]]` + `[[CONCEPT--SYMBOL-GRAPH]]` + `[[CONCEPT--PARSER-CHOICE]]` |
| #58 | PR-2: 2 ADRs + 3 FEATs | atoms only — persistence (SQLite + JSONL) + Leiden + MCP/CLI/Web contracts |
| #59 | PR-3: BLUEPRINT + parser + SQLite + JSONL + Leiden | foundation code: TS Compiler API parser, `better-sqlite3` store, JSONL exporter, Leiden adapter (with Louvain fallback) |
| #60 | PR-4: CLI + 5 MCP tools | `msp-graph` bin (6 subcommands) + `msp_symbol_lookup/neighbors/impact/community/search`. Tool count 11 → 16 |
| #62 | PR-5: Web UI Symbols tab | 6 `/api/symbols/*` endpoints + 4 React components (Cytoscape graph, list, detail, parent tab) |
| (this PR) | PR-6: AUDIT + upstream proposal | this atom + `upstream/gks-proposals/05-symbol-graph.md` + remove feature flag |

## Real numbers from MSP repo (build smoke test, PR-4 subagent)

```
$ npm run msp:graph build
[graph] built 897 symbols / 1843 edges / 143 communities, modularity=0.958, 62s
```

- 897 symbols across `src/**/*.ts` + `web/src/**/*.tsx`
- 1843 edges (calls, imports, defines, extends, implements, references)
- 143 communities at `resolution=1.0`, `seed=42`
- Modularity 0.958 — high (deeply-modular codebase)
- Build time ~62 s on this MSP repo

## Deltas from BLUEPRINT

| Topic | BLUEPRINT said | Reality | Why |
|---|---|---|---|
| Symbol count estimate | ~5000 | 897 | Per-file `ts.Program` parser misses cross-file refs without a project-wide compile context. PR-3 `parser/typescript.ts` made this trade-off explicitly to keep memory + speed tractable. Phase 2 may revisit. |
| Edge count estimate | ~20000 | 1843 | Same root cause as above. |
| Communities estimate | 8–15 | 143 | At `resolution=1.0` Leiden gives finer-grained clusters than the BLUEPRINT predicted. Default kept; tunable via `--resolution=<n>`. Lower resolution → bigger / fewer communities. Reviewers / users can experiment without code changes. |
| `graphology` import shape | `import { UndirectedGraph } from 'graphology'` | `import graphology from 'graphology'; const { UndirectedGraph } = graphology` | graphology@0.25 is published CJS; Node ESM-of-CJS rules don't expose named exports. PR-3 missed it because no test triggered the leiden import path. PR-4 fixed in-line as part of its CLI work. |
| Leiden / Louvain dep tier | `optionalDependencies` (per ADR) | `dependencies` | The fallback path uses `try/catch` around dynamic `import()` — install always succeeds with both packages present, so optional-dep machinery is redundant. ADR text describes the principle correctly; final placement was a pragmatic call. |

## Feature flag removed

Per `[[FEAT--SYMBOLS-WEB-TAB]]`, the `VITE_MSP_SYMBOL_GRAPH=1` / `?symbols=1` flag protected an empty Symbols tab from confusing first-time users. PR-6 removes it: `SymbolsTab.tsx` already has a graceful empty-state ("Run `npm run msp:graph build`" hint with copy-to-clipboard button), so the tab is now visible by default.

## Verification gates (across 6 PRs)

- `npm run typecheck` clean throughout
- `npm run build` clean throughout
- `npm test` 535 (start of session) → 625 (end of PR-5) → 625 (this PR — atoms + upstream proposal only). Every PR shipped with green tests on Node 20 + 22 CI matrix.
- `npx tsx src/validator/cli.ts --all` exit 0 throughout
- `npm run msp:check-links` OK throughout
- `gks verify-flow` per FEAT — OK after PR-2 promoted FRAME + CONCEPTs to stable
- Manual smoke (PR-4): build → query → community lookups all work
- Manual smoke (PR-5): `?symbols=1` URL renders all 3 panes; empty state when graph absent

## What's next (Phase 2 / post-roadmap)

| Item | Notes |
|---|---|
| Cross-file symbol resolution via project-wide `ts.Program` | Would push counts toward the BLUEPRINT estimate (~5000 symbols / ~20000 edges); cost: memory + build time |
| tree-sitter parser for Python / Go / Rust | Per `[[CONCEPT--PARSER-CHOICE]]` Phase 2; interface absorbs swap |
| Hierarchical Leiden levels | `parent_community_id` column reserved in SQLite; UI surface TBD |
| Real-time file-watch incremental rebuild | Phase 3 |
| GKS upstream — wait for `@freshair129/gks 4.x` | This PR files the proposal; migration to thin wrapper happens once GKS ships |
| `linked_symbols[]` cross-link from FEAT atoms to their symbol IDs | Useful but not blocking; manual labor for now, automation possible after symbol-graph is project-wide indexed |

## Lesson

The CONCEPT-stage trade-off "TS Compiler API in v1" had downstream consequences (per-file parsing limits cross-file resolution, hence the lower-than-estimated counts) that didn't surface until the first real build. The interface design absorbs the eventual swap to a project-wide checker — PR doesn't paint into a corner — but the BLUEPRINT estimate should have called out this caveat explicitly. Logged for future BLUEPRINT-writing.

## Source

- PRs #57, #58, #59, #60, #62, this PR
- `[[BLUEPRINT--SYMBOL-GRAPH-CORE]]` (PR-3) — implementation contract
- `/root/.claude/plans/symbol-graph.md` — original 6-PR plan
- Smoke test output PR-4 subagent run, 2026-05-09

## Connections
- [[ADR--SYMBOL-GRAPH-PERSISTENCE]]
- [[ADR--LEIDEN-COMMUNITY-DETECTION]]
- [[FEAT--MSP-SYMBOL-MCP]]
- [[FEAT--MSP-GRAPH-CLI]]

