# BENCH SPEC — HQL Moat (G3) + Language Expressiveness

> **สถานะ:** draft · **จุดประสงค์:** the single experiment set that decides GO/NO-GO on
> HQL-redefinition. ทุก target ต้องผ่าน falsifier ที่มีตัวเลข STOP — measure, don't assert.
> **ผูกกับ:** `genesis-interview/GENESIS.md` (G1/G2/G3), `lyra-interview/ASSESSMENT.md`,
> `kairos-interview/ADOPTION.md`, และ audit sweep 2026-07 (สรุปใน §1).

---

## 1. What is ALREADY measured — do NOT re-run (cite the audits)

The audit sweep found G1/G2 are **largely measured**; the honest read is **parity, not
dominance**. Build on these, don't reinvent them.

| Axis | Existing evidence | Honest reading |
|---|---|---|
| **G1 vector** | P20/P21: GenesisDB p50 **974µs** @recall 0.979; Chroma 990µs @0.981; Qdrant 3,301µs | ⚠️ Qdrant number is **server/gRPC (network tax)**. Fair peer = **Chroma embedded → PARITY**. **A "+10% vs Qdrant" claim is not supportable** — reframe G1 to embedded parity. |
| **G2 graph** | P22/P23/P26/P28/P31: hop1 **22µs** (114× Neo4j, 189× Kuzu); hop6 **4,902µs** | ⚠️ Neo4j/Kuzu gaps are mostly **server/JVM/columnar-category tax**. Fair peer = **DuckDB → closes to 1.06× at hop6**. G2 = parity-tier at depth. |
| **RAM** | P31: **686MB** @100k (−35% after edge-interning) | ⚠️ still **7.1× heavier than Kuzu (97MB)** — an encoding choice, not an architectural win. |

**Do NOT re-measure G1/G2 as standalone axes.** They are parity-tier. The entire investment
thesis rests on **G3** and on **HQL-as-a-language**, both of which are **unmeasured**. This spec
targets only those two gaps.

---

## 2. Fair-comparison rules (non-negotiable — from audit red flags)

The project has a **retraction history** (whitepaper retracted a "<30µs" measurement artifact)
and multiple **apples-to-oranges** traps (RocksDB payload-asymmetry, Qdrant server-vs-embedded).
Every number in this bench MUST obey:

1. **Embedded vs embedded**, or same-host with the **network/serialization tax measured
   separately and reported** — never folded into the engine number.
2. **Same materialization** on both sides (full `{node, path}` vs bare ids — the RocksDB trap).
   State exactly what each side returns.
3. **Warm AND cold cache**, reported separately. Discard the first run; report the rest.
4. **p50, p99, and tail**; ≥ **30 runs**; report **variance / 95% CI**. A doubling like Kuzu
   hop6 (60k→114k µs) is variance, not signal — CI must expose it.
5. **Round-trip count and bytes-over-wire** reported for every query (this is the *point* of G3).
6. **Pinned versions + hardware + dataset hash** in the results header. Reproducible harness or
   the number does not count.
7. **Real corpus, not synthetic** (audit flagged synthetic-only vectors). Use the actual
   GKS/GoVibe atom corpus embedded with bge-m3.

---

## 3. G3 — the cross-dimension moat benchmark (the core bet)

### 3.1 Hypothesis (falsifiable)
> A single HQL query spanning **vector + graph-hop + AS OF** executes in-engine with **fewer
> round-trips and lower end-to-end latency** than composing the same result from
> **Qdrant + (Neo4j|Kuzu) + app-layer RRF + temporal filter**.

### 3.2 Baselines (the thing we must beat)
- **B1 — compose-at-app (graphrag-rs model):** Qdrant (vector) + Kuzu/Neo4j (graph) + a
  TS/Python layer doing RRF fusion + `AS OF` filtering. **Count every round-trip**, serialization
  hop, and glue-latency. This is the real competitor.
- **B2 — single-store partial:** whatever one store can do alone (e.g. Qdrant payload-filter +
  brute graph) — to show where composition is *forced*.

### 3.3 Query set — must span ≥2 dimensions (drawn from GoVibe/MSP real jobs)
| ID | Query (natural language) | Dimensions | Source job |
|---|---|---|---|
| **Q1** | vector-search X → 2-hop TRAVERSE `references` → filter `AS OF` last-week → RRF rank | vec+graph+time+fusion | verify-flow |
| **Q2** | hybrid vector+FTS → 1-hop neighbors → current-valid only | vec+lex+graph | recall |
| **Q3** | given atom → 3-hop community + vector-similar, `AS OF` T | graph+vec+time | impact-analysis |
| **Q4** *(control)* | pure vector top-k | vec only | isolates G1 |
| **Q5** *(control)* | pure 1–3 hop traversal | graph only | isolates G2 |

