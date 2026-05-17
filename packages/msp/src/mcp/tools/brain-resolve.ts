import { z } from 'zod'

import { resolve as resolveQuery } from '../../brain/resolver.js'
import type { AtomType, BrainQuery } from '../../brain/types.js'
import { makeContext, makeSubject } from '../../policy/types.js'
import { errorResult, jsonResult, type ToolHandlerCtx, type ToolTextResult } from '../types.js'

export const name = 'msp_brain_resolve'

export const description =
  'Resolve an atom query against the Two-Brain (project + global) vaults using the deterministic per-type routing table from BLUEPRINT--BRAIN-MERGE-STRATEGY. Returns deduped hits with each atom\'s id, type, source (project|global), and absolute path. Use this instead of hardcoding atom paths.'

const ATOM_TYPES: readonly AtomType[] = [
  'ADR',
  'FEAT',
  'BLUEPRINT',
  'AUDIT',
  'CONCEPT',
  'FRAMEWORK',
  'SPEC',
  'PROTOCOL',
  'SKILL',
  'ALGO',
  'PROTO',
  'PARAMS',
  'EPISODE',
  'IDENTITY',
  'REGISTRY',
  'MOD',
  'MASTER',
  'HOTFIX',
  'INC',
  'ISSUE',
] as const

const ATOM_TYPE_SET = new Set<string>(ATOM_TYPES)

export const inputSchema = {
  id: z
    .string()
    .optional()
    .describe('Optional exact atom id to filter for (e.g. "ADR--MONOREPO-STRUCTURE").'),
  type: z
    .string()
    .optional()
    .describe(
      'Optional AtomType prefix to scope the search (e.g. "ADR", "BLUEPRINT"). Must be one of the known taxonomy v2.3 types.',
    ),
  vault_id: z
    .string()
    .optional()
    .describe('Optional vault identifier (reserved; passed through to the resolver).'),
}

interface BrainResolveArgs {
  id?: string
  type?: string
  vault_id?: string
}

export function handler(ctx: ToolHandlerCtx) {
  return async (args: BrainResolveArgs): Promise<ToolTextResult> => {
    if (args.type !== undefined && !ATOM_TYPE_SET.has(args.type)) {
      return errorResult(
        `invalid AtomType: "${args.type}". Must be one of: ${ATOM_TYPES.join(', ')}`,
      )
    }

    const query: BrainQuery = {}
    if (args.id !== undefined) query.id = args.id
    if (args.type !== undefined) query.type = args.type as AtomType
    if (args.vault_id !== undefined) query.vault_id = args.vault_id

    try {
      const subject = ctx.subject ?? makeSubject('mcp-client', 'default-mcp')
      const context = ctx.policyContext ?? makeContext('mcp-stdio', `mcp-${Date.now()}`)

      const hits = await resolveQuery(query, { subject, context })
      return jsonResult({
        ok: true,
        count: hits.length,
        hits: hits.map((h) => ({
          id: h.atom.id,
          type: h.atom.type,
          source: h.source,
          path: h.path,
        })),
      })
    } catch (err) {
      return errorResult(`brain_resolve failed: ${(err as Error).message}`)
    }
  }
}
