import { resolve } from 'node:path'

import { z } from 'zod'

import { createObsidianClient } from '../../obsidian/client.js'
import type { ObsidianClient } from '../../obsidian/types.js'
import { recall } from '../../orchestrator/retrieval/index.js'
import type { SourceName } from '../../orchestrator/retrieval/types.js'
import { makeContext, makeSubject } from '../../policy/types.js'
import { errorResult, jsonResult, type ToolHandlerCtx, type ToolTextResult } from '../types.js'

export const name = 'msp_recall'

export const description =
  'Run MSP retrieval: fan out across GKS vector, Obsidian text, episodic memory, and backlinks; fuse via RRF; return ranked hits with provenance + per-source timings. Read-only.'

export const inputSchema = {
  query: z.string().describe('Free-text query.'),
  top_k: z.number().int().positive().optional().describe('Top-K (default 10).'),
  namespace: z.string().optional().describe('Project namespace (default `evaAI`).'),
  weights: z
    .record(z.string(), z.number())
    .optional()
    .describe(
      'Per-source RRF weight overrides keyed by source name (gks-vector, obsidian-text, episodic, backlinks).',
    ),
  timeout_ms: z
    .number()
    .int()
    .positive()
    .optional()
    .describe('Total budget across all sources in ms (default 1500).'),
  root: z.string().optional().describe('Project root (default: server context root).'),
}

interface RecallToolArgs {
  query: string
  top_k?: number
  namespace?: string
  weights?: Record<string, number>
  timeout_ms?: number
  root?: string
}

/**
 * Best-effort construction of an `ObsidianClient`. Returns `undefined` on
 * any failure (env not configured, network probe failed, exception). The
 * recall orchestrator records `obsidian-text: skipped` in `fallback_reasons`
 * when the client is absent, which is the desired behaviour here.
 */
async function tryCreateObsidianClient(opts: { root: string }): Promise<ObsidianClient | undefined> {
  try {
    return await createObsidianClient({ root: opts.root })
  } catch {
    return undefined
  }
}

export function handler(ctx: ToolHandlerCtx) {
  return async (args: RecallToolArgs): Promise<ToolTextResult> => {
    const root = resolve(args.root ?? ctx.root)
    try {
      const subject = makeSubject('mcp-client', 'default-mcp')
      const context = makeContext('mcp-stdio', `mcp-${Date.now()}`)

      const obsidian = await tryCreateObsidianClient({ root })
      const result = await recall({
        query: args.query,
        root,
        namespace: args.namespace,
        obsidian,
        topK: args.top_k,
        timeoutMs: args.timeout_ms,
        weights: args.weights as Partial<Record<SourceName, number>> | undefined,
        subject,
        context,
      })
      return jsonResult({ ok: true, ...result })
    } catch (err) {
      return errorResult(`recall failed: ${(err as Error).message}`)
    }
  }
}
