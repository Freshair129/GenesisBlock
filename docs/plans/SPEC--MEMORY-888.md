# SPEC--MEMORY-888

**Title:** Hierarchical Memory Compression Protocol for `cognitive_system`
**Atom ID:** `SPEC--MEMORY-888`
**Level:** P2 SPEC
**Status:** Draft v0.1 — ready for review
**Owner:** MSP package (`packages/msp`)
**Replaces:** None (new)
**Source inspiration:** EVA v9.6.2 `MEM_PHILOSOPHY_888.md` + `MEMORY_COMPRESSION_SPEC.md` — *adapted, not ported*
**Related atoms:**
`ULTRAPLAN--888-MEMORY-PROTOCOL`, `PRD--MLL`, `SPEC--MLL`,
`UCF-SPEC` (Universal Context Framework), `PRD--GENESIS-BLOCK-CYCLE`,
`ADR--AGENTIC-MONOREPO-PIVOT`, `WALKTHROUGH--KNOWLEDGE-ARCHITECTURE-STANDARDIZATION`

---

## 0. Document Identity

This SPEC defines how `cognitive_system` implements *hierarchical memory compression* — the practice of compressing raw interaction logs (Sessions) into mid-term narratives (Cores), then into long-term identity/wisdom artifacts (Spheres), at fixed 8:1 ratios.

**Three commitments distinguish this SPEC from EVA's original:**

1. **Atom-first.** Every Core and Sphere produced by the distiller MUST emit atoms registered in `atom_registry.yaml`. There is no memory artifact in `cognitive_system` that exists outside the atom graph. Memory is a **knowledge type**, not a parallel ontology.
2. **MSP-only write authority for Session/Core/Sphere.** The Consciousness layer (active LLM context during agent execution) is the only layer where the LLM has read/write. All distilled layers are produced exclusively by the MSP distiller process. The retain tool does **not** write Cores or Spheres directly.
3. **UCF-coupled injection.** Spheres are reinjected into LLM context not as a static prelude but via the Universal Context Framework's `Resolution Gradient` — FULL when task-relevant, MENTION for identity pinning, omit when irrelevant.

**What this SPEC is not.** This is not a faithful port of EVA. EVA's own implementation status (per `MEMORY_COMPRESSION_SPEC.md` §"Current Status") is **NOT IMPLEMENTED** — its shipped `consciousness/` folder contains empty `05_core_memory/` and `06_sphere_memory/`. `cognitive_system` will be the **first runtime implementation** of the 8-8-8 protocol. We therefore take the philosophy seriously, and the unimplemented JSON examples loosely.

---

## 1. Philosophy

> *"Awareness creates Data. Subconscious creates Wisdom."*
> — EVA v9.6.2 MEM_PHILOSOPHY

We accept this dichotomy and re-ground it for our system:

| EVA term | `cognitive_system` interpretation |
| --- | --- |
| Awareness Domain | The active LLM context during an agent turn (Claude / Gemini CLI / Qwen CLI / Antigravity IDE). The LLM may read and write freely within this volatile space. |
| Subconscious | The MSP distiller process, which runs *between* agent invocations. It is the only writer for Session, Core, and Sphere layers. |
| Wisdom | Sphere-layer atoms that participate in future context hydration via UCF. |

Memory is therefore a **process** of selective forgetting, not a database of facts. The 8:1 ratio enforces that selectivity: you cannot keep everything, so you must extract what matters.

---

## 2. Authority Model

> Codified in `ADR--MEMORY-WRITE-AUTHORITY` (to be authored).

| Layer | Path | Authority | Lifecycle |
| --- | --- | --- | --- |
| **Consciousness** | (no persistent path — LLM context buffer) | **LLM: Full R/W** during a turn | Volatile; dies at turn end |
| **Sessions** | `.brain/msp/projects/<ns>/memory/sessions/` | **MSP distiller: Write Only** | Days to weeks |
| **Cores** | `.brain/msp/projects/<ns>/memory/cores/` | **MSP distiller: Write Only** | Weeks to months |
| **Spheres** | `.brain/msp/projects/<ns>/memory/spheres/` | **MSP distiller: Write Only** | Months to indefinite |

**Three hard rules:**

