import { resolve } from 'node:path'

import { z } from 'zod'

import { openSession } from '../../memory/sessions/writer.js'
import { errorResult, jsonResult, type ToolHandlerCtx, type ToolTextResult } from '../types.js'

export const name = 'msp_session_append'

export const description =
  'Append a single turn to a session JSONL log. Opens, writes, and closes the lock per call (stateless).'

export const inputSchema = {
  episodic_id: z.string().describe('Episodic id (filename of the .jsonl).'),
  turn: z
    .object({
      sessionId: z.string(),
      episodicId: z.string(),
      turnId: z.number(),
      msgId: z.string(),
      speakerId: z.string(),
      content: z.string(),
      learnId: z.string().optional(),
    })
    .describe('Single SessionTurn row.'),
  namespace: z.string().optional().describe('Project namespace (default `evaAI`).'),
  root: z.string().optional().describe('Project root (default: server context root).'),
}

export function handler(ctx: ToolHandlerCtx) {
  return async (args: {
    episodic_id: string
    turn: {
      sessionId: string
      episodicId: string
      turnId: number
      msgId: string
      speakerId: string
      content: string
      learnId?: string
    }
    namespace?: string
    root?: string
  }): Promise<ToolTextResult> => {
    const root = resolve(args.root ?? ctx.root)
    let session
    try {
      session = await openSession({
        root,
        episodicId: args.episodic_id,
        namespace: args.namespace,
      })
      await session.appendTurn(args.turn)
      await session.close()
      return jsonResult({ ok: true })
    } catch (err) {
      try {
        await session?.close()
      } catch {
        // ignore
      }
      return errorResult(`session append failed: ${(err as Error).message}`)
    }
  }
}
