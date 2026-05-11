---
id: AUDIT--FRAME-BROADER-UPSTREAMED
phase: 6
type: audit
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: Broad FRAME-- definition upstreamed — closes proposal #07
tags:
  - msp
  - gks
  - audit
  - upstream
  - taxonomy
  - framework
crosslinks: {"references":["AUDIT--ARCH-DOC-CLEANUP","FRAME--MSP-ARCHITECTURE-V2","FRAME--KNOWLEDGE-3-TIER","FRAME--PHASE-GOVERNANCE"]}
created_at: 2026-05-11T10:26:00.000Z
---

# AUDIT — Broad FRAME-- definition upstreamed

## Scope

Records the upstreaming of Proposal #07 into `Freshair129/GksV3`. This update broadens the canonical definition of the `FRAME--` prefix in `docs/KNOWLEDGE-TYPES.md` to include architectural, governance, and methodological frameworks, aligning GKS documentation with the actual practice of Memory OS implementers like MSP.

## Pre-state (violation)

- GksV3 `docs/KNOWLEDGE-TYPES.md` defined `FRAME--` narrowly as "code standards / framework rules".
- MSP had 7+ atoms (e.g., `FRAME--MSP-ARCHITECTURE-V2`, `FRAME--AUTHORITY-MATRIX`) that violated this narrow definition.
- There was no canonical home for business methodology frameworks (JTBD, Design Thinking, etc.).

## What was done

1.  **GksV3 Update**: Applied Option A (broaden definition) to `docs/KNOWLEDGE-TYPES.md` in `Freshair129/GksV3` (commit `22f1751`).
    - Widened scope to "reusable structural patterns".
    - Added explicit examples for architectural, governance, taxonomic, and methodology frames.
    - Added a distinguishing question: "is this a *reusable pattern* (vs a one-off decision)?" to help distinguish from `ADR--`.
2.  **MSP Record Keeping**:
    - Updated `upstream/gks-proposals/README.md` status: 🟡 drafted -> 🟢 merged.
    - Moved proposal file to `upstream/gks-proposals/merged/07-frame-broader-definition.md`.
    - Updated `upstream/gks-proposals/SUBMISSION.md` to record the merge.

## Implications

- The Master-Genesis-Atomic hierarchy now has a officially blessed home for its structural components (Genesis definitions often live in `FRAME--`).
- Multi-project methodologies like JTBD or 5Es now have a canonical location in the GKS ecosystem.
- Existing MSP atoms are no longer in technical violation of the GKS type system.

## Source

- Handover Report: `WALKTHROUGH--KNOWLEDGE-ARCHITECTURE-STANDARDIZATION.md` (2026-05-11).
- GksV3 Commit: `22f1751`.
