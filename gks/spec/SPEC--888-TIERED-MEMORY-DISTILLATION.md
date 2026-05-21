---
id: SPEC--888-TIERED-MEMORY-DISTILLATION
phase: 2
type: spec
status: draft
tier: process
source_type: axiomatic
vault_id: default
title: 888 Tiered Memory Distillation — cross-session synthesis protocol for
  cognitive_system
aliases: &a1
  - SPEC
cluster: implementation_flow
role: Technical specification
tags: &a2
  - msp
  - memory
  - distillation
  - 888
  - spec
  - narrative
  - identity
  - epistemic
crosslinks: &a3
  references:
    - CONCEPT--MEMORY-SUBSYSTEM
    - CONCEPT--MEMORY-EPISODIC
    - CONCEPT--EPISODE-RETENTION
    - FRAMEWORK--MSP-ARCHITECTURE-V2
    - SPEC--EPISODE-ATOM
created_at: 2026-05-17T18:00:00.000+07:00
attributes:
  id: SPEC--888-TIERED-MEMORY-DISTILLATION
  phase: 2
  type: spec
  status: draft
  tier: process
  source_type: axiomatic
  vault_id: default
  title: 888 Tiered Memory Distillation — cross-session synthesis protocol for
    cognitive_system
  aliases: *a1
  cluster: implementation_flow
  role: Technical specification
  tags: *a2
  crosslinks: *a3
  created_at: 2026-05-17T18:00:00.000+07:00
  domain: spec
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: true
  secret_type: aws_secret
  leak_risk: high
  encryption_level: none
---

# SPEC — 888 Tiered Memory Distillation

> **Version:** 1.0.0-draft
> **Audience:** T3 Architects, T2 Implementers, MSP maintainers
> **Status:** Draft — awaiting lead-dev sign-off on Q1–Q4 (see ULTRAPLAN--888-MEMORY-PROTOCOL)
> **Inspired by:** EVA 8-8-8 Memory Synthesis Protocol (EVA v9.6.2 / "The Human Algorithm")
> **Naming policy:** cognitive_system-native names used throughout; EVA names listed as aliases per ADR--ADOPT-888-AS-INSPIRATION

---

## 0. Purpose

The existing memory subsystem (`CONCEPT--MEMORY-SUBSYSTEM`) solves **within-session recall** — sessions JSONL, episodic summaries, vector/backlinks. It does not solve **cross-session distillation**: the accumulation of repeated experience into durable narrative, and the promotion of confirmed narrative into identity-level beliefs.

This spec defines the **Tiered Memory Distillation Protocol** ("888 protocol"), which adds two new memory tiers (Narrative, Identity) above the existing Episode tier, a distillation pipeline that synthesises N episodes into 1 Narrative and N Narratives into 1 Identity belief, and a confidence-tracking system (epistemic states) that runs independently of document lifecycle status.

---

## 1. Tier Hierarchy

Five tiers in ascending durability and decreasing granularity:

```
┌───────────────────────────────────────────────────────────┐
│  Tier 3 — Identity   (EVA: Sphere Memory)                 │
│  Long-term beliefs. Identity DNA. Survives project resets.│
│  Written by: MSP distiller. Read by: all agents.          │
├───────────────────────────────────────────────────────────┤
│  Tier 2 — Narrative  (EVA: Core Memory)                   │
│  Session-cluster synthesis. Cross-session patterns.        │
│  Written by: MSP distiller. Read by: MSP retrieval.       │
├───────────────────────────────────────────────────────────┤
│  Tier 1 — Episode    (EVA: Session Memory)                 │
│  Single-session summaries, key decisions, scores.          │
│  Written by: consolidator. Read by: compressor, retrieval. │
├───────────────────────────────────────────────────────────┤
│  Tier 0 — Trace      (EVA: Consciousness ≈ raw session)   │
│  Raw turn-by-turn JSONL. Append-only. Non-interpretive.    │
│  Written by: MSP session writer. Never used for reasoning. │
├───────────────────────────────────────────────────────────┤
│  Tier ∞ — Atom       (GKS canonical knowledge base)        │
│  Governance docs, ADRs, Blueprints, CONCEPTs.              │
│  Written by: human/agent via gatekeeper. Authority: final. │
└───────────────────────────────────────────────────────────┘
```

**Authority rule:** on conflict, higher persistence tier wins — **Atom beats Identity beats Narrative beats Episode**. Tier ∞ (Atom) is always the ultimate authority.

