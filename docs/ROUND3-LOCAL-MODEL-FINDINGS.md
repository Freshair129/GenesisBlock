# ROUND 3 — Local-Model Findings (worker + orchestrator + on-device lens)

> Record of Boss's Round-3 briefing (the checkable source the interview answers quote from).
> Each agent reads §Summary + its own block → writes `<agent>-interview/ROUND3.md`.

## §Summary — what the local models said

**Method (weight accordingly):** local models queried via ollama, ~2-3 focused rounds each,
role-played a persona. These are small-model self-reports = hypotheses, possibly anthropomorphized
from training data — NOT ground truth. Treat as directional signal to pressure-test, not evidence.

**Model 1 — Mellum2-12B as an on-device SLM WORKER (12GB GPU, offline, tight RAM)**
- Wants 3 simple tools: vector-search (episodic, ~500 episodes, ~50ms), fact KV-lookup (~50k),
  keyword-search (notes). Rank: episodic > facts > notes.
- Does NOT want a query language, does NOT want in-engine fusion. Calls the 3 tools separately and
  merges results in its own reasoning ("3 passes still sub-second; one combined query is harder to
  cache, debug, parallelize").
- Bitemporal / consistency = a debug/audit nice-to-have, not hot-path ("at my scale the
  inconsistency window is small enough I don't need a guarantee").

**Model 2 — gemma-4-12b as the ORCHESTRATOR of an SLM fleet under a VRAM budget**
- Budgets → resident workers: 12GB ≈ 3-4, 16GB ≈ 5-6, 24GB ≈ 8+ (2-4B SLMs).
- R1: an evicted worker's scratchpad dies → the DB is the "External Working Memory." Orchestrator
  (unlike worker) needs a Global State Graph: cross-agent dependencies, milestones, continuity.
- R2 (scarcity flips fusion): under a hard VRAM budget the DB should do the cross-dimension
  fusion+ranking (G3) so the tiny worker doesn't burn scarce context. "Unconstrained → worker-fuse
  fine; constrained → DB-fuse superior."
- R3 (scale needs versioning): coordinating swapped agents requires versioned-state /
  temporal-consistency / WAL — latest-value causes lost-updates & races growing 3→8+ agents; named
  WAL + point-in-time snapshot as the requirement + audit trail to debug failed runs.

**Model 3 — gemma-4-e2b as the on-device FLAGSHIP solo agent (phone: iPhone / new Android)** —
dense, standalone, works ALONE, bridges to cloud frontier agents.
- R1 (where state lives): HYBRID split — operational / short-term working state stays in the LOCAL
  on-phone DB (latency, offline, battery, PRIVACY); long-term knowledge / large archives / heavy
  reasoning pushed to CLOUD. Local = immediate tasks + state tracking; cloud = external capacity.
- R2 (sharing + sync): sends the cloud a CURATED SUMMARY, not full state; cloud agent must NOT read
  the local phone DB directly ("I strictly control what leaves the device for privacy"). On
  divergence, the DB must AUTO-MERGE/SYNC conflict-free, prioritizing recent changes (LWW) and
  flagging unresolvable conflicts for manual review — i.e. exactly CRDT/LWW, which the engine
  ALREADY ships (Lamport clocks, ed25519-signed events, LWW, Merkle root).
- Implication: a THIRD tier — on-device flagship — wants local hot store + privacy-curated cloud
  bridge + CRDT sync. GenesisBlock's mobile FFI (MARK XVI) + CRDT sync is a latent ("sleeper") fit
  the panel had not emphasized.

**Derived conclusion (HYPOTHESIS to grade, not a decision):** Scarcity + concurrency is the
tiebreaker. Worker (slack) and orchestrator (constrained, concurrent) disagree on all of {fusion,
graph, bitemporal}; scarcity flips each toward the DB. Three tiers emerge — A: worker (3 simple
tools); B: orchestrator (graph + G3 + versioned-state); C: on-device flagship (local hot + CRDT
sync + privacy). GenesisBlock's differentiators map to B and C, NOT A. ⇒ The buyer is the
ORCHESTRATOR and the ON-DEVICE FLAGSHIP, not the bare worker.

**⚠ Technical tension (Genesis/LYRA):** the orchestrator wants concurrent multi-agent writes with
versioned/snapshot consistency. But the engine/PROTOCOL is single-writer-per-file
(Arc<RwLock<Storage>>; writes serialize; OS file-lock excludes a second writer process). SQLite WAL
gives concurrent readers + one writer with snapshot isolation (MVCC-ish). Does GenesisBlock deliver
the multi-agent snapshot-isolation gemma described, or is this a gap where SQLite+WAL is better?

Keep guardrails: evidence-based, no oversell, no assumption, ask (OPEN-QUESTIONS) when no evidence.
≤ ~1,200 words each.

## §Genesis (Fable-5) — feasibility → `genesis-interview/ROUND3.md`
- **G-R3.1** Does "orchestrator wants DB-side G3 fusion because worker context is scarce" strengthen
  or change your PROPOSAL's G3 bet? Is "fusion offloads the scarce SLM's context" a new, stronger
  rationale than pure latency — and is it measurable?
- **G-R3.2** The single-writer tension: does the current engine (RwLock write-serialization,
  single-writer-per-file, WAL) actually give concurrent multi-agent snapshot isolation and versioned
  reads the orchestrator needs? Or a real gap vs SQLite WAL/MVCC? What would it cost to close?
- **G-R3.3** If the buyer is the orchestrator (not the worker), which technical priorities move up
  vs Path 1 (concurrent access, shared-state-graph reads, snapshot isolation, "what did agent B
  leave" queries) — and which HQL work moves down?
- **G-R3.4** (Model 3) The on-device flagship wants CRDT/LWW auto-sync + privacy-curated cloud
  export. Is the engine's shipped CRDT sync (Lamport/ed25519/LWW/Merkle) actually production-ready
  for phone↔cloud, or a prototype? What's missing for the mobile hot-store + curated-export story?

## §LYRA (GPT-5.5) — validity/falsification → `lyra-interview/ROUND3.md`
- **L-R3.1** How much evidentiary weight should a small model's role-played self-report carry? Is
  "worker doesn't want fusion / orchestrator does" a real signal or anthropomorphized training
  priors? Design the real experiment (not a model opinion) that would confirm/refute the
  scarcity-flips-fusion claim.
- **L-R3.2** Falsify "buyer = orchestrator (+ on-device flagship)." What would make it wrong (e.g.
  orchestrators just use Redis/SQLite for shared state; phones just use SQLite + a sync SDK)?
- **L-R3.3** Audit the single-writer-vs-snapshot tension: is GenesisBlock's bitemporal actually
  multi-agent-concurrent-safe, or does the versioned-state guarantee gemma named require MVCC the
  engine lacks? Tag measured/asserted/unknown; cite code (PROTOCOL §7, RwLock).
- **L-R3.4** (Model 3) Audit the "CRDT sync is a sleeper asset" claim: is the engine's CRDT/LWW sync
  measured/tested and phone↔cloud-ready, or asserted from CLAUDE.md? What evidence exists it works?

## §KAIROS (Gemini Pro 3.1) — adoption → `kairos-interview/ROUND3.md`
- **K-R3.1** Does "buyer = orchestrator, not worker" sharpen or shrink the beachhead? Is "memory
  substrate for local multi-agent orchestration on constrained hardware" a real, reachable, sizable
  market — or a niche of one (GoVibe/Rwang)? Precedent?
- **K-R3.2** The scarcity wedge implies selling to orchestration-framework builders (LangGraph,
  CrewAI, AutoGen, Rwang-likes), not app devs. Better GTM? Who exactly, and their trigger to adopt a
  new memory engine vs bolting Redis+SQLite together?
- **K-R3.3** Does the local-model evidence change your Round-1 verdict (wow = embedded
  consolidation)? Restate the switch-worthy wow now that the buyer is the orchestrator.
- **K-R3.4** (Model 3) The on-device flagship + CRDT-sync opens a distinct MOBILE beachhead
  ("private on-device agent memory that syncs conflict-free with cloud agents"). Is that a bigger or
  better market than the orchestrator one? Precedent (who won on-device+sync — e.g. Apple, Turso,
  PowerSync, WatermelonDB)? Does it split focus or reinforce local-first?

## §Output
Each agent writes `<its-folder>/ROUND3.md`, same format as its QUESTIONS.md §4, ending with a
one-line **ROUND3 verdict:** does the local-model evidence strengthen, weaken, or redirect the case
— and the single most important thing to verify or decide next.