1. The LLM **never** writes a Session, Core, or Sphere file directly. Any attempt to do so via the file system tool is a contract violation and MUST be rejected by MSP guards.
2. The `retain()` API (the canonical atom-write surface) does **not** target Session/Core/Sphere paths. It writes atoms to GKS as normal. Distillation atoms produced from Cores and Spheres are inserted into GKS by the distiller, not by user-facing tools.
3. Cores and Spheres are produced **only** by the canonical distiller process. They are never edited after creation. To revise a belief, see §7 (Belief Revision Protocol) — which always produces a *new* downgrade artifact, never an in-place mutation.

This authority model resolves the open Q1 from `ULTRAPLAN--888-MEMORY-PROTOCOL`: distiller and consolidator are different processes; consolidator (per `M7b-CONSOLIDATOR`) operates on atoms within GKS, distiller (per this SPEC) operates on memory artifacts producing atoms.

---

## 3. Memory Layer Definitions

### 3.1 Consciousness (active context)

The active LLM context during an agent turn. Not a folder; not persistent. Bounded by the LLM's context window.

The `cognitive_system` UCF spec already governs what enters the Consciousness layer (Subject hydration, Resolution Gradient, Identity pin). No new mechanism is introduced here — the 8-8-8 protocol simply consumes UCF as-is and contributes Spheres back into it.

### 3.2 Session

A Session is a single closed interaction unit. Its boundaries are defined by the host application:

- For the web UI (`apps/web`): a Session ends when the user explicitly closes a chat or after N hours of inactivity (N configurable; default 24).
- For CLI agents (Gemini CLI, Qwen CLI, Claude Code, Antigravity): a Session ends when the agent process exits.
- For programmatic use: a Session is delimited by `msp.session.open()` / `msp.session.close()` calls.

At Session end, the MSP distiller writes a `Session` artifact (see §5.1) by snapshotting all atoms produced during the Session window, plus a structured log of agent turns. This is **not** the raw chat log — it is a normalized record suitable for downstream distillation.

### 3.3 Core

A Core is the distilled output of **exactly 8 consecutive Sessions** within a single namespace. It captures the narrative arc, recurring themes, and the atoms that connect to those themes.

Cores are sequence-numbered within their Sphere. They are immutable after creation.

### 3.4 Sphere

A Sphere is the distilled output of **exactly 8 consecutive Cores** within a single namespace. It captures identity-level beliefs, behavioral patterns, and relationship structures — the "wisdom DNA" of the project namespace.

Spheres are sequence-numbered globally per namespace. They are immutable after creation (with one exception: see §7, Belief Revision).

---

## 4. Storage Layout

All paths are relative to the user's global brain root unless otherwise noted.

```
.brain/
└── msp/
    └── projects/
        └── <ns>/                              # one folder per project namespace
            ├── memory/
            │   ├── counters.json              # see §6
            │   ├── sessions/
            │   │   └── <ulid>.json            # one file per Session
            │   ├── cores/
            │   │   └── <sphere_seq>_<core_seq>.json
            │   └── spheres/
            │       └── <sphere_seq>.json
            ├── pending/                       # transient — see §8.3
            └── revisions/                     # belief-revision artifacts — see §7
```

**Filename rules:**

- Sessions use ULIDs so that listing them yields chronological order without parsing JSON. The ULID is the canonical `session_id`.
- Cores embed both their parent Sphere sequence and their own ordinal: `0001_0003.json` is the 4th Core within Sphere 1 (0-indexed Sphere, 1-indexed Core for human readability — matches EVA convention).
- Spheres use the Sphere sequence only: `0001.json` is the first Sphere.

**Why not `gks/`?** The `gks/` tree is the project's SSOT for *governance documents* (ADRs, FEATs, BLUEPRINTs, TASKs, atom_registry). Sessions, Cores, and Spheres are *runtime state* — they grow continuously and reset per environment. Putting them in `gks/` would conflate two very different lifecycles. The EVA `consciousness/10_archival_memory_storage/` layout (which we inspected) confirms this design intent: it is a runtime archival store, not a versioned governance tree.

This resolves the open Q4 from `ULTRAPLAN--888-MEMORY-PROTOCOL` and aligns with `ULTRAPLAN--888-MEMORY-PROTOCOL--REVIEW-02` §"Q4".

---

## 5. Schemas

All schemas are JSON Schema Draft 2020-12. Each schema MUST be published under `packages/msp/schemas/memory/` and referenced by version string in every artifact it validates.

### 5.1 Session Schema (`session.v1.json`)

