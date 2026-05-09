import { mkdtempSync, readFileSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import { dumpJsonl, JSONL_FILES, loadJsonl } from '../../src/symbols/store/jsonl.js'
import { SymbolStore } from '../../src/symbols/store/sqlite.js'
import type { Edge, Symbol } from '../../src/symbols/types.js'

let workDir: string

function fakeSymbol(over: Partial<Symbol> = {}): Symbol {
  return {
    id: 'src/foo.ts:bar:func',
    name: 'bar',
    kind: 'function',
    file: 'src/foo.ts',
    start_line: 1,
    end_line: 5,
    exported: true,
    parent_id: null,
    signature: 'function bar(): number',
    community_id: null,
    created_at: '2026-05-09T10:00:00.000Z',
    ...over,
  }
}

function fakeEdge(over: Partial<Edge> = {}): Edge {
  return {
    src_id: 'src/foo.ts:bar:func',
    dst_id: 'src/baz.ts:qux:func',
    type: 'calls',
    weight: 1,
    resolved: true,
    ...over,
  }
}

beforeEach(() => {
  workDir = mkdtempSync(join(tmpdir(), 'sg-store-'))
})

afterEach(() => {
  rmSync(workDir, { recursive: true, force: true })
})

describe('SymbolStore', () => {
  it('opens, migrates, and closes cleanly', () => {
    const dbPath = join(workDir, 'graph.db')
    const store = new SymbolStore()
    store.open(dbPath)
    expect(store.getMeta('schema_version')).toBe('1')
    store.close()
  })

  it('round-trips a symbol via upsert + getSymbol', () => {
    const store = new SymbolStore()
    store.open(join(workDir, 'graph.db'))
    const sym = fakeSymbol()
    store.upsertSymbol(sym)
    const fetched = store.getSymbol(sym.id)
    expect(fetched).not.toBeNull()
    expect(fetched).toEqual(sym)
    store.close()
  })

  it('upserts an edge and retrieves it via getNeighbors(depth=1)', () => {
    const store = new SymbolStore()
    store.open(join(workDir, 'graph.db'))
    const a = fakeSymbol()
    const b = fakeSymbol({ id: 'src/baz.ts:qux:func', name: 'qux', file: 'src/baz.ts' })
    store.upsertSymbol(a)
    store.upsertSymbol(b)
    store.upsertEdge(fakeEdge())

    const { nodes, edges } = store.getNeighbors(a.id, 1)
    expect(edges).toHaveLength(1)
    expect(edges[0].dst_id).toBe(b.id)
    expect(nodes.map((n) => n.id)).toEqual([b.id])
    store.close()
  })

  it('returns the right members from getCommunityMembers after community upsert', () => {
    const store = new SymbolStore()
    store.open(join(workDir, 'graph.db'))
    const a = fakeSymbol()
    const b = fakeSymbol({ id: 'src/baz.ts:qux:func', name: 'qux', file: 'src/baz.ts' })
    const c = fakeSymbol({ id: 'src/qux.ts:zap:func', name: 'zap', file: 'src/qux.ts' })
    store.upsertSymbol(a)
    store.upsertSymbol(b)
    store.upsertSymbol(c)
    store.upsertCommunity({ id: 7, size: 2, label: 'src/foo', modularity: 0.42, parent_id: null })
    store.setSymbolCommunity(a.id, 7)
    store.setSymbolCommunity(b.id, 7)
    store.setSymbolCommunity(c.id, 8)

    const members = store.getCommunityMembers(7)
    expect(members.map((m) => m.id).sort()).toEqual([a.id, b.id].sort())
    store.close()
  })

  it('round-trips JSONL: dump → fresh-load → dump → byte-identical', async () => {
    const dbA = join(workDir, 'a.db')
    const dbB = join(workDir, 'b.db')
    const dumpA = join(workDir, 'dumpA')
    const dumpB = join(workDir, 'dumpB')

    const storeA = new SymbolStore()
    storeA.open(dbA)
    const a = fakeSymbol()
    const b = fakeSymbol({
      id: 'src/baz.ts:qux:func',
      name: 'qux',
      file: 'src/baz.ts',
      community_id: 1,
    })
    storeA.upsertSymbol(a)
    storeA.upsertSymbol(b)
    storeA.upsertEdge(fakeEdge())
    storeA.upsertCommunity({ id: 1, size: 1, label: 'src/baz/qux', modularity: 0.1, parent_id: null })

    await dumpJsonl(storeA, dumpA)
    storeA.close()

    const storeB = new SymbolStore()
    storeB.open(dbB)
    storeB.clearAll()
    await loadJsonl(dumpA, storeB)
    await dumpJsonl(storeB, dumpB)
    storeB.close()

    for (const file of [JSONL_FILES.SYMBOLS_FILE, JSONL_FILES.EDGES_FILE, JSONL_FILES.COMMUNITIES_FILE]) {
      const a = readFileSync(join(dumpA, file), 'utf8')
      const b = readFileSync(join(dumpB, file), 'utf8')
      expect(b).toBe(a)
    }
  })
})
