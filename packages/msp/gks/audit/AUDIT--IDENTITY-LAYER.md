---
id: AUDIT--IDENTITY-LAYER
phase: 6
type: audit
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: M7e — identity layer implementation (profile / voice / preferences with atomic JSON store)
tags:
  - msp
  - identity
  - soul
  - profile
  - voice
  - preferences
  - m7e
  - audit
crosslinks: {"references":["FEAT--IDENTITY-LAYER","BLUEPRINT--IDENTITY-LAYER","ADR--IDENTITY-STORAGE-SHAPE","CONCEPT--IDENTITY-LAYER","FRAME--MSP-ARCHITECTURE-V2","MOD--IDENTITY","PROTOCOL--IDENTITY-API","ALGO--IDENTITY-RESOLUTION"]}
linked_symbols:
  - {"file":"src/identity/index.ts"}
  - {"file":"src/identity/types.ts"}
  - {"file":"src/identity/store.ts"}
  - {"file":"src/identity/profile.ts"}
  - {"file":"src/identity/voice.ts"}
  - {"file":"src/identity/preferences.ts"}
  - {"file":"test/identity/store.test.ts"}
  - {"file":"test/identity/profile.test.ts"}
  - {"file":"test/identity/voice.test.ts"}
  - {"file":"test/identity/preferences.test.ts"}
  - {"file":"test/identity/index.test.ts"}
created_at: 2026-05-05T13:00:00.000+07:00
---

# M7e — identity layer implementation (profile / voice / preferences with atomic JSON store)

## Scope

M7e deliverable: namespaced identity store implementing the "soul" half of the MSP passport per `FEAT--IDENTITY-LAYER`, `BLUEPRINT--IDENTITY-LAYER`, and `ADR--IDENTITY-STORAGE-SHAPE`. Persists profile / voice / preferences as a single atomic-write JSON file at `.brain/msp/projects/<namespace>/identity.json`. Pure plain-data + plain-functions design — no external runtime deps.

This implementation **replaces** an earlier YAML+zod attempt that landed during M7-prep and was inconsistent with the BLUEPRINT (different shape, different storage format, validator-coupled). The replacement is doc-faithful (matches BLUEPRINT 1:1 on shape, format, defaults, set-once semantics, lazy expiry).

## What shipped

| File | Purpose |
|---|---|
| `src/identity/types.ts` | `Identity`, `Profile`, `Voice`, `Preference`, `IdentityOptions`, `Tier`, `Formality`, `ResponseCadence`, default constructors, `DEFAULT_NAMESPACE`, `CURRENT_SCHEMA_VERSION` |
| `src/identity/store.ts` | `identityPath`, `resolveOptions`, `readIdentity` (default-construct on missing, schemaVersion>1 guard, default-merge), `writeIdentity` (atomic temp+rename, `mkdir -p` parent, force schemaVersion=1) |
| `src/identity/profile.ts` | `setProfile` — partial merge with set-once `createdAt` (caller cannot override) |
| `src/identity/voice.ts` | `setVoice` — full replace (with default-fill for any missing fields) |
| `src/identity/preferences.ts` | `setPreference` (TTL: `expiresAt` ISO wins over `expiresInMs`), `getPreference` (lazy expiry — reads do not mutate file), `prunePreferences` (eager cleanup, returns count, only writes when something pruned), `PreferenceTtl` type |
| `src/identity/index.ts` | Re-exports + `getIdentity` convenience wrapper |
| `test/identity/store.test.ts` | 13 tests — path resolution, defaults, schemaVersion guard, missing-field merge, namespace isolation, atomic-write artifact, last-writer-wins |
| `test/identity/profile.test.ts` | 6 tests — set-once createdAt, partial merge, override-defence, isolation from voice/prefs, tier enum |
| `test/identity/voice.test.ts` | 5 tests — full replace, free-form tone/language, isolation from profile/prefs |
| `test/identity/preferences.test.ts` | 10 tests — set/get, missing key, TTL absolute vs relative, lazy expiry, value shapes, last-writer-wins, prune count, prune no-op |
| `test/identity/index.test.ts` | 5 tests — default shape, round-trip, default-not-null guarantee, getPreference-on-fresh, multi-namespace isolation |

## Boundaries respected