```jsonc
{
  "schema_version": "1.0.0",
  "session_id":     "01JBXX...",                    // ULID
  "namespace":      "freshair129/cognitive_system",
  "opened_at":      "2026-05-17T13:00:00Z",
  "closed_at":      "2026-05-17T15:30:00Z",
  "agent": {
    "model":  "claude-opus-4-7",
    "tier":   "T3",
    "host":   "claude.ai/desktop"
  },
  "compression_meta": {
    "session_seq": 3,                                // 0..7 within current Core
    "core_seq":    1,                                // 0..7 within current Sphere
    "sphere_seq":  0                                 // monotonic per namespace
  },
  "summary":  "Boss reviewed three peer reviews of 888 protocol and approved Q4 storage path.",
  "key_atoms": [
    "ADR--AGENTIC-MONOREPO-PIVOT",
    "ULTRAPLAN--888-MEMORY-PROTOCOL"
  ],
  "turn_count":  47,
  "atom_writes": 8,                                  // atoms produced in this session
  "atom_reads": 142                                  // atoms referenced
}
```

**Field rules:**

- `summary` is produced by the distiller, not by the agent. It MUST be ≤ 280 characters (one short paragraph). Long-form narrative belongs in the next layer up (Core).
- `key_atoms` is a deduplicated list of atoms the Session most prominently touched (criterion: atom appeared in ≥ 3 distinct agent turns OR was the subject of a `retain()` call).
- The full chat log (turns, raw text) is **not** stored in the Session artifact. It is stored separately under `pending/raw/<session_id>.jsonl` and garbage-collected after the next Core is produced. This prevents Session artifacts from bloating.

### 5.2 Core Schema (`core.v1.json`)

We deliberately ship a **lean** Core schema, mirroring what EVA actually shipped (not what its aspirational spec promised). Affect fields are gated behind an optional policy (§9).

```jsonc
{
  "schema_version":  "1.0.0",
  "core_id":         "0000_0001",
  "namespace":       "freshair129/cognitive_system",
  "sphere_seq":      0,
  "core_seq":        1,
  "timestamp_start": "2026-05-10T00:00:00Z",         // first session's opened_at
  "timestamp_end":   "2026-05-17T15:30:00Z",         // 8th session's closed_at
  "source_sessions": [ "01JBXX...", "...", "..." ],   // 8 ULIDs

  "narrative": {
    "summary":  "One paragraph (≤ 600 chars) describing what these 8 sessions were about.",
    "themes":   [ "888-protocol-design", "ucf-integration", "review-consolidation" ],
    "arc":      "Began with architectural review; converged on schema-vs-spec gap analysis; ended with Q4 decision."
  },

  "concept_clusters": [
    {
      "label":               "memory-write-authority",
      "weight":              0.91,                    // 0..1 relative importance within this Core
      "associated_atoms":    [ "ADR--MEMORY-WRITE-AUTHORITY" ],
      "source_sessions":     [ "01JBXX...", "01JBYY..." ]
    }
  ],

  "produced_atoms": [
    "ADR--MEMORY-WRITE-AUTHORITY",
    "FEAT--MEMORY-DISTILLER"
  ],

  "integrity_hash": "sha256:abc123...",               // hash of canonicalized source_sessions
  "compression":   { "method": "llm_summarization", "model": "claude-opus-4-7", "tokens_in": 18432, "tokens_out": 1204 }
}
```

**Field rules:**

- `concept_clusters[*].weight` is normalized within a single Core (`sum(weight) ≤ 1`, allowing some weight to be unallocated as "noise").
- `associated_atoms` MUST resolve in `atom_registry.yaml`. The distiller validates this before write.
- `produced_atoms` lists atoms the distiller created *as a consequence of this Core*. These atoms enter GKS via the standard atom-write path with the Core as their citation.
- `integrity_hash` is a SHA-256 over the canonical JSON of `source_sessions` (sorted by `session_id`). It enables tampering detection.

### 5.3 Sphere Schema (`sphere.v1.json`)

