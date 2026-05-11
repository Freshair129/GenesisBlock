/**
 * Symbol Graph HTTP API (PR-5 of 6).
 *
 * Six read-only endpoints backing the Knowledge Browser Symbols tab:
 *
 *   GET /api/symbols                      list (capped 5000)
 *   GET /api/symbols/stats                meta.json
 *   GET /api/symbols/search?q=&limit=     ranked hits
 *   GET /api/symbols/community/:id        community + members + internal edges
 *   GET /api/symbols/:id/neighbors        k-hop BFS (capped depth 3)
 *   GET /api/symbols/:id                  symbol + 5-neighbor preview
 *
 * All return `{ ok: true, ... }` on success and `{ ok: false, error }` 404 when
 * the graph is missing. See FEAT--SYMBOLS-WEB-TAB for the full contract.
 *
 * Routes are registered via `registerSymbolApi(app, getRoot)` so the host
 * (`src/index.ts`) controls how the active project root is resolved.
 */
import type { Express, Response } from 'express'
import fs from 'node:fs/promises'

import { SymbolStore } from './store/sqlite.js'
import type { Edge, EdgeType, Symbol as SymbolNode, SymbolGraphMeta } from './types.js'
import { dbPath, graphExists, metaPath } from './util.js'

const SYMBOL_GRAPH_NOT_BUILT_ERROR =
  "graph not built — run 'npm run msp:graph build' first"

const KNOWN_EDGE_TYPES: EdgeType[] = [
  'defines',
  'imports',
  'calls',
  'extends',
  'implements',
  'references',
]

interface ScoredSymbolHit extends SymbolNode {
  score: number
}

/**
 * Mirrors the scoring in `src/mcp/tools/symbol-search.ts`:
 *   100 exact name, 80 case-insensitive, 60 prefix, 40 substring, 20 signature,
 *   +5 for exported, -min(5, depth) for file depth.
 */
function scoreSymbol(symbol: SymbolNode, query: string, queryLower: string): number {
  const nameLower = symbol.name.toLowerCase()
  const sigLower = (symbol.signature ?? '').toLowerCase()
  let s = 0
  if (symbol.name === query) s = 100
  else if (nameLower === queryLower) s = 80
  else if (nameLower.startsWith(queryLower)) s = 60
  else if (nameLower.includes(queryLower)) s = 40
  else if (sigLower.includes(queryLower)) s = 20
  else return 0
  if (symbol.exported) s += 5
  s -= Math.min(5, symbol.file.split('/').length)
  return s
}

function openStoreOrFail(root: string, res: Response): SymbolStore | null {
  if (!graphExists(root)) {
    res.status(404).json({ ok: false, error: SYMBOL_GRAPH_NOT_BUILT_ERROR })
    return null
  }
  const store = new SymbolStore()
  try {
    store.open(dbPath(root))
  } catch (err) {
    res.status(500).json({ ok: false, error: (err as Error).message })
    return null
  }
  if (!store.getMeta('last_built_at')) {
    store.close()
    res.status(404).json({ ok: false, error: SYMBOL_GRAPH_NOT_BUILT_ERROR })
    return null
  }
  return store
}

function safeClose(store: SymbolStore): void {
  try {
    store.close()
  } catch {
    // ignore
  }
}

