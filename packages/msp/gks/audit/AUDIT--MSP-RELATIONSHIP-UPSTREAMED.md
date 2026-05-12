---
id: AUDIT--MSP-RELATIONSHIP-UPSTREAMED
phase: 6
type: audit
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: MSP relationship docs upstreamed — closes proposal #06
tags:
  - msp
  - gks
  - audit
  - upstream
  - documentation
crosslinks: {"references":["AUDIT--ARCH-DOC-CLEANUP","CONCEPT--AGENT-AGNOSTIC","BLUEPRINT--INBOUND-TO-CANDIDATES-MIGRATION","ADR--AGENT-WRITE-BOUNDARIES"]}
created_at: 2026-05-11T18:25:00.000+07:00
---

# AUDIT — MSP relationship docs upstreamed

## Scope

Records the upstreaming of Proposal #06 into `Freshair129/GksV3`. This update synchronizes `docs/MSP_RELATIONSHIP.md` in GksV3 with the current state of the TypeScript `Freshair129/msp` implementation, specifically marking it as **agent-agnostic** and reflecting the migration away from the legacy inbound-queue workflow.

## Pre-state (drift)

- GksV3 `docs/MSP_RELATIONSHIP.md` was ~6 weeks out of date.
- It described a retired `/submit-memory` + inbound-queue workflow.
- It conflated the Python MSP-v9.1 with the TypeScript MSP.
- It lacked the "Agent-agnostic" framing established in `CONCEPT--AGENT-AGNOSTIC`.

## What was done

1.  **GksV3 Update**: Applied direct edits to `docs/MSP_RELATIONSHIP.md` in `Freshair129/GksV3` (commit `22f1751`).
    - Added "Which MSP?" disambiguation table.
    - Replaced the inbound-queue workflow diagram with the `msp_candidate` -> `.brain/.../candidates/` pipeline.
    - Updated CLI script lists to match current `package.json`.
    - Reframed `MSP-IMP-` / `MSP-TSK-` / `MSP-WKT-` prefixes as EVA-specific process artifacts.
2.  **MSP Record Keeping**:
    - Updated `upstream/gks-proposals/README.md` status: 🟡 drafted -> 🟢 merged.
    - Moved proposal file to `upstream/gks-proposals/merged/06-msp-relationship-update.md`.
    - Updated `upstream/gks-proposals/SUBMISSION.md` to record the merge.

## Implications

- Downstream GKS users now have an accurate mental model of how MSP interacts with the storage engine.
- The "Inbound Queue" is officially documented as legacy for the public TS implementation.
- Agent-agnosticism is now a first-class citizen in the cross-repo documentation.

## Source

- Handover Report: `WALKTHROUGH--KNOWLEDGE-ARCHITECTURE-STANDARDIZATION.md` (2026-05-11).
- GksV3 Commit: `22f1751`.
