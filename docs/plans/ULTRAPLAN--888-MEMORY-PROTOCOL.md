# ULTRAPLAN — Integrate EVA 8-8-8 Memory Synthesis Protocol into cognitive_system

> **Executor:** TBD (likely T3 design / T2 scaffold / T1 micro-task — same triad as UCF)
> **Reviewer:** Lead dev / Boss
> **Branch base:** `main` — per-phase branches `claude/msp-888-phase-N-<slug>`
> **Status:** **DRAFT — awaiting lead-dev sign-off**
> **Last updated:** 2026-05-17T08:00:00+07:00
> **Source material:** `H:\My Drive\888_Memory_Protocol\` (copied from `D:\The Human Algorithm\T2\agent\docs\`)

---

## 0. Context

### 0.1 What 8-8-8 is

The **8-8-8 Memory Synthesis Protocol** is the canonical memory-architecture of EVA v9.6.2 ("The Human Algorithm"). It describes a 4-tier memory hierarchy that distils raw conversational data into long-term "wisdom":

| Layer | Role | Authority | Synthesis trigger |
|---|---|---|---|
| **Consciousness** | Awareness buffer | LLM: Full R/W | Volatile — dies with session |
| **Session Memory** | Working memory (raw logs) | MSP: Write Only | Append per turn |
| **Core Memory** | Short-term narrative | MSP: Write Only | **8 Sessions → 1 Core** |
| **Sphere Memory** | Long-term wisdom (identity DNA) | MSP: Write Only | **8 Cores → 1 Sphere** |

Each distillation step applies the same **4 Pillars**:

```
Clean → Summary → Index → Relation
```

The protocol also includes:

- **Memory Domains** — `safety` / `identity-relationship` / `knowledge-skill` / `contextual` / `meta` — each with its own decay rate and promotion difficulty.
- **Epistemic States** — `hypothesis` / `confirmed` / `contested` / `deprecated` — confidence of a memory unit, **independent of document lifecycle status**.
- **Belief Revision Protocol** — if a Sphere belief is challenged repeatedly and confidence cannot recover within N sessions, MSP downgrades Sphere → Core and flags `belief_under_revision`. The belief must re-prove itself through the full 8-8-8 cycle.

Canonical references on disk:

- `H:\My Drive\888_Memory_Protocol\docs\01_Philosophies\MEM_PHILOSOPHY_888.md` (Master)
- `H:\My Drive\888_Memory_Protocol\docs\adr\010_888_Memory_Protocol_and_Documentation_Reorg.md` (EVA's ADR-010)
- `H:\My Drive\888_Memory_Protocol\specs\EVA_9_0_0\MEMORY_COMPRESSION_SPEC.md`

### 0.2 Why this matters to cognitive_system

cognitive_system (GKS + MSP) currently has three memory-shaping modules — none of which fully solves cross-session distillation:

| Module | Scope | Limit |
|---|---|---|
| `packages/msp/src/orchestrator/compressor/` | 3-tier `keep/trim/resummarise` | **Within a single episode** only |
| `packages/msp/src/orchestrator/consolidator/` (Gemini WIP, currently broken on main) | Hybrid det+LLM scoring over a session | **Within a single session** only |
| `EpisodicLayer.appendTrace` + `episodic.ts` | Persisted turn logs | **No upward synthesis** |

There is no module that:

1. Compacts **N sessions → 1 narrative** (Core).
2. Promotes **N narratives → 1 identity belief** (Sphere).
3. Tracks **epistemic confidence** as a first-class field on memory units.
4. Tags memory by **domain** with domain-specific decay / promotion policy.
5. Performs **belief revision** when long-term beliefs are contradicted.

8-8-8 fills exactly those gaps.

---

## 1. Root Cause Analysis (per CLAUDE.md MASTER BLOCK)

**Question:** Why adopt 8-8-8 rather than extending the existing compressor / consolidator?

**Answer:** The existing modules solve **density within a time window** (one episode, one session). 8-8-8 solves a **different axis** — **density across time** (many sessions → distilled identity). The two are orthogonal and complementary:

```
                    within-episode density
                            │
  consolidator ──────►──────┼──────► compressor
                            │
                  cross-time distillation
                            │
                   ┌─────── ▼ ───────┐
                   │  8-8-8 protocol │  ← gap we are filling
                   └─────────────────┘
