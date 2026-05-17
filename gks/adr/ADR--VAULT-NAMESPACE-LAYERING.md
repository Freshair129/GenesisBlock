---
id: ADR--VAULT-NAMESPACE-LAYERING
phase: 2
type: adr
status: stable
tier: process
source_type: axiomatic
vault_id: default
title: Vault / Namespace / Brain layering — Vault is a view, not a stored partition
tags:
  - msp
  - ucf
  - adr
  - vault
  - namespace
  - brain
crosslinks:
  references:
    - CONCEPT--NAMESPACE-VAULT-BRAIN
    - FRAMEWORK--UNIVERSAL-CONTEXT-FRAMEWORK
    - CONCEPT--IDENTITY-LAYER
created_at: 2026-05-14T18:37:54.239+07:00
aliases:
  - ADR
  - implementation_flow
  - Architecture decision record
cluster: implementation_flow
role: Architecture decision record
attributes:
  domain: adr
---

# ADR — Vault / Namespace / Brain layering

> Architecture decision derived from `[[CONCEPT--NAMESPACE-VAULT-BRAIN]]` (spec §5). No spec decision id.

## Context

`[[CONCEPT--NAMESPACE-VAULT-BRAIN]]` establishes three concepts that all touch "isolation" but operate at different layers: `Namespace` (storage partition), `Vault` (composition view), `Brain` (whole agent envelope). The concept atom argues they must stay distinct. This ADR commits to the **concrete structural decision** that makes them distinct: **where each lives and how Vault is represented**.

The temptation — repeatedly seen in similar systems — is to make `Vault` a stored entity that *is* a kind of Namespace ("a vault is just a tenant"). That collapse is what this ADR forbids.

## Decision

**`Vault` is a runtime view object, never a stored partition. The three layers are structurally separated:**

1. **`Namespace`** — owned by GKS. Stored, immutable, indexed. Stamped on every Resource at creation. The only thing that physically partitions storage.
2. **`Vault`** — owned by MSP. A **view**: `{ id, name, read_from: Namespace[], write_to: Namespace, default_filter?, resolution_policy? }`. A Vault is configuration, not data. It is resolved at query time into an OR-union of Namespace filters. It is **never** stamped on a Resource.
3. **`Brain`** — owned by MSP. The session envelope: `{ soul, active_vaults, cognitive_policy, current_session }`. Contains Vaults by reference; contains Soul (`[[CONCEPT--IDENTITY-LAYER]]`) directly.

Structural rules enforced:

- **A Resource is stamped with a Namespace, never with a Vault.** "Which vault is this atom in" is not a question that has an answer — a Namespace can belong to many Vaults.
- **Vault membership is config, resolved at query time.** Vault configs live in `~/.msp/vaults/*.yaml`, not in the atom store.
- **`MSP_PROJECT` becomes "default Vault id"**, backward-compatible: a single-Namespace Vault behaves exactly like the old single-Namespace `MSP_PROJECT`.
- **Switching Brains is atomic**; adding/removing Vaults within a Brain does not touch identity or storage.

## Consequences

Positive:

- The "Vault stored as a Namespace" failure mode is structurally impossible — a Vault has no storage representation to confuse with a partition.
- Audit logs can distinguish "Namespace filter excluded this atom" from "Vault membership does not include that Namespace" — because the two are different objects evaluated at different layers.
- Vault sharing is a clean ACL operation on a config object, not a data migration.
- Multi-active-vaults and "vault-in-vault" composition fall out naturally — a Vault's `read_from` is just a list.
- GKS stays unaware of Vault and Brain entirely — they are pure MSP concepts. Preserves the GKS↔MSP boundary.

Negative / accepted costs:

- Vault resolution happens on every query (config → Namespace OR-union). Cheap, and cacheable per session.
- Vault membership versioning becomes a real question (if a Namespace is removed from a Vault, what happens to in-flight sessions?). Deliberately left to spec §14 OQ-4; working assumption is session-snapshot with explicit `refresh`.
- Three concepts to teach instead of one. Accepted — `[[CONCEPT--NAMESPACE-VAULT-BRAIN]]` exists precisely to carry that teaching cost, and the analogies (filing cabinet / saved query / OS session) make it tractable.

## Alternatives considered

**Vault = a Namespace (collapse the two).** Rejected — this is the central anti-pattern. It makes cross-user sharing impossible (a Namespace has one owner) or makes policy bypass trivial (if a Vault is a tenant, joining a Vault rewrites your partition key). Auditability dies: you cannot tell "access never granted" from "access removed."

**Vault = a stored entity with its own table and atom-membership rows.** Rejected for v1: materialising Vault membership as rows means every atom move touches Vault tables, and Vault edits become data migrations. A view resolved from config is simpler and matches the actual semantics ("a Vault is a saved query").

**No Brain layer — just Vaults + Soul as peers.** Rejected: without Brain as the envelope, "switch context" has no atomic unit — you would switch Soul and Vaults separately, with a window where they are inconsistent. Brain makes the switch atomic.

## Source

- `docs/msp/UNIVERSAL-CONTEXT-FRAMEWORK_spec.md` §5 — Namespace / Vault / Brain definitions and anti-patterns.
- `[[CONCEPT--NAMESPACE-VAULT-BRAIN]]` — the concept this ADR structurally commits.
- `[[CONCEPT--IDENTITY-LAYER]]` — Soul, which Brain contains.
- `packages/gks/src/memory/types.ts` — `Namespace`, owned by GKS, unchanged.

## Connections
- [[FRAMEWORK--UNIVERSAL-CONTEXT-FRAMEWORK]]

