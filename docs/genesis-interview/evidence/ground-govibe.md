# Grounding Report — GoVibe Requirements for a Redefined HQL

All eight target docs read in full. Current working-tree HQL grammar also read (G:\GenesisBlock_Dev\GenesisBlock\src\query\hql.pest) to ground section 5.

---

## 1. The Interval model (bitemporal fields)

**Source:** `G:\govibe\docs\architecture\ERD-GoVibe-Platform-Data-Model.md` §3 (Core ERD, lines 64–146) and §4 "BI-TEMPORAL VERSION FIELDS" (lines 221–226), §7 Modeling Rules (line 257).

### Exact field names and semantics (§4, verbatim intent)

- `valid_from` / `valid_to` — **business time**: "when a fact is true for the work."
- `recorded_at` / `superseded_at` — **transaction time**: "when GoVibe learned or replaced that fact."
- Additional versioning field: `version` (string) appears alongside temporal fields on ROADMAP_NODE and TASK_ASSIGNMENT.
- Inheritance rule: "Lower-level task records can inherit hub metadata from parent roadmap nodes while keeping their own temporal version history" (§4) — i.e., temporal projection must compose with the hub-and-spoke parent link.

### Which entities carry which fields (from the ERD block, §3)

| Entity | valid_from | valid_to | recorded_at | superseded_at | version |
|---|---|---|---|---|---|
| ROADMAP_SNAPSHOT | yes | — | yes | — | (source_version) |
| ROADMAP_NODE | yes | **yes** | yes | yes | yes |
| TASK_ASSIGNMENT | yes | — | yes | yes | yes |
| HANDOFF_RECORD | yes | — | yes | yes | — |
| VERIFICATION_RECORD | yes | — | yes | yes | — |
| EXECUTION_RUN | yes | — | yes | — | — |
| EXECUTION_ARTIFACT | yes | — | yes | — | — |

Immutable/reference entities (DOCUMENT, ROADMAP_SOURCE, AGENT_PROFILE, USER_ACCOUNT, CONTEXT_PACKET, KNOWLEDGE_NODE, TRACE_LINK, AUDIT_EVENT, DOCUMENT_APPROVAL) carry **no** temporal fields — AUDIT_EVENT has only `event_at`, DOCUMENT_APPROVAL only `decided_at`. Notably only ROADMAP_NODE has `valid_to`; the state-record entities (assignment/handoff/verification) model closure via `superseded_at` — supersession chains, not interval end.

Modeling rule (§7): "Preserve bi-temporal history for mutable roadmap, assignment, handoff, verification, run, and artifact facts."

### Temporal queries the ERD implies

The ERD gives fields, not query syntax, but the field shape implies:

1. **Point-in-time AS OF on both axes** — valid-time AS OF (`valid_from <= t < valid_to`) and tx-time AS OF (`recorded_at <= t` and `superseded_at` null or `> t`). Two independent time axes → a full bitemporal AS OF needs BOTH parameters (current HQL `as_of` is a single timestamp).
2. **Current-version projection** — "latest fact per node_id where superseded_at is null" is the dominant read for TASK_ASSIGNMENT / HANDOFF_RECORD / VERIFICATION_RECORD (state records with version history).
3. **Superseded-chain walk** — `superseded_at` + `version` on the same logical key implies "give me the version history of assignment X" (walk the supersession chain, ordered by recorded_at). No explicit BETWEEN/overlap operator is demanded anywhere in the doc, but audit traceability (§5 step 7, TRACE_LINK "why does this task exist?") implies range scans over `recorded_at` for audit reconstruction.
4. **Interval semantics on ROADMAP_NODE** — with real `valid_from`/`valid_to`, "which roadmap nodes were active during sprint window [a,b)" (interval overlap) is the natural query, though the ERD only implies it, never states it.

**Bottom line for HQL:** point AS OF on two axes + supersede-chain enumeration are demanded by the model; BETWEEN/overlap is implied-but-unstated. The engine's existing bitemporal edge model (valid_from/valid_to + recorded_at + superseded_by) matches this ERD almost 1:1 — the gap is query-surface, not storage.

---

## 2. H0–H6 tiers and W-scale — exact numbers

**Source:** `G:\govibe\docs\STD-Execution-Governance.md` §3 (H-Scale Mapping), §4 (W-Scale), §2 (Complexity); hop meanings also in `G:\govibe\docs\architecture\SDD-Genesis-Block.md` §3 (Context Scaling flowchart).

### H tiers (STD §3 table + SDD-Genesis-Block §3 hop counts)

