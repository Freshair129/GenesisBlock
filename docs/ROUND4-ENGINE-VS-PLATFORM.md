---
status: current
---

# ROUND 4 — The Top-Level Fork: Engine-Wedge vs Platform-First

> **สำหรับ Boss:** เปิด session ของแต่ละ agent (โมเดลเดิม) → อ่าน charter ตัวเอง + ไฟล์นี้
> (§Brief + block ตัวเอง) → เขียนลง `<agent>-interview/ROUND4.md`. Round 4 = ตอบ *fork เดียว*
> ที่สูงกว่าทุกรอบก่อน และเปลี่ยนทุกอย่างข้างล่าง (รวมว่า Graphiti driver ยัง make sense ไหม).

---

## §Brief — the fork (self-contained)

Rounds 1–3 analyzed **"the database"** and converged: moat = *the only embedded engine with
enforced row-level bitemporal + vector ANN + graph + signed governance*; buyers = local
orchestrator (Tier B) + on-device mobile (Tier C); recommended a **channel-first / substrate**
play — ship the engine as a backend UNDER Graphiti (issue #1240) / LangGraph, "ride someone
else's growth curve."

**The commissioner pushed back with a higher-altitude question the panel never resolved:** we
**already own a full vertically-integrated stack** — why plug in as a commodity backend under
Graphiti instead of competing AS the layer with our own stack?

| Layer | We already have | Competes with |
|---|---|---|
| orchestration | **Rwang / Rwang V2 / GoVibe / G-Orchestra** | LangGraph, CrewAI, AutoGen |
| memory / identity | **MSP (Memory & Soul Passport)** | Mem0, Letta, Zep |
| knowledge / governance | **GKS** + 12-stage top-down (Block Decomposition) + 7-phase bottom-up (Block Assembly) + H0–H6 context scaling + C-0..3 complexity governance | Graphiti |
| engine | **GenesisBlockDB** | SQLite/Kuzu/LanceDB (this slot is EMPTY) |

**The fork to resolve:**
- **(1) Engine-wedge / infra company:** sell the engine; enter via Graphiti/LangGraph channel to
  borrow distribution; the engine slot is uncontested; risk = commoditization (interchangeable
  with SQLite/Kuzu), you cede the layer narrative.
- **(2) Platform-first / product company:** sell the GKS/MSP/GoVibe stack; compete with
  Graphiti/Mem0/LangGraph at the layer; own the value; differentiated by governance methodology;
  risk = you fight entrenched, funded incumbents on distribution with a huge surface area and a
  tiny team.

**Honest counterweights already on the table (grade these, don't accept them):**
- "We have the layer" ≠ "it is a product strangers adopt." The stack is currently the
  commissioner's **bespoke tooling with zero external users** (cf. Rwang's `engine.mjs` reportedly
  hardcoded to a personal path). Graphiti is `pip install` for anyone.
- The **12-stage / H0–H6 / C-0..3 governance methodology** is the biggest differentiator AND the
  biggest **adoption cost** (a stranger won't learn a 12-stage method to use a memory engine —
  the "HQL-is-a-cost" problem at platform scale).
- The **layer is crowded + funded** (Mem0/Letta/Zep/Graphiti raising and fighting); the **engine
  slot is empty** (nobody in the 2025–26 agent-memory wave ships an engine).
- Possible reconciliation: **not either/or** — engine = wedge (borrow distribution), platform =
  destination ("come for the tool, stay for the platform": SQLite→Turso, DuckDB→MotherDuck).
  Brand = platform vision; on-ramp = engine + channel; Graphiti driver = Trojan horse, not brand.
- Platform precedent that fits: **HashiCorp Vault** — narrow/deep/opinionated for the small pool
  who feel acute pain, not the mass market. GKS/H0-6/governance may be "the Vault of agent memory."

Guardrails: evidence-based, no oversell, no assumption, ask (OPEN-QUESTIONS) when no evidence,
cite real precedents. Each agent MUST end with a one-line recommendation:
**engine-wedge-first / platform-first / a specific hybrid sequence.** ≤ ~1,400 words.

---

## §Genesis (Fable-5) — feasibility → `genesis-interview/ROUND4.md`
- **G-R4.1** What would it actually take to PRODUCTIZE GKS/MSP/GoVibe/Rwang for a stranger
  (install-and-go)? How coupled is the stack to the author's environment today (hardcoded paths,
  assumed services)? Estimate the engineering gap between "my tooling" and "a product," per layer.
- **G-R4.2** Do the engine-wedge (Graphiti/LangGraph driver) and the platform SHARE code, or does
  platform-first waste the engine/HQL work? Is the wedge on the critical path to the platform
  anyway? (If yes, sequencing is free; if no, it's a real either/or.)
- **G-R4.3** If we go platform-first, which layer is the thinnest/cheapest to ship to externals
  first (engine < MSP < GKS < GoVibe/Rwang), and what's the minimum viable "install-and-go" slice?

## §LYRA (GPT-5.5) — validity/falsification → `lyra-interview/ROUND4.md`
- **L-R4.1** Falsify "we already have the layer." Audit: are GKS/MSP/GoVibe/Rwang usable by a
  non-author today? Cite what exists (repos, install paths, external users) vs what is aspirational.
  Tag measured/asserted/unknown. What is the real state (external adopters = 0?).
- **L-R4.2** Is "come for the engine, stay for the platform" a real, repeatable pattern or
  survivorship-bias narrative? What evidence distinguishes engine-first companies that climbed the
  stack from those that stayed commodities? Design the check.
- **L-R4.3** Is opinionated methodology (12-stage/H0-6) a durable moat or an adoption tax? What
  observation would settle which it is? Send unknowns to OPEN-QUESTIONS.

## §KAIROS (Gemini Pro 3.1) — adoption/GTM → `kairos-interview/ROUND4.md`
- **K-R4.1** For a solo/tiny team with ZERO external traction: does engine-wedge (infra, B2B2C
  channel) or platform-first (product, direct) win? Cite precedents BOTH ways — engine-first that
  won (SQLite/DuckDB/Redis), platform-first that won (Vault/Supabase/Retool), and full-stacks that
  died going too broad too early.
- **K-R4.2** Is there real DEMAND for the specific governance methodology (12-stage/H0-6/C-0..3),
  or is it a "solution looking for a problem" / bespoke-to-author? Who is the buyer for "governed,
  auditable agent memory," and is that market reachable or a niche of one?
- **K-R4.3** If we lead platform ("Vault of agent memory"), name the beachhead, the wedge, and a
  plausible first-10-users. If you can't, say the platform-first play has no demonstrated demand
  and recommend the wedge.

---

## §Output
Each agent writes `<its-folder>/ROUND4.md`, same format as its QUESTIONS.md §4, ending with a
one-line **ROUND4 recommendation:** engine-wedge-first, platform-first, or the exact hybrid
sequence — and the single most important thing to verify before committing.
