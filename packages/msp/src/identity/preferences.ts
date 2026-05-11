import { readIdentity, writeIdentity } from './store.js'
import type { IdentityOptions, Preference } from './types.js'

/**
 * TTL options for `setPreference`. Either `expiresAt` (absolute ISO 8601
 * timestamp) or `expiresInMs` (relative milliseconds from now). If both are
 * supplied, `expiresAt` wins (more specific). If neither, the entry never
 * expires.
 */
export interface PreferenceTtl {
  expiresAt?: string
  expiresInMs?: number
}

function computeExpiresAt(
  ttl: PreferenceTtl | undefined,
  now: () => Date,
): string | null {
  if (!ttl) return null
  if (typeof ttl.expiresAt === 'string') return ttl.expiresAt
  if (typeof ttl.expiresInMs === 'number') {
    return new Date(now().getTime() + ttl.expiresInMs).toISOString()
  }
  return null
}

function isExpired(expiresAt: string | null, now: () => Date): boolean {
  if (!expiresAt) return false
  const t = Date.parse(expiresAt)
  if (!Number.isFinite(t)) return false
  return t <= now().getTime()
}

/**
 * Set or replace a single preference, optionally with a TTL.
 *
 * - `expiresAt` (ISO string) takes precedence over `expiresInMs`.
 * - With neither, the entry never expires.
 * - Replaces any existing entry for the same key.
 */
export async function setPreference(
  opts: IdentityOptions | undefined,
  key: string,
  value: unknown,
  ttl?: PreferenceTtl,
  now: () => Date = () => new Date(),
): Promise<void> {
  const identity = await readIdentity(opts)
  const expiresAt = computeExpiresAt(ttl, now)
  identity.preferences[key] = { value, expiresAt }
  await writeIdentity(opts, identity)
}

/**
 * Read a preference value with **lazy expiry**: an expired entry returns null
 * but is NOT removed from disk. Caller can run `prunePreferences` to do eager
 * cleanup.
 *
 * Missing key → null. Expired entry → null. Otherwise → `value`.
 */
export async function getPreference(
  opts: IdentityOptions | undefined,
  key: string,
  now: () => Date = () => new Date(),
): Promise<unknown | null> {
  const identity = await readIdentity(opts)
  const entry: Preference | undefined = identity.preferences[key]
  if (!entry) return null
  if (isExpired(entry.expiresAt, now)) return null
  return entry.value
}

/**
 * Eagerly remove every expired preference. Returns the count of removed
 * entries. Only writes to disk if at least one entry was pruned (avoids
 * unnecessary file churn).
 */
export async function prunePreferences(
  opts: IdentityOptions | undefined,
  now: () => Date = () => new Date(),
): Promise<number> {
  const identity = await readIdentity(opts)
  let removed = 0
  for (const [key, entry] of Object.entries(identity.preferences)) {
    if (isExpired(entry.expiresAt, now)) {
      delete identity.preferences[key]
      removed += 1
    }
  }
  if (removed > 0) {
    await writeIdentity(opts, identity)
  }
  return removed
}