### 1.1 Tier ownership summary

| Tier | Path (workspace) | Module that writes | Module that reads |
|---|---|---|---|
| Trace | `.brain/msp/projects/<ns>/sessions/<id>.jsonl` | `memory/sessions/` | consolidator (read-only) |
| Episode | `.brain/msp/projects/<ns>/memory/episodic_memory.json` | `orchestrator/consolidator/` | compressor, retrieval, distiller |
| Narrative | `.brain/msp/projects/<ns>/memory/narrative/` | `orchestrator/distiller/` | retrieval, distiller |
| Identity | `~/.msp/memory/identity/<domain>.json` | `orchestrator/distiller/` | all agents (read), distiller (write) |
| Atom | `gks/<type>/<ID>.md` | MSP gatekeeper | GKS engine, all agents |

> **Storage decision (open — Q4):** Identity tier lives in `~/.msp/memory/identity/` (global) because identity beliefs transcend individual workspace projects. Narrative tier lives in workspace `.brain/` because it is derived from workspace sessions. This aligns with `ADR--GLOBAL-VS-WORKSPACE`.

---

## 2. Memory Domains

Every memory unit (Episode, Narrative, Identity) carries a `domain` field classifying its semantic category. Domain determines **decay rate** (how quickly a memory loses retrieval priority) and **promotion difficulty** (minimum confidence threshold to advance to a higher tier).

| Domain | EVA alias | Description | Default decay | Min confidence for promotion |
|---|---|---|---|---|
| `safety` | safety | Hard constraints, ethical boundaries, do-not-cross rules | none (permanent) | 0.95 |
| `identity-relationship` | identity-relationship | Trust bonds, relational facts about recurring users/agents | slow (180 days) | 0.85 |
| `knowledge-skill` | knowledge-skill | Learned facts, tool-use patterns, domain expertise | medium (90 days) | 0.75 |
| `contextual` | contextual | Project-specific context, ephemeral decisions | fast (30 days) | 0.65 |
| `meta` | meta | Default / uncategorised. System events, internal state. | medium (60 days) | 0.70 |

**Decay** is not deletion — it is a reduction of the `retrieval_weight` multiplier applied during RRF fusion. A decayed memory is still queryable directly by ID.

---

## 3. Epistemic States

Every memory unit carries an `epistemic_state` field that is **independent of `status`** (the GKS document lifecycle). The two fields track different things:

| Field | What it tracks | Monotonic? | Who sets it |
|---|---|---|---|
| `status` | Document lifecycle: `draft → active → stable → superseded` | **Yes** — never regresses | MSP gatekeeper, human |
| `epistemic_state` | Confidence in the memory's truth | **No** — can regress on belief revision | MSP distiller |

### 3.1 Epistemic state values

```
hypothesis   ──► confirmed ──► [stable belief]
                     │
                     ▼ (challenged)
                 contested
                     │
              ┌──────┴──────┐
              ▼             ▼
          confirmed      deprecated
          (recovered)    (discarded)
```

| State | Meaning | Confidence range |
|---|---|---|
| `hypothesis` | Observed once or twice; unverified | 0.40 – 0.69 |
| `confirmed` | Seen across multiple sessions; believed true | 0.70 – 1.00 |
| `contested` | Previously confirmed, now contradicted; under review | any |
| `deprecated` | Disproven or superseded; retained for audit only | any |

**Transition rules:**

- `hypothesis → confirmed`: confidence ≥ 0.70 after at least 2 independent source episodes.
- `confirmed → contested`: a contradiction signal arrives (user correction, conflicting episode, or Identity belief score drops below `contested_threshold`).
- `contested → confirmed`: confidence recovers above 0.70 within `belief_revision.recovery_window_sessions`.
- `contested → deprecated`: confidence cannot recover within the window.
- `deprecated` is a terminal state for a given memory unit; a *replacement* unit may be created as `hypothesis`.

> **Validator rule (Phase A):** `epistemic_state` and `status` are validated independently. A memory unit can be `status: stable` (document is authoritative) and `epistemic_state: contested` (its content is under factual review) simultaneously.

---

## 4. Memory Encoding Level

Inherited from EVA 7.0 Episodic schema. Each Episode carries an `encoding_level` that reflects the significance of the experience at write time. The distiller uses this to prioritise which episodes feed Narrative synthesis.

