# INTERVIEW — LYRA (GPT-5.5) · HQL Redefinition — Evidence Audit & Falsification

> **วิธีใช้:** อ่าน `LYRA.md` ก่อน (charter + persona + guardrails). ไฟล์นี้คือชุดคำถาม.
> ตอบลง `docs\lyra-interview\ANSWERS.md`. โหมด: **falsify / audit / ask — no oversell, no
> assumption. ไม่มีหลักฐาน = เขียนเป็นคำถามใน OPEN-QUESTIONS ห้ามเดา.**

---

## 0. Instructions

You are **LYRA** (persona in `LYRA.md`). You do **not** have the originating conversation —
the self-contained brief is `LYRA.md` §4–§5. Ground every answer in the code / cited docs.
Batches A–C mirror the Genesis interview **so the two can be compared head-to-head** — but you
answer them as an **evaluator/falsifier**, not a builder. Batch D is LYRA-only (theory lens).

---

## 1. Questions

### Batch A — Same topics as Genesis, answered in audit/falsification mode

- **A1 — Moat, evidenced?** Is "cross-dimension query locality beats compose-at-app" a claim
  with *any* current evidence, or is it entirely a hypothesis? State what would have to be
  measured to support it, and the specific result that would **falsify** it. Tag the claim
  `measured|derived|asserted|assumed|unknown`.

- **A2 — Planner boundary, factually.** From the actual grammar/AST (`hql.pest`, `ast.rs`),
  what does HQL provably express today? Identify the exact query shapes where direct-dispatch is
  claimed to suffice but, on inspection, may already require planning. Do not assume the
  no-planner claim scales — test it against a concrete shape.

- **A3 — Footgun rule, verifiable?** Is caller-parameterized `RANK BY rrf(...)` *actually*
  free of the K-Impact footgun, or does it merely relocate it? Give a concrete misuse that the
  current design would still permit. What is the minimal *checkable* rule (not a vibe)?

- **A4 — HGMem, is there evidence of value?** Separate the claim "hyperedge cluster-retrieval is
  differentiated" into (a) what is implemented (note the stubbed `merge`), and (b) what is
  merely intended. What evidence exists that it beats plain vector+RRF retrieval? If none, say so.

### Batch B — Interviewer mode (evidence gaps)

- **B1** — List the **7 assumptions this programme is currently making without evidence**,
  ranked by how much damage a wrong assumption does. For each: what is assumed, why there is no
  evidence yet, and the cheapest experiment or question that would resolve it. Phrase unresolved
  items as questions, not as your guesses.

### Batch C — Independent benchmark & target validity (highest rigor)

- **C1** — Independently design the benchmark that settles G1/G2/G3, and **critique the targets
  themselves**: Is "HQL vs Qdrant, +10%" a fair and measurable target or a category error? Is
  "HQL vs Cypher, ≤10% slower" comparing a subset to a full language? Specify dataset, query
  set, baselines, metrics (p50/p99/tail, round-trips, RAM, variance/CI), warm/cold protocol, and
  — decisively — **the number at which each target is declared failed** and HQL-redefinition
  should stop.

### Batch D — LYRA-only: semantics & measurement theory (the lens Genesis may miss)

- **D1 — Bitemporal interval soundness.** Against the GoVibe ERD (`valid_from`/`valid_to`/
  `recorded_at`/`superseded_at`) and the engine code: is the `AS OF` / interval model
  *semantically correct*? Check Allen interval relations, open vs closed bounds, null `valid_to`,
  and whether **tx-time** queries (not just valid-time `asOf`) are expressible. Name every gap.

- **D2 — Expressiveness honesty.** What is the honest expressiveness class of HQL vs Cypher
  (e.g. can it do `WITH`, aggregation, path variables, negation, cycles)? Where "equivalence" is
  claimed, is it equivalence or a convenient subset? State the precise fragment HQL can and
  cannot express.

---

## 2. Answer format (write to `docs/lyra-interview/ANSWERS.md`)

For each question ID:
```
## <ID>
**Verdict:** <one line>
**Evidence:** <cite code/doc; tag each claim measured|derived|asserted|assumed|unknown>
**Falsifier / what would settle it:** <the experiment or number>
**Open questions:** <anything you could not resolve — as questions, not guesses>
```
Rules: no number without provenance; no assumption presented as fact; ≤ ~1,800 words. End with
`## LYRA — verdict`: one paragraph — *is this programme resting on evidence or on hope, and the
single most important thing to measure or ask before proceeding.*
