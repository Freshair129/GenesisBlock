# PRD — Genesis Block Cycle

**Version:** 1.0
**Status:** draft
**Objective:** Unified vocabulary for the two complementary halves of the MSP knowledge engine — atom assembly (bottom-up) and code decomposition (top-down) — anchored on the **Genesis Block** as the central composite unit.

**References:**

- `gks/master/MASTER--MSP-DOC-TO-CODE.md` — governance source (atoms before code)
- `gks/framework/FRAMEWORK--PHASE-GOVERNANCE.md` — mechanical 7-phase enforcement
- `FRAMEWORK_MASTER_SPEC.md §8` — 12-Stage Symbol Graph DAG
- `gks/spec/SPEC--GENESIS-BLOCK-MANIFEST.md` — Block = composite knowledge unit
- `gks/concept/CONCEPT--TAXONOMY-V2-3.md` — six-layer vocabulary
- `gks/adr/ADR--TASK-TRACKING-AT-ORCHESTRATOR.md` — microtask scope
- `gks/adr/ADR--CODEGEN-MICROTASK-RUNNER.md` — runner contract
- `docs/gks/PRD--MLL.md` — prior naming ("7-Phase Creation Lifecycle" / "12-Stage Processing Pipeline")

---

## 1. Problem

This repository operates two complementary pipelines that together form the heart of the cognitive system:

1. **7-Phase Doc-to-Code Flow (P0..P6)** — defined in `FRAMEWORK--PHASE-GOVERNANCE`. Enforces that atoms exist before code (governance + planning).
2. **12-Stage Symbol Graph DAG** — defined in `FRAMEWORK_MASTER_SPEC §8`. Extracts architectural knowledge from committed code (analysis + retrieval).

Three issues with the current vocabulary:

- **No shared umbrella name.** New agents and contributors do not immediately see that the two pipelines are halves of the same cycle. `PRD--MLL §1` calls them "7-Phase Creation Lifecycle (Forward)" and "12-Stage Processing Pipeline (Reverse)" — descriptive but generic.
- **Existing names are technical, not philosophical.** Neither name surfaces the **Genesis Block** — the composite knowledge unit that `SPEC--GENESIS-BLOCK-MANIFEST` defines as the system's central artifact.
- **Direction symmetry is implicit.** One pipeline builds up (atoms → Block → shipped code), the other breaks down (codebase → symbols → atoms). The current vocabulary does not make this duality legible.

## 2. Goals

- Adopt a single umbrella name that ties both pipelines to the `Genesis Block` anchor.
- Adopt directional sub-names that make bottom-up vs top-down explicit.
- Preserve all existing technical names as aliases — no breaking changes to atom types, phase numbers, stage numbers, or enforcement rules.
- Provide a one-stop mapping from informal user vocabulary (Masterplan / Roadmap / Ultraplan / Implementation Plan / Phase / Task / Sub-task / Micro-task) to repository atoms.

## 3. Non-goals

- Do **not** rename any atom type (`BLUEPRINT--`, `CONCEPT--`, `ADR--`, etc. remain per `CONCEPT--TAXONOMY-V2-3`).
- Do **not** change `gks verify-flow` enforcement, pre-commit hook behavior, or phase/stage numbering.
- Do **not** add a new atom type (`PRD--` is an existing convention in `docs/gks/`).
- Do **not** implement Block Decomposition runner here (that work is owned by `SPEC--GENESIS-GRAPH-BACKEND`).
- Do **not** promote `MASTER--MSP-DOC-TO-CODE` from draft to stable in this PRD — that is a separate decision.

## 4. Naming Proposal

| Level | Official name | Direction | Alias of |
|---|---|---|---|
| Umbrella | **Genesis Block Cycle** | (loop) | "Doc-to-Code" workflow / `PRD--MLL` "continuous improvement loop" |
| Bottom-Up ↑ | **Block Assembly** (P0..P6) | atoms assemble into a Block | "7-Phase Doc-to-Code Flow" / "7-Phase Creation Lifecycle (Forward)" |
| Top-Down ↓ | **Block Decomposition** (12-Stage DAG) | codebase decomposes into atoms | "Symbol Graph & Processing Pipeline" / "12-Stage Processing Pipeline (Reverse)" |

### 4.1 Rationale

- The **Genesis Block** is the central composite unit per `SPEC--GENESIS-BLOCK-MANIFEST`. Naming both pipelines around it makes the system feel coherent.
- **Assembly** is an engineering metaphor: atoms are parts, `GENESIS--<NAME>.md` is a bill of materials, P6 AUDIT is the QA gate before ship.
- **Decomposition** is the mirror metaphor: a committed codebase is disassembled into symbols, communities, and process flows that re-enter the knowledge graph.
- **Cycle** captures that insights from Decomposition (new symbols, community shifts, dead-code detection) feed back into the next round of Assembly via MCP tools.

## 5. Block Assembly ↑ (P0..P6)

**Definition:** The bottom-up pipeline by which atoms are progressively assembled into a shippable Genesis Block.
**Source of truth:** `FRAMEWORK--PHASE-GOVERNANCE` (status: stable, enforced by `gks verify-flow`).
**Bill of materials:** `GENESIS--<NAME>.md` block manifest (per `SPEC--GENESIS-BLOCK-MANIFEST`).

