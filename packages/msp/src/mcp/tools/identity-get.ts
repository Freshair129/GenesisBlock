import { existsSync } from 'node:fs'
import { resolve } from 'node:path'

import { z } from 'zod'

import {
  getIdentity,
  prunePreferences,
  readIdentity,
  projectOverridePath,
} from '../../identity/index.js'
import { globalIdentityPath } from '../../lib/msp-home.js'
import { errorResult, jsonResult, type ToolHandlerCtx, type ToolTextResult } from '../types.js'

export const name = 'msp_identity_get'

export const description =
  'Read the passport identity (profile + voice + preferences). Default `view=merged` returns the global identity shallow-merged with the per-project override (per ADR--GLOBAL-VS-WORKSPACE). `view=global` reads only ~/.msp/identity.json; `view=project` reads only the workspace override. Set `explain=true` to also return the resolution chain (which file each layer came from).'

export const inputSchema = {
  namespace: z
    .string()
    .optional()
    .describe('Project namespace (default `evaAI`).'),
  prune: z
    .boolean()
    .optional()
    .describe('If true, eagerly remove expired preferences before reading (writes to disk only if at least one entry was pruned).'),
  view: z
    .enum(['merged', 'global', 'project'])
    .optional()
    .describe('Which layer to read. Default `merged` (global + project override).'),
  explain: z
    .boolean()
    .optional()
    .describe('If true, return the resolution chain alongside the identity (which paths were read).'),
  root: z.string().optional().describe('Project root (default: server context root).'),
}

interface IdentityGetArgs {
  namespace?: string
  prune?: boolean
  view?: 'merged' | 'global' | 'project'
  explain?: boolean
  root?: string
}

export function handler(ctx: ToolHandlerCtx) {
  return async (args: IdentityGetArgs): Promise<ToolTextResult> => {
    const root = resolve(args.root ?? ctx.root)
    try {
      if (args.prune === true) {
        await prunePreferences({ root, namespace: args.namespace })
      }
      const identity = await getIdentity({
        root,
        namespace: args.namespace,
        view: args.view,
      })

      if (args.explain === true) {
        const globalPath = globalIdentityPath()
        const overridePath = projectOverridePath(
          root,
          args.namespace ?? 'evaAI',
        )
        const explainBlock = {
          view: args.view ?? 'merged',
          global: {
            path: globalPath,
            present: existsSync(globalPath),
          },
          project: {
            path: overridePath,
            present: existsSync(overridePath),
            namespace: args.namespace ?? 'evaAI',
            workspace: root,
          },
        }
        return jsonResult({ ok: true, identity, explain: explainBlock })
      }

      // Touch readIdentity to ensure migration is invoked even when prune is false
      // (getIdentity already does this, but readIdentity is a more obvious surface).
      void readIdentity
      return jsonResult({ ok: true, identity })
    } catch (err) {
      return errorResult(`identity_get failed: ${(err as Error).message}`)
    }
  }
}
