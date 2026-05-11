/**
 * MSP identity layer — public API surface.
 *
 * The identity layer is the "soul" half of the MSP passport (per
 * FRAME--MSP-ARCHITECTURE-V2 + msp_spec.md §7e). It carries three sub-fields
 * across sessions:
 *
 *   - profile      stable identifying facts (name, role, tier, originStory)
 *   - voice        how the agent communicates (tone, formality, language, cadence)
 *   - preferences  per-key user/runtime overrides with optional TTL
 *
 * Storage: single JSON file at `.brain/msp/projects/<namespace>/identity.json`
 * per ADR--IDENTITY-STORAGE-SHAPE. Atomic write via temp + rename. Schema is
 * versioned (`schemaVersion: 1` is current).
 *
 * @example
 * ```ts
 * import { getIdentity, setProfile, setVoice, setPreference } from '@/identity'
 *
 * const id = await getIdentity({ root: process.cwd(), namespace: 'evaAI' })
 * await setProfile({ namespace: 'evaAI' }, { name: 'EVA', role: 'research' })
 * await setVoice({ namespace: 'evaAI' }, {
 *   tone: ['analytical'],
 *   formality: 'neutral',
 *   languagePreference: 'en',
 *   responseCadence: 'normal',
 * })
 * await setPreference({ namespace: 'evaAI' }, 'top_k', 5)
 * ```
 */

import { readIdentity } from './store.js'
import type { Identity, IdentityOptions } from './types.js'

// Public types
export type {
  Identity,
  IdentityOptions,
  Profile,
  Voice,
  Preference,
  Tier,
  Formality,
  ResponseCadence,
} from './types.js'

// Default constructors (useful for tests + advanced callers)
export {
  defaultIdentity,
  defaultProfile,
  defaultVoice,
  DEFAULT_NAMESPACE,
  CURRENT_SCHEMA_VERSION,
} from './types.js'

// Sub-module surfaces
export { setProfile } from './profile.js'
export { setVoice } from './voice.js'
export {
  setPreference,
  getPreference,
  prunePreferences,
} from './preferences.js'
export type { PreferenceTtl } from './preferences.js'

// Low-level store (exposed for advanced callers / tests)
export {
  identityPath,
  projectOverridePath,
  readIdentity,
  resolveOptions,
  writeIdentity,
} from './store.js'
export type { IdentityScope, ReadIdentityOptions } from './store.js'

// Migration (exposed primarily for tests + diagnostics)
export { migrateIfNeeded } from './migrate.js'
export type { MigrationResult } from './migrate.js'

/**
 * Convenience wrapper around `readIdentity`. Returns the full identity for a
 * namespace, default-constructed if no file exists yet (does NOT create the
 * file).
 */
export async function getIdentity(
  opts?: IdentityOptions,
): Promise<Identity> {
  return readIdentity(opts)
}
