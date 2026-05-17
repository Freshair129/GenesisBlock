---
id: CONCEPT--TAXONOMY-V2-3
phase: 1
type: concept
status: stable
tier: genesis
source_type: axiomatic
vault_id: default
title: Taxonomy v2.3 — knowledge type prefixes for the Genesis Block era
tags: &a1
  - msp
  - taxonomy
  - knowledge
  - genesis-block
  - foundation
crosslinks: &a2
  references:
    - CONCEPT--KNOWLEDGE-LAYERS-V2
    - CONCEPT--ATOMIC-WRITE-CONTRACT
    - FRAMEWORK--CROSSLINKS-VOCABULARY
    - ADR--TAXONOMY-V2-3-MIGRATION
    - SPEC--GENESIS-BLOCK-MANIFEST
created_at: 2026-05-13T12:21:49+07:00
aliases: &a3
  - CONCEPT
  - implementation_flow
  - Strategic intent / PRD
cluster: implementation_flow
role: Strategic intent / PRD
attributes:
  id: CONCEPT--TAXONOMY-V2-3
  phase: 1
  type: concept
  status: stable
  tier: genesis
  source_type: axiomatic
  vault_id: default
  title: Taxonomy v2.3 — knowledge type prefixes for the Genesis Block era
  tags: *a1
  crosslinks: *a2
  created_at: 2026-05-13T12:21:49+07:00
  aliases: *a3
  cluster: implementation_flow
  role: Strategic intent / PRD
  attributes:
    id: CONCEPT--TAXONOMY-V2-3
    phase: 1
    type: concept
    status: stable
    tier: genesis
    source_type: axiomatic
    vault_id: default
    title: Taxonomy v2.3 — knowledge type prefixes for the Genesis Block era
    tags: *a1
    crosslinks: *a2
    created_at: 2026-05-13T12:21:49+07:00
    aliases: *a3
    cluster: implementation_flow
    role: Strategic intent / PRD
    attributes:
      domain: concept
    domain: concept
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: true
    secret_type: high_entropy_string
    leak_risk: high
    encryption_level: none
  domain: concept
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: true
  secret_type: high_entropy_string
  leak_risk: high
  encryption_level: none
---

# CONCEPT — Taxonomy v2.3

## Why this concept exists

The atomic-knowledge prefix taxonomy expanded organically through ADR-012 (`extended-taxonomy`) and several follow-ups. Two pressures forced a refit:

1. **Genesis Block** introduces a *composite* unit — an executable knowledge engine assembled from atoms. (The term is pinned down in `[[SPEC--GENESIS-BLOCK-MANIFEST]]` to disambiguate from the storage-engine `[[CONCEPT--GENESIS-GRAPH-BACKEND]]`.) The composite needs a manifest atom (one entry-point file listing its members). Before v2.3 the `FRAME--` prefix was overloaded with "architectural framework" content (now-renamed `[[FRAMEWORK--MSP-ARCHITECTURE-V2]]`, `[[FRAMEWORK--PHASE-GOVERNANCE]]`, …). v2.3 first reserved `FRAME--` for the Block Manifest role; a follow-up rename retired `FRAME--` in favour of `GENESIS--` because `FRAME--` collided visually with `FRAMEWORK--`.
2. **Vocabulary drift**: `GUARDRAIL--` is verbose relative to the four-letter peers (`GUARD`, `STACK`, `SAFETY`). New prefixes proposed for the engine layer (`STACK`, `SPEC`, `MOD`, `COGNITIVE`, `SAFETY`) had no home.

v2.3 reorganises into six layers, **resolves the `FRAME--` overload by carving out `FRAMEWORK--`**, and adds the engine-layer prefixes.

## The six layers

### 1. Core Engine & Structure

| Prefix | Role |
|---|---|
| `GENESIS--` | **Block Manifest** — the index file that aggregates atoms into a Genesis Block (composite knowledge engine). One `GENESIS--<NAME>.md` per block. Frontmatter contract: `[[SPEC--GENESIS-BLOCK-MANIFEST]]`. (Replaces the v2.3 placeholder prefix `FRAME--`, which was retired due to visual collision with `FRAMEWORK--`.) |
| `STACK--` | Technology stack — language/runtime/library inventory for a subsystem |
| `SPEC--` | Specification — JSON Schema, API data shape, wire format |
| `MOD--` | Module definition — scope + public API of a module (e.g. `[[MOD--IDENTITY]]`) |

