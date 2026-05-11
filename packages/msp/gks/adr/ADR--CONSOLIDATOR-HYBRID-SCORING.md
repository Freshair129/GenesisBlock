---
id: ADR--CONSOLIDATOR-HYBRID-SCORING
phase: 2
type: adr
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: Consolidator scoring — deterministic gate + LLM borderline (hybrid)
tags:
  - msp
  - consolidator
  - importance
  - llm
  - decision
  - m7b
crosslinks: {"references":["CONCEPT--CONSOLIDATOR","CONCEPT--MEMORY-EPISODIC","CONCEPT--SLM-OLLAMA-CLIENT"]}
created_at: 2026-05-04T10:05:30.000Z
---

# ADR — consolidator hybrid scoring

## Context

The consolidator (M7b) decides which session turns become durable episodic memory. Three viable approaches:

1. **Pure deterministic** — rules over text features (length, keyword density, decision-marker patterns, time-since-previous-turn). Fast (~µs), free, but misses subtle "important but short" cases ("we'll use pgvector").
2. **Pure LLM** — call a small model with prompt "is this important enough to remember?" for every chunk. Catches subtle cases, but every session costs N × LLM-call (N = chunk count). Quickly dominates inference cost; also slows session-end UX.
3. **Hybrid** — deterministic rules make the easy verdicts (clear-drop / clear-keep). Only **borderline** chunks are sent to the LLM. Typical mix: ~80% deterministic, ~20% LLM.

User direction (M7-prep follow-up cleanup, 2026-05-04): pick **option 3 — hybrid**.

## Decision

### Scoring pipeline

```
chunk (group of turns)
    │
    ▼
┌────────────────────────────────┐
│ Tier 1 — deterministic gate    │
│  - score = sum(weights × feat) │
│  - if score < LOW_THRESH  → drop│
│  - if score > HIGH_THRESH → keep│
│  - else                  → tier-2│
└──────────┬─────────────────────┘
           │ borderline only
           ▼
┌────────────────────────────────┐
│ Tier 2 — LLM call              │
│  - prompt: "score 0..1 the     │
│    importance of these turns"  │
│  - parse score; if ≥ 0.5 → keep│
│  - else                  → drop│
└────────────────────────────────┘
```

### Tier 1 features (deterministic)

| Feature | Weight | Rationale |
|---|---|---|
| `decision_markers` (matches "we'll", "let's go with", "decided", "rejected") | +0.35 | strongest signal of a kept-decision |
| `code_artifact_mentions` (file paths, function names, ADR/FEAT IDs in body) | +0.20 | concrete > abstract |
| `numeric_specificity` (numbers, dates, version bumps) | +0.15 | lasting facts |
| `length_normalised` (turn-byte-count vs session avg) | +0.10 | longer != more important, but exclusion at the floor |
| `topic_continuity` (low keyword overlap with previous chunk → boundary) | +0.10 | episode-boundary signal |
| `dead_end_markers` (matches "nevermind", "scrap that", "tried doesn't work") | −0.30 | strong drop signal |
| `greeting_filler` (matches "hi", "thanks", "got it") | −0.20 | low-value chatter |

Thresholds: `LOW_THRESH = 0.30`, `HIGH_THRESH = 0.65`. Both tunable via `ConsolidatorOptions.thresholds`. Tracked in a `PARAM--` atom (deferred to follow-up).

### Tier 2 LLM prompt shape

```
You are scoring the importance of an agent conversation chunk for long-term memory.

Conversation chunk:
---
[chunk turns]
---

Return JSON: { "score": <0..1>, "summary": "<1 sentence>", "tags": [<3-5 keywords>] }

Scoring rubric:
  0.0–0.3: clearly forgettable (greeting, dead end, redundant)
  0.3–0.6: ambiguous — slight value if context is needed later
  0.6–1.0: clear keeper (decision, fact, learning, code reference)
```

Re-uses the existing `LlmClient` interface (`src/codegen/slm/factory.ts`) and `createSlmClient()` factory. Provider follows the project's normal SLM precedence (`MSP_LLM_PROVIDER` env: ollama / openai / anthropic / mock).

### Cost bounds

- `maxLlmCallsPerSession` (default 20) — hard cap on tier-2 invocations per consolidation pass. Beyond this, borderline chunks default to `keep` (false-positive recoverable).
- `llmCallTimeoutMs` (default 8000) — single-call timeout. Timeout → default to `keep`.
- Total bound for a session: `≤ 20 × 8s = 160s` worst-case (typical run < 30s).

### Failure modes (default to keep)

| Failure | Tier-2 verdict |
|---|---|
| LLM provider unavailable (no Ollama, no API key) | `keep` |
| Timeout | `keep` |
| Malformed JSON response | `keep` |
| Score parse error | `keep` |

Rationale: false positives (kept noise) are easily prunable later; false negatives (lost insights) are unrecoverable. Tilt toward over-keep when in doubt.

### What about determinism

LLM calls are non-deterministic by default (`temperature > 0`). Two strategies:

- For tests + reproducibility: use `provider: 'mock'` which returns fixed scores per fixture.
- For prod: accept non-determinism. The consolidator is idempotent w.r.t. *deterministic* tier — re-running produces the same tier-1 verdicts. Tier-2 outputs may drift slightly across runs, which is acceptable for a "save best-effort" gate.

## Consequences

**Positive**
- ~80% of chunks decided in microseconds (deterministic). LLM cost bounded by ~20% × maxLlmCalls cap.
- Pluggable per `LlmClient` — same factory used by codegen runner, no new dep.
- Headless / no-Ollama still works (deterministic-only with `keep` defaults for borderline).
- Test suite uses mock provider — fast + reproducible.

**Negative**
- Two code paths to maintain (deterministic features + LLM caller). Mitigated: deterministic features are pure functions, easily unit-tested.
- Threshold tuning needs real-session data. M7b ships with sensible defaults; tuning is M9-era work.
- Tier 1 false-keeps that survive into tier 2 (LLM says `keep` but actually noise) → consolidator gets noisier than ideal. Acceptable trade-off vs the alternative of false-drops.

## Alternatives considered

1. **Pure deterministic.** Rejected — misses "important but short" cases that are exactly the kind of memory we want.
2. **Pure LLM.** Rejected — cost (every session, every chunk) dominates and adds session-end latency.
3. **Importance via embedding similarity to existing episodes** (vector dedup as importance proxy). Considered — interesting but conflates "novel" with "important". Saved for M7c retrieval-side dedup, not write-side scoring.
4. **User-tagged importance** (agent annotates turns with `importance: high`). Rejected as primary mechanism — agents lie / forget. Could supplement: an explicit `importance_hint` from agent acts as a tier-1 feature with `+0.4` weight.

## What this ADR does NOT decide

- **Episode-boundary detection algorithm** — that's a separate concern in `CONCEPT--CONSOLIDATOR`. Default impl: keyword-overlap with previous chunk; below threshold = boundary.
- **Summariser prompt shape** — see `BLUEPRINT--CONSOLIDATOR`.
- **Where the consolidator runs** — caller's choice (session-end hook, MCP `msp_remember` tool, manual CLI).
- **Tier-1 thresholds finalisation** — defaults in this ADR are starting points; tuning belongs in a `PARAM--` atom (M9).

## Source

`msp_spec.md` §7c (passport consolidator), `CONCEPT--CONSOLIDATOR`, user direction (M7-prep follow-up cleanup).
