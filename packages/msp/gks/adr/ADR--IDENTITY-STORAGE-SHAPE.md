---
id: ADR--IDENTITY-STORAGE-SHAPE
phase: 2
type: adr
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: Identity storage — JSON file, atomic-write, namespaced, no GKS atom
tags:
  - msp
  - identity
  - storage
  - decision
  - m7e
crosslinks: {"references":["CONCEPT--IDENTITY-LAYER","ADR--GRAPH-IS-GKS-DOMAIN"]}
created_at: 2026-05-04T17:25:30.000Z
---

# ADR — identity storage shape

## Context

The M7e identity layer needs a persistence shape. Considered:

1. **GKS atom** — `IDENTITY--<namespace>` markdown atom in `gks/identity/`
2. **JSON file under `.brain/msp/projects/<ns>/`** — same locus as sessions/episodic
3. **YAML file** — same locus, YAML format
4. **Distributed across many small files** — `profile.json`, `voice.json`, `preferences.json`

Per `msp_spec.md` §7e, the directional choice is option 2 (`identity.json`). This ADR records the rationale and the constraints.

## Decision

### Shape

Single JSON file at `.brain/msp/projects/<namespace>/identity.json` per namespace. One namespace = one identity.

```json
{
  "schemaVersion": 1,
  "profile":      { ... },
  "voice":        { ... },
  "preferences":  { ... }
}
```

### Atomic write

Temp-file + rename pattern. Implementation MUST use:

```ts
const tmp = `${path}.tmp.${process.pid}.${Date.now()}`
await writeFile(tmp, JSON.stringify(data, null, 2), 'utf8')
await rename(tmp, path)
```

Concurrent writers are not supported (same-process is fine; cross-process is undefined behaviour). MSP is currently single-tenant per process, so this is acceptable.

### Default construction

Reading a non-existent identity file returns a **default-constructed Identity** rather than null:

```ts
{
  schemaVersion: 1,
  profile: { name: '', role: '', tier: 'T3', originStory: '', createdAt: now() },
  voice:   { tone: [], formality: 'neutral', languagePreference: 'auto', responseCadence: 'normal' },
  preferences: {}
}
```

This means callers never have to null-check `identity.profile`. First write materialises the file.

### Schema versioning

`schemaVersion: 1` is set on every write. Future schema migrations (M9-era) check this and adapt.

If an existing file has `schemaVersion > 1`, the loader **errors out** rather than silently dropping fields. (Forward-compat is hard; safer to refuse to clobber a newer-format file.)

### TTL on preferences

`preferences[key].expiresAt: string | null` — ISO timestamp. `getPreference(key)` checks against `now()` and returns `null` if expired (without rewriting the file — lazy expiry).

A separate `prunePreferences()` helper does eager cleanup, called optionally by the caller (M7f could wire this to `msp_identity_get`).

### Why NOT a GKS atom

Three reasons:

1. **Lifecycle mismatch** — atoms have `valid_until` semantics + supersede chains; identity doesn't supersede, it mutates in place
2. **Churn** — preferences could change per-session; atoms have settling time per `ADR--GRAPH-IS-GKS-DOMAIN`
3. **Scope** — `ADR--GRAPH-IS-GKS-DOMAIN` already establishes that execution state lives outside `gks/`. Identity is execution state

Same reason TASK-- left the atomic taxonomy (per ADR-015 referenced in `MSP_RELATIONSHIP.md`).

### Why NOT YAML

Identity isn't human-edited often. JSON's unambiguous parsing + native Node support beats YAML's "humans read it easier" argument when the file is mostly tool-managed.

### Why NOT one-file-per-section

`profile.json` + `voice.json` + `preferences.json` would mean three atomic writes per save, and identity-as-a-whole has no useful mid-state. Single file is simpler.

## Consequences

**Positive**
- Single read = full identity (no cross-file sync issues)
- Atomic write trivial (rename is atomic on POSIX)
- No external dep (Node's `fs/promises` covers everything)
- Default-construct on missing means every code path can assume a valid Identity
- Schema versioning gates future migrations safely

**Negative**
- Multi-writer support not provided. If MSP grows to multi-process per tenant, this needs a lockfile (M9 work).
- TTL is lazy — expired preferences linger on disk until the next set/prune. Cosmetic, not correctness.

## Alternatives considered

1. **GKS atom** — rejected, see lifecycle/churn mismatch above
2. **YAML** — rejected, JSON is simpler for tool-managed data
3. **One-file-per-section** — rejected, no win, more sync surface
4. **SQLite** — rejected, identity is < 5KB; durable file is enough
5. **In-memory only with snapshot** — rejected, identity must survive process restart

## What this ADR does NOT decide

- **MCP tool surface** — `msp_identity_get` / `msp_identity_set` are M7f
- **Identity rotation / multi-version** — out of scope for M7e
- **Cross-tenant auth model** — M9
- **Default voice / profile values** — implementation detail; sensible defaults suggested but tunable

## Source

`msp_spec.md` §7e, `CONCEPT--IDENTITY-LAYER`, `ADR--GRAPH-IS-GKS-DOMAIN` (execution state vs durable knowledge boundary).
