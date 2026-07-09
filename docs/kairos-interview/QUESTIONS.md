# INTERVIEW — KAIROS · HQL Redefinition — Would Anyone Actually Switch?

> **วิธีใช้:** อ่าน `KAIROS.md` ก่อน (charter + persona + guardrails). ไฟล์นี้คือชุดคำถาม.
> ตอบลง `docs\kairos-interview\ANSWERS.md`. โหมด: **adoption realist — 10× not 10%, quantify the
> switch, cite real precedent, no buzzword soup. ไม่รู้ user = OPEN-QUESTIONS ห้ามมโน.**

---

## 0. Instructions

You are **KAIROS** (persona in `KAIROS.md`). You do **not** have the originating conversation —
the self-contained context is `KAIROS.md` §3–§5. Batches A–C mirror the Genesis/LYRA interviews
**so the three can be compared head-to-head** — but you answer purely from the **desirability /
would-anyone-switch** lens. Batch D is KAIROS-only (category & adoption strategy).

---

## 1. Questions

### Batch A — Same topics, answered through the switching lens

- **A1 — Does the moat move a switch?** Suppose Genesis's G3 is real and cross-dimension queries
  run faster in-engine than compose-at-app. Is *that* a reason a real team rips out their stack,
  or an invisible internal win? What would the user have to *feel* for it to trigger a switch?

- **A2 — HQL as a cost.** HQL is a **new query language** = a learning curve = a switching cost.
  Does inventing a DSL help or hurt adoption vs users already knowing Cypher/SQL? What must HQL
  deliver to be worth learning instead of "just use Cypher"?

- **A3 — Caller-parameterized RRF: does the buyer care?** Is in-engine `RANK BY rrf(...)` a
  visible, demo-able selling point, or plumbing the buyer never sees? Who, concretely, pays for it?

- **A4 — HGMem: wow or footnote?** Is hyperedge cluster-retrieval a *demo-able wow* that sells a
  switch, or an internal optimization no user asks for? Be honest about which.

### Batch B — Interviewer mode (adoption gaps)

- **B1** — Ask us the **7 hardest adoption questions** we must answer before building, ranked by
  how fatal a wrong answer is: who is the first user, what do they use today, why switch, why now,
  what is the 10×, what is the distribution/wedge, and what single fact would prove no one adopts.
  For each: why it matters + what breaks if wrong. Phrase unknowns as questions, not guesses.

### Batch C — The switch bar (highest rigor)

- **C1** — Define the **switch bar** concretely: the *one* capability that is 10× / impossible-
  today, the *beachhead* user (specific: what they run now, their pain, why now), the *quantified*
  switching-cost ledger (learn HQL + migrate + ecosystem loss) vs the payoff, and the **falsifier**
  — the evidence that would show the target user is already happy with `Qdrant + a graph lib +
  glue` and won't move. If that's the likely truth, say it.

### Batch D — KAIROS-only: category & precedent

- **D1 — Compete or create?** Should Genesis Block compete as "faster graph+vector DB" (vs
  Neo4j/Qdrant distribution) or create/own a new category ("embedded agent-memory substrate")?
  Recommend one and back it with **named precedents** — engines that won *without* being fastest
  (e.g. SQLite, DuckDB, MongoDB, Postgres+pgvector) and superior engines that still died. Extract
  the pattern that applies here.

- **D2 — The new-language tax.** DSL adoption precedents: Cypher, PromQL, GraphQL succeeded;
  many query DSLs died. What separated them? What would HQL specifically have to do to earn its
  learning curve — or should it instead *speak an existing language* (Cypher/SQL subset) to
  erase the switching cost entirely?

---

## 2. Answer format (write to `docs/kairos-interview/ANSWERS.md`)

For each question ID:
```
## <ID>
**Verdict:** <one line — does this drive a switch: yes / no / not-yet>
**Reasoning:** <switching-cost vs payoff; cite a real precedent where possible>
**Evidence tag:** <precedent-backed | doc-backed | asserted | unknown>
**Open questions:** <unknown users/pains/numbers — as questions, not guesses>
```
Rules: name real precedents; quantify switching cost; no buzzword soup; ≤ ~1,800 words. End with
`## KAIROS — verdict`: one paragraph — *is there a switch-worthy wow here, who feels it, and the
single most important thing to prove about demand before building.*
