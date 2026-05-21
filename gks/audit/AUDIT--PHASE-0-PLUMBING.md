---
id: AUDIT--PHASE-0-PLUMBING
phase: 6
type: audit
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: "AUDIT - UCF Phase 0: propagation plumbing and attribute bag"
tags: &a1
  - msp
  - ucf
  - plumbing
  - audit
crosslinks: &a2
  references:
    - BLUEPRINT--PHASE-0-PLUMBING
    - CONCEPT--SUBJECT-RESOURCE-ACTION-CONTEXT
    - CONCEPT--ATTRIBUTE-BAG-MODEL
created_at: 2026-05-14T21:00:00.000+07:00
aliases: &a3
  - AUDIT
  - implementation_flow
  - Test results / quality report
cluster: implementation_flow
role: Test results / quality report
attributes:
  id: AUDIT--PHASE-0-PLUMBING
  phase: 6
  type: audit
  status: stable
  vault_id: default
  tier: process
  source_type: axiomatic
  title: "AUDIT - UCF Phase 0: propagation plumbing and attribute bag"
  tags: *a1
  crosslinks: *a2
  created_at: 2026-05-14T21:00:00.000+07:00
  aliases: *a3
  cluster: implementation_flow
  role: Test results / quality report
  attributes:
    id: AUDIT--PHASE-0-PLUMBING
    phase: 6
    type: audit
    status: stable
    vault_id: default
    tier: process
    source_type: axiomatic
    title: "AUDIT - UCF Phase 0: propagation plumbing and attribute bag"
    tags: *a1
    crosslinks: *a2
    created_at: 2026-05-14T21:00:00.000+07:00
    aliases: *a3
    cluster: implementation_flow
    role: Test results / quality report
    attributes:
      id: AUDIT--PHASE-0-PLUMBING
      phase: 6
      type: audit
      status: stable
      vault_id: default
      tier: process
      source_type: axiomatic
      title: "AUDIT - UCF Phase 0: propagation plumbing and attribute bag"
      tags: *a1
      crosslinks: *a2
      created_at: 2026-05-14T21:00:00.000+07:00
      aliases: *a3
      cluster: implementation_flow
      role: Test results / quality report
      attributes:
        domain: audit
      domain: audit
      language: markdown
      is_test: false
      is_entrypoint: false
      has_secret: false
      leak_risk: low
      encryption_level: none
    domain: audit
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: false
    leak_risk: low
    encryption_level: none
  domain: audit
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: false
  leak_risk: low
  encryption_level: none
---

# AUDIT - UCF Phase 0: Propagation Plumbing

## Scope

This audit covers Phase 0 of the Universal Context Framework (UCF) implementation.
The goal was to thread the 4-tuple (`Subject`, `Resource`, `Action`, `Context`)
through the retrieval and codegen pipeline and enable the `attributes` bag in atoms.

## What shipped

- **Core Types:** Defined `Subject`, `Resource`, `Action`, `RequestContext`, `Decision` in `packages/msp/src/policy/types.ts`.
- **Attribute Bag:** Updated re-indexer and GKS `MemoryStore` to support the `attributes` field in atom frontmatter and retrieval hits.
- **Threading:** Updated Cognitive Facade (`recall`, `remember`, `runTask`), Recall Orchestrator, and MCP tool handlers to log and propagate the UCF 4-tuple.
- **CI/CLI Integration:** Updated `msp-validate` to construct a `scheduled-job` subject for CI runs.
- **Atom Promotion:** Promoted 17 UCF-related CONCEPT, ADR, and FEAT atoms to `stable`/`active`.

## Verification

- **Unit Tests:** Added tests for policy types and verified round-trip of attributes through the memory stack.
- **Workspace Validation:** `npm run msp:validate` passes with 325 atoms.
- **Traceability:** Verified `[ucf]` debug logs in stderr across entry points.

## Sign-off

- Implemented by: Gemini CLI
- Verified by: `msp:validate` + unit tests
- Date: 2026-05-14

## Connections

- [[BLUEPRINT--PHASE-0-PLUMBING]]
- [[CONCEPT--SUBJECT-RESOURCE-ACTION-CONTEXT]]
- [[CONCEPT--ATTRIBUTE-BAG-MODEL]]