| Level | Label | Description | Distillation priority |
|---|---|---|---|
| `L0` | trace | Routine turn; low salience | lowest — eligible for GC first |
| `L1` | light | Normal interaction; minor learning | low |
| `L2` | standard | Normal episode; default level | medium |
| `L3` | deep | High-stakes decision; pattern-forming event | high |
| `L4` | critical | Identity-shaping event; safety-relevant; error post-mortem | always included |

**Promotion rule:** when the distiller selects `episodes_per_narrative` episodes for a Narrative synthesis, L3/L4 episodes are always included before L0/L1 episodes are considered, regardless of recency.

---

## 5. The 4 Pillars Distillation Pipeline

Every upward synthesis (Episode → Narrative, Narrative → Identity) runs the same 4-step pipeline:

```
Input episodes/narratives
        │
        ▼
┌─── 1. CLEAN ────────────────────────────────────────────┐
│   Remove duplicate turns, noise entries, L0 traces that  │
│   scored < 0.3 importance. Deduplicate by content hash.  │
└─────────────────────────────────────────────────────────┘
        │
        ▼
┌─── 2. SUMMARY ──────────────────────────────────────────┐
│   LLM-driven: produce a compressed narrative paragraph   │
│   + key_decisions list + unresolved_questions list.      │
│   Target: ≤ 600 tokens for Narrative, ≤ 400 for Identity.│
└─────────────────────────────────────────────────────────┘
        │
        ▼
┌─── 3. INDEX ────────────────────────────────────────────┐
│   Register the new memory unit in the vector store.      │
│   Update backlinks.jsonl with edges to source episodes.  │
│   Add to retrieval RRF pool with tier-weighted score.    │
└─────────────────────────────────────────────────────────┘
        │
        ▼
┌─── 4. RELATION ─────────────────────────────────────────┐
│   Crosslink to GKS atoms (knowledgeId, featId, adrId).   │
│   Link to related Narratives/Identity entries by domain. │
│   Detect conflicts with existing same-domain memories.   │
│   Set epistemic_state based on conflict analysis.        │
└─────────────────────────────────────────────────────────┘
        │
        ▼
Output: Narrative atom | Identity belief update
```

---

## 6. Tier 2 — Narrative

### 6.1 Trigger

The distiller runs Narrative synthesis when:

```
count(episodes since last Narrative) >= config.distillation.episodes_per_narrative   [default: 8]
```

Trigger is checked after every consolidator run (i.e. after every session ends). If the threshold is not yet reached, no synthesis occurs.

### 6.2 Schema

```jsonc
// .brain/msp/projects/<ns>/memory/narrative/NARRATIVE--<ulid>.json
{
  "narrative_id": "NARRATIVE--01J3X4Z...",
  "namespace": "evaAI",
  "created_at": "2026-05-17T18:00:00.000+07:00",
  "domain": "knowledge-skill",             // Memory domain (§2)
  "epistemic_state": "confirmed",          // Epistemic state (§3)
  "confidence": 0.82,                      // 0.0 – 1.0
  "encoding_level": "L2",                 // Peak level of source episodes

  "source_episodes": [
    { "episodic_id": "ep_042", "session_id": "sess_010", "encoding_level": "L3" },
    { "episodic_id": "ep_043", "session_id": "sess_011", "encoding_level": "L2" }
    // ... up to episodes_per_narrative entries
  ],

  "content": {
    "summary": "...",                       // ≤ 600 tokens; produced by Pillar 2
    "key_decisions": ["...", "..."],        // atom IDs or plain text
    "unresolved_questions": ["..."],
    "patterns_observed": ["..."]           // recurring behaviours / facts
  },

  "crosslinks": {
    "atom_refs": ["ADR--XYZ", "FEAT--ABC"],  // GKS atoms this Narrative produced or relates to
    "narrative_refs": ["NARRATIVE--01J3X..."],// related Narratives (same domain)
    "identity_refs": []                     // Identity entries this contributed to
  },

  "retrieval_weight": 1.0,                // Base RRF weight; reduced by domain decay
  "belief_revision": {
    "times_contested": 0,
    "last_contested_at": null,
    "recovery_deadline": null
  }
}
```

### 6.3 Storage

```
.brain/msp/projects/<ns>/memory/narrative/
├── NARRATIVE--01J3X4Z....json
├── NARRATIVE--01J3X5A....json
└── _index.jsonl           ← one line per Narrative: {id, domain, created_at, confidence}
```