### 2. Governance & Rules

| Prefix | Role |
|---|---|
| `FRAMEWORK--` | **(new)** Governance framework or architectural blueprint — DACI, MSP-Architecture, phase governance, scaling levels (formerly `FRAME--` in v2.2 and below) |
| `GUARD--` | **(renamed)** Structural guard — data-integrity invariants. Supersedes `GUARDRAIL--` |
| `PROTO--` | Internal protocol — machine-enforced invariants checked by the validator |
| `SAFETY--` | Ethical safety — AI alignment + behavioural guardrails |

### 3. Cognitive & Logic

| Prefix | Role |
|---|---|
| `COGNITIVE--` | Mental model — interpretive lens / psychological model (e.g. Erikson stages, Ego Death) |
| `ALGO--` | Execution logic — algorithmic steps the runtime can execute |
| `CONCEPT--` | Definition — abstract meaning, theory, vision (PRDs included) |

### 4. Operational & Supporting

| Prefix | Role |
|---|---|
| `RUNBOOK--` | Operational SOP |
| `FLOW--` | Process flow / sequence diagram |
| `PROTOCOL--` | External interface (API/MCP) for A2A |
| `SKILL--` | Agent capability |
| `ENTITY--` | Identity / data-model entity |

### 5. Development Lifecycle

- **Planning**: `FEAT--`, `FR--`, `NFR--`, `SLO--`, `RISK--`
- **Design**: `ADR--`, `BLUEPRINT--` (P3)
- **Execution & Audit**: `AUDIT--`, `ISSUE--`, `INC--`, `HOTFIX--`

### 6. Legacy / Preserved (non-v2.3 prefixes)

These prefixes exist in GKS today and are **preserved as legacy**. They remain valid until each receives an explicit deprecation ADR:

- `IDEA--`, `MASTER--`, `POLICY--`, `PERSONA--`, `REQ--`, `CONSTRAINT--`
- `API--`, `ENDPOINT--`, `ENTRYPOINT--`, `PARAMS--`
- `INSIGHT--`, `FACT--`, `RULE--` (auto-extracted by Consolidator)

Authors may keep using them; v2.3 makes no breaking demand on the legacy set.

## What changes (the redefinitions)

### `GENESIS--` introduced (was `FRAME--` placeholder)

| | Before v2.3 | After v2.3 |
|---|---|---|
| Meaning | "Architectural framework / methodology / code standards" | "Block Manifest" — runtime entry-point for a Genesis Block |
| Examples (before) | `FRAME-`​`-MSP-ARCHITECTURE-V2`, `FRAME-`​`-PHASE-GOVERNANCE` (since renamed to `FRAMEWORK--`) | — |
| Examples (after) | `[[GENESIS--IDENTITY-ENGINE]]` (proposed), future engine manifests | — |

Migration: all 9 existing `FRAME--*` atoms are renamed to `FRAMEWORK--*` (see `[[ADR--TAXONOMY-V2-3-MIGRATION]]`).

### `GUARDRAIL--` → `GUARD--`

Pure rename for consistency with peer four-letter prefixes. No `GUARDRAIL--` atoms currently exist in the repo, so the migration is doc-only (taxonomy tables in CLAUDE.md and KNOWLEDGE-TYPES.md).

## Crosslink hygiene

After the migration, any wikilink that still says `[[FRAME--X]]` where `X` was renamed must be updated to `[[FRAMEWORK--X]]`. The migration script handles the 270 references across 133 markdown files. `gks validate --links` must pass.

## Out of scope for this concept

- **Block Manifest schema**: the frontmatter shape of a `GENESIS--<NAME>` Block Manifest (which atoms it lists, in what order, with what authority) is specified by the follow-up `[[SPEC--GENESIS-BLOCK-MANIFEST]]` atom, not here.
- **Source code refactor for new prefix recognition**: the validator + scale-gate + SSOT priority list updates are part of the migration ADR's scope, not the taxonomy spec itself.
- **Atom contradiction across prefixes**: handled by the existing contradiction policy (`[[MASTER--ATOM-CONTRADICTION-POLICY]]`).

## Connections
- [[CONCEPT--KNOWLEDGE-LAYERS-V2]]
- [[CONCEPT--ATOMIC-WRITE-CONTRACT]]
- [[FRAMEWORK--CROSSLINKS-VOCABULARY]]