Q4/Q5 are **controls**: they isolate whether any G3 win is genuine **cross-dimension locality**
or merely a single-axis effect that composition also gets.

### 3.4 Metrics
end-to-end **p50/p99** · **round-trip count** · **bytes-over-wire** · **RAM** · and the decisive
one: **Δ (HQL vs B1)** per query, with CI.

### 3.5 Dataset
Real GKS/GoVibe atom corpus: bge-m3 (1024-dim) embeddings + crosslink graph + bitemporal edges.
Scale **100k nodes / ~500k–800k edges** (matches existing audits so numbers compose). Also run
**1M** if 100k is inconclusive.

### 3.6 Falsifier — the STOP number
- **KILL** the moat (→ keep fusion in app, adopt graphrag-rs model, stop HQL-redefinition) **if**:
  on the cross-dim queries (Q1–Q3), HQL saves **< 20% end-to-end p50** **AND** does not cut
  round-trips by **≥ 2×** vs B1.
- **PROCEED** (moat real) **if**: HQL cuts round-trips **≥ 2×** **AND** saves **≥ 30% p50** on
  Q1–Q3, with the advantage **growing** as query spans more dimensions (Q1 > Q2 > controls).
- Anything between = **inconclusive → re-run at 1M + real concurrency before deciding.**

---

## 4. HQL-vs-Cypher expressiveness test (the untested language — audit blind spot)

Audit finding: HQL has **never** been compared to Cypher as a *language* — only fixed LDBC-lite
patterns. "Expressively equivalent on the needed subset" is **asserted, not measured**.

### 4.1 Hypothesis (falsifiable)
> HQL can natively express the graph-query templates GoVibe/MSP actually use, at parity with
> Cypher, **without a planner**.

### 4.2 Method
1. **Pre-register the real templates** (before testing) from GoVibe/MSP: verify-flow,
   impact-analysis, traceability, backlinks, symbol-graph community, reverse-lookup — the exact
   query shapes in the SDDs. Freeze the list.
2. For **each** template, write it in **Cypher** (Neo4j/Kuzu) **and** in **HQL**. Record:
   - **Expressible?** native / workaround / impossible
   - **Correctness:** identical result set to Cypher (ground truth)
   - **LoC + readability** · **latency** (per §2 rules)
3. **Coverage metric:** % of pre-registered templates HQL expresses **natively** (no workaround).

### 4.3 Must-include: the tx-time / interval probe
Audit found HQL `AS OF` is **point valid-time only** — **no transaction-time** (`recorded_at`/
`superseded_at`) and **no interval-overlap** (`BETWEEN`), while GoVibe's ERD assumes full
bitemporal. Include ≥1 template that needs:
- transaction-time query ("what did we *record* as of T, regardless of valid-time")
- interval overlap ("edges valid *during* [t1,t2]")
This will concretely expose the semantic gap (LYRA D1).

### 4.4 Falsifier — the STOP number
- If HQL natively covers **< 80%** of pre-registered templates → "match Cypher" fails as stated.
  Decide: extend HQL (⚠ planner tar-pit risk) **or** speak a Cypher/SQL subset (erase the
  switching cost — see KAIROS D2).
- If the tx-time/interval probes are **impossible** in HQL → bitemporal is **incomplete** for
  GoVibe; scope the gap before claiming bitemporal as a differentiator.

---

## 5. Reuse, don't reinvent
- Extend the existing harnesses in `benches/` and the `[[bin]]` audit harnesses (`ldbc_lite`,
  `snb-ingestion`, `hql-query-stress`) — same fixture scale as P22/P31 so numbers compose.
- Reuse P20/P21 vector rig for the Q4 control; P22 graph rig for Q5.
- Pull the real corpus + crosslink graph from the GKS atom set (bge-m3 already used elsewhere).

---

## 6. Definition of done → GO/NO-GO
A single results table + a one-line verdict per gate:
- **G3 gate:** round-trip Δ + latency Δ vs falsifier §3.6 → moat REAL / MARGINAL / DEAD.
- **Expressiveness gate:** native coverage % vs §4.4 → HQL matches Cypher / needs-extension /
  adopt-subset.
- **Bitemporal gate:** tx-time + interval probes pass/fail → bitemporal complete / incomplete.

If **G3 = DEAD**, the honest call is: keep the engine as a fast embedded hybrid store, do fusion
in the app (graphrag-rs model), and **do not invest in redefining HQL**. That verdict is a
success of this bench, not a failure.
