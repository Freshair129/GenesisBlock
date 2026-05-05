import { resolve } from 'node:path'

import { z } from 'zod'

import { getIdentity, prunePreferences } from '../../identity/index.js'
import { errorResult, jsonResult, type ToolHandlerCtx, type ToolTextResult } from '../types.js'

export const name = 'msp_identity_get'

export const description =
  'Read the full passport identity (profile + voice + preferences) for a namespace. Defaults to `evaAI`. Optionally prune expired preferences before reading.'

export const inputSchema = {
  namespace: z
    .string()
    .optional()
    .describe('Project namespace (default `evaAI`).'),
  prune: z
    .boolean()
    .optional()
    .describe('If true, eagerly remove expired preferences before reading (writes to disk only if at least one entry was pruned).'),
  root: z.string().optional().describe('Project root (default: server context root).'),
}

interface IdentityGetArgs {
  namespace?: string
  prune?: boolean
  root?: string
}

export function handler(ctx: ToolHandlerCtx) {
  return async (args: IdentityGetArgs): Promise<ToolTextResult> => {
    const root = resolve(args.root ?? ctx.root)
    try {
      if (args.prune === true) {
        await prunePreferences({ root, namespace: args.namespace })
      }
      const identity = await getIdentity({ root, namespace: args.namespace })
      return jsonResult({ ok: true, identity })
    } catch (err) {
      return errorResult(`identity_get failed: ${(err as Error).message}`)
    }
  }
}
