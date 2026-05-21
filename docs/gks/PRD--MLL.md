# PRD — Meta Learning Loop (MLL)

**Version:** 1.0
**Reference:** `d:\The Human Algorithm\T2\meta_learning_loop_technical_manual.md`
**Objective:** To establish a self-refining cognitive loop that integrates execution experience with durable knowledge.

---

## 1. Vision & Strategy

The **Meta Learning Loop (MLL)** is the "Refinery" of the Cognitive System. It ensures that the system doesn't just "execute" tasks but "learns" from them. It bridges the **Block Assembly (7-Phase Doc-to-Code Flow, Forward)** and the **Block Decomposition (12-Stage Symbol Graph DAG, Reverse)** to create a continuous improvement loop — together these two halves form the **Genesis Block Cycle** (see `docs/gks/PRD--GENESIS-BLOCK-CYCLE.md` for the unified vocabulary).

## 2. Key Features

### 2.1 Skill Creator (Hermes-style)
- **Concept:** Immediate learning from successful workflows.
- **Requirement:** When an agent completes a task with a high success score, MLL identifies the pattern and proposes a `SKILL--` or `TOOL--` atom.
- **Human-in-the-loop:** All proposed skills must be staged in the `candidates/` directory for human approval before being promoted to GKS.

### 2.2 4D Completeness Check
- **Requirement:** Knowledge units (Atoms) aiming for "Master" status must satisfy the 4D requirement:
    1. **Algo:** Executable steps or logic.
    2. **Concept:** Semantic definition and meaning.
    3. **Frame:** Cognitive framework or mental model.
    4. **Proto:** Protocol/Implementation details.
    5. *(Optional)* **Param:** Configuration and business constraints.

### 2.3 Tension Event Logging
- **Requirement:** Monitor for "Tension" — conflicts between established knowledge (Atoms) and real-world behavior (Code/Logs).
- **Action:** Create Tension Event records to trigger human review or automated refinement.

### 2.4 Multi-Model Stability Verification
- **Requirement:** Use multiple LLMs to verify if an Atom's definition is stable and self-contained. High variance in interpretation results in a lower stability score.

## 3. Workflow

1.  **Experience:** Agent performs a task (P5 Implementation).
2.  **Observation:** 12-Stage Pipeline (12S) extracts the symbol graph and execution trace.
3.  **Synthesis:** MLL compares 12S data against 7P atoms.
4.  **Proposal:** MLL suggests new skills or tension updates to the `candidates/` folder.
5.  **Audit:** Human reviews candidates and promotes them to GKS.

---

## 4. Definitions

- **Genesis Block:** Functional knowledge (Skills, Tools, Algos).
- **Master Block:** Essence knowledge (Universal principles, stable across models).
- **Refinery:** The process of cleaning and structuring "raw" logs into "atomic" knowledge.
