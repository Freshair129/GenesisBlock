import { resolve } from 'node:path'

import { z } from 'zod'

import { SymbolStore } from '../../symbols/store/sqlite.js'
import { dbPath, graphExists } from '../../symbols/util.js'
import { makeContext, makeSubject } from '../../policy/types.js'
import { errorResult, jsonResult, type ToolHandlerCtx, type ToolTextResult } from '../types.js'

export const name = 'msp_symbol_community'

export const description =
  'Resolve a symbol id to its Leiden community and return the community metadata + member list. Read-only.'

export const inputSchema = {
  id: z.string().describe('Symbol id whose community to fetch.'),
  root: z.string().optional().describe('Project root (default: server context root).'),
  namespace: z.string().optional().describe('Project namespace (default: evaAI).'),
}

interface CommunityArgs {
  id: string
  root?: string
  namespace?: string
}

export function handler(ctx: ToolHandlerCtx) {
  return async (args: CommunityArgs): Promise<ToolTextResult> => {
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
      const sym = store.getSymbol(args.id)
      if (!sym) {
        return errorResult(`unknown symbol id "${args.id}"`)
      }
      if (sym.community_id === null) {
        return errorResult(`symbol "${args.id}" has no community — graph may be stale`)
      }
      const allCommunities = store.allCommunities()
      const community = allCommunities.find((c) => c.id === sym.community_id)
      if (!community) {
        return errorResult(`community ${sym.community_id} not found in store`)
      }
      const members = store.getCommunityMembers(community.id)
      return jsonResult({
        ok: true,
        community: {
          id: community.id,
          label: community.label,
          size: community.size,
          modularity: community.modularity,
        },
        members,
      })
    } catch (err) {
      return errorResult(`symbol_community failed: ${(err as Error).message}`)
    } finally {
      try {
        store.close()
      } catch {
        // ignore
      }
    }
  }
}
