---
id: ADR--BRING-YOUR-OWN-ATTRIBUTES
phase: 2
type: adr
status: stable
tier: process
source_type: axiomatic
vault_id: default
title: Bring-your-own attributes — open bag in atom metadata, GKS Namespace untouched
tags: &a1
  - msp
  - ucf
  - adr
  - attributes
  - schema
crosslinks: &a2
  references:
    - CONCEPT--ATTRIBUTE-BAG-MODEL
    - CONCEPT--NAMESPACE-VAULT-BRAIN
    - FRAMEWORK--UNIVERSAL-CONTEXT-FRAMEWORK
created_at: 2026-05-14T18:37:51.890+07:00
aliases: &a3
  - ADR
  - implementation_flow
  - Architecture decision record
cluster: implementation_flow
role: Architecture decision record
attributes:
  id: ADR--BRING-YOUR-OWN-ATTRIBUTES
  phase: 2
  type: adr
  status: stable
  tier: process
  source_type: axiomatic
  vault_id: default
  title: Bring-your-own attributes — open bag in atom metadata, GKS Namespace
    untouched
  tags: *a1
  crosslinks: *a2
  created_at: 2026-05-14T18:37:51.890+07:00
  aliases: *a3
  cluster: implementation_flow
  role: Architecture decision record
  attributes:
    id: ADR--BRING-YOUR-OWN-ATTRIBUTES
    phase: 2
    type: adr
    status: stable
    tier: process
    source_type: axiomatic
    vault_id: default
    title: Bring-your-own attributes — open bag in atom metadata, GKS Namespace
      untouched
    tags: *a1
    crosslinks: *a2
    created_at: 2026-05-14T18:37:51.890+07:00
    aliases: *a3
    cluster: implementation_flow
    role: Architecture decision record
    attributes:
      id: ADR--BRING-YOUR-OWN-ATTRIBUTES
      phase: 2
      type: adr
      status: stable
      tier: process
      source_type: axiomatic
      vault_id: default
      title: Bring-your-own attributes — open bag in atom metadata, GKS Namespace
        untouched
      tags: *a1
      crosslinks: *a2
      created_at: 2026-05-14T18:37:51.890+07:00
      aliases: *a3
      cluster: implementation_flow
      role: Architecture decision record
      attributes:
        domain: adr
      domain: adr
      language: markdown
      is_test: false
      is_entrypoint: false
      has_secret: true
      secret_type: high_entropy_string
      leak_risk: high
      encryption_level: none
    domain: adr
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: true
    secret_type: high_entropy_string
    leak_risk: high
    encryption_level: none
  domain: adr
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: true
  secret_type: high_entropy_string
  leak_risk: high
  encryption_level: none
---

# ADR — Bring-your-own attributes

> Resolves decision **D-8** in `UNIVERSAL-CONTEXT-FRAMEWORK_spec.md` §0.

## Context

`[[CONCEPT--ATTRIBUTE-BAG-MODEL]]` requires that domain-specific metadata (PHI tags, PCI tags, internal-only flags) flow through the domain-agnostic core without the core hardcoding any domain enumeration. The open question is **where the attribute bag physically lives**:

1. **Extend GKS `Namespace`** — add an `attributes?: AttributeBag` field to the existing `Namespace` interface.
2. **Atom metadata** — carry attributes in atom frontmatter (`attributes:` key) and the corresponding JSON column in the pgvector / storage backends.
3. **Separate attributes table** — a normalized side table joined on atom id.

This matters because `Namespace` in GKS is a **partition key** — immutable, indexed, coarse, and load-bearing for partition-pruning queries (Layer 1 of the five-layer pipeline). Anything that touches its shape touches GKS's storage invariants and forces a pgvector schema migration.

## Decision

**Adopt option 2 — attributes live in atom metadata (frontmatter `attributes:` + JSON column). GKS `Namespace` is untouched.**

- `Namespace` stays the 4-field composite key (`tenant_id | user_id | session_id | agent_id`), immutable, indexed.
- `AttributeBag = Record<string, JsonValue>` is stored as atom metadata — in frontmatter for `.md` atoms, in a JSON column for the vector backend.
- Composite queries combine both layers: `WHERE namespace.tenant_id = $1 AND attributes->>'classification' = $2`.
- The atom validator does **not** constrain `attributes:` contents — validation is delegated to per-domain schema files (per `[[CONCEPT--ATTRIBUTE-BAG-MODEL]]`).

## Consequences

Positive:

- GKS invariants preserved: `Namespace` stays immutable, indexed, coarse. No pgvector schema migration for the partition key.
- The two filter layers stay cleanly separated: Namespace = coarse + indexed (Layer 1), attributes = fine + queryable-but-not-indexed-by-default (Layer 2).
- Attributes are mutable (re-classification, re-tagging by classifiers) without violating the "Namespace is immutable" rule.
- GKS remains publishable as a standalone library — UCF adds metadata conventions, not GKS schema changes. Respects the GKS↔MSP boundary (`[[ADR--MONOREPO-STRUCTURE]]`).

Negative / accepted costs:

- Attribute-based filters are not indexed by default. For large vaults with heavy attribute filtering, a GIN index on the JSON column may be needed later — that is an additive migration, not a breaking one.
- Two places to read when reasoning about a Resource's access profile (its Namespace and its attributes). Accepted: they answer different questions (WHERE vs WHO) and conflating them was the failure mode this ADR avoids.

## Alternatives considered

**Option 1 — extend GKS `Namespace` with `attributes`.** Rejected. It breaks the "Namespace is an immutable partition key" invariant the moment attributes become mutable (which they must be — classifiers re-tag). It forces a pgvector schema migration on the partition key. It pushes domain-shaped data into GKS, violating the storage-engine scope. The apparent upside — "one place for all metadata" — is illusory: Namespace and attributes answer different questions and should not share a home.

**Option 3 — separate attributes table.** Rejected for v1. A normalized side table is more flexible for complex attribute queries but adds a join to every Layer 2 evaluation and a second write path on every retain. Overkill while the attribute surface is small. Reconsider if attribute query patterns outgrow JSON-column filtering.

## Source

- `docs/msp/UNIVERSAL-CONTEXT-FRAMEWORK_spec.md` §0 (D-8), §4 — attribute bag storage rationale.
- `[[CONCEPT--ATTRIBUTE-BAG-MODEL]]` — the bag model this ADR places.
- `[[CONCEPT--NAMESPACE-VAULT-BRAIN]]` — why Namespace must stay a pure partition key.
- `packages/gks/src/memory/types.ts` — the `Namespace` interface left unchanged.

## Connections

- [[FRAMEWORK--UNIVERSAL-CONTEXT-FRAMEWORK]]
