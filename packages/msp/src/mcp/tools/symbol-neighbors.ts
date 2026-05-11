import { resolve } from 'node:path'

import { z } from 'zod'

import { SymbolStore } from '../../symbols/store/sqlite.js'
import type { EdgeType } from '../../symbols/types.js'
import { dbPath, graphExists } from '../../symbols/util.js'
import { errorResult, jsonResult, type ToolHandlerCtx, type ToolTextResult } from '../types.js'

export const name = 'msp_symbol_neighbors'

export const description =
  'k-hop BFS over the Symbol Graph from a given symbol id (depth capped at 3). Returns the center symbol plus reached nodes and traversed edges. Read-only.'

export const inputSchema = {
  id: z.string().describe('Center symbol id (e.g. `src/foo.ts:bar:func`).'),
  depth: z
    .number()
    .int()
    .positive()
    .optional()
    .describe('BFS depth (default 1, max 3).'),
  edge_types: z
    .array(z.string())
    .optional()
    .describe('Filter to a subset of edge types: defines | imports | calls | extends | implements | references.'),
  root: z.string().optional().describe('Project root (default: server context root).'),
  namespace: z.string().optional().describe('Project namespace (default: evaAI).'),
}

interface NeighborsArgs {
  id: string
  depth?: number
  edge_types?: string[]
  root?: string
  namespace?: string
}

const KNOWN_EDGE_TYPES: EdgeType[] = [
  'defines',
  'imports',
  'calls',
  'extends',
  'implements',
  'references',
]

const MAX_DEPTH = 3

export function handler(ctx: ToolHandlerCtx) {
  return async (args: NeighborsArgs): Promise<ToolTextResult> => {
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
      const center = store.getSymbol(args.id)
      if (!center) {
        return errorResult(`unknown symbol id "${args.id}"`)
      }
      const requestedDepth = Math.max(1, Math.min(MAX_DEPTH, args.depth ?? 1))
      let edgeTypes: EdgeType[] | undefined
      if (args.edge_types && args.edge_types.length > 0) {
        const valid: EdgeType[] = []
        for (const t of args.edge_types) {
          if (KNOWN_EDGE_TYPES.includes(t as EdgeType)) valid.push(t as EdgeType)
        }
        edgeTypes = valid
      }
      const { nodes, edges } = store.getNeighbors(args.id, requestedDepth, edgeTypes)
      return jsonResult({ ok: true, center, nodes, edges })
    } catch (err) {
      return errorResult(`symbol_neighbors failed: ${(err as Error).message}`)
    } finally {
      try {
        store.close()
      } catch {
        // ignore
      }
    }
  }
}
