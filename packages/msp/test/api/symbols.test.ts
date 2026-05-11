/**
 * HTTP-level tests for the Symbol Graph API (PR-5 of 6).
 *
 * Strategy:
 *  - Build a tiny in-tmpdir Symbol Graph (SQLite + meta.json) by hand.
 *  - Stand up an Express app, register the symbol routes pointed at the tmp root.
 *  - Hit it via Node's built-in `fetch` against `http://127.0.0.1:<port>`.
 *
 * No new test deps: just `vitest` + `express` (already present).
 */
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import type { AddressInfo } from 'node:net'
import express from 'express'
import { afterAll, afterEach, beforeAll, beforeEach, describe, expect, it } from 'vitest'

import { registerSymbolApi } from '../../src/symbols/api.js'
import { SymbolStore } from '../../src/symbols/store/sqlite.js'
import type { Edge, Symbol, SymbolGraphMeta } from '../../src/symbols/types.js'
import { dbPath, symbolsDir, metaPath } from '../../src/symbols/util.js'

let workRoot: string
let server: ReturnType<express.Express['listen']> | null = null
let baseUrl: string

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

function buildGraph(root: string): void {
  mkdirSync(symbolsDir(root), { recursive: true })
  const store = new SymbolStore()
  store.open(dbPath(root))
  const a = fakeSymbol({ community_id: 1 })
  const b = fakeSymbol({
    id: 'src/baz.ts:qux:func',
    name: 'qux',
    file: 'src/baz.ts',
    community_id: 1,
  })
  const c = fakeSymbol({
    id: 'src/zap.ts:zap:func',
    name: 'zap',
    file: 'src/zap.ts',
    community_id: 2,
  })
  store.upsertSymbol(a)
  store.upsertSymbol(b)
  store.upsertSymbol(c)
  store.upsertEdge(fakeEdge())
  store.upsertCommunity({ id: 1, size: 2, label: 'src/foo', modularity: 0.42, parent_id: null })
  store.upsertCommunity({ id: 2, size: 1, label: 'src/zap', modularity: 0.0, parent_id: null })
  store.setMeta('last_built_at', '2026-05-09T10:00:00.000Z')
  store.close()

  const meta: SymbolGraphMeta = {
    schema_version: 1,
    last_built_at: '2026-05-09T10:00:00.000Z',
    parser: 'typescript',
    algorithm: 'leiden',
    leiden_resolution: 1.0,
    leiden_seed: 42,
    symbol_count: 3,
    edge_count: 1,
    community_count: 2,
    parse_errors: [],
  }
  writeFileSync(metaPath(root), JSON.stringify(meta, null, 2) + '\n', 'utf8')
}

async function startServer(root: string): Promise<{ server: ReturnType<express.Express['listen']>; url: string }> {
  const app = express()
  registerSymbolApi(app, () => root)
  return await new Promise((resolve) => {
    const s = app.listen(0, '127.0.0.1', () => {
      const addr = s.address() as AddressInfo
      resolve({ server: s, url: `http://127.0.0.1:${addr.port}` })
    })
  })
}

async function stopServer(s: ReturnType<express.Express['listen']>): Promise<void> {
  return await new Promise((resolve, reject) => {
    s.close((err) => (err ? reject(err) : resolve()))
  })
}

describe('Symbol Graph API — graph not built', () => {
  beforeEach(() => {
    workRoot = mkdtempSync(join(tmpdir(), 'sg-api-empty-'))
  })
  afterEach(async () => {
    if (server) {
      await stopServer(server)
      server = null
    }
    rmSync(workRoot, { recursive: true, force: true })
  })

  it('GET /api/symbols returns 404 with not-built error', async () => {
    const started = await startServer(workRoot)
    server = started.server
    baseUrl = started.url
    const res = await fetch(`${baseUrl}/api/symbols`)
    expect(res.status).toBe(404)
    const body = (await res.json()) as { ok: boolean; error: string }
    expect(body.ok).toBe(false)
    expect(body.error).toMatch(/graph not built/)
  })

  it('GET /api/symbols/stats returns 404 when meta.json missing', async () => {
    const started = await startServer(workRoot)
    server = started.server
    baseUrl = started.url
    const res = await fetch(`${baseUrl}/api/symbols/stats`)
    expect(res.status).toBe(404)
    const body = (await res.json()) as { ok: boolean; error: string }
    expect(body.ok).toBe(false)
    expect(body.error).toMatch(/graph not built/)
  })
})

