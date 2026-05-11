import { resolve } from 'node:path'

import { z } from 'zod'

import { SymbolStore } from '../../symbols/store/sqlite.js'
import type { EdgeType, Symbol } from '../../symbols/types.js'
import { dbPath, graphExists } from '../../symbols/util.js'
import { errorResult, jsonResult, type ToolHandlerCtx, type ToolTextResult } from '../types.js'

export const name = 'msp_symbol_impact'

export const description =
  'Reverse closure on `calls` ∪ `references` from a given symbol id — answers "what calls this?". Read-only.'

export const inputSchema = {
  id: z.string().describe('Symbol id to find callers of.'),
  depth: z
    .number()
    .int()
    .positive()
    .optional()
    .describe('Max reverse-BFS depth (default 5).'),
  root: z.string().optional().describe('Project root (default: server context root).'),
  namespace: z.string().optional().describe('Project namespace (default: evaAI).'),
}

interface ImpactArgs {
  id: string
  depth?: number
  root?: string
  namespace?: string
}

const REVERSE_TYPES: EdgeType[] = ['calls', 'references']
const DEFAULT_DEPTH = 5

interface ImpactCaller {
  symbol: Symbol | null
  distance: number
}

export function handler(ctx: ToolHandlerCtx) {
  return async (args: ImpactArgs): Promise<ToolTextResult> => {
    const root = resolve(args.root ?? ctx.root)
    const namespace = args.namespace
    if (!graphExists(root, namespace)) {
      return errorResult("graph not built — run 'npm run msp:graph build' first")
    }
    const store = new SymbolStore()
    try {
      store.open(dbPath(root, namespace))
      if (!store.getMeta('last_built_at')) {
        return errorResult("graph not built — run 'npm run msp:graph build' first")
      }
      const target = store.getSymbol(args.id)
      if (!target) {
        return errorResult(`unknown symbol id "${args.id}"`)
      }
      const maxDepth = Math.max(1, args.depth ?? DEFAULT_DEPTH)
      const visited = new Map<string, number>()
      visited.set(args.id, 0)
      let frontier = new Set<string>([args.id])
      const allEdges = store.allEdges()
      for (let hop = 1; hop <= maxDepth && frontier.size > 0; hop++) {
        const next = new Set<string>()
        for (const e of allEdges) {
          if (!REVERSE_TYPES.includes(e.type)) continue
          if (!frontier.has(e.dst_id)) continue
          if (visited.has(e.src_id)) continue
          visited.set(e.src_id, hop)
          next.add(e.src_id)
        }
        frontier = next
      }
      visited.delete(args.id)
      const callers: ImpactCaller[] = [...visited.entries()]
        .map(([id, distance]) => ({ symbol: store.getSymbol(id), distance }))
        .sort((a, b) => {
          if (a.distance !== b.distance) return a.distance - b.distance
          const ai = a.symbol?.id ?? ''
          const bi = b.symbol?.id ?? ''
          return ai < bi ? -1 : ai > bi ? 1 : 0
        })
      return jsonResult({ ok: true, callers, count: callers.length })
    } catch (err) {
      return errorResult(`symbol_impact failed: ${(err as Error).message}`)
    } finally {
      try {
        store.close()
      } catch {
        // ignore
      }
    }
  }
}
