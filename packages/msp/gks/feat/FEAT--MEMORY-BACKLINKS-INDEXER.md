---
id: FEAT--MEMORY-BACKLINKS-INDEXER
phase: 2
type: feat
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: msp:backlinks — full-rebuild backlinks.jsonl from atom crosslinks
tags:
  - msp
  - memory
  - backlinks
  - indexer
  - user-facing
crosslinks: {"implements":["ADR--MEMORY-BACKLINKS-INDEXER"],"references":["CONCEPT--MEMORY-BACKLINKS-INDEXER","CONCEPT--MEMORY-VECTOR-BACKLINKS"]}
linked_symbols:
  - {"file":"src/memory/backlinks/indexer.ts"}
  - {"file":"src/memory/backlinks/cli.ts"}
created_at: 2026-05-03T15:16:42.851+07:00
---

# FEAT — backlinks indexer

## User-facing behaviour

```sh
npm run msp:backlinks            # rebuild backlinks.jsonl from gks/ atoms
npm run msp:backlinks -- --dry-run   # preview output without writing
npm run msp:backlinks -- --check     # exit-1 if file would change (CI assert)
```

The TS API:

```ts
import { rebuildBacklinks } from '@/memory/backlinks/indexer'

const result = await rebuildBacklinks({
  root: '.',
  namespace: 'evaAI',     // default 'evaAI' (per ADR--PATH-ENCODING)
  dryRun: false,
})
console.log(result.edgeCount, result.changed)
```

## Acceptance criteria

- [ ] Walks `gks/<type>/*.md` recursively, skips `gks/00_index/`
- [ ] Emits one edge per `crosslinks.<predicate>` value across all atoms
- [ ] Output sorted by `(from, to, type)` — byte-identical between runs with no atom changes
- [ ] Atomic write (`<file>.tmp` + `rename`) — never leaves partial output
- [ ] `--dry-run` exits 0 and prints summary without writing
- [ ] `--check` exits 1 if generated content differs from on-disk file (for CI)
- [ ] Returns `{ edgeCount, atomCount, changed }` from the TS API
- [ ] vitest unit + CLI integration tests cover all cases
- [ ] Dogfood: running on this repo's atoms produces a non-empty `backlinks.jsonl` matching `gks verify-flow`'s view of the graph

## Surfaces

| Surface | Form |
|---|---|
| TS API | `rebuildBacklinks(opts): Promise<RebuildResult>` |
| CLI | `msp-backlinks` with `--dry-run` / `--check` / `--root` / `--namespace` |
| MCP | future |

## Out of scope

- Reverse-index lookup ("who points at X?") — computed in-memory at query time.
- Vector embedding — orchestrator concern.
- Incremental updates — `ADR--MEMORY-BACKLINKS-INDEXER` rejects this.
