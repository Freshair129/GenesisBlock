---
id: FEAT--META-LEARNING-LOOP
title: Meta Learning Loop (MLL)
type: feat
phase: 2
status: draft
tier: genesis
created_at: 2026-05-13T23:43:00.000+07:00
owner: Genesis Architect (Antigravity)
aliases:
  - FEAT
  - implementation_flow
  - Feature spec
cluster: implementation_flow
role: "Feature spec"
---
# FEAT — Meta Learning Loop (MLL)
**Reference:** `d:\The Human Algorithm\T2\meta_learning_loop_technical_manual.md`

---

## 1. Executive Summary

The **Meta Learning Loop (MLL)** is the continuous self-improvement mechanism of the Cognitive System. It monitors agent interactions, identifies successful execution patterns, and formalizes them into durable **Atoms** (Skills, Algos, Concepts) in the **GKS**. MLL acts as a "Cognitive Refinery" that ensures the system evolves based on real-world evidence rather than just static design.

## 2. Requirements

### 2.1 MLL-FR-01: Skill Creator (Hermes-style)
- **Description:** The system must proactively suggest new `SKILL--` or `TOOL--` atoms when a successful complex workflow is detected.
- **Verification:** User review of the candidate atom.

### 2.2 MLL-FR-02: Bottom-up 4D Evolution
- **Description:** MLL must evaluate Genesis Blocks for "Master" promotion readiness.
- **Criteria:** All 4 dimensions (Algo, Concept, Frame, Proto) must be present.

### 2.3 MLL-FR-03: Tension Detection
- **Description:** MLL must flag inconsistencies between the GKS knowledge graph and actual code behavior extracted by the 12-Stage pipeline.
- **Action:** Create `Tension Event` logs.

### 2.4 MLL-FR-04: Multi-Model Stability Check
- **Description:** Verify atom definitions across multiple LLM providers to ensure semantic stability.

### 2.5 MLL-FR-05: Human-in-the-loop Staging
- **Description:** All MLL suggestions must be placed in a `candidates/` staging area for human approval before being written to the official `gks/` vault.

## 3. User Experience

When the MLL identifies a new skill:
1.  The agent completes the task.
2.  MLL analyzes the log and generates a `[[SKILL--NEW-PATTERN]].md` in the `candidates/` folder.
3.  The agent notifies the user: *"I've learned a new skill for [Task X]. Would you like to review and approve it for future use?"*
4.  User approves the PR, and the skill moves to `gks/governance/skills/`.
