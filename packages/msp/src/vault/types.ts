import { z } from 'zod'
import { type Namespace } from '@freshair129/gks'

export const NamespaceSchema = z.object({
  tenant_id: z.string().optional(),
  user_id: z.string().optional(),
  session_id: z.string().optional(),
  agent_id: z.string().optional(),
})

/**
 * A Vault is a logical view over one or more GKS Namespaces.
 * See FEAT--VAULT-COMPOSITION.
 */
export const VaultSchema = z.object({
  id: z.string(),
  description: z.string().optional(),
  /** Namespaces this vault reads from (OR-union). */
  read_from: z.array(NamespaceSchema).min(1),
  /** Single namespace this vault writes to. If absent, vault is read-only. */
  write_to: NamespaceSchema.optional(),
  /** Default resolution policy for this vault. */
  resolution: z
    .object({
      default_tier: z.enum(['FULL', 'MENTION', 'SKELETON', 'SUMMARY']).default('MENTION'),
      expand_limit: z.number().int().nonnegative().default(5),
    })
    .default({}),
})

export type Vault = z.infer<typeof VaultSchema>

export interface VaultRegistry {
  version: number
  vaults: Map<string, Vault>
}
