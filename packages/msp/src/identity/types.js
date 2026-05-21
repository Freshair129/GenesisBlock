/**
 * Identity layer types — the "soul" half of the MSP passport.
 *
 * Per CONCEPT--IDENTITY-LAYER, identity has three sub-fields:
 *   - profile:     stable identifying facts (name, role, tier, originStory)
 *   - voice:       how the agent communicates (tone, formality, language, cadence)
 *   - preferences: per-key user/runtime overrides with optional TTL
 *
 * Per ADR--IDENTITY-STORAGE-SHAPE, the on-disk shape is a single JSON file
 * at `.brain/msp/projects/<namespace>/identity.json` with `schemaVersion: 1`.
 *
 * All types are plain data; constructors below produce defaults so callers
 * never have to null-check after `getIdentity`.
 */
/** Default namespace — matches the sessions / consolidator convention. */
export const DEFAULT_NAMESPACE = 'evaAI';
/** Current on-disk schema version. */
export const CURRENT_SCHEMA_VERSION = 1;
/**
 * Default profile — empty strings + T3 + empty `createdAt`.
 *
 * `createdAt` is intentionally empty so callers (specifically `setProfile`)
 * can detect first-write and stamp the actual creation time then. Once set
 * via `setProfile`, `createdAt` is preserved on every subsequent write.
 */
export function defaultProfile() {
    return {
        name: '',
        role: '',
        tier: 'T3',
        originStory: '',
        createdAt: '',
        guardrails: [],
        extensions: {},
        roles: [],
        clearance: 0,
        mfaStatus: false,
        tenantIds: [],
    };
}
/** Default voice — neutral, auto-language, normal cadence, no tone descriptors. */
export function defaultVoice() {
    return {
        tone: [],
        formality: 'neutral',
        languagePreference: 'auto',
        responseCadence: 'normal',
    };
}
/** Default-constructed identity. Used when the on-disk file is missing. */
export function defaultIdentity() {
    return {
        schemaVersion: CURRENT_SCHEMA_VERSION,
        profile: defaultProfile(),
        voice: defaultVoice(),
        preferences: {},
    };
}
