import { readIdentity, writeIdentity } from './store.js'
import { defaultVoice, type IdentityOptions, type Voice } from './types.js'

/**
 * Set the entire voice sub-field (full replace per BLUEPRINT — voice is small,
 * full replace is simpler than partial-merge).
 *
 * Missing fields in the supplied voice are filled from `defaultVoice()` so the
 * persisted shape is always complete.
 *
 * Per CONCEPT / BLUEPRINT, `tone` and `languagePreference` are free-form —
 * implementation does NOT validate against an enum.
 */
export async function setVoice(
  opts: IdentityOptions | undefined,
  voice: Voice,
): Promise<void> {
  const identity = await readIdentity(opts)
  identity.voice = { ...defaultVoice(), ...voice }
  await writeIdentity(opts, identity)
}