```jsonc
{
  "schema_version":  "1.0.0",
  "sphere_id":       "0001",
  "namespace":       "freshair129/cognitive_system",
  "sphere_seq":      1,
  "timestamp_start": "2026-04-01T00:00:00Z",
  "timestamp_end":   "2026-05-17T15:30:00Z",
  "source_cores":    [ "0000_0000", "0000_0001", "...", "0000_0007" ],

  "identity_summary":  "One paragraph (≤ 800 chars) of project-level identity at this Sphere boundary.",

  "core_beliefs": [
    {
      "statement":    "cognitive_system uses doc-to-code chain as the authoritative governance flow.",
      "confidence":   0.95,
      "epistemic":    "confirmed",                    // hypothesis | confirmed | contested | deprecated
      "domain":       "identity",                      // safety | identity | knowledge | contextual | meta
      "evidence_atoms": [ "FRAMEWORK--DOC-TO-CODE", "ADR--AGENTIC-MONOREPO-PIVOT" ]
    }
  ],

  "behavioral_patterns": [
    {
      "pattern":   "Decisions get codified into ADRs within the same session they are made.",
      "frequency": "consistent",
      "context":   "architectural-decisions"
    }
  ],

  "produced_atoms": [
    "IDENTITY--PROJECT-DNA-V1",
    "BELIEF--DOC-TO-CODE-MANDATORY"
  ],

  "integrity_hash": "sha256:...",
  "compression":   { "method": "multi_core_synthesis", "model": "claude-opus-4-7", "tokens_in": 9632, "tokens_out": 1834 }
}
```

**Field rules:**

- `core_beliefs[*].epistemic` and `core_beliefs[*].domain` enumerations match `MEM_PHILOSOPHY_888.md` §4.1–4.2 exactly. Belief Revision (§7) operates on this field set.
- `behavioral_patterns` is optional — it MAY be empty for namespaces that do not enable affect policy (§9).
- The Sphere produces at minimum one `IDENTITY--*` atom per cycle. Additional atoms are produced for each `core_belief` whose confidence ≥ 0.85.

---

## 6. The 8-8-8 Cycle & Counter Management

A single counter file governs the entire cycle for one namespace:

`.brain/msp/projects/<ns>/memory/counters.json`

```json
{
  "schema_version": "1.0.0",
  "namespace":      "freshair129/cognitive_system",
  "session_seq":    3,
  "core_seq":       1,
  "sphere_seq":     0,
  "total_sessions": 11,
  "last_session_id": "01JBXX...",
  "last_update":    "2026-05-17T15:30:00Z"
}
```

**Update protocol (atomic):**

```
on session_close(session_id):
  1. write Session artifact (§5.1) using current (session_seq, core_seq, sphere_seq)
  2. session_seq += 1; total_sessions += 1
  3. if session_seq == 8:
        emit core_distill_job(core_seq, sphere_seq, last_8_sessions)
        session_seq = 0
        core_seq += 1
        if core_seq == 8:
           emit sphere_distill_job(sphere_seq, last_8_cores)
           core_seq = 0
           sphere_seq += 1
  4. write counters.json atomically (tmp file + rename)
  5. fsync
```

**Concurrency:** The counter file MUST be updated under an OS-level advisory file lock. Concurrent Session closes on the same namespace are serialized. A second writer that finds the lock held queues its update behind the first.

**Recovery:** If MSP crashes between steps 1 and 4, the counter file is stale relative to the Session directory. On startup MSP MUST reconcile: count Session files, compare to `total_sessions`, repair counters if drift is exactly 1 (a single in-flight write), error otherwise.

**Triggering:** Distillation jobs are queued, **not** synchronous. The job runs in a background worker. The Session close call returns immediately. This implements `MEMORY_COMPRESSION_SPEC.md` §"Design Questions" Option B (asynchronous).

---

## 7. Belief Revision Protocol

From `MEM_PHILOSOPHY_888.md` §4.3: when a Sphere-level belief is repeatedly challenged and its confidence cannot recover within N sessions, it is *downgraded* from Sphere status back to Core status and marked `belief_under_revision`.

`cognitive_system` implements this as follows:

**Detection** (runs at every Session close):

1. For each `core_belief` in the most recent Sphere:
   - If the belief's `epistemic` is `confirmed` and any Session closed within the last 8 produced an atom whose stance opposes the belief (detected by a separate `ADR--BELIEF-CONFLICT-DETECTION`), increment a `challenge_counter` keyed `<sphere_seq>:<belief_index>`.
2. If `challenge_counter` ≥ 3 within a rolling window of 8 Sessions, trigger downgrade.

**Downgrade artifact:**

```
.brain/msp/projects/<ns>/memory/revisions/<ulid>.json
```

```jsonc
{
  "revision_id":      "01JBZZ...",
  "downgrade_target": { "sphere_id": "0001", "belief_index": 2 },
  "trigger":          "challenge_threshold_exceeded",
  "challenge_sessions": [ "...", "...", "..." ],
  "new_epistemic":    "contested",
  "new_confidence":   0.4,
  "rationale":        "Three Sessions in the last cycle produced atoms (ADR-X, AUDIT-Y, DEVLOG-Z) whose stances contradict the original belief statement.",
  "created_at":       "2026-05-17T15:30:00Z"
}
```

