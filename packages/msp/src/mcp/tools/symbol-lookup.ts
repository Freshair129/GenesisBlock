import { resolve } from 'node:path'

import { z } from 'zod'

import { SymbolStore } from '../../symbols/store/sqlite.js'
import type { Symbol, SymbolKind } from '../../symbols/types.js'
import { dbPath, graphExists } from '../../symbols/util.js'
import { makeContext, makeSubject } from '../../policy/types.js'
import { errorResult, jsonResult, type ToolHandlerCtx, type ToolTextResult } from '../types.js'

export const name = 'msp_symbol_lookup'

export const description =
  'Look up Symbol Graph nodes by exact or prefix name match. Read-only. Returns hits ranked by exported-first then file depth. Requires `msp:graph build` to have run.'

export const inputSchema = {
  name: z.string().describe('Symbol name to look up (exact-match preferred, prefix fallback).'),
  kind: z
    .string()
    .optional()
    .describe('Optional filter: function | method | class | interface | type | enum | const | module.'),
  root: z.string().optional().describe('Project root (default: server context root).'),
  namespace: z.string().optional().describe('Project namespace (default: evaAI).'),
}

interface LookupArgs {
  name: string
  kind?: string
  root?: string
  namespace?: string
}

const KNOWN_KINDS: SymbolKind[] = [
  'function',
  'method',
  'class',
  'interface',
  'type',
  'enum',
  'const',
  'module',
]

function fileDepth(file: string): number {
  return file.split('/').length
}

function rank(symbols: Symbol[]): Symbol[] {
  return [...symbols].sort((a, b) => {
    if (a.exported !== b.exported) return a.exported ? -1 : 1
    const da = fileDepth(a.file)
    const db = fileDepth(b.file)
    if (da !== db) return da - db
    if (a.file !== b.file) return a.file < b.file ? -1 : 1
    return a.id < b.id ? -1 : a.id > b.id ? 1 : 0
  })
}

export function handler(ctx: ToolHandlerCtx) {
  return async (args: LookupArgs): Promise<ToolTextResult> => {
    const root = resolve(args.root ?? ctx.root)
    const namespace = args.namespace
    if (!graphExists(root, namespace)) {
      return errorResult("graph not built — run 'npm run msp:graph build' first")
    }
    const store = new SymbolStore()
    try {
      const subject = ctx.subject ?? makeSubject('mcp-client', 'default-mcp')
      const context = ctx.policyContext ?? makeContext('mcp-stdio', `mcp-${Date.now()}`)

      store.open(dbPath(root, namespace), { subject, context })
      if (!store.getMeta('last_built_at')) {
        return errorResult("graph not built — run 'npm run msp:graph build' first")
      }
      if (args.kind && !KNOWN_KINDS.includes(args.kind as SymbolKind)) {
        return errorResult(`unknown kind "${args.kind}"`)
      }
      const all = store.allSymbols()
      let exact = all.filter((s) => s.name === args.name)
      let prefix = all.filter((s) => s.name !== args.name && s.name.startsWith(args.name))
      if (args.kind) {
        exact = exact.filter((s) => s.kind === args.kind)
        prefix = prefix.filter((s) => s.kind === args.kind)
      }
      const hits = [...rank(exact), ...rank(prefix)]
      return jsonResult({ ok: true, hits })
    } catch (err) {
      return errorResult(`symbol_lookup failed: ${(err as Error).message}`)
    } finally {
      try {
        store.close()
      } catch {
        // ignore close errors
      }
    }
  }
}