- **No `gks/` writes** — identity is execution state, not durable knowledge (per `ADR--GRAPH-IS-GKS-DOMAIN` spirit). No identity atoms, no crosslinks from identity to atoms.
- **No MCP tool wrappers** — `msp_identity_get` / `msp_identity_set` are M7f scope.
- **No cross-namespace operations** — every public function takes `IdentityOptions` and operates on exactly one namespace.
- **No lockfile / multi-process write safety** — same-process writes are sequenced; cross-process is undefined behaviour per ADR. Multi-process safety is M9 work.
- **No new runtime deps** — `node:fs/promises` + `node:path` cover everything. Removed the prior implementation's `yaml` + `zod` dependencies (they remain in package.json for other modules).
- **No enum validation on `voice.tone` / `voice.languagePreference`** — both are intentionally free-form per BLUEPRINT.
- **No edits to `src/memory/` or `src/orchestrator/`** — orthogonal concerns.

## Atoms landed

| Atom | Phase | Type |
|---|---|---|
| `CONCEPT--IDENTITY-LAYER` | 1 | concept (existed) |
| `PROTOCOL--IDENTITY-API` | 2 | protocol (NEW) |
| `ALGO--IDENTITY-RESOLUTION` | 2 | algo (NEW) |
| `ADR--IDENTITY-STORAGE-SHAPE` | 2 | adr (existed) |
| `FEAT--IDENTITY-LAYER` | 2 | feat (existed) |
| `MOD--IDENTITY` | 2 | mod (NEW) |
| `BLUEPRINT--IDENTITY-LAYER` | 3 | blueprint (existed) |
| `AUDIT--IDENTITY-LAYER` | 6 | audit (updated) |

## Verification

```
npm ci                              → 163 packages installed (worktree caveat per CLAUDE.md)
npm test                            → 345 passed (49 files)
npm run typecheck                   → clean
npx tsx src/validator/cli.ts --all  → 112 passed, 0 failed
npm run msp:check-links             → OK (112 atoms scanned)
```

Test count delta: 348 → 345 (net −3, but +39 new identity tests on top of removing 42 prior YAML+zod tests). The new tests fully cover the BLUEPRINT verification plan plus several extra edge cases (free-form tone/language, last-writer-wins per key, default-not-null guarantee, atomic-write tmp-file cleanup). Per-module breakdown:

- `store.test.ts`: 13 tests (target 6 — exceeded)
- `profile.test.ts`: 6 tests (target 4 — exceeded)
- `voice.test.ts`: 5 tests (target 3 — exceeded)
- `preferences.test.ts`: 10 tests (target 6 — exceeded)
- `index.test.ts`: 5 tests (target 3 — exceeded)

The headline 348→370 number from FEAT was computed assuming a clean baseline (no prior identity impl). Because main already shipped a divergent YAML-based identity in commit `fe013cd`, the net delta differs even though every BLUEPRINT module / test target is met or exceeded. Net coverage of the BLUEPRINT contract is complete.

## Acceptance criteria from `FEAT--IDENTITY-LAYER`

- [x] `getIdentity(opts)` reads `.brain/msp/projects/<ns>/identity.json`, returns default-constructed Identity if missing, **does NOT** create the file
- [x] `setProfile(opts, partial)` merges into `profile`; preserves existing fields not in `partial`; bumps `createdAt` only on first write
- [x] `setVoice(opts, voice)` replaces the entire `voice` object
- [x] `setPreference(opts, key, value, ttl?)` — `expiresAt` ISO or `expiresInMs` relative; both unset → no expiry; both supplied → `expiresAt` wins
- [x] `getPreference(opts, key)` returns the value if present + non-expired, `null` otherwise; does **NOT** mutate the file (lazy expiry verified by mtime assertion)
- [x] `prunePreferences(opts)` rewrites the file with expired entries removed; returns count of pruned entries; no-op when nothing is expired (no rewrite)
- [x] All writes are atomic (tmp file + rename per `ADR--IDENTITY-STORAGE-SHAPE`)
- [x] Reading a file with `schemaVersion > 1` throws (refuse to clobber newer format)
- [x] Namespace isolation — operations on namespace A never touch namespace B
- [x] No `gks/` writes
- [x] No crosslinks from identity to GKS atoms

## Decisions during impl

These choices were not pre-specified by the BLUEPRINT and are recorded for future tuning:

