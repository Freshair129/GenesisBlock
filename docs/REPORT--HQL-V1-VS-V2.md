---
proposed_id: REPORT--HQL-V1-VS-V2
type: report
status: historical
tier: process
cluster: implementation_flow
role: "Comparison report HQL v1 → v2: what each change fixes, Definition of Done, TDD workflow, benchmark measurement protocol"
date: 2026-07-03
related:
  - SPEC--HQL-V1
  - SPEC--HQL-V2
  - PLAN--HQL-REFINEMENT
  - AUDIT--P31-POST-MARKXIII-REGRESSION
  - AUDIT--HQL-FUZZ
---

# REPORT — HQL v1 vs v2: what changes, DoD, TDD, benchmarks

## บทสรุปผู้บริหาร (Thai Executive Summary)

- **v1 มี "บั๊กจริง" ไม่ใช่แค่ฟีเจอร์ขาด**: `SEARCH`/`MATCH…SIMILAR` คำนวณ fuzzy-resolve target แล้ว **ทิ้ง** (จ่ายค่า trigram + jaro-winkler + neural fallback ฟรี), hybrid ล็อกค่า K=10 ตายตัวในโค้ด, knob ที่แก้ recall@500k (`EF`) กับ `OVERSAMPLE` ตั้งจาก HQL ไม่ได้เลย, `{id:…}` anchor ยังสแกนทุก node, และ `Utc::now()` ถูกเรียก **ต่อ edge** ใน hot loop
- **v2 แบ่ง 3 PR ตามลำดับ**: P0 = แก้บั๊ก + เปิด knob ที่ engine มีอยู่แล้ว (ทุกข้อ ≤1 วัน), P1 = พลัง pattern (variable-length paths `*1..d` ที่ทำให้ HQL เขียน query เดียวกับ bench ของ Kuzu/LadybugDB ได้เป็นครั้งแรก + frontier cap + lazy bindings), P2 = OR/วงเล็บ, `count(*)`, label index
- **Breaking change มีข้อเดียว**: ตัวเลขที่ overflow จะ **error แทนที่จะเงียบๆ กลายเป็น default** (`K 999…` → error, ไม่ใช่ K=5) — จงใจ เพราะ silent wrong-answer อันตรายกว่า error
- **DoD 3 ระดับ**: ระดับ task (test เขียนก่อน + gate ผ่าน + ห้ามแก้ test เก่า), ระดับ PR/phase (suite เขียว + bench ไม่ถดถอย + docs sync), ระดับโปรแกรม (spec v2 = พฤติกรรมจริง 100%, ทุกตัวอย่างใน spec เป็น test ที่รันผ่าน)
- **TDD**: RED = test ของพฤติกรรมใหม่ต้อง fail บน v1 ก่อน; GREEN = ทำให้ผ่าน; REFACTOR ใต้ green; **test เก่าคือ oracle ห้ามแตะ** (กติกาเดียวกับ VQ plan); fuzz corpus ขยายทุกครั้งที่ grammar โต
- **Benchmark**: ต้องเก็บ **baseline v1 ก่อน merge P0** (ไม่งั้นเทียบไม่ได้), ใช้ `hql-query-stress` + `graph-bench` (`--release --features bins`, C: SSD), รัน ≥2 รอบตามบทเรียน P31 §4 (deep-hop แปรปรวนสูง), no-regression envelope ±11% สำหรับ hop-class; ของที่ต้อง "ดีขึ้นแบบวัดได้" มีเป้าชัด (id-anchor O(N)→O(1) ต้องเห็นหลัก 100–1000× บน 100k-node fixture)

---

## 1. What v2 changes and what each change fixes

Class legend: **DEFECT** = v1 does the wrong thing · **EXPOSURE** = engine already can, HQL can't ask · **SCOPE** = declared v1 non-goal now delivered · **PERF** = same answer, cheaper.

