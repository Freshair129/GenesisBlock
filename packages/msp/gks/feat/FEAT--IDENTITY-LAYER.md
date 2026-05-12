---
id: FEAT--IDENTITY-LAYER
phase: 2
type: feat
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: Identity layer — namespaced JSON store with profile / voice / preferences
tags:
  - msp
  - identity
  - soul
  - profile
  - voice
  - preferences
  - m7e
  - user-facing
crosslinks: {"implements":["ADR--IDENTITY-STORAGE-SHAPE"],"references":["CONCEPT--IDENTITY-LAYER"]}
linked_symbols:
  - {"file":"src/identity/index.ts"}
  - {"file":"src/identity/types.ts"}
  - {"file":"src/identity/profile.ts"}
  - {"file":"src/identity/voice.ts"}
  - {"file":"src/identity/preferences.ts"}
  - {"file":"src/identity/store.ts"}
created_at: 2026-05-05T01:26:00.000+07:00
---

# Identity layer — namespaced JSON store with profile / voice / preferences

## User-facing API

```ts
import {
  getIdentity,
  setProfile,
  setVoice,
  getPreference,
  setPreference,
  prunePreferences,
} from '@/identity'

// Read
const id = await getIdentity({ root: process.cwd(), namespace: 'evaAI' })
console.log(id.profile.name)              // 'EVA' or '' if not set
console.log(id.voice.tone)                // ['analytical', 'warm', ...]

// Write profile (immutable fields silently kept on first set)
await setProfile(
  { root, namespace: 'evaAI' },
  { name: 'EVA', role: 'research assistant', tier: 'T3' },
)

// Write voice
await setVoice(
  { root, namespace: 'evaAI' },
  { tone: ['analytical', 'concise'], formality: 'neutral', languagePreference: 'thai+english' },
)

// Preferences (with optional TTL)
await setPreference(
  { root, namespace: 'evaAI' },
  'default_top_k',
  5,
  { expiresAt: null },              // no expiry
)
await setPreference(
  { root, namespace: 'evaAI' },
  'session_verbose',
  true,
  { expiresInMs: 60 * 60 * 1000 },  // 1 hour
)

// Get with lazy expiry
const topK = await getPreference({ root, namespace: 'evaAI' }, 'default_top_k')
// → 5 (or null if expired/missing)

// Optional eager cleanup
await prunePreferences({ root, namespace: 'evaAI' })
```

## Acceptance criteria

- [ ] `getIdentity(opts)` reads `.brain/msp/projects/<ns>/identity.json`, returns default-constructed Identity if missing (does NOT create the file)
- [ ] `setProfile(opts, partial)` merges into `profile`; preserves existing fields not in `partial`; bumps `createdAt` only on first write
- [ ] `setVoice(opts, voice)` replaces the entire `voice` object (voice is small; full replace is simpler than partial-merge)
- [ ] `setPreference(opts, key, value, opts?)` writes one key with optional TTL; either `expiresAt` (ISO string) or `expiresInMs` (computed). Both unset → no expiry
- [ ] `getPreference(opts, key)` returns the value if present + non-expired, `null` otherwise. Does NOT mutate the file (lazy expiry)
- [ ] `prunePreferences(opts)` rewrites the file with expired entries removed; returns count of pruned entries
- [ ] All writes are **atomic** (tmp file + rename per `ADR--IDENTITY-STORAGE-SHAPE`)
- [ ] Reading a file with `schemaVersion > 1` throws (refuse to clobber newer format)
- [ ] **Namespace isolation** — operations on namespace A never touch namespace B
- [ ] No `gks/` writes
- [ ] No crosslinks from identity to GKS atoms

## Surfaces

| Surface | Form |
|---|---|
| TS API | `getIdentity`, `setProfile`, `setVoice`, `getPreference`, `setPreference`, `prunePreferences` |
| Types | `Identity`, `Profile`, `Voice`, `Preference`, `IdentityOptions` |
| Storage | `.brain/msp/projects/<namespace>/identity.json` |
| Tests | `test/identity/{store,profile,voice,preferences,index}.test.ts` |

## Test target

348 → ~370 (+22):

- store.test.ts: ~6 tests (atomic write, default-construct on missing, schemaVersion guard, namespace isolation)
- profile.test.ts: ~4 tests (set + get, partial merge, createdAt set-once)
- voice.test.ts: ~3 tests (set replaces fully, defaults, validation passes through)
- preferences.test.ts: ~6 tests (set/get, expiresAt absolute, expiresInMs, lazy expiry on get, prune count)
- index.test.ts: ~3 integration (round-trip, multi-namespace, default values surface)

## Out of scope

- MCP tool wrapping → M7f
- Cross-namespace operations
- Multi-process write safety (M9)
- Schema migration / version bump beyond v1
