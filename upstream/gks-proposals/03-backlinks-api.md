# 🔵 Proposal 03 — Stable backlinks derivation API (`gks backlinks`)

**Filed upstream**: [Freshair129/GksV3#30](https://github.com/Freshair129/GksV3/issues/30) (2026-05-07)

## Why

GKS's `SCOPE.md` declares atomic graph traversal (note ↔ note backlinks) as
**in-scope** for the storage engine. It already has the building blocks:

- `gks/00_index/atomic_index.jsonl` — every atom's frontmatter incl. `crosslinks`
- `ObsidianAdapter.backlinksOf()` — runtime resolution through Obsidian REST

But there's no **stable CLI / TS API** that derives a flat backlinks JSONL
from the index alone. So MSP built `src/memory/backlinks/` (M3c-1) inside
the MSP repo, duplicating part of GKS's domain.

`MSP_RELATIONSHIP.md` "Compatibility checklist" says backlinks derivation is
the Memory OS implementer's job — but it's a near-universal need, and every
Memory OS reimplementing it independently is wasted work + drift risk.

This proposal moves backlinks derivation upstream so MSP (and other Memory
OS layers) can replace ~200 LoC with a thin call.

## What

### CLI

```
gks backlinks [--emit=jsonl|json] [--out=<path>] [--filter-type=<predicate>]
```

- Walks `gks/00_index/atomic_index.jsonl` (or `gks/<type>/*.md` for fresh derivation)
- Emits one edge per `crosslinks.<predicate>` entry
- Default `--emit=jsonl` (one edge per line, sorted by `from` for git diff stability)
- Optional `--filter-type` to emit only certain predicates (e.g. `--filter-type=implements,references`)

### TS API

```ts
// src/memory/backlinks.ts (new file)

export interface BacklinkEdge {
  from: string      // source atom ID (the atom whose crosslinks declared this edge)
  to: string        // target atom ID
  type: string      // crosslinks predicate name (references, implements, supersedes, ...)
}

export interface BacklinksOptions {
  filterTypes?: string[]
  sort?: boolean    // default true; sort by `from` then `to` then `type`
}

export async function deriveBacklinks(
  store: MemoryStore,
  opts?: BacklinksOptions,
): Promise<BacklinkEdge[]>

export async function emitBacklinksJsonl(
  store: MemoryStore,
  outPath: string,
  opts?: BacklinksOptions,
): Promise<{ edgeCount: number; bytes: number }>
```

### File: `src/memory/backlinks.ts` (sketch)

```ts
export async function deriveBacklinks(store, opts = {}) {
  const edges: BacklinkEdge[] = []
  for (const atom of await store.atomic.all()) {
    const crosslinks = atom.frontmatter.crosslinks ?? {}
    for (const [predicate, targets] of Object.entries(crosslinks)) {
      if (opts.filterTypes && !opts.filterTypes.includes(predicate)) continue
      for (const target of (Array.isArray(targets) ? targets : [targets])) {
        if (typeof target === 'string') {
          edges.push({ from: atom.id, to: target, type: predicate })
        }
      }
    }
  }
  if (opts.sort !== false) {
    edges.sort((a, b) =>
      a.from.localeCompare(b.from) ||
      a.to.localeCompare(b.to) ||
      a.type.localeCompare(b.type)
    )
  }
  return edges
}
```

### MCP tool (optional)

```
gks_backlinks({ filter_types?: string[] }) → { edges: BacklinkEdge[] }
```

## Compat

- **New API** — additive, no existing call sites change.
- **MSP migration**: `src/memory/backlinks/` becomes a thin wrapper that calls `deriveBacklinks(store)`. ~200 LoC → ~20 LoC. Both produce the same JSONL output (assuming sort + predicate normalisation match).
- **Storage**: GKS doesn't have to persist backlinks. It can derive on demand. The MSP-side `vector/backlinks.jsonl` becomes a *cache* the orchestrator owns; GKS provides the derivation function.

## Test

```ts
describe('deriveBacklinks', () => {
  it('emits one edge per crosslinks entry', async () => {
    await seed([
      { id: 'A', crosslinks: { references: ['B', 'C'], implements: ['D'] } },
      { id: 'B' }, { id: 'C' }, { id: 'D' },
    ])
    const edges = await deriveBacklinks(store)
    expect(edges).toEqual([
      { from: 'A', to: 'B', type: 'references' },
      { from: 'A', to: 'C', type: 'references' },
      { from: 'A', to: 'D', type: 'implements' },
    ])
  })

  it('filters by type', async () => {
    const edges = await deriveBacklinks(store, { filterTypes: ['implements'] })
    expect(edges.map(e => e.type)).toEqual(['implements'])
  })

  it('sorts deterministically for git diff stability', async () => {
    const a = await deriveBacklinks(store)
    const b = await deriveBacklinks(store)
    expect(a).toEqual(b)
  })
})
```

## Atom reference

- MSP: `gks/adr/ADR--GRAPH-IS-GKS-DOMAIN.md` (this PR)
- MSP: `gks/concept/CONCEPT--MEMORY-VECTOR-BACKLINKS.md` (planned upstream note added this PR)
- MSP migration target on landing: `src/memory/backlinks/` → thin caller
- MSP atom that follows: `FEAT--MEMORY-BACKLINKS-INDEXER` will get `superseded_by: GKS native API`

## Drafted

2026-05-04, M7-prep follow-up audit.
