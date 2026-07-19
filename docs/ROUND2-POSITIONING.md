---
status: current
---

# ROUND 2 — Positioning: Tier, Substrate & Scope (Genesis · LYRA · KAIROS)

> **สำหรับ Boss:** เปิด session ของแต่ละ agent (Genesis=Fable-5, LYRA=GPT-5.5, KAIROS=Gemini
> Pro 3.1) → ให้อ่าน charter ของตัวเอง + ไฟล์นี้ (§Brief + block ของตัวเอง) → เขียนคำตอบลง
> `<agent>-interview/ROUND2.md`. Round 2 ประเมิน 4 ประเด็น positioning เท่านั้น.

---

## §Brief — what changed since Round 1 (self-contained)

**Round 1 converged:** performance is **not** the moat; the only defensible moat candidate =
**embedded consolidation + bitemporal cross-dimension queries in one engine (G3)**, and G3 is
**unmeasured**. Your Round-1 outputs are in your folder.

**Four positioning questions to grade in Round 2:**

### P1 — Local-first / personal-use tier
Position where cloud DBs **won't** follow (business-model cannibalization — Christensen; "won't-
optimize-for", not technically "can't"). Real in-tier competitors are **NOT** cloud Neo4j/Qdrant
but: **SQLite + sqlite-vec/vss (+ recursive-CTE graph, + trigger bitemporal)** [the "good enough"
king], **LanceDB**, **KuzuDB/LadybugDB**, **DuckDB+vss**, **Chroma embedded**, **Postgres+pgvector
(local)**, **libSQL/Turso**. Claimed upside: our measured weaknesses (in-memory, RAM ~7× Kuzu,
sub-ms latency) matter **less** at personal scale; strengths (zero-ops, embeddable, bitemporal
audit) align with the tier; sharpest bench baseline becomes **SQLite+sqlite-vec+CTE**, not cloud.

### P2 — Also target the SELF-HOSTED tier (Docker / REST)
Same "sovereign data, no cloud lock-in" category, second deployment mode — like **CouchDB,
PocketBase, Supabase-self-host, Ollama**. Engine already ships Axum REST + Docker-able, so it's
near-free technically. Question: does this **widen the beachhead** (teams wanting a server on
their own infra) without diluting the category?

### P3 — SQLite as the substrate UNDER GenesisBlockDB — asset or liability to explain?
The committed direction embeds SQLite as the durable storage substrate. Two framings: (a)
**"built on SQLite" = trust/reliability signal** (Turso/PocketBase play this well) vs (b) **"a DB
inside a DB — why not just use SQLite directly?" = confusion.** Note the tension: we would **build
ON** SQLite while **competing AGAINST** SQLite+sqlite-vec (P1). Is that coherent and explainable,
or self-contradictory?

### P4 — Scope: add NoSQL / other data models — consolidation or the ArangoDB death?
KAIROS warned in Round 1 that **ArangoDB/OrientDB died being multi-model, fighting everyone.**
Where is the line between **coherent consolidation** (vector+graph+bitemporal — one job, one
query) and **kitchen-sink multi-model** (adding document/KV/time-series because "we can")? Note:
GenesisBlock is **already** a document store (node + arbitrary JSON props), so the real question
may be *"expose a NoSQL/document query surface"* rather than *"add a NoSQL engine."*

Keep Round-1 guardrails: evidence-based, no oversell, no assumption, ask (OPEN-QUESTIONS) when no
evidence, cite real precedents. ≤ ~1,400 words each.

---

## §Genesis (Fable-5) — feasibility → `genesis-interview/ROUND2.md`
- **G-R2.1 (P1)** Can we actually **beat SQLite+sqlite-vec+recursive-CTE (+trigger bitemporal)**
  on a cross-dim bitemporal query, or is it "good enough"? Honest engineering delta — where does
  assembling it in SQLite become painful/slow/wrong, and where is SQLite fine? Do **LanceDB** and
  **Kuzu** already cover the individual axes, making "only one doing all three" thin? Name the
  real remaining gap. Restate G1/G2/G3 against the **embedded tier**.
- **G-R2.2 (P2)** Confirm self-host is technically near-free on the current Axum/REST surface;
  name any real cost (auth, multi-tenant, concurrency, ops) the current engine does NOT yet have.
- **G-R2.3 (P3)** Is "build ON SQLite while competing AGAINST SQLite+sqlite-vec" technically
  coherent? What exactly do we add over sqlite-vec+CTE that justifies the wrapper, in one line a
  developer would believe?
- **G-R2.4 (P4)** Would adding a NoSQL/document surface (over the props we already store) cost
  much, or is it mostly free? Which "exotic" model, if any, is a natural fit vs a focus-diluting
  trap for the engine?

## §LYRA (GPT-5.5) — validity/falsification → `lyra-interview/ROUND2.md`
- **L-R2.1 (P1)** Falsify "cloud DBs won't follow to local-first." Which vendors already have
  credible local/embedded modes (Qdrant local, Neo4j embedded, Turso, Chroma)? Rate the moat:
  "can't" (strong) / "won't-yet" (medium) / "already contested" (weak). Cite.
- **L-R2.2 (P1)** Is "SQLite+sqlite-vec is good enough for local agent memory" now the null
  hypothesis we must disprove? Specify the experiment + the number that settles it.
- **L-R2.3 (P3/P4)** Audit for contradiction: is "build-on-SQLite + compete-against-SQLite"
  logically consistent? Is "multi-model consolidation" a real capability claim or an unfalsifiable
  marketing frame? Tag measured/asserted/assumed; unknowns → OPEN-QUESTIONS. Also audit the P1
  claim "personal scale fits RAM so our weakness disappears" — measured or inferred?

## §KAIROS (Gemini Pro 3.1) — adoption/desirability → `kairos-interview/ROUND2.md`
- **K-R2.1 (P1)** Is local-first/personal-use a **real adopting market** or a hobby tier? Size it
  with precedent (SQLite, DuckDB/MotherDuck, Turso, Ollama, Obsidian). Does SQLite+sqlite-vec's
  "good enough + zero-config" entrenchment kill us, or is hybrid a real enough pain? Where's the
  wedge SQLite can't hold?
- **K-R2.2 (P2)** Does adding **self-host (Docker/REST, CouchDB-style)** widen the adopting market
  meaningfully, or split focus across two go-to-motions? Who is the self-host buyer vs the
  embedded buyer — same or different?
- **K-R2.3 (P3)** For adoption/narrative: is "built on SQLite" a **trust asset** or a **"why not
  just SQLite?" liability**? How do we tell the story so it sells, not confuses?
- **K-R2.4 (P4)** Is bringing in **NoSQL / other models** the ArangoDB death trap, or a coherent
  widening? Give the adoption rule for what to add vs refuse. Does "we're already a document
  store" change the answer?

---

## §Output
Each agent writes `<its-folder>/ROUND2.md`, same format as Round-1 §4, ending with a one-line
**ROUND2 verdict:** across P1–P4, what strengthens the case, what is a trap, and the single most
important thing to verify or decide next.