describe('Symbol Graph API — graph built', () => {
  beforeAll(async () => {
    workRoot = mkdtempSync(join(tmpdir(), 'sg-api-built-'))
    buildGraph(workRoot)
    const started = await startServer(workRoot)
    server = started.server
    baseUrl = started.url
  })
  afterAll(async () => {
    if (server) {
      await stopServer(server)
      server = null
    }
    rmSync(workRoot, { recursive: true, force: true })
  })

  it('GET /api/symbols returns symbols + communities envelope', async () => {
    const res = await fetch(`${baseUrl}/api/symbols`)
    expect(res.status).toBe(200)
    const body = (await res.json()) as {
      ok: boolean
      symbols: Symbol[]
      communities: { id: number }[]
    }
    expect(body.ok).toBe(true)
    expect(body.symbols.length).toBe(3)
    expect(body.communities.map((c) => c.id).sort()).toEqual([1, 2])
  })

  it('GET /api/symbols/stats returns the meta document', async () => {
    const res = await fetch(`${baseUrl}/api/symbols/stats`)
    expect(res.status).toBe(200)
    const body = (await res.json()) as { ok: boolean; symbol_count: number; algorithm: string }
    expect(body.ok).toBe(true)
    expect(body.symbol_count).toBe(3)
    expect(body.algorithm).toBe('leiden')
  })

  it('GET /api/symbols/:id returns the symbol + neighborPreview', async () => {
    const id = 'src/foo.ts:bar:func'
    const res = await fetch(`${baseUrl}/api/symbols/${encodeURIComponent(id)}`)
    expect(res.status).toBe(200)
    const body = (await res.json()) as {
      ok: boolean
      symbol: Symbol
      neighborPreview: Symbol[]
    }
    expect(body.ok).toBe(true)
    expect(body.symbol.id).toBe(id)
    expect(body.neighborPreview.map((n) => n.id)).toEqual(['src/baz.ts:qux:func'])
  })

  it('GET /api/symbols/:id returns 404 for unknown id', async () => {
    const res = await fetch(`${baseUrl}/api/symbols/${encodeURIComponent('does/not:exist:func')}`)
    expect(res.status).toBe(404)
    const body = (await res.json()) as { ok: boolean; error: string }
    expect(body.ok).toBe(false)
    expect(body.error).toMatch(/unknown symbol/)
  })

  it('GET /api/symbols/:id/neighbors returns center + reached nodes + edges', async () => {
    const id = 'src/foo.ts:bar:func'
    const res = await fetch(`${baseUrl}/api/symbols/${encodeURIComponent(id)}/neighbors?depth=1`)
    expect(res.status).toBe(200)
    const body = (await res.json()) as {
      ok: boolean
      nodes: Symbol[]
      edges: Edge[]
    }
    expect(body.ok).toBe(true)
    // Center is included as the first node
    expect(body.nodes.map((n) => n.id)).toContain(id)
    expect(body.nodes.map((n) => n.id)).toContain('src/baz.ts:qux:func')
    expect(body.edges.length).toBe(1)
  })

  it('GET /api/symbols/community/:id returns members + internal edges', async () => {
    const res = await fetch(`${baseUrl}/api/symbols/community/1`)
    expect(res.status).toBe(200)
    const body = (await res.json()) as {
      ok: boolean
      community: { id: number; label: string | null }
      members: Symbol[]
      edges: Edge[]
    }
    expect(body.ok).toBe(true)
    expect(body.community.id).toBe(1)
    expect(body.members.map((m) => m.id).sort()).toEqual([
      'src/baz.ts:qux:func',
      'src/foo.ts:bar:func',
    ])
    // Both endpoints in community 1, so the calls edge is internal.
    expect(body.edges.length).toBe(1)
  })

  it('GET /api/symbols/search ranks exact-name match highest', async () => {
    const res = await fetch(`${baseUrl}/api/symbols/search?q=bar`)
    expect(res.status).toBe(200)
    const body = (await res.json()) as {
      ok: boolean
      hits: (Symbol & { score: number })[]
    }
    expect(body.ok).toBe(true)
    expect(body.hits.length).toBeGreaterThan(0)
    expect(body.hits[0].name).toBe('bar')
  })

  it('GET /api/symbols/search 400 on empty q', async () => {
    const res = await fetch(`${baseUrl}/api/symbols/search?q=`)
    expect(res.status).toBe(400)
    const body = (await res.json()) as { ok: boolean; error: string }
    expect(body.ok).toBe(false)
    expect(body.error).toMatch(/`q` required/)
  })
})
