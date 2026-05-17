---
id: SPEC--META-LEARNING-LOOP
title: Meta Learning Loop (MLL) Technical Specification
type: spec
phase: 2
status: draft
tier: process
created_at: 2026-05-13T23:44:00.000+07:00
framework: FRAMEWORK--MSP-ARCHITECTURE-V2
aliases:
  - SPEC
  - implementation_flow
  - Technical specification
cluster: implementation_flow
role: Technical specification
attributes:
  domain: spec
---
# SPEC — Meta Learning Loop (MLL) Implementation

---

## 1. Technical Architecture

**MLL (Meta Learning Loop)** คือระบบบริหารจัดการความรู้ที่ดำเนินการโดย **MSP (Orchestrator)** เพื่อรักษาและพัฒนาความสมบูรณ์ของ **GKS (Vault)** โดยทำหน้าที่เป็นสะพานเชื่อมระหว่างการทำงานจริง (Execution) และฐานความรู้ถาวร (Durable Knowledge)

### 1.1 The Synergy Cycle
1.  **Forward Path (7P):** Intent -> ADR -> Blueprint -> Task -> Code.
2.  **Reverse Path (12S):** Code -> AST -> Symbol Graph -> Execution Trace.
3.  **Synthesis (MLL):** Graph + Trace vs. Intent.

## 2. Component: Skill Creator (Hermes)

### 2.1 Trigger Condition
- Task success (Acceptance Tests passed).
- Workflow complexity > threshold (e.g., > 3 tool calls in sequence).
- Novelty check: Pattern does not exist in `gks/governance/skills/`.

### 2.2 Candidate Generation
- **Path:** `.brain/msp/projects/<ns>/candidates/SKILL--<id>.md`
- **Logic:**
    - Extract `slots` and `inputs` from the execution trace.
    - Draft the `SKILL.md` using the standard template.
    - Assign a `stability_score` using the multi-model check.

## 3. Data Entities

### 3.1 Tension Event
Used to track drift between documentation and code.
- **Fields:** `master_id`, `symptom`, `evidence_id`, `severity`, `proposed_fix`.

### 3.2 4D Completeness Matrix
- **Required Atoms:** `ALGO--`, `CONCEPT--`, `FRAMEWORK--`, `PROTOCOL--`.
- **Validation Rule:** A `GENESIS--` Block Manifest cannot be promoted to `status: stable` unless all 4D members are present and have `stability_score > 0.85`.

## 4. Multi-Model Stability Check (Algorithm)

1.  Input: `definition_text`
2.  Parallel Request:
    - Model A (Gemini 1.5 Pro): "Summarize this in 3 words."
    - Model B (Claude 3.5 Sonnet): "Summarize this in 3 words."
    - Model C (Qwen 2.5 Coder): "Summarize this in 3 words."
3.  Compute: `SemanticSimilarity(A, B, C)`
4.  Output: `stability_score`

## 5. Directory Structure

```text
C:\Users\freshair\cognitive_system\
├── docs\gks\
│   ├── FEAT--META-LEARNING-LOOP.md    (PRD)
│   └── SPEC--META-LEARNING-LOOP.md    (Technical Spec)
└── .brain\msp\projects\<ns>\candidates\
    └── (MLL-generated Skill/Tool candidates)
```

## Connections
- [[FRAMEWORK--MSP-ARCHITECTURE-V2]]
- [[FEAT--META-LEARNING-LOOP]]

