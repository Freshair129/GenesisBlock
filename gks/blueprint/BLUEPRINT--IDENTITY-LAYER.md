---
id: BLUEPRINT--IDENTITY-LAYER
phase: 3
type: blueprint
scale_level: L2
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: BLUEPRINT — identity layer implementation plan
tags: &a1
  - msp
  - identity
  - blueprint
  - implementation
  - m7e
crosslinks: &a2
  implements:
    - FEAT--IDENTITY-LAYER
  references:
    - ADR--IDENTITY-STORAGE-SHAPE
    - CONCEPT--IDENTITY-LAYER
linked_symbols: &a3
  - file: packages/msp/src/identity/index.ts
  - file: packages/msp/src/identity/types.ts
  - file: packages/msp/src/identity/store.ts
  - file: packages/msp/src/identity/profile.ts
  - file: packages/msp/src/identity/voice.ts
  - file: packages/msp/src/identity/preferences.ts
  - file: packages/msp/test/identity/store.test.ts
  - file: packages/msp/test/identity/profile.test.ts
  - file: packages/msp/test/identity/voice.test.ts
  - file: packages/msp/test/identity/preferences.test.ts
  - file: packages/msp/test/identity/index.test.ts
created_at: 2026-05-05T00:26:30.000+07:00
aliases: &a4
  - BLUEPRINT
  - implementation_flow
  - Implementation plan
cluster: implementation_flow
role: Implementation plan
attributes:
  id: BLUEPRINT--IDENTITY-LAYER
  phase: 3
  type: blueprint
  scale_level: L2
  status: stable
  vault_id: default
  tier: process
  source_type: axiomatic
  title: BLUEPRINT — identity layer implementation plan
  tags: *a1
  crosslinks: *a2
  linked_symbols: *a3
  created_at: 2026-05-05T00:26:30.000+07:00
  aliases: *a4
  cluster: implementation_flow
  role: Implementation plan
  attributes:
    id: BLUEPRINT--IDENTITY-LAYER
    phase: 3
    type: blueprint
    scale_level: L2
    status: stable
    vault_id: default
    tier: process
    source_type: axiomatic
    title: BLUEPRINT — identity layer implementation plan
    tags: *a1
    crosslinks: *a2
    linked_symbols: *a3
    created_at: 2026-05-05T00:26:30.000+07:00
    aliases: *a4
    cluster: implementation_flow
    role: Implementation plan
    attributes:
      id: BLUEPRINT--IDENTITY-LAYER
      phase: 3
      type: blueprint
      scale_level: L2
      status: stable
      vault_id: default
      tier: process
      source_type: axiomatic
      title: BLUEPRINT — identity layer implementation plan
      tags: *a1
      crosslinks: *a2
      linked_symbols: *a3
      created_at: 2026-05-05T00:26:30.000+07:00
      aliases: *a4
      cluster: implementation_flow
      role: Implementation plan
      attributes:
        domain: blueprint
      domain: blueprint
      language: markdown
      is_test: false
      is_entrypoint: false
      has_secret: true
      secret_type: high_entropy_string
      leak_risk: high
      encryption_level: none
    domain: blueprint
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: true
    secret_type: high_entropy_string
    leak_risk: high
    encryption_level: none
  domain: blueprint
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: true
  secret_type: high_entropy_string
  leak_risk: high
  encryption_level: none
---

# BLUEPRINT — identity layer implementation plan