| # | Axis | v1 behavior (SPEC--HQL-V1) | v2 behavior (SPEC--HQL-V2) | Class | Task | PR |
|---|------|---------------------------|---------------------------|-------|------|-----|
| 1 | SEARCH/hybrid target | Resolved (incl. expensive `~` fuzzy) then **discarded**; query runs on literal vector only | Vector omitted → search-by-node (stored embedding); unresolvable → named error; vector present → identical to v1 | DEFECT | P0-T1 | PR1 |
| 2 | Hybrid pool | `k` hardcoded 10; no syntax to change | `K <n>` clause, default 10 | DEFECT | P0-T2 | PR1 |
| 3 | Recall knobs | `ef_search`/`oversample` forced `None` | `EF <n>` / `OVERSAMPLE <n>` clauses → engine chain | EXPOSURE | P0-T3 | PR1 |
| 4 | TRAVERSE reach | out-only, single rel | `DIRECTION in\|out\|both`, `REL a\|b` | EXPOSURE | P0-T4 | PR1 |
| 5 | Bad numbers | Overflow silently becomes default (K→5, DEPTH→1, ALPHA→0.5) | Parse error naming the field (**the one breaking change**) | DEFECT | P0-T5 | PR1 |
| 6 | Retraction timestamp | `Utc::now().to_rfc3339()` **per edge** in 2 hot loops | Once per query; single consistent instant | PERF | P0-T6 | PR1 |
| 7 | Id-anchored pattern | O(N) full node scan | O(1) interned-id lookup, result-identical | PERF | P0-T7 | PR1 |
| 8 | Colon ids | `user:5` unquoted unparseable; ADR example broken | Qualified-id in seed/target (gated) or docs corrected | ergonomics | P0-T0/T9 | PR1 |
| 9 | Doc truth | Both HQL ADRs `status: candidate`; CLAUDE.md lists 4 of 5 forms | De-staled; all examples parse | hygiene | P0-T9 | PR1 |
| 10 | Path length | Single fixed hops only; **cannot express the P26/P30 competitor-bench query** | `-[:R*min..max]->` var-length | SCOPE | P1-T1 | PR2 |
| 11 | Fan-out safety | Unbounded intermediate rows on hubs | Frontier cap, actionable error | SCOPE | P1-T2 | PR2 |
| 12 | Binding cost | Every bound var eagerly serialized per row | Lazy: materialize only referenced vars; byte-identical output | PERF | P1-T3 | PR2 |
| 13 | Repeated vars | `(a)…(a)` binds independently (no cycles) | Identity join — same entity required | SCOPE | P1-T4 | PR2 |
| 14 | Edge types | One rel type per edge pattern | `-[:R1\|R2]->` alternation | SCOPE | P1-T5 | PR2 |
| 15 | WHERE logic | AND-only | `OR` + parentheses, AND-precedence, both clause systems | SCOPE | P2-T1 | PR3 |
| 16 | Aggregation | None | `count(*)` (count-only, post-WHERE) | SCOPE | P2-T2 | PR3 |
| 17 | Label anchors | `(:Label)` scans all nodes | `label_idx`-assisted, result-identical | PERF | P2-T3 | PR3 |
| 18 | CONTEXT clauses | None | Design-gated; ship-or-drop with recorded rationale | SCOPE | P2-T4 | PR3 |
| 19 | Text query (path 3) | Impossible without caller vector | `TEXT` reserved; ADR decides (must reconcile Wave 2.5) | SCOPE | P3-T0 | own |

**Back-compat guarantee across all 19 rows:** every pre-existing test file passes **unedited**; the only intentional behavior change for previously-"working" queries is row 5 (garbage numerics error instead of silently mis-executing).

---

## 2. Definition of Done — three levels

### L1 — Task DoD (every task in PLAN--HQL-REFINEMENT)

A task is done when **all** hold:
1. Its RED test(s) existed and failed before the implementation (see §3), and now pass.
2. The plan's acceptance criteria are met verbatim (named test files, named assertions).
3. The Opus 4.8 review-gate checklist for that task passes — no open items.
4. Compiles and tests green under `--no-default-features` (Linux-CI link) **and** default features; grammar tasks also `npm run build:debug` cleanly (NAPI surface intact).
5. **Zero pre-existing tests edited.** If an old test must change, that is a spec question → back to the design gate, not a test edit.
6. Doc deltas that belong to the task (ADR section, error text) are in the same diff.

### L2 — Phase/PR DoD (PR1 = P0, PR2 = P1, PR3 = P2)

