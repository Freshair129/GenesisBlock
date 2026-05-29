---
id: INC--TAXONOMY-PREFIX-DRIFT
phase: 6
type: inc
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: INC — Taxonomy Prefix Drift (Protocol vs. Runbook confusion)
tags:
  - msp
  - gks
  - taxonomy
  - incident
  - rca
crosslinks:
  references:
    - CONCEPT--TAXONOMY-V2-3
created_at: 2026-05-29T12:00:00+07:00
---

# INC — Taxonomy Prefix Drift

## 1. Description
On 2026-05-29, during the planning of the "Shadow Repo" architecture, the Gemini CLI agent proposed using the `PROTOCOL--` prefix for a procedural synchronization task. A user intervention correctly identified that per **Taxonomy v2.3**, procedural SOPs MUST use the `RUNBOOK--` prefix, while `PROTOCOL--` is reserved for external technical interfaces (API/ABI).

## 2. Root Cause Analysis (RCA)
- **Documentation Inconsistency:** The primary guidance file `AGENT.md` (Section 7) was not fully synchronized with `CONCEPT--TAXONOMY-V2-3.md`, missing the `RUNBOOK--`, `PROTO--`, and `SAFETY--` prefixes.
- **Linguistic Ambiguity:** The term "Protocol" was colloquially interpreted as "Procedure" by the LLM, despite the technical disambiguation existing in deeper framework specs.
- **Contextual Decay:** Older framework documents provided conflicting examples that had not been fully deprecated or updated during the v2.3 migration.

## 3. Resolution
- **Immediate Fix:** The "Shadow Repo" plan was corrected to use `RUNBOOK--BRAIN-SYNC-PROCEDURE` and `PROTO--BRAIN-INDEXING-MANDATE`.
- **Systemic Fix:** Updated `AGENT.md` and `GEMINI.md` to explicitly define the difference between `RUNBOOK--`, `PROTO--`, and `PROTOCOL--`.
- **Validation:** Added a disambiguation note to the core taxonomy table in `AGENT.md`.

## 4. Prevention Plan
- **Mandatory Schema Check:** Agents must verify atom types against `atom_schema.yaml` before proposing new atoms.
- **Sync Audit:** Periodic cross-check between `CONCEPT--TAXONOMY` and `AGENT.md` to ensure zero drift.
- **Disambiguation Rule:** "RUNBOOK = SOP (How-to); PROTOCOL = Interface (API/ABI); PROTO = Invariant (Machine-rule)."

## 5. Timeline
- **16:00:** Incident occurred (Agent proposed `PROTOCOL--` for sync procedure).
- **16:05:** User intervention identified the taxonomy violation.
- **16:15:** RCA performed; identified stale taxonomy table in `AGENT.md`.
- **16:30:** INC report filed; Fix plan initiated.
