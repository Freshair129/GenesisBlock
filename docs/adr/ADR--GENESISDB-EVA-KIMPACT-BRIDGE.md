---
proposed_id: ADR--GENESISDB-EVA-KIMPACT-BRIDGE
type: adr
status: proposed
tier: strategy
cluster: implementation_flow
role: "ADR — boundary contract for EVA-computed K_Impact feeding the engine's node.impact slot; producer/consumer separation, no engine-side cognition"
date: 2026-07-04
deciders: Boss
related:
  - adr/ADR--GENESISDB-KIMPACT-AS-SIGNAL
  - adr/ADR--GENESISDB-KIMPACT-ALGORITHM
  - SPEC--GRAPH-RETRIEVAL-LAYER
---

# ADR: EVA K_Impact → GenesisBlock `impact`-slot bridge

**Status:** Proposed · **Date:** 2026-07-04 · **Deciders:** Boss

## 1. Context

Two distinct things are both named "K_Impact", and conflating them is the central risk this ADR exists to prevent:

1. **EVA/GKS K_Impact — the PRODUCER.** A rich cognitive scalar in `[0.0, 1.0]` computed **outside** the engine (`C:\Users\freshair\Downloads\k_impact_engine.py`, EVA 6.0). It composes Meaning, Integration, and Ethical sub-scores: `core = 0.135·RIM + 0.230·RI_global + 0.325·M_core + 0.310·I_core`, plus a risk/conscience step-function bonus `{0.00, 0.05, 0.10, 0.15}`, clamped to `[0,1]` (`k_impact_engine.py:281-294`). Its own header (`k_impact_engine.py:8-11`) states it is *intentionally* separated from the generic "MAS_engine metric hub" so the hub stays free of GKS-specific logic.
2. **GenesisBlock "K-Impact" — the CONSUMER mechanism.** The engine's α-blend + the `impact` slot on nodes/edges. `hybrid_search` ranks by `similarity·(1-alpha) + node.impact·alpha` (`src/lib.rs:3584`), α default `0.0` / opt-in per [[ADR--GENESISDB-KIMPACT-AS-SIGNAL]]. `EdgeInput.impact` exists (`src/lib.rs:150`) and GRL FR3 ranks `Score = 0.5·Semantic + 0.3·K-Impact + 0.2·GraphProximity` (`docs/SPEC--GRAPH-RETRIEVAL-LAYER.md:27`). The engine **stores and ranks by** an importance value; it does **not** compute importance cognition.

The architecture is exactly the EVA pattern: **generic substrate (GenesisBlock ≈ MAS_engine) + external cognition (EVA K_Impact) that feeds a scalar into a slot (`node.impact`), used opt-in.** This ADR specifies the boundary contract so the producer and consumer never merge.

### Data flow (producer → slot → consumer)

```
  EVA cognition (external)              GenesisBlock (substrate)
  ┌────────────────────────┐           ┌───────────────────────────────┐
  │ Meaning / Integration  │           │  node.impact  (storage slot)  │
  │ Ethical + risk bonus   │  scalar   │  = importance_in ∈ [0,1]      │
  │ compute_k_impact() ────┼──────────▶│  set at ingest / supersede    │
  │  k_impact ∈ [0,1]      │  importance_in │  ▲ history preserved      │
  └────────────────────────┘           │  │                            │
       (re-scores over time)           │  └─ opt-in read: hybrid_search │
                                        │     α-blend + GRL FR3 (0.3·K)  │
                                        └───────────────────────────────┘
```

## 2. Decision

**The engine never computes importance; EVA computes it and writes the scalar into the `impact` slot. The engine only stores, versions, and (opt-in) ranks by it.**

### D1 — Boundary contract
- **Producer (EVA, external):** owns the entire Meaning/Integration/Ethical computation, the weights, and the risk bonus. Emits one clamped scalar per episode.
- **Slot (engine):** `node.impact: Option<f64>` accepts an externally-supplied value at **ingest** and on **supersede** (belief revision — a re-scored memory gets an updated impact).
- **Consumer (engine, opt-in):** the existing α-blend read path (`src/lib.rs:3584`) and GRL FR3 rank by the stored value. Unchanged; α stays `0.0` by default per [[ADR--GENESISDB-KIMPACT-AS-SIGNAL]].

### D2 — Naming disambiguation rule (critical)
To stop the two K_Impacts from merging in code and docs:
- The engine-facing write value is documented as **`importance_in`** — "externally-supplied importance signal, `[0,1]`". The on-disk/storage slot name stays `impact` (no format churn), but every doc/param comment refers to the inbound value as `importance_in`.
- **"K-Impact" is reserved to mean ONLY the engine's α-blend/GRL ranking mechanism** (the consumer). It never names the EVA scalar.
- **"EVA K_Impact"** (with the `EVA` qualifier) names the producer scalar.
- **Rule for all future docs:** producer = "EVA K_Impact" → written as `importance_in`; consumer = "K-Impact (blend)". No doc may use bare "K_Impact" to mean both.

### D3 — Where impact enters (audit: what exists vs. what's needed)

