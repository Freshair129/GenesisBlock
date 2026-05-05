import { resolve } from 'node:path'

import { z } from 'zod'

import {
  getIdentity,
  setPreference,
  setProfile,
  setVoice,
} from '../../identity/index.js'
import type { Profile, Voice } from '../../identity/types.js'
import { errorResult, jsonResult, type ToolHandlerCtx, type ToolTextResult } from '../types.js'

export const name = 'msp_identity_set'

export const description =
  'Mutate the passport identity. Discriminated by `kind`: `profile` partial-merges Profile fields (createdAt is set-once), `voice` full-replaces the Voice sub-field, `preference` sets a single preference key with optional TTL. Returns the full identity post-write.'

const profileShape = z.object({
  name: z.string().optional(),
  role: z.string().optional(),
  tier: z.enum(['T1', 'T2', 'T3']).optional(),
  originStory: z.string().optional(),
  guardrails: z.array(z.string()).optional(),
  extensions: z.record(z.string(), z.unknown()).optional(),
})

const voiceShape = z.object({
  tone: z.array(z.string()),
  formality: z.enum(['casual', 'neutral', 'formal']),
  languagePreference: z.string(),
  responseCadence: z.enum(['terse', 'normal', 'verbose']),
})

export const inputSchema = {
  kind: z
    .enum(['profile', 'voice', 'preference'])
    .describe('Which sub-field to mutate.'),
  partial: profileShape
    .optional()
    .describe('Partial profile fields (only for kind=profile).'),
  voice: voiceShape
    .optional()
    .describe('Full voice replacement (only for kind=voice).'),
  key: z
    .string()
    .optional()
    .describe('Preference key (only for kind=preference).'),
  value: z
    .unknown()
    .optional()
    .describe('Preference value, JSON-serialisable (only for kind=preference).'),
  expires_at: z
    .string()
    .optional()
    .describe('Preference TTL: absolute ISO 8601 expiry. Wins over expires_in_ms.'),
  expires_in_ms: z
    .number()
    .optional()
    .describe('Preference TTL: relative ms from now.'),
  namespace: z
    .string()
    .optional()
    .describe('Project namespace (default `evaAI`).'),
  root: z.string().optional().describe('Project root (default: server context root).'),
}

type ProfilePartial = Partial<
  Pick<Profile, 'name' | 'role' | 'tier' | 'originStory' | 'guardrails' | 'extensions'>
>

interface IdentitySetArgs {
  kind: 'profile' | 'voice' | 'preference'
  partial?: ProfilePartial
  voice?: Voice
  key?: string
  value?: unknown
  expires_at?: string
  expires_in_ms?: number
  namespace?: string
  root?: string
}

export function handler(ctx: ToolHandlerCtx) {
  return async (args: IdentitySetArgs): Promise<ToolTextResult> => {
    const root = resolve(args.root ?? ctx.root)
    const opts = { root, namespace: args.namespace }
    try {
      switch (args.kind) {
        case 'profile': {
          if (!args.partial) {
            return errorResult('kind=profile requires `partial` field')
          }
          await setProfile(opts, args.partial)
          break
        }
        case 'voice': {
          if (!args.voice) {
            return errorResult('kind=voice requires `voice` field')
          }
          await setVoice(opts, args.voice)
          break
        }
        case 'preference': {
          if (typeof args.key !== 'string' || args.key.length === 0) {
            return errorResult('kind=preference requires non-empty `key`')
          }
          if (!('value' in args)) {
            return errorResult('kind=preference requires `value` field')
          }
          const ttl =
            args.expires_at !== undefined || args.expires_in_ms !== undefined
              ? {
                  expiresAt: args.expires_at,
                  expiresInMs: args.expires_in_ms,
                }
              : undefined
          await setPreference(opts, args.key, args.value, ttl)
          break
        }
        default: {
          // Defensive — Zod enum should reject this earlier.
          return errorResult(
            `unknown kind: ${(args as { kind: string }).kind}`,
          )
        }
      }

      const identity = await getIdentity(opts)
      return jsonResult({ ok: true, identity })
    } catch (err) {
      return errorResult(`identity_set failed: ${(err as Error).message}`)
    }
  }
}