| Tier | Scope (STD §3) | Hop meaning (SDD-Genesis-Block §3) |
|---|---|---|
| H0 | Subtask / PR — local change, no broad context | 0 hops: direct file only |
| H1 | Task / Component — component assembly, local imports/exports | 1 hop: self + I/O neighbors |
| H2 | Story / Feature — feature folder, nearby types, data contracts | 2 hops: feature folder |
| H3 | Epic / Module — module integration, API/event contracts | 3 hops: module scope |
| H4 | Phase / Architecture — system architecture, governance, security | 4 hops: system architecture |
| H5 | Masterplan / Roadmap — platform vision, enterprise-wide context | 5 hops: full GKS knowledge |
| H6 | Full Network / Enterprise Ceiling — "rare full-network traversal, systemic coupling analysis, final escalation ceiling" (STD §3) | unbounded/full-network (STD only; SDD-Genesis-Block's flowchart stops at H5, its §4 names H6 as ceiling) |

Complexity→hop mapping (STD §3 yaml): `C-0: H0, C-1: H1, C-2: H2, C-3: H3-H6`. Rule: "Use H6 only as the hard ceiling for full-network traversal."

**Token/size budgets:** NO numeric token budgets per tier appear in any of these docs. CONTEXT_PACKET carries a `token_budget` field (ERD §3, line 168) and STD §4 warns about "context packets that risk token explosion," but the budget value is caller-supplied, not standardized. (HQL's existing `CONTEXT ... BUDGET <n>` matches: budget is a caller parameter.)

### W-scale fan-out (STD §4, exact numbers)

| W | Meaning | Rule |
|---|---|---|
| W2 | Optimal | **3–5** sibling/peer connections; normal operation |
| W3 | Warning | **6–8** connections; lead review required |
| W4 | Super-hub danger | **9+** connections; block deployment until refactored |

W applies to: graph node degree, roadmap branching width, decomposition breadth, context-packet token explosion. Key design point: **"H governs hop depth; W governs fan-out or branching width" — controlled separately** (STD §4 opening line). A conforming traversal operator therefore needs a per-hop fan-out cap, not just a depth cap.

---

## 3. The 4-layer hybrid retrieval

**Source:** `G:\govibe\docs\CONCEPT--HYBRID-RETRIEVAL-FTS-LAYER.md` (Problem/Hypothesis/Scope sections), which quotes `FRAMEWORK_MASTER_SPEC.md` §13.

### Layer order (Problem section, verbatim)

1. **Atomic** — `gks_lookup` (exact id, **O(1)**)
2. **FTS** — keyword grep across `gks/<type>/*.md` (this doc's addition; case-insensitive substring + token-overlap score, pure Node, no inverted index because "N is small — hundreds of atoms; O(N) scan is fine")
3. **Vector** — `gks_recall` (semantic via embeddings)
4. **Graph** — `gks_backlinks` (relationship traversal)

### Fusion point and params

- **RRF sits on top of all layers**: "the existing MSP `orchestrator/retrieval/` implements (1)(3)(4)... **with RRF reranking on top**" (Problem section). RRF fuses in `createCognitiveLayer.recall` (Scope section).
- **Cheap-cascade short-circuit** (§13.2 referenced): exact-id atomic match short-circuits — FTS only fires when atomic missed. So the pipeline is not pure parallel fan-out; it is a *cascade with fusion of survivors*.
- **FTS score contract**: `score = matches / tokens.length`, score ∈ (0,1], must be "a comparable scalar" for RRF blending. `limit` honoured; empty query → `[]`; frontmatter stripped from snippet.
- **No RRF k-constant or per-layer weights are given** in this doc — weights are left to the fusing layer. This is consistent with the guardrail: RRF must be caller-parameterized.
- Thai tokenization explicitly deferred (Scope/Out) — relevant since the engine already has Thai fuzzy tests.

---

## 4. Concrete query shapes the stack needs (the G2/G3 benchmark set)

**Primary source:** `G:\govibe\docs\architecture\SDD-GoVibe-MSP-GKS-Integration.md` §3 contracts table; shapes elaborated in CONCEPT--HYBRID-JIT-CONTEXT and SDD-Symbol-Graph-Traceability-Boundary.

### 4.1 `query_genesis_graph(target, hops)` — the flagship shape

- **Contract row:** SDD-Integration §3 — "JIT hop-limited render (to be added as the 11th GoVibe MCP tool)", direction GoVibe → MSP/Compute.
- **Concrete example:** CONCEPT--HYBRID-JIT-CONTEXT §3: `query_genesis_graph(target="FEAT--TAX-DEDUCT", hops=2)` — engine "pulls only the nodes within 2 hops" and the JIT renderer returns a **Virtual Document** (temp file or text string), which the agent uses without ever seeing the full file tree (§2 Layer 2 + Operational Workflow Addendum "Virtual Rendering Contract").
- **Inputs:** target atom id, hop count (H0–H6 per STD). **Implied bounds:** depth = H tier, fan-out should respect W-scale (STD §4), output bounded by a token budget (ERD CONTEXT_PACKET.token_budget).
- **Output:** rendered bounded context (Virtual Document / text snapshot), not a bare node list. §4 adds a second dimension: render = **scope (hops) × format (template)** — format-adaptive output, which lives above the DB but means the DB must return structured nodes+bodies, not pre-flattened text.
- **Write-side note (Addendum):** bulk ingest must use `bulk_add_nodes()` / `bulk_add_edges()`.
- **Benchmark shape (G2):** k-hop bounded expansion from a seed with per-hop fan-out cap + payload retrieval, depths 1–5, degree-limited.

### 4.2 `gks_lookup(id)` — layer 1

Exact-id O(1) point read (CONCEPT--FTS Problem section). Benchmark: point-lookup latency floor.

### 4.3 `gks_recall(query)` — layer 3 + fused pipeline

Semantic vector search over atoms, fused by RRF with FTS/atomic/graph hits in `createCognitiveLayer.recall` (CONCEPT--FTS). Inputs: text query, limit. Output: `RetrievalHit[]` with scalar score and `metadata.matchedBy` provenance tag per layer. **Benchmark shape (G3):** one call = atomic short-circuit + FTS + vector + backlinks + RRF — this is exactly the multi-round-trip app-composition the moat query must beat.

### 4.4 `gks_backlinks(id)` — layer 4

Reverse-edge lookup ("relationship traversal"). SDD-Symbol-Graph §4 lists the concrete uses: "backlinks and reverse lookups", "symbol-to-doc traceability". Benchmark: reverse-adjacency (in-edges) of a node, with provenance per edge (§7: "runtime must preserve provenance for every node and edge").

### 4.5 Drift / community / traceability lookups (SDD-Symbol-Graph §4, §6)

- **doc-code drift detection** — compare doc-declared links vs code-derived symbol links; evidence packet fields (§6): `symbol_count, edge_count, community_count, doc_symbol_links, broken_links, unmapped_doc_sections, drift_score`. Query shape: set-difference over two edge classes + counts (aggregation).
- **community clustering for module boundaries** — `community_count` implies a clustering/community-detection read (heaviest shape; likely offline, but the counts must be queryable).
- **broken-link enumeration** — edges whose target node is absent (anti-join).
- **impact analysis** — forward closure from a changed symbol (bounded traversal again).

### 4.6 Structural invariants (SDD-Genesis-Block §5, BLUEPRINT §5)

- **Acyclic Invariant Enforcement:** "validate the graph every time before an agent starts work" — cycle-detection query over the containment tree.
- **Deterministic Backlink Injection** — one-directional parent backlinks; BLUEPRINT §5 AUDIT checklist: Acyclic Check, Compaction Check (max depth L7), `block_id` must point back to GKS_CORE (reachability-to-root check).
- **Metadata hub inheritance** (BLUEPRINT Core Principles; ERD §4): resolving an atom's full metadata = walk spoke→hub via `block_id` and merge — a fixed 1-hop (or chain) parent walk during every context render.

### 4.7 Governance passthrough

SDD-Integration §3 last row: "governance tiers MASTER/SPEC/GOV/ADR/USER enforced in GenesisBlockDB engine — defense-in-depth gate." §6: dual surface NAPI fast-path + MCP is required for latency ("MCP round-trip latency vs in-process → dual surface — GenesisBlockDB already ships both").

---

## 5. What these docs demand that current working-tree HQL cannot express

Grounded against the working tree `src/query/hql.pest` (which already has: `AS OF <string>` on SEARCH/TRAVERSE/MATCH/pattern-MATCH; WHERE/ORDER BY/LIMIT/RETURN clauses; Cypher-style linear `MATCH (a)-[:REL]->(b)` patterns; `CONTEXT FOR <target> TIER H0..H5 BUDGET n`; TRAVERSE DIRECTION in/out/both; EF/OVERSAMPLE; multi-rel `A|B`).

1. **Two-axis bitemporal AS OF.** ERD §4 defines independent valid-time and tx-time. HQL has a single `as_of = { ^"AS" ~ ^"OF" ~ string_lit }` (hql.pest:25) — one timestamp, one axis. No `AS OF VALID <t> SYSTEM <t2>`, no BETWEEN, no interval-overlap predicate, no way to say `includeInvalid` from HQL text.
2. **Supersede-chain walk.** No HQL construct enumerates a node's version history (`superseded_by`/`caused_by` chain). Engine stores it; HQL cannot ask for it.
3. **Per-hop fan-out cap (W-scale).** STD §4: depth and breadth "must be controlled separately." TRAVERSE has `DEPTH n` (hql.pest:52) but no breadth/degree limit — no `WIDTH 5` / `FANOUT 8` clause, and pattern-MATCH hops have neither. W2/W3/W4 enforcement is impossible to request.
4. **H6 tier.** `tier = { "H0" | ... | "H5" }` (hql.pest:87) — H6 (STD §3's escalation ceiling) is not in the grammar.
5. **Variable-length path patterns.** `MATCH (a)-[:REL*1..3]->(b)` does not exist; `hop*` chains are fixed-length only (hql.pest:72–73). `query_genesis_graph(target, hops=N)` maps to TRAVERSE, but the Cypher-pattern surface can't express bounded var-length — already flagged as P1 in docs/PLAN--HQL-REFINEMENT.md.
6. **Caller-parameterized fusion (RANK BY rrf(...)).** The 4-layer pipeline fuses atomic+FTS+vector+graph via RRF (CONCEPT--FTS). HQL's only fusion is `MATCH ... ALPHA <a>` (a single vector/text blend scalar, hql.pest:53). There is no RRF operator, no named-signal weights (vector/recency/hops/epistemic), and no way to fuse a TRAVERSE result set with a SEARCH result set in one statement — exactly the G3 moat query. Today that composition requires N round-trips + app-side RRF.
7. **Composed cross-dimension query.** Nothing chains operators: `HYBRID "<q>" TRAVERSE <rel> DEPTH n AS OF <t> RANK BY ...` is unparseable — the grammar is `query = { SOI ~ (search | traverse | match_pattern | hybrid | context) ~ EOI }` (hql.pest:93), five mutually exclusive commands with no pipeline.
8. **Reverse-lookup as a first-class cheap op (gks_backlinks).** Expressible today only as `TRAVERSE FROM x DEPTH 1 REL <r> DIRECTION in` — but that requires naming the relation; "all in-edges regardless of type" has no wildcard REL. (`rel` requires an identifier, hql.pest:8–11.)
9. **Aggregations for drift/audit.** `symbol_count`/`edge_count`/`broken_links`/anti-joins (SDD-Symbol-Graph §6): HQL has no COUNT, no set-difference, no "edges whose target is missing" predicate. These would otherwise be app-side scans.
10. **Cycle/reachability checks.** Acyclic Invariant Enforcement (SDD-Genesis-Block §5) and "block_id points back to root" (BLUEPRINT §5) need a reachability/cycle primitive; not expressible.
11. **Virtual Document rendering with token budget on arbitrary traversals.** `CONTEXT FOR <target> TIER Hn BUDGET n` exists, but only from a single target with the engine's fixed tiering — you cannot attach a BUDGET to a TRAVERSE or a pattern MATCH, and the tier semantics in HQL are engine-defined rather than the STD H-map. (Bodies + structure output for format-adaptive rendering is above the DB, but budget-bounded structured output is the DB's half of the contract.)

Items 1, 3, 5, 6, 7 are the load-bearing gaps for the G2/G3 targets; 8–11 are cheap wins or explicit non-goals to renegotiate.

---

## Doc paths (all read in full)

- G:\govibe\docs\architecture\ERD-GoVibe-Platform-Data-Model.md
- G:\govibe\docs\CONCEPT--HYBRID-JIT-CONTEXT.md
- G:\govibe\docs\STD-Execution-Governance.md
- G:\govibe\docs\CONCEPT--HYBRID-RETRIEVAL-FTS-LAYER.md
- G:\govibe\docs\architecture\SDD-GoVibe-MSP-GKS-Integration.md
- G:\govibe\docs\architecture\SDD-Genesis-Block.md
- G:\govibe\docs\architecture\SDD-Symbol-Graph-Traceability-Boundary.md
- G:\govibe\docs\blueprints\BLUEPRINT-Genesis-Knowledge-System.md
- G:\GenesisBlock_Dev\GenesisBlock\src\query\hql.pest (working tree, for section 5)

Caveats: token budgets per H tier are NOT specified anywhere in these docs (budget is caller-supplied via CONTEXT_PACKET.token_budget); the referenced FRAMEWORK_MASTER_SPEC.md §13 and SRS-GKS-RETRIEVAL-LAYER were not in the target list and may carry additional RRF parameters.