export function registerSymbolApi(app: Express, getRoot: () => string): void {
  app.get('/api/symbols', async (req, res) => {
    const store = openStoreOrFail(getRoot(), res)
    if (!store) return
    try {
      const offsetRaw = Number(req.query.offset ?? 0)
      const limitRaw = Number(req.query.limit ?? 5000)
      const offset =
        Number.isFinite(offsetRaw) && offsetRaw >= 0 ? Math.floor(offsetRaw) : 0
      const limit =
        Number.isFinite(limitRaw) && limitRaw > 0 ? Math.min(5000, Math.floor(limitRaw)) : 5000
      const allSymbols = store.allSymbols()
      const symbols = allSymbols.slice(offset, offset + limit)
      const communities = store.allCommunities()
      res.json({ ok: true, symbols, communities })
    } catch (err) {
      res.status(500).json({ ok: false, error: (err as Error).message })
    } finally {
      safeClose(store)
    }
  })

  app.get('/api/symbols/stats', async (_req, res) => {
    try {
      const root = getRoot()
      const metaFile = metaPath(root)
      let raw: string
      try {
        raw = await fs.readFile(metaFile, 'utf-8')
      } catch {
        res.status(404).json({ ok: false, error: SYMBOL_GRAPH_NOT_BUILT_ERROR })
        return
      }
      let meta: SymbolGraphMeta
      try {
        meta = JSON.parse(raw) as SymbolGraphMeta
      } catch (err) {
        res.status(500).json({
          ok: false,
          error: `meta.json unreadable: ${(err as Error).message}`,
        })
        return
      }
      res.json({ ok: true, ...meta })
    } catch (err) {
      res.status(500).json({ ok: false, error: (err as Error).message })
    }
  })

  app.get('/api/symbols/search', async (req, res) => {
    const store = openStoreOrFail(getRoot(), res)
    if (!store) return
    try {
      const q = String(req.query.q ?? '')
      if (!q) {
        res.status(400).json({ ok: false, error: 'query parameter `q` required' })
        return
      }
      const limitRaw = Number(req.query.limit ?? 20)
      const limit =
        Number.isFinite(limitRaw) && limitRaw > 0 ? Math.min(200, Math.floor(limitRaw)) : 20
      const queryLower = q.toLowerCase()
      const all = store.allSymbols()
      const scored: ScoredSymbolHit[] = []
      for (const s of all) {
        const v = scoreSymbol(s, q, queryLower)
        if (v > 0) scored.push({ ...s, score: v })
      }
      scored.sort((a, b) => {
        if (a.score !== b.score) return b.score - a.score
        return a.id < b.id ? -1 : a.id > b.id ? 1 : 0
      })
      res.json({ ok: true, hits: scored.slice(0, limit) })
    } catch (err) {
      res.status(500).json({ ok: false, error: (err as Error).message })
    } finally {
      safeClose(store)
    }
  })

  app.get('/api/symbols/community/:id', async (req, res) => {
    const store = openStoreOrFail(getRoot(), res)
    if (!store) return
    try {
      const communityId = Number(req.params.id)
      if (!Number.isFinite(communityId)) {
        res.status(400).json({ ok: false, error: 'community id must be a number' })
        return
      }
      const allCommunities = store.allCommunities()
      const community = allCommunities.find((c) => c.id === communityId)
      if (!community) {
        res.status(404).json({ ok: false, error: `community ${communityId} not found` })
        return
      }
      const members = store.getCommunityMembers(communityId)
      const memberIds = new Set(members.map((m) => m.id))
      const allEdges = store.allEdges()
      const edges: Edge[] = allEdges.filter(
        (e) => memberIds.has(e.src_id) && memberIds.has(e.dst_id),
      )
      res.json({ ok: true, community, members, edges })
    } catch (err) {
      res.status(500).json({ ok: false, error: (err as Error).message })
    } finally {
      safeClose(store)
    }
  })

  app.get('/api/symbols/:id/neighbors', async (req, res) => {
    const store = openStoreOrFail(getRoot(), res)
    if (!store) return
    try {
      const id = req.params.id
      const center = store.getSymbol(id)
      if (!center) {
        res.status(404).json({ ok: false, error: `unknown symbol id "${id}"` })
        return
      }
      const depthRaw = Number(req.query.depth ?? 1)
      const depth =
        Number.isFinite(depthRaw) && depthRaw > 0
          ? Math.min(3, Math.max(1, Math.floor(depthRaw)))
          : 1
      const typesParam = req.query.types
      let edgeTypes: EdgeType[] | undefined
      if (typeof typesParam === 'string' && typesParam.length > 0) {
        const list = typesParam.split(',').map((s) => s.trim()).filter(Boolean)
        const valid = list.filter((t): t is EdgeType =>
          KNOWN_EDGE_TYPES.includes(t as EdgeType),
        )
        if (valid.length > 0) edgeTypes = valid
      }
      const { nodes, edges } = store.getNeighbors(id, depth, edgeTypes)
      const allNodes: SymbolNode[] = [center, ...nodes]
      res.json({ ok: true, nodes: allNodes, edges })
    } catch (err) {
      res.status(500).json({ ok: false, error: (err as Error).message })
    } finally {
      safeClose(store)
    }
  })

  app.get('/api/symbols/:id', async (req, res) => {
    const store = openStoreOrFail(getRoot(), res)
    if (!store) return
    try {
      const id = req.params.id
      const symbol = store.getSymbol(id)
      if (!symbol) {
        res.status(404).json({ ok: false, error: `unknown symbol id "${id}"` })
        return
      }
      const { nodes } = store.getNeighbors(id, 1)
      const neighborPreview: SymbolNode[] = nodes.slice(0, 5)
      res.json({ ok: true, symbol, neighborPreview })
    } catch (err) {
      res.status(500).json({ ok: false, error: (err as Error).message })
    } finally {
      safeClose(store)
    }
  })
}
