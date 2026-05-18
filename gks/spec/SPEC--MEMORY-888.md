---
id: SPEC--MEMORY-888
phase: 2
type: spec
status: draft
vault_id: default
tier: process
source_type: axiomatic
title: SPEC — 8-8-8 Memory Synthesis Protocol — runtime memory layers + atom production contract
tags:
  - msp
  - memory
  - distillation
  - 8-8-8
  - epistemic-state
  - belief-revision
  - ucf
  - mll
  - spec
crosslinks:
  references:
    - SPEC--EPISODE-ATOM
    - CONCEPT--CONTEXT-COMPRESSION
    - CONCEPT--EPISODE-RETENTION
    - CONCEPT--RESOLUTION-GRADIENT
    - CONCEPT--TAXONOMY-V2-3
    - FEAT--COMPRESSOR
    - FRAMEWORK--MSP-ARCHITECTURE-V2
    - ADR--AGENTIC-MONOREPO-PIVOT
created_at: 2026-05-17T18:00:00.000+07:00
aliases:
  - SPEC
  - implementation_flow
  - Technical specification
cluster: implementation_flow
role: Technical specification
---

# SPEC — 8-8-8 Memory Synthesis Protocol

> **Status: draft** — adapted from EVA v9.6.2 `MEM_PHILOSOPHY_888.md` + `MEMORY_COMPRESSION_SPEC.md`; **not a faithful port** (EVA's own status is "NOT IMPLEMENTED"). cognitive_system will be the first runtime implementation.
> Tracked in `docs/plans/ULTRAPLAN--888-MEMORY-PROTOCOL.md`. Synthesised from REVIEW-01 (lead-dev seat), REVIEW-02 (second opinion), REVIEW-03 (Gemini third opinion), and the V2 draft at `docs/plans/SPEC--MEMORY-888-2.md`.

## 0. Three commitments that distinguish this SPEC from EVA's original

1. **Atom-first via production, not via storage.** Session / Core / Sphere artifacts live as **runtime state in `.brain/msp/projects/<ns>/memory/`**, NOT as atoms in `gks/`. They **produce** atoms (`NARRATIVE--`, `IDENTITY--`, `BELIEF--`, `RELATION--`, `AUDIT--BELIEF-DOWNGRADE`) which enter `gks/` through the standard `retain()` path. Memory artifacts are subordinate to the atom graph; they feed it, they are not part of it.

2. **MSP-only write authority for distilled layers.** Only the **Consciousness layer** (active LLM context during an agent turn) is LLM read/write. **Session / Core / Sphere are exclusively written by the MSP distiller process** between agent invocations. `retain()` does NOT target these paths; the file system tool MUST reject direct writes there (guard enforced in `packages/msp/src/guards/`).

3. **UCF-coupled re-injection.** Spheres are re-injected into the LLM via the **Universal Context Framework's Resolution Gradient** — FULL when task-relevant, MENTION for identity pinning, OMIT when irrelevant. No static prelude; no auto-injection of Cores.

## 1. Philosophy

> *"Awareness creates Data. Subconscious creates Wisdom."* — EVA v9.6.2

Re-grounded for cognitive_system:

| EVA term | cognitive_system interpretation |
|---|---|
| Awareness Domain | The active LLM context during an agent turn (Claude / Gemini CLI / Qwen CLI / Antigravity). Volatile; LLM has full R/W. |
| Subconscious | The MSP distiller process, running *between* agent invocations. Sole writer of Session / Core / Sphere. |
| Wisdom | Sphere-layer atoms participating in future context hydration via UCF. |

Memory is a **process of selective forgetting**, not a database of facts. The 8:1 ratio enforces that selectivity.

## 2. Authority Model

> Codified in `ADR--MEMORY-WRITE-AUTHORITY` (to be authored — Phase 0).

| Layer | Path | Authority | Lifetime |
|---|---|---|---|
| **Consciousness** | (no persistent path — LLM context buffer) | LLM: Full R/W during a turn | Volatile; dies at turn end |
| **Sessions** | `.brain/msp/projects/<ns>/memory/sessions/` | MSP distiller: Write Only | Days to weeks |
| **Cores** | `.brain/msp/projects/<ns>/memory/cores/` | MSP distiller: Write Only | Weeks to months |
| **Spheres** | `.brain/msp/projects/<ns>/memory/spheres/` | MSP distiller: Write Only | Months to indefinite |

**Three hard rules:**

1. The LLM **never** writes a Session / Core / Sphere file directly. Any such attempt MUST be rejected by MSP guards as a contract violation.
2. `retain()` does **not** target Session/Core/Sphere paths. It writes atoms to GKS as normal. Atoms produced *by* distillation enter GKS via the standard atom-write path.
3. Cores and Spheres are produced **only** by the canonical distiller. They are never edited after creation — belief revision (§7) always produces a *new* downgrade artifact, never an in-place mutation.

## 3. Memory Layer Definitions

### 3.1 Consciousness (active context)

Active LLM context during an agent turn. Not a folder; not persistent. Bounded by the LLM's context window. Governed by UCF's existing Subject hydration + Resolution Gradient + Identity pin — no new mechanism here.

### 3.2 Session

A closed interaction unit. Boundaries defined by the host application:

- Web UI (`apps/web`): chat close OR N hours idle (default N=24).
- CLI agents (Gemini / Qwen / Claude Code / Antigravity): agent process exit.
- Programmatic: `msp.session.open()` / `msp.session.close()`.

At Session end, the MSP distiller writes a **Session artifact** (§5.1) — a *normalized* record (not the raw chat log) suitable for downstream distillation. The full raw log is staged separately under `pending/raw/<session_id>.jsonl` and garbage-collected after the next Core is produced (default 30-day cap; see §17 Q3).

### 3.3 Core

Distilled output of **exactly N consecutive Sessions** (N=8 default; configurable per `ADR--DISTILLATION-RATIO-CONFIGURABLE`) within a single namespace. Captures narrative arc, recurring themes, and atoms connecting to those themes. Sequence-numbered within its parent Sphere. Immutable.

### 3.4 Sphere

Distilled output of **exactly N consecutive Cores** (N=8 default) within a single namespace. Captures identity-level beliefs, behavioural patterns, relationship structures — the "wisdom DNA" of the namespace. Sequence-numbered globally per namespace. Immutable (with one exception: belief revision per §7).

## 4. Storage Layout

```
.brain/
└── msp/
    └── projects/
        └── <ns>/
            ├── memory/
            │   ├── counters.json              # see §6
            │   ├── sessions/<ulid>.json
            │   ├── cores/<sphere_seq>_<core_seq>.json
            │   └── spheres/<sphere_seq>.json
            ├── pending/                       # in-flight distillation, see §8.3
            └── revisions/                     # belief-revision artifacts, see §7
```

**Filename rules:**

- Sessions use **ULIDs** so listing yields chronological order without parsing JSON. The ULID is the canonical `session_id`.
- Cores embed parent Sphere sequence + own ordinal: `0001_0003.json` = 4th Core within Sphere 1 (0-indexed Sphere, 1-indexed Core — matches EVA convention).
- Spheres use Sphere sequence only: `0001.json` = first Sphere.

**Why not `gks/`?** The `gks/` tree is the project's SSOT for **governance documents** (ADRs, FEATs, BLUEPRINTs, atom_registry). Sessions / Cores / Spheres are **runtime state** — grow continuously, reset per environment. Putting them in `gks/` would conflate two very different lifecycles and bloat `atomic_index.jsonl` with hundreds of runtime files. The EVA reference filesystem confirms this design intent.

**This resolves Q4 from `ULTRAPLAN--888-MEMORY-PROTOCOL`.**

## 5. Schemas

All schemas are **JSON Schema Draft 2020-12**, published under `packages/msp/schemas/memory/` and referenced by `schema_version` in every artifact:

- `session.v1.json`
- `core.v1.json`
- `sphere.v1.json`

The full schema definitions (with field rules, integrity hashes, and example payloads) live in those schema files. This SPEC pins their **shape contract** at a high level:

### 5.1 Session artifact — required fields

`schema_version`, `session_id` (ULID), `namespace`, `opened_at`, `closed_at`, `agent` (`{model, tier, host}`), `compression_meta` (`{session_seq, core_seq, sphere_seq}`), `summary` (≤ 280 chars, distiller-produced not agent-produced), `key_atoms[]` (atoms touched in ≥ 3 turns OR subject of `retain()`), `turn_count`, `atom_writes`, `atom_reads`. Raw chat NOT included (lives in `pending/raw/`).

### 5.2 Core artifact — required fields

`schema_version`, `core_id` (`<sphere_seq>_<core_seq>`), `namespace`, `sphere_seq`, `core_seq`, `timestamp_start`, `timestamp_end`, `source_sessions[]` (8 ULIDs), `narrative` (`{summary ≤ 600 chars, themes[], arc}`), `concept_clusters[]` (`{label, weight ∈ [0,1], associated_atoms[], source_sessions[]}` — `sum(weight) ≤ 1`), `produced_atoms[]`, `integrity_hash` (SHA-256 over canonicalised `source_sessions`), `compression` (`{method, model, tokens_in, tokens_out}`).

### 5.3 Sphere artifact — required fields

`schema_version`, `sphere_id`, `namespace`, `sphere_seq`, `timestamp_start`, `timestamp_end`, `source_cores[]` (8 ids), `identity_summary` (≤ 800 chars), `core_beliefs[]` (`{statement, confidence ∈ [0,1], epistemic, domain, evidence_atoms[]}`), `behavioral_patterns[]` (optional; gated by affect policy §9), `produced_atoms[]`, `integrity_hash`, `compression`.

**`epistemic` enum:** `hypothesis | confirmed | contested | deprecated` — per EVA `MEM_PHILOSOPHY_888.md` §4.2.
**`domain` enum:** `safety | identity | knowledge | contextual | meta` — per EVA §4.1.

## 6. The 8-8-8 Cycle & Counter Management

A single counter file governs the entire cycle per namespace:

`.brain/msp/projects/<ns>/memory/counters.json`

```json
{
  "schema_version": "1.0.0",
  "namespace": "freshair129/cognitive_system",
  "session_seq": 3,
  "core_seq": 1,
  "sphere_seq": 0,
  "total_sessions": 11,
  "last_session_id": "01JBXX...",
  "last_update": "2026-05-17T15:30:00Z"
}
```

**Update protocol (atomic):**

```
on session_close(session_id):
  1. write Session artifact (§5.1) using current (session_seq, core_seq, sphere_seq)
  2. session_seq += 1; total_sessions += 1
  3. if session_seq == ratio.sessions_per_core:
        emit core_distill_job(core_seq, sphere_seq, last_N_sessions)
        session_seq = 0; core_seq += 1
        if core_seq == ratio.cores_per_sphere:
           emit sphere_distill_job(sphere_seq, last_N_cores)
           core_seq = 0; sphere_seq += 1
  4. write counters.json atomically (tmp file + rename)
  5. fsync
```

**Concurrency:** Counter updates MUST hold an OS-level advisory file lock. Concurrent session closes on the same namespace are serialized.

**Recovery:** If MSP crashes between steps 1 and 4, on startup MSP MUST reconcile: count Session files vs `total_sessions`; repair if drift is exactly 1 (single in-flight write), error otherwise.

**Triggering:** Distillation jobs are **queued, not synchronous** — they run in a background worker. `session_close()` returns immediately. (EVA's `MEMORY_COMPRESSION_SPEC.md` §"Design Questions" Option B.)

**Anti-magic-number rule (validator enforced):** The literal integers `8` for `sessions_per_core` and `cores_per_sphere` MUST live ONLY in config files (`msp.config.distillation.*`). The validator MUST refuse any PR introducing literal `8` constants for these knobs in `packages/msp/src/orchestrator/distiller/`. Range: `[2, 32]`. See `ADR--DISTILLATION-RATIO-CONFIGURABLE`.

## 7. Belief Revision Protocol

From EVA §4.3: when a Sphere-level belief is challenged repeatedly and confidence cannot recover within N sessions, downgrade Sphere → Core and mark `belief_under_revision`.

**Detection** (runs at every session close):

1. For each `core_belief` in the most recent Sphere where `epistemic === 'confirmed'`:
   - If any session closed in the last N=8 sessions produced an atom whose stance opposes the belief (per `ADR--BELIEF-CONFLICT-DETECTION` — to be authored), increment `challenge_counter[<sphere_seq>:<belief_index>]`.
2. If `challenge_counter ≥ 3` within a rolling window of 8 Sessions, trigger downgrade.

**Downgrade artifact** at `.brain/msp/projects/<ns>/memory/revisions/<ulid>.json`:

```jsonc
{
  "revision_id": "01JBZZ...",
  "downgrade_target": { "sphere_id": "0001", "belief_index": 2 },
  "trigger": "challenge_threshold_exceeded",
  "challenge_sessions": ["...", "...", "..."],
  "new_epistemic": "contested",
  "new_confidence": 0.4,
  "rationale": "Three Sessions in the last cycle produced atoms whose stances contradict the original belief.",
  "created_at": "2026-05-17T15:30:00Z"
}
```

**Effect:** The next Sphere distillation consumes the revision file. The new Sphere either restates the belief with lowered confidence, removes it, or restates as a new `hypothesis`. **The original Sphere file is NOT mutated** — revision is monotonic.

**Audit trail:** Revisions also emit an `AUDIT--BELIEF-DOWNGRADE-<ULID>` atom into GKS, citing both the revision file and the original Sphere. **Manual override** (operator restores an incorrectly-downgraded belief) emits a second revision with `kind: sphere_restore`.

## 8. The Distillation Pipeline — 4 Pillars

> Codified in `BLUEPRINT--DISTILLER-PIPELINE` (to be authored).

The distiller is a single MSP component implementing 4 pillars in order — per EVA §2:

| Pillar | Input | Action | Output |
|---|---|---|---|
| **Clean** | Raw turn log + Session artifacts | Strip greetings, boilerplate, duplicate clarifications. Remove turns where atom impact = 0. | Cleaned turn list |
| **Summary** | Cleaned turn list | LLM call producing `narrative.summary`, `themes[]`, `arc` (Core) or `identity_summary`, `core_beliefs[]` (Sphere). Bounded output (1–2k tokens). | Narrative object |
| **Index** | Narrative + source atoms | Compute embeddings; insert into project vector store under `memory/{core,sphere}` namespace. | Vector entries |
| **Relation** | Narrative + atoms | For each `concept_cluster` (Core) or `core_belief` (Sphere), produce a `RELATION--*` atom linking cluster/belief to evidence atoms. Insert into GKS. | New atoms |

Each pillar is separately testable. Mid-pillar crash leaves Session/Core intact — only the in-flight artifact is lost (§8.3).

### 8.2 LLM Choice — per-namespace config

| Layer | Default | Rationale |
|---|---|---|
| Core distillation | Claude Sonnet 4.6 | Good summarization; lower cost than Opus; fires every 8 sessions |
| Sphere distillation | Claude Opus 4.7 | Identity synthesis benefits from stronger model; fires once per 64 sessions |

Override via `memory.config.json` (`{"core_llm": "...", "sphere_llm": "..."}`).

### 8.3 Pending / Failure Handling

In-flight artifacts live under `.brain/msp/projects/<ns>/memory/pending/`. On success: `rename(2)` into canonical directory; commit counter. On failure (LLM error / schema mismatch / atom-registry rejection): retain pending file for inspection; do NOT advance counter; retry on next MSP startup. After 3 consecutive failures on the same artifact: emit `FAILURE--DISTILLER-<ULID>` atom + notification. Namespace is **not** blocked — new Sessions continue to accumulate.

## 9. Affect Policy — three tiers

EVA's published spec includes a rich affect model (`stress_load`, `social_warmth`, `qualia.intensity`, `Resonance_index`, etc.) — but EVA's **shipped** Core/Sphere schemas contain none of it. We resolve via tiered policy:

| Tier | Name | Behaviour |
|---|---|---|
| **a** | Spartan (**default**) | No affect fields. Capture *what was discussed and decided*, not *how it felt*. |
| **b** | LLM-scored | Distiller asks LLM for a small affect vector per Core (`engagement`, `urgency`, `confidence`). Stored under optional `affect` block. |
| **c** | PhysioCore-backed | Affect sourced from biosensor / telemetry. **Out of scope.** Requires `PRD--PHYSIOCORE` (not yet authored). |

Tier (a) is what EVA actually shipped — we default to evidence-of-working over aspirational. Tier (b) is opt-in per namespace. Tier (c) forbidden until prerequisite subsystem exists.

## 10. UCF Integration: Sphere Injection

> Codified in `FEAT--SPHERE-CONTEXT-INJECTION` (to be authored).

Spheres participate in UCF Subject hydration. When UCF assembles context, it consults the current Sphere (or all Spheres in namespace history, bounded by `max_spheres_in_context`, default 1) and emits at a Resolution Gradient appropriate to task class:

| Task class | Sphere resolution |
|---|---|
| Identity / personality / "who are you" | FULL — entire `identity_summary` + all `core_beliefs` with `confidence ≥ 0.7` |
| Architectural / design decisions | SUMMARY — `identity_summary` + `core_beliefs` with `domain = "identity"` |
| Code editing / tactical work | MENTION — single sentence: "Per Sphere 0001 identity (4 sessions ago): …" |
| Pure factual lookup unrelated to project | OMIT |

Task class is determined by UCF's existing classifier (`docs/msp/UNIVERSAL-CONTEXT-FRAMEWORK_spec.md` §3) — no new classifier.

**Token budget:** Sphere injection MUST respect UCF's existing budget. If FULL exceeds remaining budget after subject hydration, the distiller's pre-computed SUMMARY substitutes automatically.

**Cores are NOT auto-injected** — they serve only as raw material for Sphere distillation and as fallback retrieval for temporal queries (§11.2).

## 11. Retrieval Strategy

### 11.1 Default — HeptStreamRAG memory streams

Existing HeptStreamRAG gains three new streams:

1. `sphere_stream` — vector search over `memory/spheres/`. Highest priority for identity-bearing queries.
2. `core_stream` — vector search over `memory/cores/`. Medium priority for "what did we decide last week" queries.
3. `session_stream` — vector search over `memory/sessions/`. Lowest priority; fallback for fine-grained reconstruction.

**Merge order:** atom-graph > sphere > core > obsidian > session. Each stream uses cosine similarity normalised to atoms' retention score scale.

### 11.2 Temporal retrieval

Queries with temporal phrases ("two weeks ago", "last cycle") trigger temporal-window lookup. MSP resolves the phrase to a date range, fetches Cores whose `timestamp_start..timestamp_end` overlaps, injects at SUMMARY resolution regardless of default Sphere policy.

## 12. MLL Integration & Boundaries

### 12.1 MLL pipeline position

The distiller is the **tail of the MLL** (`PRD--MLL`, `SPEC--MLL`) 12-stage pipeline:

- MLL stages 1–7: raw artifact → cleaned → tagged → classified → linked → atomized → registered.
- **This SPEC adds stages 8–12:** registered atoms within Session window → Session artifact → Core distillation → Sphere distillation → UCF re-injection.

**The distiller IS an MLL component.**

### 12.2 Distiller-vs-Consolidator boundary

> Codified in `ADR--DISTILLER-VS-CONSOLIDATOR-BOUNDARY` (to be authored).

The M7b consolidator merges duplicate / stale atoms **within GKS**. It is **not** the distiller. Different inputs, outputs, cadences:

| | Distiller | Consolidator |
|---|---|---|
| Input | Sessions | Atoms in GKS |
| Output | Memory artifacts → produced atoms | Merged / superseded atoms |
| Cadence | Every 8 Sessions | Continuous |

### 12.3 Distiller-vs-SkillCreator boundary

> Codified in `ADR--DISTILLER-VS-SKILL-CREATOR-BOUNDARY` (to be authored).

EVA §5 mentions "Habit Memory (Procedural)" also owned by MSP. **This SPEC defers all procedural-pattern extraction to the MLL Skill Creator.** Cores / Spheres capture **narrative and factual content** — not reusable agent skills. If a Core reveals a recurring procedural pattern, the Core notes it as a theme; actual skill extraction is the Skill Creator's job.

## 13. Atom Production Rules & Budget

### 13.1 Per Core

| Trigger | Atom prefix | Cardinality |
|---|---|---|
| Core created | `NARRATIVE--CORE-<ns>-<sphere>-<core>` | 1 per Core |
| Each `concept_cluster` with `weight ≥ 0.5` | `CONCEPT--<slug>` | N per Core |
| Each pair `(cluster, source_session)` | `RELATION--CONCEPT-IN-SESSION` | M per Core |

### 13.2 Per Sphere

| Trigger | Atom prefix | Cardinality |
|---|---|---|
| Sphere created | `IDENTITY--PROJECT-DNA-<ns>-<sphere>` | 1 per Sphere |
| Each `core_belief` with `confidence ≥ 0.85` | `BELIEF--<slug>` | N per Sphere |
| Each `behavioral_pattern` (if affect policy ≥ b) | `PATTERN--<slug>` | M per Sphere |
| Belief Revision triggered | `AUDIT--BELIEF-DOWNGRADE-<ULID>` | 1 per revision |

All produced atoms inherit the namespace of their originating Core/Sphere and carry a `produced_by` field pointing back to the artifact file. They participate in the atom graph like any other atom.

### 13.3 Atom budget (REVIEW-02 §2.3 mitigation)

Hard caps: **20 atoms per Core**, **40 atoms per Sphere**. Excess clusters / beliefs are merged or dropped. On cap hit, emit `FAILURE--ATOM-BUDGET-EXCEEDED` atom and retry distillation with a stricter summarisation prompt.

## 14. Telemetry & Observability (REVIEW-03 mandate)

Distiller MUST emit (Prometheus naming, namespace label `ns`):

- `msp_memory_sessions_total{ns}` — counter
- `msp_memory_cores_total{ns}` — counter
- `msp_memory_spheres_total{ns}` — counter
- `msp_memory_distill_duration_seconds{ns,layer}` — histogram (layer ∈ {core, sphere})
- `msp_memory_distill_tokens_in{ns,layer}` — counter
- `msp_memory_distill_tokens_out{ns,layer}` — counter
- `msp_memory_distill_failures_total{ns,layer,stage}` — counter
- `msp_memory_atoms_produced_total{ns,layer,prefix}` — counter
- `msp_memory_belief_revisions_total{ns}` — counter
- `msp_memory_sphere_injection_resolution{ns,resolution}` — counter

Telemetry is **non-optional**. A Phase-5 acceptance gate is dashboard render at `apps/web/observability/memory`.

## 15. Validation

The validator (`packages/msp/src/validator/cli.ts`) MUST enforce:

1. New atom-frontmatter fields `domain` and `epistemic_state` on all NEW atoms produced by distillation (§13). Warning for legacy atoms missing them — one-version grace.
2. `epistemic_state` transition legality per §5.3 enum.
3. `status` monotonicity (already enforced; unchanged).
4. `core_id` / `sphere_id` reference integrity against `counters.json`.
5. `produced_atoms[*]` must resolve in `atom_registry.yaml` (no dangling).
6. `integrity_hash` on Core/Sphere validates against canonicalised `source_*` array.
7. Belief-revision log events well-formed JSON, time-ordered.
8. Distillation ratio knobs within `[2, 32]` when configured.
9. **Anti-magic-number:** No literal `8` for ratio knobs in `packages/msp/src/orchestrator/distiller/*.ts`. Validator regex check.
10. Memory artifacts (`.brain/msp/.../memory/`) MUST NOT appear in `atomic_index.jsonl` — validator skips them by path prefix.

## 16. Out of Scope (explicit)

- **PhysioCore subsystem** — biosensor-sourced affect; requires own PRD.
- **Somatic imprint** — body-state memory; tied to PhysioCore.
- **Cross-namespace Sphere sharing** — per-namespace v0.
- **Real-time emotion modelling during a turn** — distillation runs between turns, never during.
- **Procedural skill extraction** — owned by MLL Skill Creator.
- **Atom mutation by the LLM** — atoms produced by distiller via MSP write authority, never by agent directly.
- **EVA's qualia / RIM / 9D matrix / Endocrine state** — belong to EVA's PhysioCore tier; not ported.
- **Implementation phases & test plans** — belong in `BLUEPRINT--DISTILLER-PIPELINE`, not this SPEC.

## 17. Open Questions

Surfaced by REVIEW-01/02/03; tracked here for traceability. SPEC takes provisional positions where forced.

| # | Question | Provisional answer | Status |
|---|---|---|---|
| Q1 | Naming — EVA literal vs. cognitive_system-native? | EVA names (Session/Core/Sphere) for runtime layers + EVA-style atom prefixes (`NARRATIVE--`, `IDENTITY--`, `BELIEF--`, `RELATION--`) for produced atoms | implicit-resolved |
| Q2 | Distiller shares infra with M7b consolidator? | No — distinct process per `ADR--DISTILLER-VS-CONSOLIDATOR-BOUNDARY` | resolved |
| Q3 | Failure mode when affect policy = `llm_scored` returns invalid affect? | Strip the affect block; do not fail the whole Core; log warning | resolved |
| Q4 | Storage path — workspace-local vs. identity-global? | Workspace-local `.brain/msp/projects/<ns>/memory/` — see §4 rationale | resolved |
| Q5 | Sphere injection rate-limit on hot loops (50 turns/10 min)? | UCF Subject hydration already caches; defer unless profiling shows a hot path | resolved |
| Q6 | Namespace cold start — what does Sphere injection do with zero Sessions? | OMIT entirely; first Sphere appears at Session #64 | resolved |
| Q7 | Cores embedded into vector store fully or summary-only? | Both — full text for fallback retrieval, summary for fast match | resolved |
| Q8 | Retention for `pending/raw/*.jsonl`? | 30 days then delete on next Core write | resolved |
| Q9 | LLM provider for distillation — pluggable or pinned? | Pluggable per namespace; defaults Sonnet 4.6 (Core) / Opus 4.7 (Sphere); see §8.2 | resolved |

The remaining gate before promotion to `status: active` is **lead-dev sign-off + atom_registry.yaml entries for the manifest in §18**.

## 18. Atom manifest

Atoms this SPEC introduces (to be added to `atom_registry.yaml` in Phase 0 of `ULTRAPLAN--888-MEMORY-PROTOCOL`):

```yaml
# Specs (this file)
- id: SPEC--MEMORY-888
  level: P2
  cluster: memory

# ADRs (children)
- id: ADR--MEMORY-WRITE-AUTHORITY
  level: P2
  cluster: memory
- id: ADR--DISTILLATION-RATIO-CONFIGURABLE
  level: P2
  cluster: memory
- id: ADR--DISTILLER-VS-CONSOLIDATOR-BOUNDARY
  level: P2
  cluster: memory
- id: ADR--DISTILLER-VS-SKILL-CREATOR-BOUNDARY
  level: P2
  cluster: memory
- id: ADR--AFFECT-POLICY-DEFAULT
  level: P2
  cluster: memory
- id: ADR--BELIEF-CONFLICT-DETECTION
  level: P2
  cluster: memory
- id: ADR--EPISTEMIC-VS-LIFECYCLE-STATUS
  level: P2
  cluster: memory

# FEATs
- id: FEAT--MEMORY-DISTILLER
  level: P2
  cluster: memory
- id: FEAT--COMPRESSION-COUNTER-MGMT
  level: P2
  cluster: memory
- id: FEAT--SPHERE-CONTEXT-INJECTION
  level: P2
  cluster: memory
- id: FEAT--BELIEF-REVISION-DOWNGRADE
  level: P2
  cluster: memory
- id: FEAT--MEMORY-RAG-STREAMS
  level: P2
  cluster: memory

# BLUEPRINTs
- id: BLUEPRINT--DISTILLER-PIPELINE
  level: P3
  cluster: memory
- id: BLUEPRINT--MEMORY-STORAGE-LAYOUT
  level: P3
  cluster: memory
- id: BLUEPRINT--SPHERE-INJECTION-FLOW
  level: P3
  cluster: memory
```

These form the **`memory` cluster** — a new cluster proposed for taxonomy v2.4 (or fit into an existing cluster, pending taxonomy review).

## 19. References

**Internal:**
- `docs/plans/ULTRAPLAN--888-MEMORY-PROTOCOL.md` — the plan this SPEC implements
- `docs/plans/ULTRAPLAN--888-MEMORY-PROTOCOL--REVIEW-01.md` — lead-dev seat review
- `docs/plans/ULTRAPLAN--888-MEMORY-PROTOCOL--REVIEW-02.md` — second-opinion review
- `docs/plans/ULTRAPLAN--888-MEMORY-PROTOCOL--REVIEW-03.md` — Gemini third opinion
- `docs/plans/SPEC--MEMORY-888-2.md` — V2 draft this SPEC is hybridised from
- `docs/msp/UNIVERSAL-CONTEXT-FRAMEWORK_spec.md`
- `docs/gks/PRD--MLL.md`, `docs/gks/SPEC--MLL.md`
- `docs/gks/PRD--GENESIS-BLOCK-CYCLE.md`
- `atom_registry.yaml`

**External (inspiration only — not authoritative):**
- EVA v9.6.2 `MEM_PHILOSOPHY_888.md` (mirrored at `H:\My Drive\888_Memory_Protocol\docs\01_Philosophies\`)
- EVA v9.6.2 `MEMORY_COMPRESSION_SPEC.md` (mirrored at `H:\My Drive\888_Memory_Protocol\specs\EVA_9_0_0\`)

## 20. Connections
- [[SPEC--EPISODE-ATOM]] — immutable evidence pattern reused for Session artifacts
- [[CONCEPT--CONTEXT-COMPRESSION]] — orthogonal (within-episode vs. cross-session)
- [[CONCEPT--EPISODE-RETENTION]] — Session retention interacts with §4 + §17 Q8
- [[CONCEPT--RESOLUTION-GRADIENT]] — Sphere = top tier; consumed by §10
- [[FEAT--COMPRESSOR]] — within-episode compressor; feeds Session content
- [[FRAMEWORK--MSP-ARCHITECTURE-V2]] — places distiller in orchestrator layer
- [[ADR--AGENTIC-MONOREPO-PIVOT]] — packages/msp ownership of memory subsystem