```

Extending compressor or consolidator to do cross-session work would **violate Single Responsibility** and **conflict with `ADR--COMPRESSOR-THREE-TIER`** (whose decision rationale is "within-episode three-tier shrink-to-fit"). 8-8-8 deserves its own module, its own ADR, and its own atoms.

**Confirmed root cause:** Missing cross-session distillation primitive + missing confidence-as-first-class-data. Both are architectural gaps, not bugs.

---

## 2. Mapping — 8-8-8 ↔ cognitive_system

| 8-8-8 concept (EVA) | Cognitive_system equivalent | Status |
|---|---|---|
| Consciousness | Active LLM context (transient, not stored) | ✅ implicit |
| Session Memory | `EpisodicLayer.appendTrace` + `<brain>/session/` | ✅ exists |
| **Core Memory** | — | ❌ **new module needed** |
| **Sphere Memory** | — | ❌ **new module needed** |
| 4 Pillars (Clean / Summary / Index / Relation) | `compressor` (Clean/Summary) + vector indexer (Index) + backlinks (Relation) | 🟡 exists scattered — needs unified pipeline |
| Memory Domains | — | ❌ **new schema field needed** |
| Epistemic States | `status` (lifecycle, not confidence) | ❌ **new schema field needed** |
| Belief Revision (Sphere → Core downgrade) | — | ❌ **new protocol needed** |

---

## 3. Open questions for lead-dev sign-off

Three decisions must be made before Phase 1 begins. Each affects atom IDs and module names.

### Q1 — Naming
Adopt EVA's literal names (`Consciousness` / `Session` / `Core` / `Sphere`) **or** translate to cognitive_system-native (`Trace` / `Episode` / `Narrative` / `Identity` — or similar)?

- **Pro literal:** preserves cross-project vocabulary, lets EVA conversations stay readable.
- **Pro native:** matches existing GKS terminology (Episode, Atomic, etc.), avoids dual-vocab confusion.
- **Recommendation:** **Native names with EVA names as aliases in CONCEPT atom**. Rationale: cognitive_system has many readers who never touched EVA; consistency wins. Atom can carry `aliases: [Consciousness, Session, Core, Sphere]` for searchability.

### Q2 — Scope of first roll-out
Ship the full 4-5-PR roadmap (§5) at once **or** ship Phase A-B (schema + Session→Core) first, evaluate, then commit to Phase C-D?

- **Recommendation:** **Phase A-B first, then re-evaluate**. Distillation across 8 sessions is the new primitive; once that's running we'll have real telemetry to inform Sphere/belief-revision design.

### Q3 — Relationship to existing `consolidator`
Distiller should **replace** consolidator, **extend** it, or **sit alongside**?

- **Pro replace:** consolidator currently broken on main; rewrite under 8-8-8 vocabulary.
- **Pro extend:** consolidator is Gemini's WIP — replacing would step on toes.
- **Pro alongside:** clearest separation (consolidator = within-session; distiller = cross-session).
- **Recommendation:** **Sit alongside.** Distiller consumes consolidator's output (consolidated episodes) as input to Core Memory. This respects ownership boundaries and keeps each module's scope narrow.

---

## 4. Atom plan (doc-to-code order per CLAUDE.md §"Doc-to-code workflow (mandatory)")

### Phase 1 — CONCEPT atoms (`gks/concept/`)

| ID | Purpose | Size |
|---|---|---|
| `CONCEPT--TIERED-MEMORY-DISTILLATION` | Cross-session synthesis pipeline; 4 Pillars; configurable ratio (8 is default, not magic) | ~80 lines |
| `CONCEPT--EPISTEMIC-STATES` | Confidence as first-class data; `hypothesis` / `confirmed` / `contested` / `deprecated`; **distinct from** `status` lifecycle | ~60 lines |
| `CONCEPT--MEMORY-DOMAINS` | `safety` / `identity-relationship` / `knowledge-skill` / `contextual` / `meta`; per-domain decay + promotion policy | ~70 lines |
| `CONCEPT--BELIEF-REVISION` | Sphere → Core downgrade trigger + recovery rules | ~60 lines |

**Crosslinks to reconcile (in each CONCEPT):**

- `CONCEPT--CONTEXT-COMPRESSION` — orthogonal (within-episode vs. cross-session)
- `CONCEPT--EPISODE-RETENTION` — distiller supersedes ad-hoc retention
- `CONCEPT--RESOLUTION-GRADIENT` — Sphere = top-tier resolution
- `CONCEPT--TAXONOMY-V2-3` — domain + epistemic_state added to atom frontmatter

### Phase 2 — ADR atoms (`gks/adr/`)

| ID | Decision |
|---|---|
| `ADR--ADOPT-888-AS-INSPIRATION` | Adopt the **architecture** of 8-8-8 (not a literal EVA port); use cognitive_system-native naming (per Q1 decision) |
| `ADR--DISTILLATION-RATIO-CONFIGURABLE` | `8` is a config default at `msp.config.distillation.session_per_core` and `cores_per_sphere`. Validators may use {2..32} range |
| `ADR--EPISTEMIC-VS-LIFECYCLE-STATUS` | Separate field `epistemic_state` from existing `status`. Validator rule: `status` is monotonic (draft→active→stable→superseded); `epistemic_state` is non-monotonic and may regress on belief revision |

### Phase 3 — BLUEPRINT atoms (`gks/blueprint/`)

| ID | Scope |
|---|---|
| `BLUEPRINT--TIERED-DISTILLATION` | New module `packages/msp/src/orchestrator/distiller/` — consumes consolidator output, runs 4 Pillars pipeline, writes Core/Sphere atoms |
| `BLUEPRINT--BELIEF-REVISION-PROTOCOL` | Sphere downgrade trigger + epistemic state transition machine + `belief_under_revision` flag |

---

## 5. Code roadmap (after Phase 1-3 sign-off)

Each row = one PR. Branch: `claude/msp-888-phase-<N>-<slug>`.

| # | Scope | LoC est. | Session est. | Depends on |
|---|---|---|---|---|
| **A** | Schema extension — atom frontmatter `domain` + `epistemic_state`. Update `CONCEPT--TAXONOMY-V2-3`, validator (`packages/msp/src/validator/`), and `gks` types. Migration script for existing atoms (default `domain: meta`, `epistemic_state: confirmed`). | ~200 | 1 | Phase 1-3 done |
| **B** | `packages/msp/src/orchestrator/distiller/` — Session → Core extractor. LLM-driven 4 Pillars pipeline. Output: Core atom written via existing `retain()` API. | ~400 | 1-2 | A |
| **C** | Core → Sphere distiller. Reuses (B) pipeline; different threshold trigger. | ~300 | 1 | B |
| **D** | Belief revision protocol. Background task that scans Spheres, computes confidence trend, triggers downgrade. | ~200 | 1 | C |
| **E** | MCP integration: tool `distill` + CLI `gks distill` + dashboard hook + tests. | ~250 | 1 | D |

**Total estimate:** ~1350 LoC, 5-6 sessions of T3 work + Qwen microtasks for tests.

---

## 6. Out of scope (deferred to future ULTRAPLAN if pursued)

- **Habit Memory (Procedural)** — Section 5 of EVA's `MEM_PHILOSOPHY_888.md`. Procedural / response-pattern memory. Not in MSP today; separate effort.
- **Somatic Imprint (Body Memory) / PhysioCore Bias** — physiological-signal-backed memory. Requires PhysioCore subsystem, doesn't exist in cognitive_system.
- **Migration of existing 339 atoms** to populated `domain` field. Schema lands with default `meta` value; per-atom curation is operational, not technical.

---

## 7. Risks

| Risk | Mitigation |
|---|---|
| `gemini/m7b-consolidator` TS errors still on main (test 20/22 fail every PR) | Land that fix first OR `gh pr merge --admin` per Phase A-E PRs (matches what we did for Genesis Graph P3.1) |
| `8` ratio is wrong for cognitive_system's actual cadence | Already addressed by `ADR--DISTILLATION-RATIO-CONFIGURABLE` — start at 8, tune from telemetry |
| Distiller LLM cost balloons (one LLM call per 8 sessions × every user) | Phase B includes `costTracker` integration; budget cap configurable per tenant |
| Sphere belief contradicts atom system (atom is canonical) | `ADR--EPISTEMIC-VS-LIFECYCLE-STATUS` clarifies: atoms = doc lifecycle (authoritative); Sphere = derived wisdom (subordinate). On contradiction, atom wins; Sphere flagged `contested` |
| Naming bikeshed delays start (Q1) | Time-box Q1 discussion to one async pass; default to "native names + EVA aliases" recommendation |

---

## 8. Acceptance / Sign-off

This ULTRAPLAN is ready to execute when:

- [ ] Q1 (naming) answered by lead dev
- [ ] Q2 (scope) answered by lead dev
- [ ] Q3 (consolidator relationship) answered by lead dev
- [ ] `gemini/m7b-consolidator` TS errors fixed on main (or admin-merge approved)
- [ ] Phase 1 (CONCEPT atoms) authored and validated (`npm run msp:validate` green)
- [ ] Phase 2-3 atoms reviewed and promoted to `status: stable`

Once those gates pass, Phase A code work can start under the branch convention `claude/msp-888-phase-A-schema`.

---

## 9. References

- **Source canonical docs** — `H:\My Drive\888_Memory_Protocol\` (Google Drive synced from `D:\The Human Algorithm\T2\agent\docs\`)
  - `docs/01_Philosophies/MEM_PHILOSOPHY_888.md` — master philosophy
  - `docs/adr/010_888_Memory_Protocol_and_Documentation_Reorg.md` — EVA's ADR-010
  - `specs/EVA_9_0_0/MEMORY_COMPRESSION_SPEC.md` — runtime spec
- **Cognitive_system context**
  - `CLAUDE.md` §"Doc-to-code workflow (mandatory)"
  - `CLAUDE.md` `# ⚠️ MASTER BLOCK: ROOT CAUSE ANALYSIS MANDATE ⚠️`
  - `packages/msp/src/orchestrator/compressor/` — existing 3-tier within-episode compressor
  - `packages/msp/src/orchestrator/consolidator/` — Gemini WIP within-session consolidator
  - `gks/concept/CONCEPT--TAXONOMY-V2-3.md` — taxonomy that schema (Phase A) will extend
- **Similar ULTRAPLANs in this repo**
  - `docs/plans/ULTRAPLAN--UCF-IMPLEMENTATION.md` — UCF rollout (reference for format + phasing)
  - `docs/plans/ULTRAPLAN--AGENTIC-MONOREPO-PIVOT.md` — earlier major restructure