```yaml
metadata:
  title: "MSP identity layer (profile / voice / preferences)"
  parent_feat: FEAT--IDENTITY-LAYER

architectural_pattern: |
  Six small modules, all plain functions over plain data. No class hierarchies.

    types.ts       - Identity, Profile, Voice, Preference, IdentityOptions, defaults
    store.ts       - low-level: identityPath, readIdentity, writeIdentity (atomic),
                     defaultIdentity, schemaVersion guard
    profile.ts     - setProfile (partial merge with createdAt set-once)
    voice.ts       - setVoice (full replace)
    preferences.ts - setPreference (TTL handling), getPreference (lazy expiry),
                     prunePreferences
    index.ts       - re-export public API + getIdentity convenience wrapper

  Atomic write is the only "tricky" piece — temp + rename pattern.
  Re-uses no external deps; everything in node:fs/promises + node:path.

data_logic: |
  src/identity/store.ts
    identityPath(root, namespace) → string
      = resolve(root, '.brain/msp/projects', namespace, 'identity.json')

    defaultIdentity() → Identity
      profile: { name: '', role: '', tier: 'T3', originStory: '', createdAt: now() }
      voice:   { tone: [], formality: 'neutral', languagePreference: 'auto', responseCadence: 'normal' }
      preferences: {}
      schemaVersion: 1

    readIdentity(opts) → Promise<Identity>
      try read JSON
      if ENOENT → return defaultIdentity()
      if schemaVersion > 1 → throw (per ADR)
      shallow-merge with defaultIdentity to fill any missing fields

    writeIdentity(opts, identity) → Promise<void>
      ensure parent dir exists (mkdir -p)
      tmp = `${path}.tmp.${pid}.${Date.now()}`
      writeFile(tmp, JSON.stringify(identity, null, 2))
      rename(tmp, path)

  src/identity/profile.ts
    setProfile(opts, partial: Partial<Profile>) →
      load identity
      if profile.createdAt empty → set createdAt = now()
      profile = { ...identity.profile, ...partial }
      writeIdentity

  src/identity/voice.ts
    setVoice(opts, voice: Voice) →
      load identity
      identity.voice = { ...defaultVoice, ...voice }   // ensure no missing keys
      writeIdentity

  src/identity/preferences.ts
    setPreference(opts, key, value, ttl?: { expiresAt?: string; expiresInMs?: number }) →
      load identity
      let expiresAt: string | null = null
      if ttl?.expiresAt → expiresAt = ttl.expiresAt
      else if ttl?.expiresInMs → expiresAt = new Date(now() + ttl.expiresInMs).toISOString()
      identity.preferences[key] = { value, expiresAt }
      writeIdentity

    getPreference(opts, key) →
      load identity
      pref = identity.preferences[key]
      if !pref → null
      if pref.expiresAt && Date(pref.expiresAt) <= now() → null   // lazy expiry
      return pref.value

    prunePreferences(opts) →
      load identity
      count = 0
      for [k, p] of preferences:
        if p.expiresAt && Date(p.expiresAt) <= now():
          delete preferences[k]; count++
      if count > 0 → writeIdentity
      return count

  src/identity/index.ts
    export type { ... }
    export { setProfile, setVoice, setPreference, getPreference, prunePreferences }
    export async function getIdentity(opts) = readIdentity(opts)

geography:
  - "packages/msp/src/identity/types.ts"          # ~30 lines
  - "packages/msp/src/identity/store.ts"          # ~80 lines (atomic write + default)
  - "packages/msp/src/identity/profile.ts"        # ~30 lines
  - "packages/msp/src/identity/voice.ts"          # ~25 lines
  - "packages/msp/src/identity/preferences.ts"    # ~70 lines (TTL logic)
  - "packages/msp/src/identity/index.ts"          # ~30 lines (re-exports)
  - "packages/msp/test/identity/store.test.ts"
  - "packages/msp/test/identity/profile.test.ts"
  - "packages/msp/test/identity/voice.test.ts"
  - "packages/msp/test/identity/preferences.test.ts"
  - "packages/msp/test/identity/index.test.ts"

api_contracts:
  - name: getIdentity
    signature: |
      function getIdentity(opts: IdentityOptions): Promise<Identity>
    types: |
      interface IdentityOptions {
        root?: string       // default cwd
        namespace?: string  // default 'evaAI' (matches sessions / consolidator default)
      }
      interface Identity {
        schemaVersion: 1
        profile: Profile
        voice: Voice
        preferences: Record<string, Preference>
      }
      interface Profile {
        name: string
        role: string
        tier: 'T1' | 'T2' | 'T3'
        originStory: string
        createdAt: string  // ISO 8601
      }
      interface Voice {
        tone: string[]
        formality: 'casual' | 'neutral' | 'formal'
        languagePreference: string  // free-form: 'en', 'th', 'thai+english', 'auto'
        responseCadence: 'terse' | 'normal' | 'verbose'
      }
      interface Preference {
        value: unknown
        expiresAt: string | null  // ISO 8601 or null
      }

  - name: setProfile / setVoice / setPreference / getPreference / prunePreferences
    signatures: |
      function setProfile(opts: IdentityOptions, partial: Partial<Profile>): Promise<void>
      function setVoice(opts: IdentityOptions, voice: Voice): Promise<void>
      function setPreference(
        opts: IdentityOptions,
        key: string,
        value: unknown,
        ttl?: { expiresAt?: string; expiresInMs?: number },
      ): Promise<void>
      function getPreference(opts: IdentityOptions, key: string): Promise<unknown | null>
      function prunePreferences(opts: IdentityOptions): Promise<number>

verification_plan:
  - vitest: getIdentity on missing file returns default-constructed Identity (no file created)
  - vitest: getIdentity on file with schemaVersion=2 throws
  - vitest: setProfile sets createdAt on first write; preserves on subsequent
  - vitest: setProfile partial merge keeps unspecified fields
  - vitest: setVoice replaces entire voice object
  - vitest: setPreference with no TTL → expiresAt: null
  - vitest: setPreference with expiresInMs computes correct expiresAt
  - vitest: getPreference returns null for expired entry (lazy, no file mutation)
  - vitest: getPreference returns value for non-expired
  - vitest: prunePreferences removes expired, returns count, leaves non-expired
  - vitest: namespace isolation — operations on ns A don't touch ns B
  - vitest: atomic write — concurrent setProfile + setVoice in same process don't lose either (sequenced)
  - vitest: round-trip — write Identity, read Identity, deep equal

  Test count: 348 → ~370 (+22)

implementation_order:
  T1 TYPES         types.ts: Identity, Profile, Voice, Preference, IdentityOptions, default constructors
  T2 STORE         store.ts: atomic writeIdentity + readIdentity (default-construct, schemaVersion guard) + 6 tests
  T3 PROFILE       profile.ts: setProfile (partial merge + createdAt set-once) + 4 tests
  T4 VOICE         voice.ts: setVoice (full replace) + 3 tests
  T5 PREFERENCES   preferences.ts: TTL math + lazy expiry + prune + 6 tests
  T6 INDEX         index.ts: re-exports + getIdentity wrapper + 3 integration tests
  T7 AUDIT         AUDIT--IDENTITY-LAYER atom recording shipped behaviour + counts
```