| Phase | Atom | Devlog ID | Role in Assembly |
|---|---|---|---|
| P0 | `FRAME--` / `IDEA--` / `GENESIS--` | — | foundation, parts catalog |
| P1 | `CONCEPT--` | `MSP-CON-` | intent, high-level API draft |
| P2 | `ADR--`, `ENTITY--`, `API--`, `FEAT--` | `MSP-DES-` | decisions, schema, endpoints |
| P3 | `BLUEPRINT--` (the implementation plan) | `MSP-IMP-` | assembly diagram, phase plan |
| P4 | `T*.task.yaml` (microtask) | `MSP-TSK-` | per-piece work order |
| P5 | `src/` | `MSP-ACT-` | assembled product |
| P6 | `AUDIT--` | `MSP-WKT-` | QA sign-off |

**Skip rules** (per `MASTER--MSP-DOC-TO-CODE`):

- P4 may be skipped for single-developer slices (Directive #4).
- P1..P3 may be skipped only with a `HOTFIX` commit tag and a 48-hour backfill window (`ADR--HOTFIX-ESCAPE-HATCH`).

## 6. Block Decomposition ↓ (12-Stage DAG)

**Definition:** The top-down, automated pipeline that decomposes a committed codebase into atom-level knowledge graph entries.
**Source of truth:** `FRAMEWORK_MASTER_SPEC §8`.
**Output:** `GenesisGraphBackend` → MCP tools (`gks_backlinks`, `symbol_trace`).

| Stage | Activity | Output |
|---|---|---|
| 1 | Scan | file paths and sizes |
| 2 | Structure | folder/file hierarchy tree |
| 3 | Specialized Parse: Markdown | atom extraction |
| 4 | Specialized Parse: COBOL | legacy AST |
| 5 | Symbolic Parse (Tree-sitter) | functions, classes, methods |
| 6 | Framework: Routes | API entry points |
| 7 | Framework: Tools | MCP / RPC handlers |
| 8 | Framework: ORM | database schema relations |
| 9 | Cross-File Resolution | import/export graph |
| 10 | MRO | inheritance map |
| 11 | Communities (Leiden) | functional clusters |
| 12 | Processes | execution flow traces |

## 7. The Cycle (Assembly ↑ + Decomposition ↓)

```
       Block Assembly ↑                       Block Decomposition ↓
       (bottom-up, manual + agent)            (top-down, automated DAG)
       ─────────────────────────              ──────────────────────────
   ┌── P6 AUDIT          (ship) ─┐         ┌── Stage 1  Scan ──┐
   │   P5 CODE                   │         │   Stage 2  Structure
   │   P4 TASK                   │         │   Stage 3-4 Specialized Parse
   ↑   P3 BLUEPRINT              │ commit  ↓   Stage 5  Symbolic Parse
   │   P2 ADR/FEAT      ─── code ┴───────→ │   Stage 6-8 Framework
   │   P1 CONCEPT                          │   Stage 9  Cross-File
   │   P0 FRAME                            │   Stage 10 MRO
   └── (atoms)                             │   Stage 11 Communities
                                           └── Stage 12 Processes ──┐
                                                                    ↓
                                                          GenesisGraphBackend
                                                                    ↓
                  ◀─── feedback into next P1 / P2 ◀─────── MCP tools
```

The cycle closes when discoveries from Decomposition (new symbols, refactored communities, dead-code reports) inform the next iteration of Assembly. This is the same loop described in `PRD--MLL` as the "self-refining cognitive loop," now named around its central artifact.

## 8. User Mental Model Mapping

| Informal term | Repository concept | Role in Cycle |
|---|---|---|
| Masterplan / spec | P0–P2 atoms (FRAMEWORK + CONCEPT + ADR / FEAT + SPEC + PROTOCOL) | Assembly base |
| Roadmap | `ROADMAP.md` (markdown view, not an atom) | progress tracking |
| Ultraplan | `ULTRAPLAN--*.md` (optional, cross-cutting) | Assembly compiler |
| **Implementation Plan** | **`BLUEPRINT--*`** atom (P3) | Assembly diagram |
| Phase | P0..P6 (Assembly axis) or P3.x sub-phase inside a BLUEPRINT | structural axis |
| Task | Milestone row (M7a, M7b, …) in `ROADMAP.md` | progress unit |
| Sub-task | row in a BLUEPRINT phase table | work breakdown |
| Micro-task | `T*.task.yaml` at P4 (outside the `gks/` vault) | codegen runner input |

## 9. Acceptance Criteria

- [ ] `docs/gks/PRD--GENESIS-BLOCK-CYCLE.md` lands in the repo.
- [ ] `CLAUDE.md` "Doc-to-code workflow (mandatory)" section carries an alias note pointing at this PRD.
- [ ] `FRAMEWORK_MASTER_SPEC.md §8` header carries an alias note pointing at this PRD.
- [ ] `npm run msp:check-links` passes — all references named in this PRD resolve.
- [ ] No existing atom's `status`, `phase`, or semantics is modified.

## 10. Out of Scope

- Renaming any atom type (`BLUEPRINT--`, `CONCEPT--`, `ADR--`, etc.).
- Changing phase numbers (P0..P6) or stage numbers (1..12).
- Implementing the Block Decomposition runner (`SPEC--GENESIS-GRAPH-BACKEND` owns that work).
- Promoting `MASTER--MSP-DOC-TO-CODE` from draft to stable (separate ADR decision).
