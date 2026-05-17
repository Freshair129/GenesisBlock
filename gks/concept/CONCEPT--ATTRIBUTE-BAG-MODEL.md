---
id: CONCEPT--ATTRIBUTE-BAG-MODEL
phase: 1
type: concept
status: stable
tier: process
source_type: axiomatic
vault_id: default
title: Attribute bag — bring-your-own open-schema metadata
attributes:
  domain: [ucf, msp]
tags:
  - msp
  - ucf
  - concept
  - attributes
  - schema
  - plugins
crosslinks: {"references":["FRAMEWORK--UNIVERSAL-CONTEXT-FRAMEWORK","CONCEPT--SUBJECT-RESOURCE-ACTION-CONTEXT"]}
created_at: 2026-05-13T08:59:39.761+07:00
aliases:
  - CONCEPT
  - implementation_flow
  - Strategic intent / PRD
cluster: implementation_flow
role: "Strategic intent / PRD"
---

# CONCEPT — Attribute bag

> The mechanism by which **domain-specific** metadata flows through MSP's **domain-agnostic** core. Every Subject and Resource carries an open key-value bag; only policy files and classifier plugins interpret it. The core never inspects the contents.
>
> Spec section: `UNIVERSAL-CONTEXT-FRAMEWORK_spec.md` §4. Related decision: D-8 (atom metadata, not Namespace).

## Problem

A core that hardcodes `sensitivity: medical | financial | legal` cannot accommodate a new domain without a code change. Each new vertical re-implements the same plumbing (PHI tags, PCI tags, NDA tags) with slightly different schemas. Worse, hardcoded enumerations leak domain assumptions into framework code that is supposed to be generic.

At the same time, an **unconstrained** open map produces drift: every team invents its own field names, capitalization, value spaces, until policy files become impossible to audit and validation impossible to enforce.

We need a middle ground.

## Hypothesis

A two-tier model resolves both problems:

1. **Storage and propagation** treat attributes as opaque `Record<string, JsonValue>` (`AttributeBag`). The core never branches on attribute contents.
2. **Validation and discovery** are governed by **declared schema files** (one per domain or per policy pack). Classifier plugins publish the attributes they emit; policy files declare the attributes they read; a linter verifies that every consumer has a producer.

Result: domain teams add attributes by dropping schema + classifier + policy files; the framework runtime never needs to learn about them.

## Scope

In:

- TypeScript alias `AttributeBag = Record<string, JsonValue>`.
- Frontmatter convention: atoms carry `attributes:` as a top-level key in their YAML frontmatter. The atom validator does **not** constrain the contents (per D-8); validation is the schema files' responsibility.
- Schema declaration format (YAML) listing expected attribute names, types, value enums, regex patterns.
- Classifier plugin interface: `Classifier { id, outputs: string[], classify(resource): Promise<AttributeBag> }`.
- Built-in universal classifiers: `PathClassifier`, `ContentClassifier` (regex), `GraphCommunityClassifier`, `EmbeddingClusterClassifier`, `FrontmatterClassifier`.
- Provenance tracking: every attribute on a Resource may carry `__source` and `__confidence` sibling fields recording which classifier emitted it.
- Precedence: frontmatter (human) > domain-pack classifier > universal classifier.

Out:

- Domain-specific attribute taxonomies. Those ship as separate **policy packs** (`pack-medical`, `pack-finance`, `pack-source-code`) and are not part of the core.
- A formal type system over the attribute bag. JSON schema is sufficient at v1; richer typing (e.g. Cedar's strong types) can come later if needed.
- Migration of existing GKS metadata into the bag. Existing `metadata` fields stay where they are; the bag is additive.

## Why not extend GKS `Namespace` instead

`Namespace` in GKS (`tenant_id, user_id, session_id, agent_id`) is a **partition key** — immutable, indexed, cheap. Adding mutable attribute fields breaks its invariants and forces a pgvector schema migration. D-8 records the decision to keep the bag in **atom metadata** (frontmatter / JSON column) instead. This preserves:

- Namespace as the coarse, indexed filter (Layer 1 of the five-layer pipeline).
- Attributes as the fine, queryable-but-not-indexed-by-default filter (Layer 2).

Composite queries combine both: `WHERE namespace.tenant_id = $1 AND attributes->>'classification' = $2`.

## Example bags across domains

```yaml
# Medical (pack-medical)
{ classification: phi, patient_id: P000123, hipaa_covered: true }

# Finance (pack-finance)
{ classification: pci, account_tier: vip, region: eu }

# Internal code review
{ classification: internal, domain: [auth, session], team: security }

# Multi-tenant SaaS (universal)
{ tenant: acme, workspace: eng, visibility: private }
```

The framework cannot distinguish these — and that is the point.

## Verification

- Atom validator accepts arbitrary `attributes:` frontmatter without complaint.
- A new schema file (`attributes/<pack>.yaml`) is loaded at startup; its declared attributes appear in policy-author autocompletion.
- Classifier plugin can be loaded via a config entry and its outputs appear on Resources after the next re-index.
- Provenance fields (`__source`, `__confidence`) round-trip through retain → recall.

## Out of scope

- The PDP that consumes attributes — see `[[FEAT--POLICY-DECISION-POINT]]`.
- Specific classifier implementations beyond the built-in five — see `[[BLUEPRINT--PHASE-6-CLASSIFIERS]]`.
- Cross-classifier conflict resolution beyond the three-tier precedence rule.

## Source

- `docs/msp/UNIVERSAL-CONTEXT-FRAMEWORK_spec.md` §4 — bring-your-own-schema rationale, provenance model, classifier interface.
- Decision §0 D-8 — keep attributes in atom metadata, Namespace untouched.
- `[[FRAMEWORK--UNIVERSAL-CONTEXT-FRAMEWORK]]` — context for where the bag fits in the four-tuple.
- `[[CONCEPT--SUBJECT-RESOURCE-ACTION-CONTEXT]]` — the request shape that carries the bag.