`_index.jsonl` is the retrieval fast-path — the distiller updates it after every Narrative write. Full JSON files are loaded only when a Narrative is selected for Tier 3 synthesis or returned as a retrieval hit.

---

## 7. Tier 3 — Identity

### 7.1 Trigger

```
count(Narratives of same domain since last Identity synthesis) >= config.distillation.narratives_per_identity   [default: 8]
```

Identity synthesis runs per-domain independently. The `knowledge-skill` domain may trigger synthesis while `safety` has only 2 Narratives.

### 7.2 Schema

```jsonc
// ~/.msp/memory/identity/<domain>.json
// One file per domain. Written by the distiller; a single object (not a collection).
{
  "domain": "knowledge-skill",             // One of the 5 domains (§2)
  "schema_version": 1,
  "last_synthesised_at": "2026-05-17T18:00:00.000+07:00",
  "epistemic_state": "confirmed",
  "confidence": 0.88,

  "beliefs": [
    {
      "belief_id": "BLF--knowledge-skill--001",
      "statement": "...",                  // ≤ 200 tokens; the distilled belief
      "confidence": 0.90,
      "epistemic_state": "confirmed",
      "source_narratives": ["NARRATIVE--01J3X4Z..."],
      "first_observed_at": "2026-04-01T10:00:00.000+07:00",
      "times_confirmed": 3,
      "times_contested": 0,
      "belief_revision": {
        "under_revision": false,
        "contested_since": null,
        "recovery_deadline": null,
        "downgrade_target": "narrative"    // if revision fails, demote to Narrative
      }
    }
  ],

  "synthesis_history": [
    {
      "synthesised_at": "2026-05-17T18:00:00.000+07:00",
      "source_narrative_count": 8,
      "beliefs_added": 1,
      "beliefs_revised": 0
    }
  ]
}
```

### 7.3 Storage

```
~/.msp/memory/
├── identity/
│   ├── safety.json
│   ├── identity-relationship.json
│   ├── knowledge-skill.json
│   ├── contextual.json
│   └── meta.json
└── identity_audit.jsonl       ← append-only log of every belief state change
```

`identity_audit.jsonl` — every mutation to a belief (creation, confidence change, epistemic state transition, downgrade) appends a row before the mutation is committed. This provides rollback evidence and prevents silent data loss.

---

## 8. Belief Revision Protocol

When a new episode or Narrative contains a `conflicts_with` pointer to an existing Identity belief:

```
1. Mark the belief: epistemic_state = "contested"
                    belief_revision.contested_since = now
                    belief_revision.recovery_deadline = now + recovery_window_sessions

2. Append to identity_audit.jsonl BEFORE mutating the belief file.

3. On subsequent distillation runs within recovery_deadline:
   a. If supporting evidence outweighs contradicting evidence:
      → confidence recovers above contested_threshold (default 0.70)
      → epistemic_state = "confirmed"
      → belief_revision.under_revision = false
   b. If confidence cannot recover by recovery_deadline:
      → Belief is "demoted":
          * Create a Narrative from the demoted belief content
          * Set belief.epistemic_state = "deprecated"
          * belief_revision.downgrade_target records the new Narrative ID

4. Replacement belief may be authored as a new hypothesis from subsequent Narratives.
```

**Manual override:** an operator or agent with appropriate scope can call `msp_belief_override` (MCP tool) to force a belief's epistemic_state. Override is always logged in `identity_audit.jsonl` with the operator ID.

---

## 9. Integration with Existing Modules

### 9.1 Consolidator (within-session, untouched)

The consolidator produces `Episode` objects (scored, tagged, summarised). The distiller **consumes consolidator output** — it reads from `episodic_memory.json` after the consolidator writes it. No changes to consolidator interface. The Episode schema gains two new optional fields in Phase A:

```ts
interface Episode {
  // ... existing fields ...
  domain?: MemoryDomain          // default: 'meta' if absent
  epistemic_state?: EpistemicState  // default: 'hypothesis' if absent
  encoding_level?: EncodingLevel    // default: 'L2' if absent
}
```

These are **optional with defaults** so existing episodes validate without migration.

### 9.2 Compressor (within-episode, untouched)

The compressor receives `CompressorEpisode[]` sorted by importance. No changes. The distiller does not interact with the compressor directly.

### 9.3 Retrieval (extended)

The retrieval orchestrator (`orchestrator/retrieval/`) currently fuses: vector + episodic + obsidian + backlinks (RRF). After Phase B/C, it also fuses:

- **Narrative tier**: RRF weight = `config.retrieval.narrative_weight` (default: 1.4 — higher than Episode because Narratives are pre-distilled)
- **Identity tier**: RRF weight = `config.retrieval.identity_weight` (default: 1.8)

Identity beliefs are always included in context as a preamble block (not subject to RRF cutoff) when the agent is loading its persona.

### 9.4 Distiller module (new — Phase B)

```
packages/msp/src/orchestrator/distiller/
├── index.ts              ← distill(opts: DistillOptions): Promise<DistillResult>
├── pillar-clean.ts       ← Pillar 1: dedup, noise removal
├── pillar-summary.ts     ← Pillar 2: LLM synthesis
├── pillar-index.ts       ← Pillar 3: vector + backlinks registration
├── pillar-relation.ts    ← Pillar 4: crosslink + conflict detection
├── narrative.ts          ← Narrative write + _index.jsonl update
├── identity.ts           ← Identity belief upsert + audit log
├── belief-revision.ts    ← Belief revision state machine
└── types.ts              ← DistillOptions, DistillResult, NarrativeUnit, IdentityBelief
```

---

## 10. Configuration Reference

All thresholds configurable via `packages/msp/config/distillation.defaults.yaml` (Layer 2 internal) and optionally overridden in `config/distillation.yaml` (Layer 1 operator — if promoted).

```yaml
# packages/msp/config/distillation.defaults.yaml
distillation:
  episodes_per_narrative: 8          # range: 2..32
  narratives_per_identity: 8         # range: 2..32

  min_encoding_level_for_narrative: L1   # L0 episodes never trigger Narrative synthesis

  summary_max_tokens:
    narrative: 600
    identity_belief: 200

  belief_revision:
    contested_threshold: 0.70          # confidence below this triggers "contested"
    recovery_window_sessions: 8        # sessions before a contested belief is deprecated
    times_contested_before_auto_deprecate: 3

retrieval:
  narrative_weight: 1.4              # RRF weight for Narrative tier hits
  identity_weight: 1.8               # RRF weight for Identity tier hits

cost:
  max_llm_calls_per_distillation: 4  # guard rail per distill() call
  llm_timeout_ms: 30000
```

---

## 11. Schema Extension — Phase A

The following fields are added to the existing Episode frontmatter in `episodic_memory.json`. All are **optional with defaults** — existing episodes do not require migration to pass validation.

| Field | Type | Default | Added in |
|---|---|---|---|
| `domain` | `MemoryDomain` | `'meta'` | Phase A |
| `epistemic_state` | `EpistemicState` | `'hypothesis'` | Phase A |
| `encoding_level` | `EncodingLevel` | `'L2'` | Phase A |

TypeScript types (added to `packages/msp/src/orchestrator/consolidator/types.ts`):

```ts
export type MemoryDomain =
  | 'safety'
  | 'identity-relationship'
  | 'knowledge-skill'
  | 'contextual'
  | 'meta'

export type EpistemicState =
  | 'hypothesis'
  | 'confirmed'
  | 'contested'
  | 'deprecated'

export type EncodingLevel =
  | 'L0'   // trace
  | 'L1'   // light
  | 'L2'   // standard
  | 'L3'   // deep
  | 'L4'   // critical
```

Validator rule added in Phase A:

```
epistemic_state_not_monotonic:
  severity: warning
  message: "epistemic_state is non-monotonic; confirmed→hypothesis is valid on belief revision"
  # No error — validator only ensures field is one of the 5 values
```

---

## 12. MCP Tool Surface

New tools added in Phase E (after core distiller is stable):

| Tool | Input | Output | Notes |
|---|---|---|---|
| `msp_distill` | `{ namespace, dry_run?, domain? }` | `DistillResult` | Triggers distillation cycle manually. `dry_run=true` reports what would be synthesised without writing. |
| `msp_narrative_list` | `{ namespace, domain?, limit? }` | `NarrativeUnit[]` | Returns Narrative `_index.jsonl` entries. |
| `msp_narrative_get` | `{ narrative_id, namespace }` | `NarrativeUnit` | Full Narrative JSON. |
| `msp_identity_beliefs` | `{ domain? }` | `IdentityBelief[]` | Returns beliefs from `~/.msp/memory/identity/<domain>.json`. |
| `msp_belief_override` | `{ belief_id, epistemic_state, reason, operator_id }` | `{ ok, audit_entry }` | Manual epistemic state override. Writes to `identity_audit.jsonl` before applying. |

