---
id: PROTO--MASTER-PRIORITY-AUTHORITY
phase: 2
type: proto
status: draft
severity: error
tier: safety
source_type: axiomatic
vault_id: default
title: enforce user-only authority for Master priority P0/P1 assignments
tags: &a1
  - msp
  - master
  - priority
  - authority
  - governance
  - validator
crosslinks: &a2
  references:
    - ADR--CLAUDE-MD-MASTER-PRIORITY-SECTORS
    - CONCEPT--MASTER-PRIORITY-SECTORS
    - FRAMEWORK--KNOWLEDGE-3-TIER
    - ADR--AGENT-WRITE-BOUNDARIES
created_at: 2026-05-17T02:55:00.000+07:00
aliases: &a3
  - PROTO
  - implementation_flow
  - Machine-enforced invariant
cluster: implementation_flow
role: Machine-enforced invariant
attributes:
  id: PROTO--MASTER-PRIORITY-AUTHORITY
  phase: 2
  type: proto
  status: draft
  severity: error
  tier: safety
  source_type: axiomatic
  vault_id: default
  title: enforce user-only authority for Master priority P0/P1 assignments
  tags: *a1
  crosslinks: *a2
  created_at: 2026-05-17T02:55:00.000+07:00
  aliases: *a3
  cluster: implementation_flow
  role: Machine-enforced invariant
  attributes:
    id: PROTO--MASTER-PRIORITY-AUTHORITY
    phase: 2
    type: proto
    status: draft
    severity: error
    tier: safety
    source_type: axiomatic
    vault_id: default
    title: enforce user-only authority for Master priority P0/P1 assignments
    tags: *a1
    crosslinks: *a2
    created_at: 2026-05-17T02:55:00.000+07:00
    aliases: *a3
    cluster: implementation_flow
    role: Machine-enforced invariant
    attributes:
      domain: proto
    domain: proto
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: true
    secret_type: high_entropy_string
    leak_risk: high
    encryption_level: none
  domain: proto
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: true
  secret_type: high_entropy_string
  leak_risk: high
  encryption_level: none
---

# PROTO — MASTER-PRIORITY-AUTHORITY

## Rule

Any Master atom (located in `gks/master/`) that specifies a `priority:` of `P0` (Always Loaded) or `P1` (High Priority Index) must have verifiable user-authority evidence within the same pull request or commit. Agents are strictly forbidden from self-promoting Master directives to these foundational tiers without explicit human intervention.

## Rationale

Master atoms with P0/P1 priority form the "instinct" layer of the agentic system. Because P0 atoms are loaded into every session's context and P1 atoms are indexed globally, their content has a disproportionate impact on agent behavior and token consumption. 

The risks of ungated autonomous P0/P1 promotion include:
1. **Instinct Drift**: Agents could silently alter the core mandates that govern their own behavior, potentially bypassing safety rules or structural requirements.
2. **Context Bloat**: Agents might over-prioritize their own task-specific knowledge, causing P0 to grow beyond the context window's efficient processing capacity.
3. **Circular Logic**: An agent could create a P0 mandate that reinforces its own flawed assumptions, making them harder to correct in future sessions.

This PROTO enforces the "Human-in-the-Loop" requirement for the most sensitive layers of the knowledge hierarchy, ensuring that every foundation-level change is traceable to a specific user authorization.

## Scope

- **Target Files**: All files matching the pattern `gks/master/*.md`.
- **Change Triggers**: Any commit or pull request that performs one of the following actions:
    - Creates a new Master atom file with `priority: P0` or `priority: P1`.
    - Updates an existing Master atom's `priority` field from a lower band (P2, P3, P4) to a higher band (P0, P1).
    - Modifies the body or frontmatter of an existing P0/P1 Master atom where the priority was recently assigned.

## Predicate

When a Master atom in `gks/master/<ID>.md` has its `priority:` field added or changed to `P0` or `P1` in a change set, the validator must confirm the presence of EITHER:

### (a) PR Description Authorization
An explicit user-authority comment in the PR description (or commit message if no PR is available) using the following exact format:

`[priority-approval] user authorized priority: <P0|P1> for <atom_id>`

- `<P0|P1>`: Must match the priority level specified in the atom's frontmatter.
- `<atom_id>`: Must match the `id:` field of the Master atom being modified.
- The string must be present in the raw text of the PR description at the time of the validation gate check.

### (b) Promotion ADR Evidence
A new promotion ADR committed in the same change set that documents the user authority and provides the formal justification for the priority level.

- **File Path**: The ADR must be located in the `gks/adr/` directory.
- **ID Pattern**: The ADR ID must start with `ADR--MASTER-PROMOTION-` or `ADR--MASTER-PRIORITY-PROMOTION-`.
- **Status**: The ADR must be in `status: draft`, `status: active`, or `status: stable`.
- **Reference**: The ADR's `crosslinks.references` array must include the ID of the Master atom being promoted.

## Failure Mode

- **Validator Error Code**: `master-priority-authority-missing`
- **Error Message**: `Master <id> has priority: <P0|P1> but no user-authority evidence found (PR comment or promotion ADR in the same change set)`

## Severity

`error` (blocks merge). This is a critical safety gate that protects the system's core governance layer.

## Exemption: Initial Schema Migration

The one-time commit that performs the broad migration of existing Master atoms to the new P0–P4 priority schema is exempted from this authority check. 

- **Mechanism**: The validator implementation checks the current commit hash against a hardcoded or configured `SCHEMA_MIGRATION_COMMIT_SHA`.
- **Rationale**: The migration itself is authorized by `[[ADR--CLAUDE-MD-MASTER-PRIORITY-SECTORS]]`, which already contains the user's consent for the initial P0 assignments.
- **Duration**: This exemption is strictly for the migration commit. All subsequent changes to P0/P1 atoms, or new promotions, must follow the evidence requirements.

## Implementation Note

The implementation of this predicate logic belongs in:
`packages/msp/src/validator/rules/master-priority-authority.ts`

### Logic Flow (Pseudo-code)
1. Identify all changed files in the current diff.
2. Filter for files in `gks/master/`.
3. For each Master file:
    a. Parse frontmatter using the atomic parser.
    b. Check if `priority` is `P0` or `P1`.
    c. If YES, check if this is the first time `priority` is `P0/P1` or if it changed from a lower value in this diff.
    d. If evidence is required:
        i. Scan the environment-provided PR description for the `[priority-approval]` token.
        ii. Scan the current change set for a new file matching `gks/adr/ADR--MASTER-PROMOTION-*.md`.
    e. If neither evidence source is found, emit the `master-priority-authority-missing` error.

This PROTO declares the rule; the validator code implementation is deferred to a follow-up BLUEPRINT.

## Source

- `[[ADR--CLAUDE-MD-MASTER-PRIORITY-SECTORS]]` — The primary decision gate.
- `[[CONCEPT--MASTER-PRIORITY-SECTORS]]` — The underlying design.
- `[[ADR--AGENT-WRITE-BOUNDARIES]]` — General authority model.
- `[[FRAMEWORK--KNOWLEDGE-3-TIER]]` — Master tier definitions.

## Connections

- [[ADR--CLAUDE-MD-MASTER-PRIORITY-SECTORS]]
- [[CONCEPT--MASTER-PRIORITY-SECTORS]]
- [[FRAMEWORK--KNOWLEDGE-3-TIER]]
- [[ADR--AGENT-WRITE-BOUNDARIES]]
