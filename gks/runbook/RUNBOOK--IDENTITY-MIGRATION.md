---
id: RUNBOOK--IDENTITY-MIGRATION
phase: 6
type: runbook
status: stable
tier: process
source_type: axiomatic
vault_id: default
title: Identity Migration SOP — Global to Workspace Transition
tags: &a1
  - msp
  - identity
  - migration
  - sop
  - projects
crosslinks: &a2
  references:
    - ADR--GLOBAL-VS-WORKSPACE
    - ALGO--IDENTITY-RESOLUTION
created_at: 2026-05-14T20:10:00+07:00
aliases: &a3
  - RUNBOOK
  - ops
  - Operational response guide
cluster: ops
role: Operational response guide
attributes:
  id: RUNBOOK--IDENTITY-MIGRATION
  phase: 6
  type: runbook
  status: stable
  tier: process
  source_type: axiomatic
  vault_id: default
  title: Identity Migration SOP — Global to Workspace Transition
  tags: *a1
  crosslinks: *a2
  created_at: 2026-05-14T20:10:00+07:00
  aliases: *a3
  cluster: ops
  role: Operational response guide
  attributes:
    id: RUNBOOK--IDENTITY-MIGRATION
    phase: 6
    type: runbook
    status: stable
    tier: process
    source_type: axiomatic
    vault_id: default
    title: Identity Migration SOP — Global to Workspace Transition
    tags: *a1
    crosslinks: *a2
    created_at: 2026-05-14T20:10:00+07:00
    aliases: *a3
    cluster: ops
    role: Operational response guide
    attributes:
      id: RUNBOOK--IDENTITY-MIGRATION
      phase: 6
      type: runbook
      status: stable
      tier: process
      source_type: axiomatic
      vault_id: default
      title: Identity Migration SOP — Global to Workspace Transition
      tags: *a1
      crosslinks: *a2
      created_at: 2026-05-14T20:10:00+07:00
      aliases: *a3
      cluster: ops
      role: Operational response guide
      attributes:
        domain: runbook
      domain: runbook
      language: markdown
      is_test: false
      is_entrypoint: false
      is_operational: true
      issue_status: stable
      has_secret: false
      leak_risk: low
      encryption_level: none
    domain: runbook
    language: markdown
    is_test: false
    is_entrypoint: false
    is_operational: true
    issue_status: stable
    has_secret: false
    leak_risk: low
    encryption_level: none
  domain: runbook
  language: markdown
  is_test: false
  is_entrypoint: false
  is_operational: true
  issue_status: stable
  has_secret: false
  leak_risk: low
  encryption_level: none
---

# RUNBOOK — Identity Migration

Procedural SOP for migrating identities from legacy single-root setups to the monorepo's global-vs-workspace architecture.

## 1. Pre-flight Checks

- [ ] Verify `MSP_HOME` (global) and `MSP_PROJECT` (workspace) environment variables are set.
- [ ] Backup existing `identity.json` from the root directory.

## 2. Global Baseline Setup

1. Copy the primary identity into `~/.msp/identity.json`.
2. This serves as the fallback for all projects.
3. Validate with `msp-identity-get --view=global`.

## 3. Project Workspace Specialisation

1. Identify project-specific overrides (e.g., custom `role` or `tone`).
2. Author `.brain/msp/projects/<ns>/identity.override.json`.
3. Use only the keys that differ from the global baseline (sparse JSON).

## 4. Verification

1. Run `msp-identity-get --view=merged`.
2. Confirm that overrides correctly mask global values.
3. Verify `schemaVersion: 1` is preserved.

## 5. Clean-up

1. Delete legacy `identity.json` from the repository root.
2. Update `.gitignore` to ensure local identity files are not committed.

## Connections

- [[ADR--GLOBAL-VS-WORKSPACE]]
- [[ALGO--IDENTITY-RESOLUTION]]
