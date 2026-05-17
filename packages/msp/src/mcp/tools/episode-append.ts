import { resolve } from 'node:path'

import { z } from 'zod'

import { appendEpisode } from '../../memory/episodic/writer.js'
import type { Episode } from '../../memory/episodic/types.js'
import { makeContext, makeSubject } from '../../policy/types.js'
import { errorResult, jsonResult, type ToolHandlerCtx, type ToolTextResult } from '../types.js'

export const name = 'msp_episode_append'

export const description =
  'Append (or overwrite by episodicId) an episode in episodic_memory.json. Idempotent on episodicId.'

export const inputSchema = {
  episode: z
    .object({
      episodicId: z.string(),
      sessionId: z.string(),
      projectId: z.string(),
      timestamp: z.string().optional(),
      importance_score: z.number().min(0).max(1),
      range: z.array(z.string()).min(1),
      anchor: z.object({ content: z.string(), msgId: z.string() }).optional(),
      context: z
        .object({
          topic: z.string().optional(),
          participants: z.array(z.string()).optional(),
          mood: z.string().optional(),
        })
        .optional(),
      content: z.object({
        summary: z.string(),
        key_decisions: z.array(z.string()).optional(),
        unresolved_questions: z.array(z.string()).optional(),
      }),
      tags: z.array(z.string()).optional(),
      associations: z
        .object({
          related_event_ids: z.array(z.string()).optional(),
          entity_links: z.array(z.string()).optional(),
          knowledgeId: z.string().optional(),
          learnId: z.string().optional(),
        })
        .optional(),
    })
    .describe('A complete Episode object.'),
  namespace: z.string().optional().describe('Project namespace (default `evaAI`).'),
  root: z.string().optional().describe('Project root (default: server context root).'),
}

export function handler(ctx: ToolHandlerCtx) {
  return async (args: {
    episode: Episode
    namespace?: string
    root?: string
  }): Promise<ToolTextResult> => {
    const root = resolve(args.root ?? ctx.root)
    try {
      const subject = ctx.subject ?? makeSubject('mcp-client', 'default-mcp')
      const context = ctx.policyContext ?? makeContext('mcp-stdio', `mcp-${Date.now()}`)

      await appendEpisode(args.episode, { root, namespace: args.namespace, subject, context })
      return jsonResult({ ok: true, episodicId: args.episode.episodicId })
    } catch (err) {
      return errorResult(`episode append failed: ${(err as Error).message}`)
    }
  }
}
