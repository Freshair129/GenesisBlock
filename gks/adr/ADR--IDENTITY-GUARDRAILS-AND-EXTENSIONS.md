---
id: ADR--IDENTITY-GUARDRAILS-AND-EXTENSIONS
phase: 2
type: adr
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: Identity profile — add guardrails and extensions fields (M7e follow-up)
tags:
  - msp
  - identity
  - profile
  - guardrails
  - extensions
  - decision
  - m7e
crosslinks: {"references":["FEAT--IDENTITY-LAYER","ADR--IDENTITY-STORAGE-SHAPE","CONCEPT--IDENTITY-LAYER","ADR--ANTI-HALLUCINATION-RULES"]}
created_at: 2026-05-05T12:30:00.000+07:00
---

# ADR — identity profile guardrails + extensions

## Context

`[[FEAT--IDENTITY-LAYER]]` (M7e, just shipped via PR #19) defines `Profile` with five fields: `name`, `role`, `tier`, `originStory`, `createdAt`. Lean, doc-faithful.

Two fields from the M7e parallel-fanout impl (commit `fe013cd`, replaced by PR #19) carried real value but didn't match the BLUEPRINT and were dropped:

1. **`guardrails: string[]`** — per-agent hard rules ("Never invent atom IDs"; "Never claim a test pass without naming the runner output"). These are anti-hallucination personalisation that pairs naturally with `[[ADR--ANTI-HALLUCINATION-RULES]]`. Validator rules are project-wide; guardrails are agent-specific.

2. **`extensions: Record<string, unknown>`** — free-form bag for harness-specific data (Claude Code, Cursor, EVA). MSP itself never reads this; harnesses use it to attach state without forking the schema.

Three operational fields from the old impl deliberately stay out:
- `operational_state.last_session_id` → belongs in sessions (M7c retrieval reads this)
- `operational_state.ongoing_context_pointer` → belongs in episodic + consolidator (M7b/d)
- `retrieval.source_weights` → belongs in M7c orchestration config, not identity

Per `[[ADR--GRAPH-IS-GKS-DOMAIN]]`'s execution-state-vs-durable-knowledge boundary, these belong with their owning subsystems.

## Decision

Extend `Profile` with two optional, additive fields:

```ts
export interface Profile {
  name: string
  role: string
  tier: Tier
  originStory: string
  createdAt: string
  /** Per-agent hard rules. Anti-hallucination personalisation. */
  guardrails: string[]
  /** Free-form bag for harness-specific data. MSP never reads this. */
  extensions: Record<string, unknown>
}
```

Both default to **empty** (`[]` / `{}`) so existing identity files remain readable without migration. The schema version stays `1`.

### Why no schema version bump

Adding a field that defaults to empty is **forward-compatible**: a v1 file without the field reads cleanly when defaults are merged. The schemaVersion-guard logic in `store.ts` (refuse `> 1`) is unchanged. Migration to v2 would be required only when removing a field or changing types.

### `guardrails` semantics

- **Agent-side hint, not runtime enforcement** — MSP doesn't validate that the agent honours its guardrails. They're injected into prompts (M7c retrieval will fold them into context).
- **Free-form strings** — no parsing, no enum.
- **Order matters** — earlier guardrails dominate (similar to `voice.tone`).
- **Reasonable upper bound** — soft cap of 50 entries; not enforced. Beyond that, the user has likely confused guardrails with system prompts.

### `extensions` semantics

- **MSP NEVER reads this** — pure passthrough storage.
- **Harnesses own their keys** — by convention, namespace your extension keys (e.g. `extensions["claude-code/notes"]`) to avoid collision with other harnesses sharing the same identity.
- **Atomic-write still applies** — set via `setProfile({ ..., extensions: {...} })`.
- **No size limit** — but the entire identity file is loaded eagerly, so don't store megabytes here.

### API impact

`setProfile(opts, partial: Partial<Profile>)` already merges partials; the new fields slot in naturally. Callers that don't pass them keep whatever's on disk.

```ts
await setProfile(opts, {
  guardrails: [
    "Never invent atom IDs.",
    "Never claim a test pass without naming the runner output.",
  ],
  extensions: { 'claude-code/notes': 'starter profile' },
})
```

## Consequences

**Positive**
- Recovers the genuinely useful fields from the dropped impl
- Forward-compatible — no migration, no schema bump
- `guardrails` formalises an anti-hallucination patten that was implicit before
- `extensions` gives harnesses an escape hatch without forking

**Negative**
- Slight schema growth — but both fields are tiny and default to empty
- Harnesses must namespace their `extensions` keys by convention (no enforcement)
- A user with hundreds of guardrails would hurt prompt budget; we don't guard

## Alternatives considered

1. **Don't add anything; tell users to use preferences for harness data.** Rejected — preferences have TTL semantics; harness data is permanent.
2. **Add a separate `harnesses.json` file.** Rejected — extra atomic-write target, sync surface.
3. **Add `operational_state` too.** Rejected per `[[ADR--GRAPH-IS-GKS-DOMAIN]]` — that's session/episodic territory.
4. **Add `retrieval.source_weights` here.** Rejected — that's M7c orchestration config, not identity. Belongs in M7c PR.

## What this ADR does NOT change

- `voice` shape — unchanged
- `preferences` shape — unchanged
- Storage path / atomic-write / schemaVersion — unchanged
- Operational state — still belongs to sessions/episodic
- M7c retrieval tuning — separate config

## Source

Option C from the user's M7e merge decision (PR #19 review, 2026-05-05). Recovers fields from `fe013cd` parallel-fanout impl that were dropped during BLUEPRINT alignment but had genuine architectural value.

## Connections
- [[ADR--IDENTITY-STORAGE-SHAPE]]
- [[CONCEPT--IDENTITY-LAYER]]

