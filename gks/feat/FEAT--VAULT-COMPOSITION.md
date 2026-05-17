---
id: FEAT--VAULT-COMPOSITION
phase: 2
type: feat
status: active
tier: process
source_type: axiomatic
vault_id: default
title: Vault composition — multi-namespace views with one write target
tags:
  - msp
  - ucf
  - feat
  - vault
  - namespace
crosslinks: {"references":["CONCEPT--NAMESPACE-VAULT-BRAIN","ADR--VAULT-NAMESPACE-LAYERING","FRAMEWORK--UNIVERSAL-CONTEXT-FRAMEWORK"]}
created_at: 2026-05-14T19:42:02.052+07:00
aliases:
  - FEAT
  - implementation_flow
  - Feature spec
cluster: implementation_flow
role: "Feature spec"
---

# FEAT — Vault composition

> The user-facing API contract for the `Vault` concept in `[[CONCEPT--NAMESPACE-VAULT-BRAIN]]`, structurally shaped by `[[ADR--VAULT-NAMESPACE-LAYERING]]` (Vault is a view, never a stored partition).

## User-facing behaviour

Vault configs live in `~/.msp/vaults/*.yaml`. Each file declares one Vault:

```yaml
id: alice-engineering
name: Alice @ Engineering
read_from:
  - { user_id: alice }
  - { tenant_id: eng-team }
  - { tenant_id: company-public }
write_to: { user_id: alice }
default_filter: { visibility: [internal, public] }    # optional
resolution_policy: { default: { hop_0: FULL, hop_1+: MENTION } }   # optional
```

A new module `packages/msp/src/vault/` exports:

```ts
function loadVaults(dir: string): VaultRegistry
function resolveVault(registry: VaultRegistry, id: string): Vault

// Vault → GKS query parameters
function vaultReadNamespaces(vault: Vault): Namespace[]    // OR-union for recall
function vaultWriteNamespace(vault: Vault): Namespace      // single target for retain
```

Behaviour contract:

- **Read** through a Vault = recall filtered by the OR-union of `read_from` Namespaces, then `default_filter` applied as a Layer 2 attribute filter.
- **Write** through a Vault = retain stamped with the single `write_to` Namespace. A Vault with no `write_to` is read-only; retain through it errors.
- **A Vault is never stamped on a Resource** (per `[[ADR--VAULT-NAMESPACE-LAYERING]]`). It is resolved at query time into Namespace filters.
- **`MSP_PROJECT` env var** becomes "default Vault id for the session", backward-compatible: a single-Namespace Vault behaves identically to the old single-Namespace `MSP_PROJECT`.
- **Multiple Vaults can be active** in one Brain; recall across active Vaults is the OR-union of all their `read_from` sets.
- **Vault membership is config**: editing a `*.yaml` and calling `loadVaults` again (or hot-reload) changes the view; no atom data moves.

CLI `msp-vault`:

```sh
msp-vault list
msp-vault show alice-engineering          # resolved Namespace sets
msp-vault check alice-engineering --atom=CONCEPT--FOO   # is this atom visible in this vault?
```

## Verification

- A Vault with `read_from: [{user_id: alice}, {tenant_id: eng-team}]` returns the union of atoms from both Namespaces on a single `recall()`.
- Retain through `alice-engineering` stamps `{ user_id: alice }`, never the team Namespace.
- A read-only Vault (no `write_to`) rejects retain with a clear error.
- Two active Vaults → recall returns the union; an atom in neither is absent.
- Editing a vault YAML + reload changes visibility without touching the atom store (verified by diffing atom mtimes — unchanged).
- `msp-vault check` agrees with actual `recall` visibility.

## Out of scope

- Vault sharing protocol (granting / revoking another user access to a Vault config) — future FEAT.
- Cross-Namespace promote (moving an atom from `personal` to `team`) — `[[FEAT--CROSS-NAMESPACE-PROMOTE]]`, not this PR.
- Vault membership versioning (what happens to in-flight sessions when a Namespace leaves a Vault) — spec §14 OQ-4; working assumption session-snapshot.
- `Brain` assembly (Soul + active Vaults + cognitive policy) — separate FEAT.
- `resolution_policy` semantics — owned by `[[FEAT--RESOLUTION-EXPAND-ON-DEMAND]]` and `[[CONCEPT--RESOLUTION-GRADIENT]]`; this FEAT only carries the field through.

## Source

- `docs/msp/UNIVERSAL-CONTEXT-FRAMEWORK_spec.md` §5 — Vault definition and composition.
- `[[CONCEPT--NAMESPACE-VAULT-BRAIN]]` — the concept this FEAT implements.
- `[[ADR--VAULT-NAMESPACE-LAYERING]]` — Vault is a runtime view, never a stored partition.
- `packages/gks/src/memory/types.ts` — the `Namespace` type Vaults compose.

## Connections
- [[FRAMEWORK--UNIVERSAL-CONTEXT-FRAMEWORK]]

