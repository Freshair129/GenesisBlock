import { z } from 'zod'

/**
 * A single preference entry. `value` is JSON-serialisable; the schema permits
 * primitives, arrays, and plain objects — anything yaml.stringify can round-
 * trip without a custom tag.
 *
 * `expires_at` is an ISO-8601 timestamp string or null (never expires).
 * Expired entries are pruned lazily on read; a separate sweep is intentionally
 * not provided — the cost of keeping stale rows on disk is negligible and a
 * sweep would race with concurrent writers.
 */
const PrefValueSchema: z.ZodType<PrefValue> = z.lazy(() =>
  z.union([
    z.string(),
    z.number(),
    z.boolean(),
    z.null(),
    z.array(PrefValueSchema),
    z.record(PrefValueSchema),
  ]),
)

export type PrefValue =
  | string
  | number
  | boolean
  | null
  | PrefValue[]
  | { [key: string]: PrefValue }

export const PreferenceEntrySchema = z
  .object({
    value: PrefValueSchema,
    expires_at: z.string().datetime().nullable(),
  })
  .strict()

export type PreferenceEntry = z.infer<typeof PreferenceEntrySchema>

export const PreferencesSchema = z.record(PreferenceEntrySchema)
export type Preferences = z.infer<typeof PreferencesSchema>

export function isExpired(entry: PreferenceEntry, now: Date = new Date()): boolean {
  if (entry.expires_at === null) return false
  const exp = Date.parse(entry.expires_at)
  if (!Number.isFinite(exp)) return false
  return exp <= now.getTime()
}

/**
 * Build an entry. `ttlMs` is milliseconds from now; omit or pass null for a
 * non-expiring entry.
 */
export function makeEntry(
  value: PrefValue,
  ttlMs?: number | null,
  now: Date = new Date(),
): PreferenceEntry {
  const expires_at =
    ttlMs === undefined || ttlMs === null
      ? null
      : new Date(now.getTime() + ttlMs).toISOString()
  return { value, expires_at }
}
