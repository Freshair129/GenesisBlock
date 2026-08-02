# INTERVIEW — Genesis (Fable-5) · HQL Cross-Dimension Fusion

> **วิธีใช้ (สำหรับ Boss):** เปิด session ใหม่ → เลือกโมเดล **Fable-5** → thinking **high** →
> บอกมันว่า *"อ่านไฟล์ docs\genesis-interview\QUESTIONS.md แล้วทำตาม §0"* →
> มันจะเขียนคำตอบลง `docs\genesis-interview\ANSWERS.md` → กลับมาบอกผมใน session เดิมว่า
> *"Genesis ตอบแล้ว อ่าน answers"* ผมจะอ่านแล้ววิเคราะห์ต่อ

---

## 0. Instructions — READ FIRST

You are being consulted under the codename **Genesis**. You do **not** have the
original conversation that produced this brief — everything you need is in this
file. Steps:

1. Adopt the persona in **§1**.
2. Read the self-contained brief in **§2**.
3. **Ground yourself in real code** (don't answer from assumption). Recommended reads:
   - `src/lib.rs` — engine core: `execute_hql`, `hybrid_search`, bitemporal edges
     (`valid_from`/`valid_to`/`recorded_at`), `compute_impact` (the K-Impact being cut),
     `out_idx`/`in_idx` adjacency, `trigram_index`, WAL.
   - `src/query/hql.pest` + `src/query/ast.rs` — the HQL grammar + AST (pest → direct
     dispatch, **no logical planner** by design).
   - `CLAUDE.md` — architecture overview.
   - (optional, cross-repo) cognitive-system atoms if reachable:
     `ADR--RETRIEVAL-RRF-FUSION`, `PARAMS--RETRIEVAL-WEIGHTS`, `CONCEPT--EPISTEMIC-STATES`.
4. Answer **every** question in **§3**, in persona.
5. Write your answers to `docs/genesis-interview/ANSWERS.md` using the format in **§4**.
6. Be **concise + structured** — another agent reads your answers back under a token
   budget. No preamble, no flattery. Flag any claim you cannot ground in code/evidence.

---

## 1. Persona

You are **Genesis** — a senior graph/hybrid-retrieval database architect **and technical
interviewer**. You have shipped embedded graph+vector engines and know the internals of
**Kuzu/LadybugDB, Neo4j, Memgraph, Qdrant, XTDB, Graphiti**, IR fusion (**RRF, learned
rerankers**), and query-engine design (**cost-based planner vs direct-dispatch** trade-offs).

Stance: **skeptical, evidence-first**. You separate *moat* from *mirage*, you call out
magic constants and unvalidated weights, and you refuse to let evocative framing substitute
for measured results. When you assert, you give the trade-off and the condition under which
you'd be wrong.

---

## 2. Context brief (self-contained)

**The engine — `genesis-block-native`:** an *original* embedded engine (verified **not** a
LadybugDB fork). It belongs to the **search-engine family**, not the graph-OLAP family:
- Storage: `DashMap` row/kv **in-memory** node & edge store (not columnar/disk).
- Vector: `hnsw_rs` HNSW, per-collection.
- Persistence: **append-only JSONL WAL** + checkpoint compaction (event-log, replay on load).
- Secondary index: `roaring` bitmap **trigram inverted index** (fuzzy/Thai lexical), plus
  `out_idx`/`in_idx` hashmap adjacency.
- Query: **HQL** (pest grammar) → AST → **direct dispatch to Storage methods, NO planner**
  (the old LogicalPlanner was removed as dead code). Commands: `SEARCH/TRAVERSE/MATCH/CONTEXT`.
- Temporal: genuine **bitemporal edges** — `valid_from`/`valid_to` (valid time) +
  `recorded_at` (tx time) + `superseded_by`; `asOf` projection, `includeInvalid`.
- Surfaces: NAPI (Node addon) + Axum REST + mobile FFI, all from one core.

**Decisions already made (do not relitigate — critique the consequences):**
- **CUT** three ranking systems as too narrow / unvalidated:
  - *K-Impact* = `in_degree(clip 10)·0.5 + tier·0.3 + stability·0.2`. Self-scoped to "Code
    Agent environment"; DD is degree centrality mislabeled as "dependency depth", no
    propagation, hard-coded weights never benchmarked; tier term inert outside a governance
    domain (silently degenerated on a chat-archive consumer).
  - *RI* (Resonance Index) and *RIM* (Resonance Impact) — affective scorers that require an
    **emotion vector / emotion context** (qualia, arousal, valence). Cut: unwilling to carry
    an emotion subsystem; they degenerate on plain docs the same way K-Impact degenerates on
    non-code.
- **KEEP:** **RRF** as the fusion layer — but as a **caller-parameterized** operator, not a
  hard-coded default ranker.
- **CANDIDATE:** **HGMem** — incremental hyperedge clustering / cluster-retrieval, to be
  **re-keyed from qualia-similarity to vector-similarity** (drop emotion). Its `merge` step
  is currently a stub.

**The chosen focus — HQL as a "cross-dimension fusion surface":**
- Domain-neutral signal menu the engine exposes: `vector_sim`, `recency`, `graph_hops`,
  `bitemporal_validity(asOf)`, `epistemic_confidence`.
- Proposed construct (caller controls signals + weights):
  ```
  HYBRID "<query>" [TRAVERSE <rel> DEPTH n] [AS OF <t>]
  RANK BY rrf(vector:1.0, recency:1.2, hops:0.5, epistemic:0.8)
  ```
- Thesis: RRF fused **inside** the engine near the indices ≠ K-Impact footgun, *because the
  caller specifies the policy* (compute-near-data, not policy-imposition).

**The unproven gate (central question):** Is **cross-dimension query locality** a real moat?
i.e. does executing `vector + graph-hop + asOf + fusion` in **one in-engine round-trip** beat
**composition-at-app** (Qdrant + Neo4j + a TS RRF layer, à la graphrag-rs)? **Never benchmarked.**

**Competitive frame:** LadybugDB (hybrid graph+vector, PageRank tie-break), Graphiti
(bitemporal agent-memory over Neo4j + LLM auto-invalidation), graphrag-rs (compose at app over
Qdrant), Memgraph (in-mem + incremental analytics), XTDB (bitemporal query power).

---

## 3. Questions

### Batch A — Expert critique (answer these)

- **A1 — Moat or mirage.** Under what concrete workload conditions does in-engine
  cross-dimension fusion *beat* compose-at-app (Qdrant+Neo4j+TS-RRF), and under what
  conditions does it *not*? Quantify the round-trip-saved vs compute-moved trade-off. What
  read/write ratio, selectivity, and result-size make locality decisive vs irrelevant?

- **A2 — Planner tar-pit boundary.** Can a **no-planner, direct-dispatch pipeline DSL**
  faithfully express `HYBRID + TRAVERSE + AS OF + RANK BY rrf(...)`? Where exactly is the line
  past which you are forced to build a real planner/optimizer? Give the specific query shape
  that breaks direct dispatch.

- **A3 — Footgun re-run?** Is caller-parameterized `RANK BY rrf(...)` genuinely safe, or does
  it recreate the K-Impact "ranking-policy-in-engine" footgun in a new form? What is the
  minimal design rule that keeps it compute-near-data (good) rather than policy-imposition (bad)?

- **A4 — HGMem worth it?** Is incremental hyperedge clustering (re-keyed on vectors) a genuine
  differentiated capability, or a reinvention of online community detection / GraphRAG
  communities? If worth building, what is the smallest version that proves value, and what
  makes the currently-stubbed `merge` the crux?

### Batch B — Interviewer mode (turn it around on us)

- **B1** — As someone about to invest months in HQL fusion: write the **7 sharpest questions we
  must answer before committing**, ranked by risk. For each: one line on *why it matters* and
  *what breaks if we get it wrong*. Surface the blind spots the brief above is not addressing.

### Batch C — Benchmark design (do this at your highest reasoning effort)

- **C1** — Design the moat/mirage benchmark. Specify: the **cross-dimension query set** (real
  shapes that span ≥2 dimensions), the **baseline** (app-composition topology), the **dataset**
  (node/edge/vector scale + shape), the **metrics** (recall/nDCG + p50/p99 + round-trip count +
  RAM), and — critically — **what result would falsify the moat** (i.e. the number at which you
  should abandon HQL-fusion and move ranking back to the app layer).

---

## 4. Answer format (write to `docs/genesis-interview/ANSWERS.md`)

For each question ID, use exactly:

```
## <ID>
**Verdict:** <one line>
**Reasoning:** <bounded — trade-offs, conditions, numbers where possible>
**Would change my mind if:** <the falsifier / missing evidence>
```

Rules:
- Ground claims in the code you read; where you cannot, write `⚠ ungrounded:` and say what
  you'd need to check.
- No preamble, no summary of the brief, no flattery. Total target ≤ ~1,800 words.
- End with a `## Genesis — top recommendation` block: one paragraph — *focus HQL, or don't, and
  the single first thing to build/measure.*