| Surface | Today | Needed for the bridge |
|---|---|---|
| `EdgeInput.impact` | **Exists** — `Option<f64>` (`src/lib.rs:150`), written straight through in `add_edge` (`src/lib.rs:2772` `impact: args.impact`). | none |
| `NodeInput.impact` | **Absent** — `NodeInput` (`src/lib.rs:99-110`) has no impact field. `add_node` **hardcodes** `impact: Some(0.7)` (`src/lib.rs:2747`). | **ADD** `pub importance_in: Option<f64>` to `NodeInput`; `add_node` uses it if present, else current default `0.7`. |
| `supersede_node` | Carries impact forward via `old_node.clone()` (`src/lib.rs:2818`); the only mutable param is `new_props` (`src/lib.rs:2796-2799`). A re-scored memory **cannot** update its impact. | **ADD** an optional `importance_in` param so a belief-revision supersede can set the new score (old value preserved in the prior version). |

**Asymmetry flag:** edges can carry an externally-supplied impact at ingest; nodes cannot. Since EVA scores *episodes* (nodes/memories), the node path is the primary one and is exactly the gap. Minimal addition = one field on `NodeInput` + one optional param on `supersede_node`. *This ADR proposes; it does not implement.*

### D4 — Freshness / staleness (bitemporal re-scoring)
EVA re-scores episodes as context accrues — importance **drifts**. The bridge sets `importance_in` at ingest, and updates it via **`supersede_node`**, which is append-mostly and bitemporal: the old version (with its old impact) keeps `valid_to` set and stays in the log (`src/lib.rs:2815-2816`), while the new version carries the fresh score. So a re-scored memory **never loses its prior value** — time-travel/`as_of` queries still see the impact that was live then. This directly answers the agent-memory **staleness open-problem**: importance is a first-class versioned attribute, not a destructively overwritten field.

### D5 — What stays OUT of the engine
- The Meaning/Integration/Ethical sub-computations, their weights (`0.135/0.230/0.325/0.310`), and the risk/conscience step bonus — **all EVA's, none baked into GenesisBlock.**
- **Rationale (the K-Impact-death lesson):** [[ADR--GENESISDB-KIMPACT-AS-SIGNAL]] already demoted the engine's own importance cognition to an opt-in signal because engine-side ranking policy is a footgun in non-governance domains. Engine-side cognition dies; **only the scalar slot survives.** Re-importing EVA's cognition into the core would repeat that mistake.

## 3. Options considered

| Option | Summary | Verdict |
|---|---|---|
| **A. Bridge (chosen)** | EVA computes; engine stores `importance_in` at ingest + supersede; opt-in read. | **Adopt.** Preserves the substrate/cognition boundary; minimal engine surface. |
| B. Port K_Impact into the engine | Reimplement Meaning/Integration/Ethical in Rust. | Reject — re-couples GKS cognition into the core; violates [[ADR--GENESISDB-KIMPACT-AS-SIGNAL]] and the `k_impact_engine.py:8-11` separation. |
| C. Props-only (no typed slot) | Stash EVA score in `node.props` JSON. | Reject — invisible to the α-blend/GRL ranker; loses the typed, versioned, rankable slot. |
| D. Recompute-on-read | Engine calls out to EVA at query time. | Reject — engine must stay local-first and cognition-free; couples read latency to an external service. |

## 4. Consequences

- **Positive:** Substrate stays domain-agnostic and cognition-free; EVA evolves its formula freely without engine releases; importance is bitemporally versioned (staleness solved by construction); no on-disk format change (slot already persisted).
- **Positive:** Symmetry restored — nodes gain the ingest-time impact write edges already have.
- **Negative:** Adds one field + one optional param to the node API (both front-ends — NAPI method + REST route — must be wired; they drift per CLAUDE.md). Callers that never supply `importance_in` are unaffected (default `0.7` retained).
- **Neutral:** No change to the read path — α default stays `0.0`; ranking behavior is opt-in exactly as today.

## 5. Non-goals
- Not implementing engine changes (owner-approval proposal only).
- Not defining EVA's formula, weights, or sub-scores — those live in `k_impact_engine.py`.
- Not changing the α default, GRL FR3 weights, or the edge impact path.
- Not adding an engine→EVA callback or scheduled re-scoring loop (re-scoring is EVA's job; the engine only accepts the next `supersede`).

## 6. Action items (on approval)
1. Add `importance_in: Option<f64>` to `NodeInput` (`src/lib.rs:99-110`); `add_node` uses it, else `0.7`.
2. Add optional `importance_in` param to `supersede_node` (`src/lib.rs:2796`) for belief-revision re-scoring.
3. Wire both into the NAPI async wrapper (`src/lib.rs:5720`, `5734`) **and** the REST routes (`src/router.rs`) — parity per CLAUDE.md.
4. Document the D2 naming rule in the node-API doc comment and cross-link this ADR.
5. Add a bitemporal test: ingest with `importance_in`, supersede with a new value, assert `as_of` still returns the old impact.

---
### Related links
- **Builds on:** [[ADR--GENESISDB-KIMPACT-AS-SIGNAL]] (opt-in signal; α default 0.0)
- **Amended by chain:** [[ADR--GENESISDB-KIMPACT-ALGORITHM]] (the retired default-ranking policy)
- **Ranking consumer:** `docs/SPEC--GRAPH-RETRIEVAL-LAYER.md` FR3
- **EVA producer source:** `C:\Users\freshair\Downloads\k_impact_engine.py` (EVA 6.0)