**Effect:** The next Sphere distillation (when current Sphere closes) MUST consume the revision file. The new Sphere either restates the belief with the lowered confidence, removes it entirely, or restates it as a new `hypothesis`. The original Sphere file is **not** mutated — revision is monotonic.

**Audit trail:** Revisions are also written as `AUDIT--BELIEF-DOWNGRADE-<ULID>` atoms into GKS, citing both the revision file and the original Sphere.

---

## 8. The Distillation Pipeline

> Codified in `BLUEPRINT--DISTILLER-PIPELINE` (to be authored).

The distiller is a single MSP component implementing four pillars in order: **Clean → Summary → Index → Relation**. This matches `MEM_PHILOSOPHY_888.md` §2.

### 8.1 The Four Pillars

| Pillar | Input | Action | Output |
| --- | --- | --- | --- |
| **Clean** | Raw turn log + Session artifacts | Strip greetings, repeated boilerplate, duplicate clarifications. Remove turns where atom impact = 0. | Cleaned turn list |
| **Summary** | Cleaned turn list | LLM call producing `narrative.summary`, `themes[]`, `arc` (for Core) or `identity_summary`, `core_beliefs[]` (for Sphere). Bounded output (1-2k tokens). | Narrative object |
| **Index** | Narrative object + source atoms | Compute embeddings; insert into the project's vector store under `memory/{core,sphere}` namespace. | Vector entries |
| **Relation** | Narrative object + atoms | For each `concept_cluster` (Core) or `core_belief` (Sphere), produce a `RELATION--*` atom linking the cluster/belief to its evidence atoms. Insert into GKS. | New atoms |

Each pillar is a separately testable function with a clear input/output contract. Crashes mid-pillar leave Sessions/Cores intact (only the in-flight artifact is lost — see §8.3).

### 8.2 LLM Choice

The distiller is LLM-agnostic. The choice is per-namespace configuration. Defaults:

- **Core distillation:** Claude Sonnet 4.6 (good summarization, lower cost than Opus).
- **Sphere distillation:** Claude Opus 4.7 (identity-level synthesis benefits from the stronger model; less frequent — once per 64 Sessions).

A simple namespace config (`memory.config.json`) overrides defaults.

### 8.3 Pending / Failure Handling

While a distillation job runs, its in-flight artifact lives under `.brain/msp/projects/<ns>/memory/pending/`. On success it is `rename(2)`'d into the canonical directory and the counter is committed. On failure (LLM error, schema validation failure, atom-registry rejection), the pending file is retained for inspection but the counter is **not** advanced — the job retries on the next MSP startup.

After three consecutive failures on the same artifact, MSP escalates by writing a `FAILURE--DISTILLER-<ULID>` atom and emitting a notification. The namespace is *not* blocked — new Sessions continue to accumulate. When distillation eventually succeeds, the queued Sessions are processed in order.

---

## 9. Affect Policy