Existing tool changes:

| Tool | Change |
|---|---|
| `msp_recall` | RRF fusion now includes Narrative + Identity tiers. Response includes `tier` field on each hit: `'episode' \| 'narrative' \| 'identity' \| 'atom'`. |
| `msp_memory_write` | Accepts optional `domain`, `epistemic_state`, `encoding_level` on Episode writes. |

---

## 13. Storage Layout (complete picture)

```
~/.msp/                                   ← Global (MSP_HOME)
├── identity.json                         ← Agent identity (existing)
├── preferences.json                      ← Agent preferences (existing)
├── projects.yaml                         ← Projects registry (existing)
└── memory/
    └── identity/                         ← Tier 3 (NEW)
        ├── safety.json
        ├── identity-relationship.json
        ├── knowledge-skill.json
        ├── contextual.json
        ├── meta.json
        └── identity_audit.jsonl          ← Append-only belief revision log

.brain/msp/projects/<ns>/                 ← Workspace
├── sessions/<id>.jsonl                   ← Tier 0 Trace (existing)
├── memory/
│   ├── episodic_memory.json              ← Tier 1 Episode (existing)
│   └── narrative/                        ← Tier 2 Narrative (NEW)
│       ├── NARRATIVE--<ulid>.json
│       └── _index.jsonl
└── vector/
    └── backlinks.jsonl                   ← Extended with Narrative edges
```

---

## 14. Acceptance Criteria

- [ ] Phase A: `domain`, `epistemic_state`, `encoding_level` fields optional on Episode; validator accepts missing fields with defaults.
- [ ] Phase A: `npm run msp:validate` green count same as baseline on `main` before Phase A lands.
- [ ] Phase B: `distill({ namespace, dry_run: true })` returns the episodes that would feed the next Narrative without writing anything.
- [ ] Phase B: Narrative synthesis runs for a namespace with ≥ `episodes_per_narrative` unconsolidated episodes; output file appears at correct path.
- [ ] Phase B: LLM call count per distillation is bounded by `max_llm_calls_per_distillation`.
- [ ] Phase C: Identity synthesis triggers correctly when Narrative count per domain reaches `narratives_per_identity`.
- [ ] Phase C: `identity_audit.jsonl` receives an entry before any Identity mutation.
- [ ] Phase D: Belief revision: a belief set to `confirmed` that receives 3 consecutive contradiction signals is downgraded to `deprecated` within `recovery_window_sessions`; a new Narrative is created from its content.
- [ ] Phase D: Manual override via `msp_belief_override` writes audit entry with operator_id before state change.
- [ ] Phase E: `msp_recall` returns hits tagged with `tier` field; Identity beliefs appear in preamble, not ranked results.
- [ ] Config: changing `episodes_per_narrative` in `distillation.defaults.yaml` and restarting the distiller changes the trigger threshold with zero code changes.
- [ ] Storage: Identity tier written to `~/.msp/memory/identity/` (global); Narrative tier written to workspace `.brain/`.

---

## 15. Out of Scope (deferred)

- **Habit/Procedural memory** — response-pattern memory; separate protocol.
- **Per-atom `domain` / `epistemic_state` frontmatter on GKS atoms** — atoms are canonical governance documents, not derived memory. The two systems are parallel, not merged.
- **Multi-user Identity** — one Identity file per domain per MSP_HOME. Multiple users → multiple MSP_HOME values.
- **Encryption at rest for Identity files** — tracked in `SPEC--EPISODE-ATOM §9`; same mechanism applies when implemented.
- **Somatic / physiological state** — requires PhysioCore subsystem (EVA-specific, not in cognitive_system scope).

---

## 16. Connections

- [[CONCEPT--MEMORY-SUBSYSTEM]] — existing 3-layer memory foundation this spec extends
- [[CONCEPT--MEMORY-EPISODIC]] — Episode tier schema and read patterns
- [[CONCEPT--EPISODE-RETENTION]] — GC policy for Tier 0/1; distiller does not GC
- [[FRAMEWORK--MSP-ARCHITECTURE-V2]] — top-level architecture authority
- [[SPEC--EPISODE-ATOM]] — Episode atom contract
- [[SPEC--META-LEARNING-LOOP]] — upstream protocol that may consume Identity beliefs
