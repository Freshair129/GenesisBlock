---
id: CONCEPT--ATOMIC-WRITE-CONTRACT
phase: 1
type: concept
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: Atomic write contract — the schema every gks/ atom obeys
tags: &a1
  - msp
  - contract
  - schema
  - atomic
crosslinks: &a2
  references:
    - FRAMEWORK--MSP-ARCHITECTURE-V2
    - CONCEPT--KNOWLEDGE-LAYERS-V2
    - CONCEPT--CODEGEN-MICROTASK-CONTRACT
    - SPEC--ATOM-REGISTRY-SCHEMA
created_at: 2026-05-03T14:01:51.814+07:00
aliases: &a3
  - CONCEPT
  - implementation_flow
  - Strategic intent / PRD
cluster: implementation_flow
role: Strategic intent / PRD
attributes:
  id: CONCEPT--ATOMIC-WRITE-CONTRACT
  phase: 1
  type: concept
  status: stable
  vault_id: default
  tier: genesis
  source_type: axiomatic
  title: Atomic write contract — the schema every gks/ atom obeys
  tags: *a1
  crosslinks: *a2
  created_at: 2026-05-03T14:01:51.814+07:00
  aliases: *a3
  cluster: implementation_flow
  role: Strategic intent / PRD
  attributes:
    id: CONCEPT--ATOMIC-WRITE-CONTRACT
    phase: 1
    type: concept
    status: stable
    vault_id: default
    tier: genesis
    source_type: axiomatic
    title: Atomic write contract — the schema every gks/ atom obeys
    tags: *a1
    crosslinks: *a2
    created_at: 2026-05-03T14:01:51.814+07:00
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

# CONCEPT — atomic write contract

Every file in `gks/<type>/` obeys a single schema: required frontmatter fields, conditional fields based on `type`, a forbidden-fields blacklist, and field-level constraints. The validator (`src/validator/`) enforces it.

## The contract has five parts

| Part | Source | Atom |
|---|---|---|
| Required fields | §4.1 | `[[ADR--ATOMIC-CONTRACT-SCHEMA]]` (TBD M3) and `[[SPEC--ATOM-REGISTRY-SCHEMA]]` |
| Conditional fields | §4.2 | same |
| Forbidden fields | §4.3 | `[[ADR--FORBIDDEN-FIELDS-LIST]]` |
| Field constraints | §4.4 | same |
| Anti-hallucination rules | §4.5 | `[[ADR--ANTI-HALLUCINATION-RULES]]` |
| Epistemic + crosslinks block | §4.6 | `[[CONCEPT--EPISTEMIC-METADATA]]` + `[[FRAMEWORK--CROSSLINKS-VOCABULARY]]` |
| **Codegen Contract** | §5 | [[CONCEPT--CODEGEN-MICROTASK-CONTRACT]] (Sibling contract) |

## What "atomic" means here

An *atom* in MSP terminology is a single markdown file under `gks/<type>/<ID>.md` carrying:

1. **A frontmatter block** between two `---` delimiters with declarative metadata (id, phase, type, status, vault_id, summary, created_at/by, optional crosslinks/linked_symbols/geography).
2. **A body** in markdown that humans can read.
3. **A unique ID** that matches the filename and the canonical pattern (`[[TYPE--SLUG]]` uppercase, or `ADR-NNN` numeric).

Atoms are **content-addressable through the index** — `gks/00_index/atomic_index.jsonl` is a derived flat list of every atom's frontmatter (rebuilt by `npm run msp:index`).

## What MSP does NOT contractually enforce

- **Quality of the body text.** The validator only inspects frontmatter shape + wikilink integrity. Whether an ADR's "Decision" section is well-reasoned is a human review concern.
- **Code-symbol existence.** `linked_symbols` claims are not verified against the actual codebase — that's GitNexus / `gks lookup-by-symbol` territory.
- **Cross-tenant authority.** Multi-tenancy is the orchestrator's job above MSP.

## Why a contract at all

Without one, four predictable failure modes appear (see [[CONCEPT--KNOWLEDGE-LAYERS-V2]] § "Legacy Failure Modes"). The contract makes each failure mode mechanically detectable.

## Source

`msp_spec.md` §4 (Atomic Write Contract).

## Connections
- [[FRAMEWORK--MSP-ARCHITECTURE-V2]]