EVA's spec includes a rich affect model (`stress_load`, `social_warmth`, `joy_level`, `qualia.intensity`, `Resonance_index`, etc.). EVA's *shipped* Core and Sphere schemas, however, contain none of this (see filesystem inspection in this conversation's transcript: `Core_Memory_Schema.json` has only `core_id`, `timestamp`, `description`, `concept_clusters`, `integrity_hash`).

We resolve this gap with a three-tier policy:

| Tier | Name | Behavior |
| --- | --- | --- |
| **a** | Spartan (default) | No affect fields. Core/Sphere capture *what was discussed and decided*, not *how it felt*. |
| **b** | LLM-scored | The distiller asks the LLM to produce a small affect vector per Core (`engagement`, `urgency`, `confidence`) inferred from the agent's word choices. Stored under an optional `affect` block in the Core/Sphere schema. |
| **c** | PhysioCore-backed | Affect is sourced from a biosensor or telemetry subsystem (heart rate, click cadence, dwell time). **Out of scope for this SPEC.** Would require a new subsystem (`PRD--PHYSIOCORE` — not yet authored). |

**Default:** Tier (a). Tier (b) is opt-in per namespace via `memory.config.json` (`{"affect_policy": "llm_scored"}`). Tier (c) is forbidden until its prerequisite subsystem exists.

Rationale: EVA's published JSON examples in `MEMORY_COMPRESSION_SPEC.md` look like tier (c) but EVA's actual schemas look like tier (a). We default to what was actually shipped because it is the only configuration with evidence of working in practice.

---

## 10. UCF Integration: Sphere Injection

> Codified in `FEAT--SPHERE-CONTEXT-INJECTION` (to be authored).

The 8-8-8 protocol's value depends on Spheres being read back into the LLM's Consciousness, not merely archived. This SPEC requires that:

**Spheres participate in UCF Subject hydration.** When UCF assembles context for an agent turn, it consults the current Sphere (or all Spheres in the namespace's history, bounded by `max_spheres_in_context`, default 1) and emits them at a Resolution Gradient appropriate to the task.

| Task class | Sphere resolution |
| --- | --- |
| Identity / personality / "who are you" prompts | FULL — entire `identity_summary` plus all `core_beliefs` with `confidence ≥ 0.7` |
| Architectural / design decisions | SUMMARY — `identity_summary` only, plus `core_beliefs` with `domain = "identity"` |
| Code editing / tactical work | MENTION — a single sentence: "Per Sphere 0001 identity (4 sessions ago): …" |
| Pure factual lookup unrelated to the project | OMIT |

The task class is determined by UCF's existing classifier (currently sketched in `docs/msp/UNIVERSAL-CONTEXT-FRAMEWORK_spec.md` §3). No new classifier is introduced.

**Token budget:** Sphere injection MUST respect UCF's existing token budget. If the Sphere's FULL representation would exceed the remaining budget after subject hydration, the distiller's pre-computed SUMMARY representation is substituted automatically.

**Cores are not auto-injected.** Cores serve two purposes only: as raw material for Sphere distillation, and as a fallback retrieval target when a query explicitly references a time period (see §11.2). Cores do not enter the Consciousness layer proactively.

---

## 11. Retrieval Strategy

### 11.1 Default retrieval (HeptStreamRAG with memory streams)

The existing HeptStreamRAG retrieval pipeline gains three new streams:

1. `sphere_stream` — vector search over `memory/spheres/`. Highest priority for identity-bearing queries.
2. `core_stream` — vector search over `memory/cores/`. Medium priority for "what did we decide last week" style queries.
3. `session_stream` — vector search over `memory/sessions/`. Lowest priority; fallback for fine-grained reconstruction.

These streams sit alongside the existing atom-graph stream and obsidian stream. RAG merge order: atom-graph > sphere > core > obsidian > session. The merge respects each stream's score normalization (atoms use their native retention score; memory streams use cosine similarity normalized to atoms' scale).

### 11.2 Temporal retrieval

A query containing temporal phrases ("two weeks ago", "last cycle", "before the pivot") triggers a temporal-window lookup. MSP resolves the phrase to a date range, then fetches the Cores whose `timestamp_start..timestamp_end` overlaps the range. These Cores are injected at SUMMARY resolution regardless of the default Sphere injection policy.

---

## 12. MLL Integration & Boundaries

The Memory Lifecycle Layer (`PRD--MLL`, `SPEC--MLL`) already defines a 12-stage reverse pipeline for converting raw artifacts into atoms. The 888 distiller fits cleanly into this existing chain:

- **Stages 1-7** (MLL): raw artifact → cleaned → tagged → classified → linked → atomized → registered.
- **Stages 8-12** (this SPEC adds): registered atoms within a Session window → Session artifact → Core distillation → Sphere distillation → UCF re-injection.

**The distiller IS an MLL component.** It runs at the tail of the MLL pipeline, consuming MLL output and producing Cores/Spheres.

### 12.1 Habit Memory boundary (the MLL Skill Creator question)

`PRD--MLL` mentions a Skill Creator that extracts procedural patterns from agent behavior. `MEM_PHILOSOPHY_888.md` §5 mentions "Habit Memory (Procedural)" as a specialized memory class also owned by MSP. These overlap.

**This SPEC defers all procedural-pattern extraction to the MLL Skill Creator.** Cores and Spheres in this SPEC capture **narrative and factual** content. They do not produce reusable agent skills. If a Core's `concept_clusters` reveal a recurring procedural pattern, the Core notes it as a theme; the actual skill extraction is the Skill Creator's job.

A separate atom — `ADR--DISTILLER-VS-SKILL-CREATOR-BOUNDARY` — should be authored alongside this SPEC to make this division explicit. Until that ADR exists, conservative interpretation: distiller is text-narrative-only; Skill Creator is procedural-only.

### 12.2 Consolidator boundary

The M7b consolidator (per `HANDOFF--M7b-CONSOLIDATOR-CONFIG`) merges duplicate or stale atoms within GKS. It is **not** the 888 distiller. They have different inputs (atoms vs Sessions), different outputs (atoms vs memory artifacts producing atoms), and different cadences (continuous vs every 8 Sessions). `ADR--DISTILLER-VS-CONSOLIDATOR-BOUNDARY` should be authored to codify this.

---

## 13. Atom Production Rules

Every distillation produces atoms. The rules:

### 13.1 Per Core

| Trigger | Atom prefix | Cardinality |
| --- | --- | --- |
| Core created | `NARRATIVE--CORE-<ns>-<sphere>-<core>` | 1 per Core |
| Each concept_cluster with `weight ≥ 0.5` | `CONCEPT--<slug>` | N per Core |
| Each pair (concept_cluster, source_session) | `RELATION--CONCEPT-IN-SESSION` | M per Core |

### 13.2 Per Sphere

| Trigger | Atom prefix | Cardinality |
| --- | --- | --- |
| Sphere created | `IDENTITY--PROJECT-DNA-<ns>-<sphere>` | 1 per Sphere |
| Each `core_belief` with `confidence ≥ 0.85` | `BELIEF--<slug>` | N per Sphere |
| Each `behavioral_pattern` (if affect policy ≥ b) | `PATTERN--<slug>` | M per Sphere |
| Belief Revision triggered | `AUDIT--BELIEF-DOWNGRADE-<ULID>` | 1 per revision |

All produced atoms inherit the namespace of the originating Core/Sphere and carry a `produced_by` field pointing back to the artifact file. They participate in the atom graph like any other atom — including bi-temporal versioning.

### 13.3 Atom budget

To prevent atom-registry bloat (a Review #02 concern), the distiller enforces a hard cap of **20 atoms per Core** and **40 atoms per Sphere**. Excess clusters or beliefs are merged or dropped. A `FAILURE--ATOM-BUDGET-EXCEEDED` atom is written when the cap is hit and the distillation is retried with a stricter summarization prompt.

---

## 14. Implementation Phases

### Phase 0 — Foundations (pre-coding)

- [ ] Author `ADR--MEMORY-WRITE-AUTHORITY`, `ADR--DISTILLER-VS-CONSOLIDATOR-BOUNDARY`, `ADR--DISTILLER-VS-SKILL-CREATOR-BOUNDARY`, `ADR--AFFECT-POLICY-DEFAULT`.
- [ ] Add this SPEC and the four ADRs as atoms in `atom_registry.yaml`.
- [ ] Author `BLUEPRINT--DISTILLER-PIPELINE`, `BLUEPRINT--MEMORY-STORAGE-LAYOUT`.
- [ ] Resolve the consolidator-broken claim from `ULTRAPLAN--888-MEMORY-PROTOCOL--REVIEW-01` (verify against latest `main`).

### Phase 1 — Session capture

- [ ] Implement counter management (§6) under `packages/msp/memory/counters/`.
- [ ] Implement `msp.session.open()` / `msp.session.close()` lifecycle.
- [ ] Implement Session artifact writer.
- [ ] Tests: 100 Sessions in a synthetic loop produce 100 well-formed Session files with correct `compression_meta`.

### Phase 2 — Core distillation

- [ ] Implement the Four-Pillars pipeline (§8) end-to-end for Core.
- [ ] Implement pending/failure handling (§8.3).
- [ ] Tests: 8 Sessions produce exactly 1 Core; counters advance correctly; produced atoms validate.

### Phase 3 — Sphere distillation

- [ ] Extend the pipeline to Sphere granularity.
- [ ] Implement `FEAT--SPHERE-CONTEXT-INJECTION` against UCF.
- [ ] Tests: 64 synthetic Sessions produce 1 Sphere with `identity_summary`, `core_beliefs`, ≥ 1 produced `IDENTITY--*` atom.

### Phase 4 — Belief Revision

- [ ] Implement the challenge-counter detection.
- [ ] Implement revision-artifact writing.
- [ ] Tests: a synthetic stream of 8 Sessions whose stance contradicts an established belief produces a revision artifact and a downgrade audit atom.

### Phase 5 — Hardening, telemetry, retrieval

- [ ] HeptStreamRAG integration (§11).
- [ ] Telemetry per §15.
- [ ] Backfill: if existing `cognitive_system` projects have prior Session-like data, write a one-time migration that retroactively creates Sessions and triggers Core/Sphere distillation up to the current counter state.

---

## 15. Telemetry & Observability

The distiller MUST emit the following metrics (Prometheus naming convention; namespace label `ns`):

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

Telemetry is non-optional. A Phase 5 acceptance gate is that the dashboard at `apps/web/observability/memory` renders these metrics live.

---

## 16. Out of Scope (explicit)

The following are deliberately excluded:

- **PhysioCore subsystem.** Biosensor-sourced affect. Requires its own PRD.
- **Somatic imprint.** Body-state memory. Tied to PhysioCore.
- **Cross-namespace Sphere sharing.** Spheres are per-namespace. Cross-project identity transfer is a future feature.
- **Real-time emotion modeling during a turn.** Distillation runs between turns, never during.
- **Procedural skill extraction.** Owned by MLL Skill Creator.
- **Atom mutation by the LLM.** Atoms are produced by the distiller via MSP write authority, never by the agent directly.
- **EVA's qualia / RIM / 9D matrix / Endocrine state.** These belong to EVA's PhysioCore tier. We do not port them.

---

## 17. Open Questions

| # | Question | Default if unresolved |
| --- | --- | --- |
| Q1 | Should the distiller share infrastructure with the M7b consolidator, or remain a distinct process? | Distinct process. (`ADR--DISTILLER-VS-CONSOLIDATOR-BOUNDARY` will codify.) |
| Q2 | When affect policy = `llm_scored`, what's the failure mode if the LLM returns invalid affect? | Strip the affect block; do not fail the whole Core. Log as a warning. |
| Q3 | What is the retention period for `pending/raw/*.jsonl` raw chat logs? | 30 days, then delete on next Core write. |
| Q4 | How are Sphere injections rate-limited if a user starts 50 turns in 10 minutes (same Sphere injected each turn)? | UCF already caches Subject hydration; no new mechanism needed unless profiling shows a hot path. |
| Q5 | When a namespace is first initialized (zero Sessions), what does Sphere injection do? | OMIT entirely. The very first Sphere appears after Session #64. |
| Q6 | Do Cores get embedded into the vector store, or only their summaries? | Both: full text for fallback retrieval, summary for fast match. |

---

## 18. Atom Manifest

Atoms this SPEC introduces (to be added to `atom_registry.yaml` in Phase 0):

```yaml
# Specs
- id: SPEC--MEMORY-888
  level: P2
  cluster: memory

# ADRs (children of this SPEC)
- id: ADR--MEMORY-WRITE-AUTHORITY
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

These atoms collectively form the **memory** cluster — a new cluster proposed for taxonomy v2.4 (or a fit into an existing cluster, pending taxonomy review).

---

## 19. References

**Internal:**

- `docs/plans/ULTRAPLAN--888-MEMORY-PROTOCOL.md`
- `docs/plans/ULTRAPLAN--888-MEMORY-PROTOCOL--REVIEW-01.md`
- `docs/plans/ULTRAPLAN--888-MEMORY-PROTOCOL--REVIEW-02.md`
- `docs/plans/ULTRAPLAN--888-MEMORY-PROTOCOL--REVIEW-03.md`
- `docs/gks/PRD--MLL.md`, `docs/gks/SPEC--MLL.md`
- `docs/gks/PRD--GENESIS-BLOCK-CYCLE.md`
- `docs/msp/UNIVERSAL-CONTEXT-FRAMEWORK_spec.md`
- `docs/gks/adr/ADR--AGENTIC-MONOREPO-PIVOT.md`
- `docs/gks/MSP_RELATIONSHIP.md`
- `atom_registry.yaml` (v1.3.0+)

**External (source inspiration only — not authoritative):**

- EVA v9.6.2 `MEM_PHILOSOPHY_888.md`
- EVA v9.6.2 `MEMORY_COMPRESSION_SPEC.md`
- EVA `consciousness/` reference filesystem (see this conversation's transcript for inspection notes)

---

## 20. Change Log

| Version | Date | Notes |
| --- | --- | --- |
| 0.1 | 2026-05-17 | Initial draft. Authored after three peer reviews of `ULTRAPLAN--888-MEMORY-PROTOCOL` and inspection of EVA's shipped `consciousness/` filesystem. |
