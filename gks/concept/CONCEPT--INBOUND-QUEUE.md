---
id: CONCEPT--INBOUND-QUEUE
phase: 1
type: concept
status: superseded
vault_id: default
tier: process
source_type: axiomatic
title: Inbound queue — sole legal write path to gks/
tags:
  - msp
  - inbound
  - write-path
  - gatekeeper
  - superseded
crosslinks:
  references:
    - FRAMEWORK--MSP-ARCHITECTURE-V2
  superseded_by:
    - CONCEPT--KNOWLEDGE-LAYERS-V2
created_at: 2026-05-03T14:01:50.297+07:00
aliases:
  - CONCEPT
  - implementation_flow
  - Strategic intent / PRD
cluster: implementation_flow
role: Strategic intent / PRD
attributes:
  domain: concept
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: true
  secret_type: high_entropy_string
  leak_risk: high
  encryption_level: none
---

> ⚠️ **Superseded by `[[CONCEPT--KNOWLEDGE-LAYERS-V2]]`** (Phase 4 of `[[BLUEPRINT--INBOUND-TO-CANDIDATES-MIGRATION]]`, 2026-05-09). The inbound queue has been removed; runtime atom proposals now write to `.brain/msp/projects/<ns>/candidates/` via the `msp_candidate` MCP tool. The body below is preserved as historical context.

# CONCEPT — inbound queue

The inbound queue is the only place agents are allowed to drop atoms intended for `gks/`. Direct writes to `gks/<type>/` are blocked by filesystem ACL and the pre-commit hook.

## Why a queue at all

Without one, four failure modes appear immediately:

1. **SSOT corruption** — agents overwrite approved atoms with wrong data.
2. **ID collision** — two agents pick the same `ADR-007`.
3. **Frontmatter hallucination** — agents synthesise fields like `commit_hash`.
4. **Dangling crosslinks** — `[[FEAT--XXX]]` points at nothing.

The queue gives the validator + human reviewer a chokepoint where each violation is caught before it pollutes the canonical store.

## Lifecycle

```
.brain/msp/projects/<ns>/inbound/<ID>.rev-<reviewId>.md
   │
   ├── (a) validator passes  → human approves  → gks/<type>/<ID>.md
   │
   ├── (b) validator fails   → .../rejected/<YYYY-MM-DD>/  + rejection_reason.md
   │
   └── (c) reviewer rejects  → same rejected dir + reviewer + reason
```

## Filename convention

```
{yyyymmddHHMMSS}-{agent_id}-{proposal_type}-{slug}.md   # legacy / spec
{ID}.rev-{reviewId}.md                                   # GKS published
```

The current `@freshair129/gks` package uses the `rev-` form; MSP accepts either at the validator boundary.

## What this concept does NOT describe

- The exact frontmatter envelope → see `[[CONCEPT--SUBMISSION-ENVELOPE]]`
- The proposal types (new/update/supersede/deprecate) → see `[[CONCEPT--PROPOSAL-TYPES]]`
- The promote command + level transitions → see `[[ADR--PROMOTION-LEVELS]]`

## Source

`msp_spec.md` §3 (Inbound Flow).

## Connections

- [[FRAMEWORK--MSP-ARCHITECTURE-V2]]
