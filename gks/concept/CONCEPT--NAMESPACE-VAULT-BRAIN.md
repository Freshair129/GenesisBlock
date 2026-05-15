---
id: CONCEPT--NAMESPACE-VAULT-BRAIN
phase: 1
type: concept
status: stable
tier: process
source_type: axiomatic
vault_id: default
title: Namespace, Vault, and Brain — three layers of isolation and composition
attributes:
  domain: [ucf, msp]
tags:
  - msp
  - ucf
  - concept
  - namespace
  - vault
  - brain
  - isolation
crosslinks: {"references":["FRAMEWORK--UNIVERSAL-CONTEXT-FRAMEWORK","FRAMEWORK--MSP-ARCHITECTURE-V2","CONCEPT--IDENTITY-LAYER"]}
created_at: 2026-05-13T08:59:41.079+07:00
---

# CONCEPT — Namespace, Vault, and Brain

> Three distinct concepts share the rough idea of "isolation," but they operate at **different layers** of the system. Collapsing them produces either inflexibility, insecurity, or unobservability. This atom defines each precisely and locks in the relationship.
>
> Spec section: `UNIVERSAL-CONTEXT-FRAMEWORK_spec.md` §5.

## Problem

Developers reading the codebase and the FRAMEWORK_MASTER_SPEC consistently conflate three related-but-different ideas:

- The composite partition key in GKS (`tenant_id`, `user_id`, `session_id`, `agent_id`).
- The "workspace" / "project" concept in user-facing UX.
- The full identity envelope of the agent (`identity.json`, voice, preferences, etc.).

Obsidian's vault model adds to the confusion — its "vault" is a folder, binary on/off, and cannot reference other vaults. Users with intuition from Obsidian expect MSP's notion to behave the same way, which is **wrong** for a multi-user, federated-vault, identity-aware system.

A precise vocabulary is needed before any policy or composition logic can be written about which atoms are visible to whom.

## Hypothesis

The three concepts each serve a distinct purpose and live at a distinct layer. Naming them explicitly and defining their interfaces removes the conflation:

```
LAYER             CONCEPT         OWNED BY         PROPERTIES
─────────────────────────────────────────────────────────────────────────
Storage           Namespace       GKS              partition key; immutable; indexed; coarse
UX composition    Vault           MSP              named view over Namespaces; composable; shareable
Whole agent       Brain           MSP              Soul + Vaults + Cognitive policy + Session
```

These are not synonyms or aliases. They compose. Switching Brains changes everything; switching Vaults changes which storage partitions are read; Namespaces never switch — they are stamped at creation.

## Scope

In:

- `Namespace` — already exists in GKS (`packages/gks/src/memory/types.ts`). No schema change (per D-8). Properties: composite of `tenant_id | user_id | session_id | agent_id`; all optional; empty `{}` is the global default; stamped on every Resource at creation; filters every retrieval; bypass requires `crossNamespace: true`.
- `Vault` — new MSP concept. A named view over one or more Namespaces:

  ```ts
  interface Vault {
    id: string
    name: string
    read_from: Namespace[]           // OR-union for reads
    write_to: Namespace              // single target for writes
    default_filter?: AttributeBag    // optional ABAC filter applied to all reads
    resolution_policy?: ResolutionPolicy
  }
  ```

  A user can have multiple active Vaults; a Namespace can appear in many Vaults; a Vault can compose across organizational boundaries (e.g. `personal + my-team + company-public`).

- `Brain` — new MSP concept. The whole active agent envelope:

  ```ts
  interface Brain {
    id: string
    soul: SoulProfile                // identity / voice / preferences (existing MSP passport)
    active_vaults: VaultBinding[]    // which vaults are mounted right now
    cognitive_policy: CognitivePolicy
    current_session: SessionContext
  }
  ```

  Switching brains = switching identity + vaults + policy + session atomically. Within a brain, vaults can be added/removed/muted without losing identity.

- Mental model documented in MSP terminology guide:

  | Concept | Real-world analogy | Software analogy |
  |---|---|---|
  | Namespace | Locked filing cabinet | DB row's `tenant_id` |
  | Vault | "Documents I can access today" | Saved query / view |
  | Brain | "Me, at work, on this project" | OS user session |

Out:

- Changes to GKS `Namespace` schema (rejected by D-8).
- Implementation of Vault and Brain stores — see `FEAT--VAULT-COMPOSITION` and `FEAT--BRAIN-SESSION`.
- Membership versioning policy (snapshot vs query-time) — see spec §14 OQ-4.

## Why three layers, not one

A common temptation is to collapse these into a single concept (typically called "workspace"). It produces one of three failure modes:

- **Workspace = Namespace** → no sharing across users.
- **Workspace = Vault stored as a Namespace** → policy bypass is one config flip away; auditors cannot tell the difference between "no access granted" and "access removed."
- **Workspace = everything** → audit log cannot answer "why did query Q return atom A?" because the workspace conflates partition + view + identity.

The three-layer split keeps each layer's invariants enforceable:

- Namespace stays cheap and immutable (precondition for partition-pruning indexes).
- Vault stays flexible and composable (precondition for sharing and "vault-in-vault" UX).
- Brain stays the human-facing concept (precondition for understanding "switch context" without it meaning "lose your identity").

## Relationship to existing MSP concepts

- **Soul / Passport** (`CONCEPT--IDENTITY-LAYER`) — Brain contains Soul. The passport pattern (identity flowing with the agent) is unchanged.
- **Workspace state at `.brain/msp/projects/<ns>/`** — this is **Namespace storage**, not a Vault. A Vault may read from one such workspace, or compose multiple.
- **`MSP_PROJECT` environment variable** — currently selects a single Namespace. After Vault landing, this becomes "the default Vault id for the session" — backward-compatible with a one-Namespace-one-Vault Vault.

## Why "Vault" and not "Workspace" / "Brain" / "Lens"

- **Workspace** is overloaded (Notion, Slack, IDE). It does not suggest security or access control.
- **Brain** is reserved for the outer envelope. Using it for the inner concept would force renaming again later.
- **Lens** is too cinematic; suggests view-only with no write/sharing.
- **Vault** is precedented (Obsidian, HashiCorp Vault, KeePass), suggests both container and access control, and is what users already intuit. MSP improves on Obsidian's binary-vault model by allowing composition and multi-active-vaults.

## Verification

- `Vault` and `Brain` TypeScript types compile.
- A user with two Vaults active (`personal + team`) sees the union of atoms from their underlying Namespaces on a single `recall()` call.
- Switching `current_brain` clears the active session and reloads vaults atomically.
- Audit log records distinguish "Namespace filter dropped this atom" from "Vault membership does not include this Namespace."

## Out of scope

- Cross-Namespace promote action (move an atom from `personal` to `team`) — see `FEAT--CROSS-NAMESPACE-PROMOTE`.
- Vault sharing protocol (granting / revoking access) — see `FEAT--VAULT-COMPOSITION`.
- Membership versioning policy — see spec §14 OQ-4; working assumption is session-snapshot with explicit `refresh`.

## Source

- `docs/msp/UNIVERSAL-CONTEXT-FRAMEWORK_spec.md` §5 — full Namespace / Vault / Brain definitions and anti-patterns.
- `packages/gks/src/memory/types.ts` — existing `Namespace` interface (unchanged).
- `CONCEPT--IDENTITY-LAYER` — Soul layer that Brain contains.
- `FRAMEWORK--MSP-ARCHITECTURE-V2` — parent three-layer ecosystem; Vault is positioned at the Memory OS layer.