1. Full `cargo test --no-default-features` green (all test binaries), `npm test` green after `npm run build:debug`.
2. Fuzz suite green **with the corpus extended** to the phase's new keywords/productions (new mutation-generator entries — not just old inputs).
3. Benchmark protocol of §4 executed: baseline vs post-phase, ≥2 runs, numbers + exact commands in the PR body; no-regression envelope respected; each claimed improvement demonstrated.
4. **Spec-example sweep:** every syntax example in SPEC--HQL-V2 belonging to this phase parses and runs as a test.
5. Doc surfaces synced: index.d.ts `executeHql` docstring, MCP `query_hql` description, CLAUDE.md if command shapes changed.
6. `cargo fmt --check` + clippy clean (the PR #43 lesson); CI green on Linux + Windows.
7. Merged to `main` before the next phase dispatches.

### L3 — Program DoD (v2 declared shipped)

1. P0+P1+P2 merged; P3-T0 ADR accepted (implementation may be a later program).
2. SPEC--HQL-V2 `status: target → current`; SPEC--HQL-V1 archived; zero spec-vs-code deltas (final conformance sweep).
3. `hql-query-stress` pattern rows (incl. the P26/P30-shape named case) recorded in the benchmark suite docs, eligible for the public benchmark page (competitive ADR Wave 0.4).
4. Defect register in SPEC--HQL-V1 §5: every row marked fixed-with-PR or explicitly re-deferred with rationale.
5. Memory/positioning updated: no external claim says "HQL can't filter/traverse-in/express multi-hop patterns" anymore.

---

## 3. TDD workflow (RED → GREEN → REFACTOR, per task)

**Composes the `tdd-workflow` skill; the VQ-plan oracle rule is law: tests that encode old behavior are never edited to make new code pass.**

### 3.1 The cycle per task

1. **RED** — author the failing test *first*, in the task's named file. Two kinds:
   - *New-behavior tests*: assert v2 semantics; MUST fail on pre-task `main` (executor confirms the failure output before implementing — this validates the test).
   - *Back-compat oracle*: identify which existing tests pin the v1 behavior being preserved; run them, record green, **freeze**.
2. **GREEN** — smallest implementation that passes RED + keeps the oracle green.
3. **REFACTOR** — under full green only; gate reviews the final shape, not intermediate states.

### 3.2 Test map (where RED tests live)

| Task | Test file | Representative RED test |
|------|-----------|------------------------|
| P0-T1 | `tests/hql.rs` | `SEARCH known_node K 5` (no vector) returns that node's nearest; `SEARCH ghost K 5` errors naming `ghost` |
| P0-T2 | `tests/hql.rs` | `… ALPHA 0.5 K 50 LIMIT 5` finds a hit constructed to be outside the top-10 pool |
| P0-T3 | `tests/hql.rs` | `EF 512` recovers a neighbor the default ef misses (P32 fixture pattern) |
| P0-T4 | `tests/hql.rs` | `DIRECTION in` returns the reverse-edge neighbor; `REL a\|b` returns the union |
| P0-T5 | `tests/hql_fuzz_tests.rs` | `K 99999999999999999999` → `Err`, not K=5 |
| P0-T6 | existing retraction/cypher suites (oracle-only) + bench | no new semantics — oracle green is the test |
| P0-T7 | `tests/hql_cypher_tests.rs` | id-anchored pattern rows == scan-path rows on a 10k fixture (dual-run equality) |
| P1-T1 | `tests/hql_cypher_tests.rs` | `*1..1` ≡ single hop; diamond + cycle fixtures return the ADR's worked row counts |
| P1-T2 | `tests/hql_cypher_tests.rs` | 200k-fanout hub errors at cap with documented message |
| P1-T3 | `tests/hql_cypher_tests.rs` | wide-pattern output byte-identical pre/post (oracle) |
| P1-T4 | `tests/hql_cypher_tests.rs` | triangle fixture: cycles only, no independent-binding false rows |
| P1-T5 | `tests/hql_cypher_tests.rs` | `-[:SENT\|FORWARDED]->` union |
| P2-T1 | `tests/hql_filter_tests.rs` + property test | random predicate trees vs a reference evaluator (precedence proof); unbalanced parens → error |
| P2-T2 | both clause suites | `count(*)` shape `[{"count":n}]`; `count(*) ORDER BY` → parse error |
| P2-T3 | new `tests/label_index_tests.rs` | mutation-site matrix (add/bulk/supersede/WAL-replay/load/compact → index correct); anchor dual-run equality |

### 3.3 Fuzz as continuous TDD

Every grammar-growing task extends the mutation generators (`hql_fuzz_tests.rs` categories 3/10) with the new keywords **in the same PR** — the no-panic invariant is re-proven against ~5k inputs including the new surface, every phase.

---

## 4. Benchmark: how results are measured

### 4.1 Protocol (fixed, from repo bench conventions + P31 lessons)

- **Build:** `cargo run --release --features bins --bin <harness>` (bins are feature-gated — forgetting `--features bins` exits 101).
- **Disk:** `GB_VBENCH` on **C: (SSD)**, never G: (HDD).
- **Runs:** minimum **2 per configuration**; report p50/p95/p99; single-run deep-hop numbers are variance-prone (P31 §4: Kuzu-class hop6 doubled between runs on identical code — never causally interpret one run).
- **Record:** exact command lines, env vars, commit hash, and both runs' numbers in the PR body — same evidence discipline as the AUDIT-- series.

### 4.2 Baseline capture — **before P0 merges** (mandatory first step)

The v1 baseline must be captured on pre-P0 `main` or ratios are unmeasurable afterwards:

```
$env:GB_VBENCH="C:\Users\freshair\gb_vbench"
cargo run --release --features bins --bin hql-query-stress     # v1 HQL-layer baseline
$env:GB_GRAPH_N="100000"; $env:GB_GRAPH_FANOUT="8"
cargo run --release --features bins --bin graph-bench          # Storage-layer control (P31-comparable)
```

`graph-bench` doubles as the **control**: if HQL-layer numbers move but graph-bench moved equally, the cause is the environment, not the HQL change.

### 4.3 Per-phase measurement matrix

| Phase | Metric | Harness / fixture | Target | Regression envelope |
|-------|--------|-------------------|--------|---------------------|
| P0-T6 | hop3/hop6 p50 (traversal) | `graph-bench` @100k/800k | no regression; any gain = bonus, not a claim | within P31 variance (±11% hop-class) |
| P0-T7 | id-anchored pattern p50 | `hql-query-stress` new row, 100k nodes | **orders-of-magnitude** drop (O(N)→O(1); expect 100–1000× on this fixture) | n/a (must improve; if not, task failed) |
| P0 (all) | every pre-existing stress row | `hql-query-stress` | unchanged | ±10% p50 across 2 runs |
| P1-T1 | var-length `*1..3`, `*1..6` p50/p95 | new stress rows, fanout-8 corpus | recorded (first-ever numbers — no target, establishes baseline) | n/a |
| P1-T1 | P26/P30-shape named case | `MATCH (a {id:…})-[:LINK*1..6]->(b) RETURN b.id LIMIT 1000` | same node set as `neighbors(depth=6)` modulo documented visited policy; latency within same class as `neighbors` + transform overhead | — |
| P1-T3 | wide-pattern latency + process RSS | stress row: 4-var pattern, RETURN 1 var | measurable drop vs eager (rows × unused vars no longer serialized); output byte-identical | correctness absolute |
| P1-T2 | cap behavior | hub fixture | errors at cap; sub-cap queries unaffected (±10%) | — |
| P2-T3 | `(:Label)` anchor p50 | stress row, 100k nodes, selective label | ≥10× vs scan on selective labels; **RSS delta of `label_idx` measured and stated** | non-selective labels no worse than scan +10% |
| P2-T1/T2 | clause overhead | stress rows with OR-trees / count | transform stays <5% of total query time on the standard corpus | — |

### 4.4 Standing rules

1. **No self-invented harnesses for claims** — extend `hql-query-stress`/`graph-bench` so numbers stay comparable release-over-release and feed the independent benchmark suite (`benchmark/`).
2. **A perf claim without its command line + 2-run numbers doesn't merge** (gate item in every perf-touching task).
3. **Honest-conditions rule** (competitive ADR discipline): every published number carries corpus size, fanout, disk, commit, and payload semantics (pattern rows return bound entities; `neighbors` returns node+path — flag when comparing).
4. After P1, the P26/P30-shape case makes an **HQL-vs-Cypher apples-to-apples head-to-head possible for the first time** — a future audit (P3x) can bench `MATCH …*1..6` against LadybugDB's `MATCH (a)-[:LINK*1..6]->(b)` instead of benching `neighbors()` as a proxy.

---

## 5. Risk & rollback summary

| Change | Risk | Rollback |
|---|---|---|
| P0 grammar additions | low (optional clauses) | remove clause; grammar additive |
| P0-T5 strict numbers | behavioral for garbage inputs (intended) | restore `unwrap_or` (one-line each) |
| P1-T1 var-length | **high** — visited-set semantics define row multiplicity | grammar rejects `*` (additive); semantics locked by P1-T0 fixtures before code |
| P1-T3 lazy bindings | medium — refactor of the row engine | revert to eager; oracle tests catch any output drift |
| P2-T3 label index | medium-high — index desync class | anchor falls back to scan; index is derived state, rebuilt on load |
| All | worktree/PR flow; nothing lands on `main` without its phase gates | revert the PR |

## 6. Traceability

Task ↔ spec section ↔ test ↔ bench row are cross-linked: PLAN--HQL-REFINEMENT (tasks + gates) → SPEC--HQL-V2 (normative behavior, §2–§5) → §3.2 test map (RED tests) → §4.3 measurement matrix (evidence). A change that cannot cite all four is out of process.