## Implementation notes for the implementer

- **Atomic write** is critical. Use `writeFile(tmp, ...)` + `rename(tmp, path)`. Don't write to the destination directly. POSIX rename is atomic; Windows is not (M9 issue per `[[CONCEPT--MSP-ROADMAP]]` §3 M9f).

- **`mkdir -p`** the parent dir before write — first write creates the namespace dir.

- **Default-construct** on read-from-missing rather than returning null. Every caller can assume a valid Identity object.

- **No sync APIs** — everything async (`fs/promises`).

- **No new deps**. Stay within node:fs / node:path / node:os.

- **Lazy expiry** for preferences — `getPreference` checks `expiresAt` against `now()` but doesn't rewrite the file. Caller decides when to `prunePreferences`.

## Implementer: do NOT do

- Write to `gks/` (identity is execution state, not durable knowledge)
- Add MCP tool wrappers (M7f)
- Build cross-namespace operations
- Add a lockfile / multi-process safety (M9)
- Validate voice values against an enum (formality is constrained, but tone/languagePreference are free-form)
- Persist transient runtime state beyond profile/voice/preferences (e.g. last session id — that's session-side, not identity)

## Connections

- [[FEAT--IDENTITY-LAYER]]
- [[ADR--IDENTITY-STORAGE-SHAPE]]
- [[CONCEPT--IDENTITY-LAYER]]
