---
id: FEAT--DECISION-ATROPHY-GUARDS
phase: 2
type: feat
status: draft
vault_id: default
tier: genesis
source_type: axiomatic
title: FEAT — Decision Atrophy Guards — automatic detection of expired atoms
tags: &a1
  - msp
  - lifecycle
  - atrophy
  - validator
  - m9a
aliases: &a2
  - FEAT
  - implementation_flow
  - Feature specification
cluster: implementation_flow
role: Feature specification
crosslinks: &a3
  references:
    - PROTO--VALID-UNTIL
    - FRAMEWORK--PHASE-GOVERNANCE
    - CONCEPT--MSP-ROADMAP
created_at: 2026-05-18T11:15:00+07:00
attributes:
  id: FEAT--DECISION-ATROPHY-GUARDS
  phase: 2
  type: feat
  status: draft
  vault_id: default
  tier: genesis
  source_type: axiomatic
  title: FEAT — Decision Atrophy Guards — automatic detection of expired atoms
  tags: *a1
  aliases: *a2
  cluster: implementation_flow
  role: Feature specification
  crosslinks: *a3
  created_at: 2026-05-18T11:15:00+07:00
  domain: feat
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: false
  leak_risk: low
  encryption_level: none
---

# FEAT — Decision Atrophy Guards

## 1. Summary

The Decision Atrophy Guards feature implements a proactive monitoring and reporting system for the lifecycle of GKS atoms. It leverages the `valid_until` frontmatter field to identify documentation that is potentially outdated ("atrophied") and mandates periodic reviews to ensure the knowledge base remains an accurate source of truth.

## 2. Motivation

Documentation that is never reviewed eventually becomes an "enforced lie." In a fast-moving monorepo, ADRs, ROADMAPs, and Plans can quickly fall out of sync with reality. Without a systematic way to track and enforce expiration dates, the system's cognitive reliability degrades. M9a aims to prevent this by turning documentation expiry into a first-class operational concern.

## 3. Requirements

### 3.1 Expiry Detection

- The system must scan all atoms in the GKS index.
- Identify atoms where the `valid_until` date is in the past.
- Identify atoms that are "near expiry" (e.g., within 14 days of `valid_until`).

### 3.2 Reporting

- Provide a CLI command `msp-atrophy report` that outputs a summary of expired and near-expiry atoms.
- Output should include: Atom ID, Title, Expiry Date, and Days Since Expiry (or Days Remaining).
- Support `--json` format for automated ingestion.

### 3.3 CI Integration

- Integrate with `msp:validate`.
- **Expired Atoms:** Emit a `warning` (non-blocking for now, as per `PROTO--VALID-UNTIL`).
- **Near-Expiry Atoms:** Emit an `info` message.
- *Future Work:* Option to promote warnings to errors for specific high-stakes atoms (e.g., SECURITY policies).

### 3.4 Governance Flow

- Provide a standard procedure for "refreshing" an atom (updating `valid_until` after review).
- Integrate with the `ADR--DELEGATION-POLICY` to allow T3 agents to approve refreshes for L1/L2 atoms.

## 4. Acceptance Criteria

- [ ] A CLI command exists to list all expired atoms.
- [ ] `npm run msp:validate` correctly flags expired atoms as warnings.
- [ ] The system differentiates between `expired` and `near-expiry` states.
- [ ] Unit tests verify the scanning logic across various edge cases (leap years, different timezones).

## 5. Connections

- `[[PROTO--VALID-UNTIL]]` — the machine-enforced rule.
- `[[CONCEPT--MSP-ROADMAP]]` §3 M9a.
