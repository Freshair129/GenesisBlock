import { resolve } from 'node:path'
import { z } from 'zod'
import { SymbolStore } from '../../symbols/store/sqlite.js'
import { dbPath, graphExists } from '../../symbols/util.js'
import { SymbolTracer } from '../../symbols/tracer/tracer.js'
import { makeContext, makeSubject } from '../../policy/types.js'
import { errorResult, jsonResult, type ToolHandlerCtx, type ToolTextResult } from '../types.js'

export const name = 'msp_symbol_trace'

export const description =
  'Trace execution flow (downward to callees or upward to callers) from a given symbol. Returns multiple paths with depth and cycle info. Read-only.'

export const inputSchema = {
  id: z.string().min(1).describe('Source symbol ID to start tracing from.'),
  direction: z.enum(['down', 'up']).optional().describe('Direction of trace: "down" (callees) or "up" (callers). Default "down".'),
  maxDepth: z.number().int().positive().optional().describe('Max hops to follow (default 8).'),
  root: z.string().optional().describe('Project root (default: server context root).'),
  namespace: z.string().optional().describe('Project namespace (default: evaAI).'),
}

interface TraceArgs {
  id: string
  direction?: 'down' | 'up'
  maxDepth?: number
  root?: string
  namespace?: string
}

export function handler(ctx: ToolHandlerCtx) {
  return async (args: TraceArgs): Promise<ToolTextResult> => {
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
      
      const tracer = new SymbolTracer(store)
      const paths = await tracer.trace(args.id, {
        direction: args.direction ?? 'down',
        maxDepth: args.maxDepth ?? 8
      })

      return jsonResult({ ok: true, paths })
    } catch (err) {
      return errorResult(`symbol_trace failed: ${(err as Error).message}`)
    } finally {
      try {
        store.close()
      } catch {
        // ignore
      }
    }
  }
}

