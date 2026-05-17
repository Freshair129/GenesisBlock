---
id: FEAT--MSP-GRAPH-CLI
phase: 2
type: feat
status: stable
tier: process
source_type: axiomatic
vault_id: default
title: msp:graph CLI — build / query / community / impact / stats / dump-jsonl subcommands
tags:
  - msp
  - symbol-graph
  - cli
  - feat
  - bin
crosslinks:
  implements:
    - ADR--SYMBOL-GRAPH-PERSISTENCE
    - ADR--LEIDEN-COMMUNITY-DETECTION
  references:
    - FRAMEWORK--SYMBOL-GRAPH
    - CONCEPT--PARSER-CHOICE
linked_symbols:
  - file: packages/msp/src/symbols/cli.ts
  - file: package.json
created_at: 2026-05-09T16:53:00.000+07:00
aliases:
  - FEAT
  - implementation_flow
  - Feature spec
  - Feature
cluster: implementation_flow
role: "Feature spec"
---

# FEAT — msp:graph CLI

## User-facing behaviour

Single CLI binary `msp-graph` (registered in `package.json` `bin`) plus `npm run msp:graph` script alias for development. Six subcommands.

```
msp-graph <subcommand> [args] [flags]
```

| Subcommand | Args | Flags | Exit |
|---|---|---|---|
| `build` | — | `--root=<dir>` `--out=<dir>` `--resolution=<n>` `--include="src/**/*.ts,web/src/**/*.tsx"` `--seed=<n>` | 0 ok / 1 parse-fail / 2 persist-fail |
| `query <name>` | symbol name (exact or prefix) | `--kind=<k>` `--json` | 0 found / 1 not-found |
| `community` | — | `--id=<n>` OR `--symbol=<id>` ; `--visualize=mermaid|cytoscape-json|stdout` ; `--resolution=<n>` (re-detect on the fly) | 0 ok / 2 stale-graph |
| `impact <id>` | symbol id | `--depth=<n>` `--json` | 0 ok |
| `stats` | — | `--json` | 0 ok / 2 not-built |
| `dump-jsonl` | — | `--out=<dir>` | 0 ok — re-emits JSONL from current SQLite |

Every subcommand respects `--root=<dir>` (default = `process.cwd()`). All produce structured output via `--json` flag; default is human-readable.

## Build flow (the most common subcommand)

```
msp-graph build --root=. --resolution=1.0 --seed=42
  ↓ (1) discover files via glob from --include
  ↓ (2) parser.parseFile() per file → Symbol[] + Edge[]
  ↓ (3) resolve cross-file edges (imports, calls)
  ↓ (4) persist to SQLite (.brain/msp/projects/<ns>/symbols/graph.db)
  ↓ (5) run Leiden → assign community_id per symbol
  ↓ (6) dump JSONL (symbols.jsonl, edges.jsonl, communities.jsonl, meta.json)
  ↓ (7) print summary: "5234 symbols / 18411 edges / 12 communities, modularity=0.74, took 4.2s"
```

Default `--include` = `"src/**/*.ts,web/src/**/*.tsx"` (excludes `node_modules`, `.brain`, `dist`). Files that fail to parse are skipped with a per-file warning; `meta.parse_errors[]` records them.

## Bin entry

`package.json`:
```json
"bin": {
  ...
  "msp-graph": "./dist/symbols/cli.js"
}
"scripts": {
  ...
  "msp:graph": "tsx src/symbols/cli.ts"
}
```

The `chmod-bins.mjs` step ensures the dist file is executable post-build.

## Verification

- Unit tests in `test/symbols/cli.test.ts` (PR-4): each subcommand exits with the right code on fixtures
- Smoke test: `npm run msp:graph build && npm run msp:graph stats` round-trip on the MSP repo itself
- `dump-jsonl` round-trip: build → dump → fresh-load → dump → byte-compare (asserts deterministic export)

## Out of scope

- Real-time file-watch incremental rebuild — Phase 3
- IDE click-through richer than `vscode://file/<path>:<line>` URI — Phase 2
- Cross-repo build — Phase 2

## Source

- `[[ADR--SYMBOL-GRAPH-PERSISTENCE]]`, `[[ADR--LEIDEN-COMMUNITY-DETECTION]]`
- Existing CLI shape: `src/codegen/cli.ts` + `bin/msp-validate` pattern
- Existing `chmod-bins.mjs` post-build step

## Connections
- [[FRAMEWORK--SYMBOL-GRAPH]]
- [[CONCEPT--PARSER-CHOICE]]

