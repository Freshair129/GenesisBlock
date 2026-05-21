import { readIdentity, writeIdentity } from './store.js';
function computeExpiresAt(ttl, now) {
    if (!ttl)
        return null;
    if (typeof ttl.expiresAt === 'string')
        return ttl.expiresAt;
    if (typeof ttl.expiresInMs === 'number') {
        return new Date(now().getTime() + ttl.expiresInMs).toISOString();
    }
    return null;
}
function isExpired(expiresAt, now) {
    if (!expiresAt)
        return false;
    const t = Date.parse(expiresAt);
    if (!Number.isFinite(t))
        return false;
    return t <= now().getTime();
}
/**
 * Set or replace a single preference, optionally with a TTL.
 *
 * - `expiresAt` (ISO string) takes precedence over `expiresInMs`.
 * - With neither, the entry never expires.
 * - Replaces any existing entry for the same key.
 */
export async function setPreference(opts, key, value, ttl, now = () => new Date()) {
    const identity = await readIdentity(opts);
    const expiresAt = computeExpiresAt(ttl, now);
    identity.preferences[key] = { value, expiresAt };
    await writeIdentity(opts, identity);
}
/**
 * Read a preference value with **lazy expiry**: an expired entry returns null
 * but is NOT removed from disk. Caller can run `prunePreferences` to do eager
 * cleanup.
 *
 * Missing key → null. Expired entry → null. Otherwise → `value`.
 */
export async function getPreference(opts, key, now = () => new Date()) {
    const identity = await readIdentity(opts);
    const entry = identity.preferences[key];
    if (!entry)
        return null;
    if (isExpired(entry.expiresAt, now))
        return null;
    return entry.value;
}
/**
 * Eagerly remove every expired preference. Returns the count of removed
 * entries. Only writes to disk if at least one entry was pruned (avoids
 * unnecessary file churn).
 */
export async function prunePreferences(opts, now = () => new Date()) {
    const identity = await readIdentity(opts);
    let removed = 0;
    for (const [key, entry] of Object.entries(identity.preferences)) {
        if (isExpired(entry.expiresAt, now)) {
            delete identity.preferences[key];
            removed += 1;
        }
    }
    if (removed > 0) {
        await writeIdentity(opts, identity);
    }
    return removed;
}
