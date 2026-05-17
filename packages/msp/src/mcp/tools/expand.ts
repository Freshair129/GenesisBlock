import { z } from 'zod'
import { createCognitiveLayer } from '../../cognitive/index.js'
import { makeContext, makeSubject } from '../../policy/types.js'
import { jsonResult, type ToolHandlerCtx, type ToolTextResult } from '../types.js'

export const name = 'msp_expand'

export const description =
  'Expand an atom from MENTION to FULL resolution (UCF Phase 3). Returns the full body if permitted by policy.'

export const inputSchema = {
  id: z.string().describe('Atom ID to expand.'),
  to: z.enum(['FULL', 'SUMMARY', 'SKELETON']).optional().default('FULL').describe('Target resolution tier.'),
}

export function handler(ctx: ToolHandlerCtx) {
  return async (args: { id: string; to?: any }): Promise<ToolTextResult> => {
    try {
      const subject = ctx.subject ?? makeSubject('mcp-client', 'default-mcp')
      const context = ctx.policyContext ?? makeContext('mcp-stdio', `mcp-${Date.now()}`)

      const layer = await createCognitiveLayer({ root: ctx.root })
      const result = await layer.expand({
        id: args.id,
        to: args.to,
      }, { subject, context })
      return jsonResult(result)
    } catch (err) {
      return {
        content: [{ type: 'text', text: `expand failed: ${(err as Error).message}` }],
        isError: true,
      }
    }
  }
}
