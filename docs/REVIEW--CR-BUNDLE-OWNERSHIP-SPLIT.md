---
proposed_id: REVIEW--CR-BUNDLE-OWNERSHIP-SPLIT
type: review
status: complete
tier: strategy
cluster: implementation_flow
role: "Architecture review of the externally-authored CR-BUNDLE--SYSTEM-OWNERSHIP-SPLIT (MSP / GKS / GenesisBlockDB+SQLite ownership split)"
date: 2026-07-04
reviewer: agent (Fable orchestration)
related:
  - adr/ADR--GENESISDB-EMBEDDED-SQLITE-SUBSTRATE
  - adr/ADR--GENESISDB-EVA-KIMPACT-BRIDGE
  - adr/ADR--GENESISDB-KIMPACT-AS-SIGNAL
  - SPEC--GRAPH-RETRIEVAL-LAYER
---

# REVIEW — CR-BUNDLE: System Ownership Split

**Subject:** `CR-BUNDLE--SYSTEM-OWNERSHIP-SPLIT` (externally authored, "GPT"), proposing a 3-layer ownership split — MSP (orchestrator/policy) / GKS (knowledge SSOT) / GenesisBlockDB + SQLite (storage) — bundling ~11 CRs across four owners.

## Verdict: **ACCEPT as governing architecture + 4 conditions. Do NOT reject.**

The document is sound. Its most important property: it arrives **independently** at the same system boundaries GenesisBlock derived on its own (2026-07-03/04 sessions) — engine = generic substrate, cognition/policy = external, "don't put orchestration in the engine." Two independent derivations agreeing is strong evidence the architecture is right, not an accident.

### Why it is sound (accept)
- **"MSP decides / GKS describes / GenesisBlockDB stores"** matches the engine-as-substrate reframing (ADR--GENESISDB-EMBEDDED-SQLITE-SUBSTRATE, the MSP pivot).
- **GenesisBlockDB "Must Not Own"** (agent scheduling, belief-revision policy, ABAC, atom promotion) is the correct boundary — same lesson as ADR--GENESISDB-KIMPACT-AS-SIGNAL (engine-side cognition dies; only the signal/slot survives).
- **§7 Final Boundary Rules** ("Do not put orchestration in GenesisBlockDB", "Do not treat KV cache as memory", "Do not let SQLite become the canonical KB") are all correct.
- **CR-GBDB-1 CoverageReport** ("GRL never returns partial context as complete") is the honest-failure principle we already committed to — and GenesisBlock is *already implementing its first increment* (the `ceiling_hit`/`truncated`/`hops_served` signal on `ContextPackage`, PR #66). Convergent, not conflicting.

---

## Conditions (must hold before the affected CRs proceed)

### C1 — 🔴 SQLite ownership — **RESOLVED by owner ruling (2026-07-04)**
The bundle lists SQLite as an MSP-adjacent Layer-3 store; ADR--GENESISDB-EMBEDDED-SQLITE-SUBSTRATE (merged, PR #65) embeds SQLite *inside* the engine under WAL authority. This looked like a collision. **Owner ruling settles it: "SQLite ของใครของมัน" — each system owns its own SQLite,** justified by the **decoupling test: remove GenesisBlock and MSP must still work.**

Therefore they are **two distinct databases, two owners, never shared:**

| Store | Owner | Scope | Lifetime |
|---|---|---|---|
| **Engine-embedded SQLite** | GenesisBlockDB | props / labels / FTS5 / durable-memory projection, under signed WAL | dies with the engine |
| **MSP-runtime SQLite** | MSP | checkpoints / branches / sessions / handoff & memory-commit **indexes** | independent; survives an engine swap |

**Rule (add to the CR):** MSP's runtime SQLite stores *indexes/pointers to canonical IDs* (the CR's own CR-SQLITE-1 acceptance already says this) and **must not** persist runtime state inside the engine's embedded SQLite — that coupling would fail the decoupling test. This also aligns with the earlier "engine must earn its socket / swappable storage interface" position: MSP can replace GenesisBlockDB precisely because its own runtime state is independent.

### C2 — 🟡 CoverageReport ↔ ceiling signal: converge, don't duplicate
GenesisBlock is landing a minimal `ceiling_hit`/`truncated`/`hops_requested`/`hops_served` signal now (PR #66). CR-GBDB-1 proposes the fuller `CoverageReport` (status enum + counts). **Resolution:** shape the current signal as a `coverage` object (a subset of CR-GBDB-1) so the fuller report is a superset we grow into — no rename later. The full CoverageReport remains its own CR with its own gate (see C4).

### C3 — 🟡 `needs_decomposition` is slightly opinionated for the engine
Per the agreed "engine emits facts, consumer decides" rule, the engine should emit **factual** coverage statuses (`partial_budget`, `partial_frontier`, `budget_exceeded`) and let **MSP derive** "needs decomposition" (CR-MSP-3 consumes the facts). The CR currently lists `needs_decomposition` as a sibling engine status — demote it out of the engine surface.

### C4 — 🟡 This is a roadmap (11 CRs), not one actionable change
Accept the **ownership split + the 5 cross-system contracts** as the governing architecture immediately. But each CR needs its own ADR/design gate before implementation — which the bundle itself models correctly in CR-GBDB-3 (a *design gate*, not a premature build). Do not read "accept" as "implement all 11 now."

---

## Minor notes
- **CR-GBDB-2 Context Fitness Diagnostics** (`target_too_large`, `edge_explosion`, …) is borderline engine/analysis. Acceptable **only** as mechanical thresholds over facts the engine already has (node/edge counts, token estimate, frontier size) — not as "recommendations."
- The bundle is **orchestration plumbing**, not the cognitive moat. It does not design contradiction-detection / belief-revision (the กุ้งเผา/2child core) — correctly, that is MSP/EVA cognition (see ADR--GENESISDB-EVA-KIMPACT-BRIDGE). Read this bundle as the skeleton, not the brain.
- **HQL Context Mode (CR-GBDB-3)** correctly keeps `delegated` out of HQL so HQL does not become an orchestration language — consistent with the HQL-stays-thin invariant across its three ADRs.

## Alignment ledger (bundle ↔ shipped/in-flight GenesisBlock work)
| Bundle item | GenesisBlock status |
|---|---|
| CR-GBDB-1 CoverageReport | first increment in flight (PR #66 ceiling signal) → reshape to `coverage` object (C2) |
| SQLite runtime store (CR-SQLITE-1) | distinct from engine SQLite (C1 ruling); engine SQLite = substrate ADR #65 |
| Frontier cap / SuperNode compression (GBDB owns) | SuperNode + budget already implemented; frontier cap is HQL P1-T2 (planned) |
| MSP belief-revision / memory commits | EVA K_Impact bridge ADR (PR #68) supplies the importance-signal slot |
| "Don't put orchestration/cognition in engine" | matches ADR--GENESISDB-KIMPACT-AS-SIGNAL + the H6 governor = signal (PR #66) |

## Recommendation
Adopt the ownership split and the five contracts as the standing architecture. Apply the four conditions. Sequence per the bundle's own §6 order, gating each CR. The one blocking prerequisite is **C1**, now resolved by the owner's "each owns its own SQLite" ruling — record that rule in the bundle and in the substrate ADR's consequences so no future work re-collides the two SQLite roles.