1. **`createdAt` defaults to `''` (empty string), not `now().toISOString()`.** The BLUEPRINT shows `createdAt: now()` in `defaultProfile()` but also says `setProfile` should detect "createdAt empty → set createdAt = now()". These two are contradictory if the default already stamps a value. Resolved by keeping the default empty string so the set-once detection in `setProfile` works naturally. Empty-string convention surfaces clearly in tests + serialised output.

2. **`createdAt` is set-once at the `setProfile` level even against caller override.** A caller passing `{ createdAt: '1999-01-01' }` in `partial` is silently ignored after first write. This is stricter than "API does NOT enforce immutability" from CONCEPT, but matches the FEAT acceptance ("bumps `createdAt` only on first write"). Documented in profile.ts and tested directly. Callers who genuinely need to backdate identity would use `writeIdentity` directly (low-level escape hatch).

3. **`expiresAt` (absolute) wins over `expiresInMs` (relative) when both are supplied.** Neither BLUEPRINT nor FEAT specified the precedence. Picked "more specific wins" (absolute > relative).

4. **`prunePreferences` is a true no-op when nothing is expired.** Avoids writing the file (and bumping mtime) when there's no work. Tested directly.

5. **`writeIdentity` always forces `schemaVersion = 1` on the persisted payload.** Defends against accidental object-literal corruption in callers.

6. **`readIdentity` shallow-merges nested objects with defaults.** A partial on-disk file (e.g. only `voice.tone` written) surfaces a complete shape after read, so callers can rely on every field being present without null-checks. This is forward-compat friendly: future v1.x additions arrive default-filled.

7. **`now: () => Date` injection on every state-changing function.** Keeps tests deterministic (fixture timestamps) without needing fake-timers globally. Pattern matches the consolidator's `ConsolidateOptions.now`.

8. **Replaced (not amended) the prior YAML+zod implementation.** The prior impl stored `identity.yaml` with a flat shape (`name`, `voice`, `preferences`, `origin_story`) and used zod for runtime validation. This conflicts with:
   - BLUEPRINT's JSON-with-`schemaVersion`+`profile`+`voice`+`preferences` envelope
   - ADR's "no external dep" goal
   - FEAT's `Identity` / `Profile` types with `tier`, `originStory`, `createdAt`, `responseCadence`, `formality` fields
   - The `setProfile` / `setVoice` / `setPreference` / `getPreference` / `prunePreferences` API
   The replacement is doc-faithful and removes a small amount of bundle weight (no zod schemas at module scope).

## Public API surface

```ts
import {
  getIdentity,
  setProfile,
  setVoice,
  setPreference,
  getPreference,
  prunePreferences,
  type Identity,
  type Profile,
  type Voice,
  type Preference,
  type IdentityOptions,
  type PreferenceTtl,
} from '@/identity'

// Read (default-constructed if no file yet)
const id = await getIdentity({ root: process.cwd(), namespace: 'evaAI' })

// Profile (partial merge; createdAt set-once)
await setProfile({ namespace: 'evaAI' }, { name: 'EVA', tier: 'T3' })

// Voice (full replace)
await setVoice({ namespace: 'evaAI' }, {
  tone: ['analytical'],
  formality: 'neutral',
  languagePreference: 'thai+english',
  responseCadence: 'normal',
})

// Preferences (with optional TTL)
await setPreference({ namespace: 'evaAI' }, 'top_k', 5)
await setPreference({ namespace: 'evaAI' }, 'verbose', true, { expiresInMs: 60_000 })
const k = await getPreference({ namespace: 'evaAI' }, 'top_k')   // 5 or null

// Eager TTL cleanup
const removed = await prunePreferences({ namespace: 'evaAI' })   // count
```

Low-level escape hatches are also exported: `identityPath`, `readIdentity`, `writeIdentity`, `defaultIdentity`, `defaultProfile`, `defaultVoice`.

## Out of scope (deferred)

- M7f — MCP tool wrappers (`msp_identity_get` / `msp_identity_set`)
- M9 — Multi-process write safety (lockfile)
- M9 — Schema migration beyond v1 (when bumping schemaVersion)
- M9 — Cross-namespace / multi-tenant auth
- Identity validation rules (forbidden voice combos, name length caps, etc.) — over-engineering for current needs

## Sign-off

- Implemented by: @claude-opus-4-7
- Verified by: 39 new tests + validator (112/112) + check-links OK + typecheck clean + npm ci clean
- Branch: `claude/msp-m7e-identity-impl`
