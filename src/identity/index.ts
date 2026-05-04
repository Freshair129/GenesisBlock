import {
  DEFAULT_NAMESPACE,
  emptyProfile,
  loadProfile,
  mergeProfile,
  saveProfile,
  type IdentityProfile,
} from './profile.js'
import {
  isExpired,
  makeEntry,
  type PrefValue,
  type PreferenceEntry,
} from './preferences.js'
import { mergeVoice, type Voice } from './voice.js'

export type { IdentityProfile } from './profile.js'
export type { PrefValue, PreferenceEntry } from './preferences.js'
export type { Voice } from './voice.js'
export { IdentityParseError, identityFilePath } from './profile.js'

export interface IdentityOpts {
  /** Project root. Defaults to `process.env.MSP_ROOT ?? process.cwd()`. */
  root?: string
}

function resolveRoot(opts?: IdentityOpts): string {
  return opts?.root ?? process.env.MSP_ROOT ?? process.cwd()
}

/**
 * Load the agent's identity for a namespace. Returns an empty profile when
 * no `identity.yaml` exists yet — callers can treat the result as
 * always-present and rely on `setProfile` / `setVoice` / `setPreference` to
 * materialise the file on first write.
 */
export async function getIdentity(
  namespace: string = DEFAULT_NAMESPACE,
  opts?: IdentityOpts,
): Promise<IdentityProfile> {
  const root = resolveRoot(opts)
  const profile = await loadProfile(root, namespace)
  return profile ?? emptyProfile()
}

/**
 * Merge a partial profile onto whatever's on disk. Pass only the fields you
 * want to change; everything else is preserved. Voice and preferences merge
 * field-by-field — see `setVoice` / `setPreference` for finer-grained edits.
 */
export async function setProfile(
  namespace: string,
  patch: Partial<IdentityProfile>,
  opts?: IdentityOpts,
): Promise<IdentityProfile> {
  const root = resolveRoot(opts)
  const prior = (await loadProfile(root, namespace)) ?? emptyProfile()
  const next = mergeProfile(prior, patch)
  await saveProfile(root, namespace, next)
  return next
}

export async function setVoice(
  namespace: string,
  voice: Partial<Voice>,
  opts?: IdentityOpts,
): Promise<IdentityProfile> {
  const root = resolveRoot(opts)
  const prior = (await loadProfile(root, namespace)) ?? emptyProfile()
  const merged = mergeVoice(prior.voice, voice)
  const next: IdentityProfile = { ...prior, voice: merged }
  await saveProfile(root, namespace, next)
  return next
}

/**
 * Returns the preference value, or undefined when missing or expired.
 * Expired entries are pruned from disk on read so the next caller sees a
 * clean view.
 */
export async function getPreference(
  namespace: string,
  key: string,
  opts?: IdentityOpts,
): Promise<PrefValue | undefined> {
  const root = resolveRoot(opts)
  const prior = await loadProfile(root, namespace)
  if (!prior?.preferences) return undefined
  const entry: PreferenceEntry | undefined = prior.preferences[key]
  if (!entry) return undefined
  if (isExpired(entry)) {
    const { [key]: _removed, ...rest } = prior.preferences
    void _removed
    await saveProfile(root, namespace, { ...prior, preferences: rest })
    return undefined
  }
  return entry.value
}

/**
 * Set or replace a preference. `ttlMs` is milliseconds from now until
 * expiry — omit or pass null for an entry that never expires.
 */
export async function setPreference(
  namespace: string,
  key: string,
  value: PrefValue,
  ttlMs?: number | null,
  opts?: IdentityOpts,
): Promise<PreferenceEntry> {
  const root = resolveRoot(opts)
  const prior = (await loadProfile(root, namespace)) ?? emptyProfile()
  const entry = makeEntry(value, ttlMs)
  const preferences = { ...(prior.preferences ?? {}), [key]: entry }
  await saveProfile(root, namespace, { ...prior, preferences })
  return entry
}
