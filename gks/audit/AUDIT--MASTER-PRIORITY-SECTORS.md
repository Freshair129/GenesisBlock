---
id: AUDIT--MASTER-PRIORITY-SECTORS
phase: 6
type: audit
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: Master priority sectors — P0-P4 design + RCA promotion + 3-Master
  migration + CLAUDE.md restructure
tags: &a1
  - msp
  - audit
  - master
  - priority
  - sectors
  - rca
  - claude-md
crosslinks: &a2
  references:
    - CONCEPT--MASTER-PRIORITY-SECTORS
    - ADR--CLAUDE-MD-MASTER-PRIORITY-SECTORS
    - CONCEPT--ROOT-CAUSE-ANALYSIS
    - ADR--MASTER-PROMOTION-ROOT-CAUSE-ANALYSIS
    - MASTER--ROOT-CAUSE-ANALYSIS
    - BLUEPRINT--MSP-MASTER-COMPOSE-LOADER
    - PROTO--MASTER-PRIORITY-AUTHORITY
    - FRAMEWORK--KNOWLEDGE-3-TIER
linked_symbols: &a3
  - file: CLAUDE.md
  - file: gks/master/MASTER--ROOT-CAUSE-ANALYSIS.md
  - file: gks/master/MASTER--MSP-DOC-TO-CODE.md
  - file: gks/master/MASTER--ATOM-CONTRADICTION-POLICY.md
  - file: gks/concept/CONCEPT--ROOT-CAUSE-ANALYSIS.md
  - file: gks/adr/ADR--MASTER-PROMOTION-ROOT-CAUSE-ANALYSIS.md
  - file: gks/concept/CONCEPT--MASTER-PRIORITY-SECTORS.md
  - file: gks/adr/ADR--CLAUDE-MD-MASTER-PRIORITY-SECTORS.md
  - file: gks/blueprint/BLUEPRINT--MSP-MASTER-COMPOSE-LOADER.md
  - file: gks/proto/PROTO--MASTER-PRIORITY-AUTHORITY.md
created_at: 2026-05-17T03:05:00.000+07:00
aliases: &a4
  - AUDIT
  - implementation_flow
  - Test results / quality report
cluster: implementation_flow
role: Test results / quality report
attributes:
  id: AUDIT--MASTER-PRIORITY-SECTORS
  phase: 6
  type: audit
  status: stable
  vault_id: default
  tier: process
  source_type: axiomatic
  title: Master priority sectors — P0-P4 design + RCA promotion + 3-Master
    migration + CLAUDE.md restructure
  tags: *a1
  crosslinks: *a2
  linked_symbols: *a3
  created_at: 2026-05-17T03:05:00.000+07:00
  aliases: *a4
  cluster: implementation_flow
  role: Test results / quality report
  attributes:
    id: AUDIT--MASTER-PRIORITY-SECTORS
    phase: 6
    type: audit
    status: stable
    vault_id: default
    tier: process
    source_type: axiomatic
    title: Master priority sectors — P0-P4 design + RCA promotion + 3-Master
      migration + CLAUDE.md restructure
    tags: *a1
    crosslinks: *a2
    linked_symbols: *a3
    created_at: 2026-05-17T03:05:00.000+07:00
    aliases: *a4
    cluster: implementation_flow
    role: Test results / quality report
    attributes:
      id: AUDIT--MASTER-PRIORITY-SECTORS
      phase: 6
      type: audit
      status: stable
      vault_id: default
      tier: process
      source_type: axiomatic
      title: Master priority sectors — P0-P4 design + RCA promotion + 3-Master
        migration + CLAUDE.md restructure
      tags: *a1
      crosslinks: *a2
      linked_symbols: *a3
      created_at: 2026-05-17T03:05:00.000+07:00
      aliases: *a4
      cluster: implementation_flow
      role: Test results / quality report
      attributes:
        domain: audit
      domain: audit
      language: markdown
      is_test: false
      is_entrypoint: false
      has_secret: true
      secret_type: high_entropy_string
      leak_risk: high
      encryption_level: none
    domain: audit
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: true
    secret_type: high_entropy_string
    leak_risk: high
    encryption_level: none
  domain: audit
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: true
  secret_type: high_entropy_string
  leak_risk: high
  encryption_level: none
---

## Scope

Implements `[[CONCEPT--MASTER-PRIORITY-SECTORS]]` and `[[ADR--CLAUDE-MD-MASTER-PRIORITY-SECTORS]]` — partitions Master atoms into priority bands P0-P4 with user-only authority gate for P0/P1.

## Changes

### RCA promotion (CONCEPT → ADR → MASTER chain)

- New: `gks/concept/CONCEPT--ROOT-CAUSE-ANALYSIS.md` — source concept
- New: `gks/adr/ADR--MASTER-PROMOTION-ROOT-CAUSE-ANALYSIS.md` — evidence ADR
- New: `gks/master/MASTER--ROOT-CAUSE-ANALYSIS.md` — promoted master (P0)

### Sector design

- New: `gks/concept/CONCEPT--MASTER-PRIORITY-SECTORS.md` — design intent
- New: `gks/adr/ADR--CLAUDE-MD-MASTER-PRIORITY-SECTORS.md` — schema decision

### Master schema migration (back-fill priority + constituents)

- Updated: `gks/master/MASTER--ROOT-CAUSE-ANALYSIS.md` — priority: P0
- Updated: `gks/master/MASTER--MSP-DOC-TO-CODE.md` — priority: P0
- Updated: `gks/master/MASTER--ATOM-CONTRADICTION-POLICY.md` — priority: P0

### Loader BLUEPRINT

- New: `gks/blueprint/BLUEPRINT--MSP-MASTER-COMPOSE-LOADER.md` — implementation plan for `msp master compose` CLI (T1-T6 microtasks)

### Validator PROTO

- New: `gks/proto/PROTO--MASTER-PRIORITY-AUTHORITY.md` — enforces user-only authority for P0/P1 assignment

### CLAUDE.md restructure

- Replaced ad-hoc "MASTER BLOCK: ROOT CAUSE ANALYSIS MANDATE" narrative with `## MASTER BLOCKS` sector structure (P0-P4)
- P0 section pre-populated with the 3 existing Masters

## Verification

- All 9 new/updated atoms validate green via `npm run msp:validate`
- Indexer scans 348 atoms; no dangling wikilinks introduced by this session

## Notes

- Loader implementation (BLUEPRINT T1-T6) deferred to next milestone
- Validator rule for PROTO--MASTER-PRIORITY-AUTHORITY is also deferred (rule declared, implementation in future BLUEPRINT)
- GEMINI.md and qwen.md sync is handled by companion task B
