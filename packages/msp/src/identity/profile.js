import { readIdentity, writeIdentity } from './store.js';
/**
 * Set (partial-merge) the profile sub-field.
 *
 * Behaviour per BLUEPRINT / FEAT:
 *   - Reads the on-disk identity (default-constructed if missing).
 *   - `partial` fields overwrite; unspecified fields are preserved.
 *   - `createdAt` is **set-once**: empty `createdAt` → stamped `now()` on this
 *     write; non-empty → preserved (caller cannot overwrite via `partial`).
 *
 * Per CONCEPT, `name`/`role`/`tier`/`originStory` are conventionally append-
 * mostly but the API does NOT enforce immutability — callers can rename
 * deliberately if needed.
 */
export async function setProfile(opts, partial, now = () => new Date()) {
    const identity = await readIdentity(opts);
    // Capture the prior createdAt before the merge so `partial.createdAt`
    // cannot override set-once semantics.
    const priorCreatedAt = identity.profile.createdAt;
    const merged = {
        ...identity.profile,
        ...partial,
        createdAt: priorCreatedAt || now().toISOString(),
    };
    identity.profile = merged;
    await writeIdentity(opts, identity);
}
