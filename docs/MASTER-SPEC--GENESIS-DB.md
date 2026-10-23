---
title: "GenesisBlockDB Technical Architecture and Capability Composition"
doc_id: "MASTER-SPEC-GENESISBLOCKDB"
status: current
version: "2.3.58b"
updated: "2026-10-23"
owner: "GenesisBlockDB Architecture"
source_of_truth: true
related_issue: 84
related_docs:
  - "docs/BRD--GENESISBLOCKDB.md"
  - "docs/PRD--GENESISBLOCKDB-PLATFORM.md"
  - "docs/SRS--GENESISBLOCKDB.md"
  - "docs/contracts/CONTRACT--CLIENT-NAMESPACE-AND-SCHEMA.md"
  - "docs/adr/ADR--GENESISBLOCKDB-DOMAIN-NEUTRAL-CORE.md"
  - "docs/SPEC--GENESISDB-P6-GENERATIONS-LEASES-ACL.md"
  - "docs/adr/ADR--GENESISDB-HQL2-PATTERN-CONSTRAINTS.md"
  - "docs/adr/ADR--GENESISDB-HQL2-P8-COMPLETION-ADDENDUM.md"
  - "docs/adr/ADR--GENESISDB-TYPED-QUERY-IR-AGENT-BOUNDARY.md"
  - "docs/adr/ADR--GENESISDB-PEER-MEMBERSHIP-AUTHORIZATION.md"
  - "docs/SPEC--GENESISDB-TYPED-QUERY-IR-V1.md"
  - "docs/SPEC--WAVE-A-COMMIT-CORRECTNESS.md"
  - "docs/ADR--GENESISRAG17-SEPARATE-WORKER-PUBLICATION.md"
  - "docs/FLOW--GENESISRAG17-PIPELINE.md"
  - "docs/GENESISRAG17-EXTENSION-MAP.md"
---

## Approved G4 Aggregate frontend diagnostics - 2026-10-23

Owner approved tests/docs-only qualification at merged main `1821da9`
(LOW, C-3). The existing 121-bag normal HQL/IR/P7 oracle is retained.
The added selected matrix pairs HQL/IR global and grouped Aggregate through
Storage: empty input, all NULL, repeated positive values and repeated signed
values. All seven existing aggregate functions are checked against independent
P7 bags; global emits one row even on empty input, grouped outputs are
hand-specified as 0/1/3/3. Values/Aggregate/Project row counters are fixture
cardinalities; other counters keep typed unknowns and explicit units.
Plan requests precede execution/publication; denied and output-budget controls
preserve observed frontier/file state after explicit publication. Lock and
SQLite shared-memory bookkeeping remain size-only; no universal internal or
transient-write claim is made. Snapshot/metadata mode equality is consistency,
not independent provenance acceptance. Same-request plan identity never
implies cross-frontend semantic equality or access authority.
Local runtime passes79/79 unique tests across8 targets, zero failed/ignored;
focused2/2 is included once. Owner explicitly expanded the limit to eight
existing files for the pre-existing Join fixture date correction: capture one
valid_at after setup and reuse before Plan, retaining every row/counter/state
assertion. The original regression failure and RCA remain recorded below.
Strict core/default all-target Clippy, formatting, docs244/0, version0.2.10 and
diff checks pass. Hosted qualification is NOT_RUN for this candidate.
No production/runtime/planner/API/P6/WAL/hash/schema/ACL changes. Broader
operator breadth, G4/P8/P13, independent runtime/device/release gates stay open.

PR259 head `83e35fd6274f8d0982c3f3f071e8c739a7f57365` merged as
`1821da9b89dd7b418ac7b507d2a510c2b799d36e` after four successful
exact-head workflows and 15 SUCCESS checks plus one schedule-only skip.
Tests `39826465866`, Security `39826465863`, worker `39826465865`,
performance `39826466200` completed successfully, including Rust Ubuntu,
Windows and macOS. Merged tree `00ac16259742eb2fe6473bc097cc70c6d22bb2cc`
equals the tested tree; this is not a fresh post-merge runtime rerun.
Earlier pending statements below are historical checkpoints. The deferred
Minor Join temporal RCA pointer is corrected in the P8 report to
`query/hql2/request.rs::normalize`; raw historical logs are retained.



## Approved G4 storage-backed Join diagnostic qualification - 2026-10-23

Owner approved tests/docs-only scope at merged main `e2be296` (LOW, C-3),
extending the existing HQL/IR NodeScan/RowScan Join P7 differential target.
The paired matrix qualifies Inner/Left/Semi/Anti with duplicate keys,
missing-property SQL NULL, JSON null and left/right/both-empty inputs.
Normal and Analyze result bags have independent P7 expectations; counters
are specified from fixture cardinalities, not inferred from reported actuals.
Plan precedes query execution/publication in each fixture and preserves
observed frontier/file state. Error guards use an explicitly published
generation; lock/shared-memory contents retain the established size-only
comparison. This is observation, not internal operator instrumentation.
Local runtime passes78/78 unique tests across8 targets (focused2 included);
strict core/default all-target Clippy and formatting pass; docs244/0 and
version0.2.10 checks pass. Hosted qualification is pending.
No runtime/planner/API/P6/WAL/hash/schema/ACL model changes. UnionAll,
Aggregate and broad G4/P8/P13, independent runtime/device/release gates remain open.

PR258 head `5e5dd30121bd0fd50324421b4fffcd401e0b19ec` merged as
`e2be296e2e2ae2cf3320b19d24220d096b4ff345` after four successful
exact-head workflows and 15 SUCCESS checks plus one schedule-only skip.
Tests run `39787302554` passed Ubuntu, Windows and macOS. Merged tree equals
tested tree `29f05366e2229f722110c60e51bfeff4b482aeb5`; this equality
is not a post-merge test rerun. Historical pending/NOT_RUN records remain
historical checkpoints, not retroactively executed acceptance.

## Approved G4 composed-query state/identity qualification - 2026-10-22

Owner approved tests/docs-only scope at main `2832ad42` (LOW, C-3).
The fresh read-only review of PR256/257 found no Critical/Important defect
and three Minor evidence improvements: full annotation identity, direct
composed-path state guards and dated hosted-evidence reconciliation.
The focused test target now compares all annotation reference fields and
captures frontier/fixture-file state around composed Plan, annotation denial
and budget-error requests; a separate Plan control runs before any query or
explicit generation publication. Lock/shared-memory bookkeeping uses only
the existing size comparison; no new byte-comparison exclusions are added.
Local156/156 across10 targets and strict core/default all-target Clippy,
format/docs/version/diff gates pass; hosted qualification remains pending.
No runtime/API/P6/WAL/hash/schema/ACL model changes.
Broad G4/P8/P13 and independent runtime/device/release acceptance stay open.

### Current hosted evidence reconciliation

PR256 head `3395e75e408163be7ad961070c1dfcf677db70e8` merged as
`e8f1a58815eac36e45c887f3c73f4d23e3c28d5b`; PR257 head
`c73ec15b35087c6d5540fa97635c89e9dedd11bd` merged as
`2832ad421e1260d57475c330f2bf9cc2e48eab7f`. Each had four successful
exact-head workflows and 15 SUCCESS checks plus one schedule-only skip.
PR257 macOS runner-acquisition cancellations in attempts1/2 were retained;
Tests run `39467118959` succeeded on retry3 before merge. PR256 Tests run
`39448311999` succeeded before merge. Earlier pending/NOT_RUN entries are
historical checkpoints, not current CI failures or retroactive local execution.
This source/evidence review covers only the bounded PR256/257 increment;
it is not whole-G4 independent runtime acceptance.


## Approved G4 annotation-authority composition qualification - 2026-10-21

Owner approved tests/docs-only scope at merged main `e8f1a588` (LOW, C-3).
PR256 passed all four exact-head workflows including Rust on three OSes;
its permissive provenance matrix and namespace-denied controls remain valid.
The next target-local test qualifies enforced Namespace(Read) plus
Annotation(namespace,Read) in HQL/IR lookup and exact KNN/original Rerank ->
parallel-edge Expand -> lookup. Policy revisions grant/remove/regrant the
annotation permission between independent requests, each with a freshly
published pinned generation. Plan stays catalog-only; empty input behavior
is qualified separately from nonempty lookup authorization. No eager ACL,
concurrent revocation, runtime/API/P6/WAL/hash/schema contract is introduced.
Local154/154 across10 targets, strict core/default all-target Clippy,
format/docs244/0/version0.2.10/diff gates pass. Hosted qualification pending;
broad G4/P8/P13, independent/device/release gates remain open.



## Approved G4 provenance/completeness qualification - 2026-10-20

Owner approved tests/docs-only qualification at main `d1bc37e` (LOW, C-3).
The new `hql2_g4_provenance_completeness_tests` target independently checks
full snapshot, four semantics fields, completeness reason/eligible count,
index frontiers and cursor for HQL/IR normal and Analyze requests.
Fixtures cover AnnotationLookup and exact KNN/original Rerank followed by
parallel-edge Expand and AnnotationLookup; entity/revision and score owner
references must retain fixture identity. Empty input is checked separately.
No generic provenance field, truncated-query contract, runtime/API/P6/WAL or
shape-hash change is introduced. Local runtime passes 141 unique tests across eight targets (focused2/2),
with zero failed/ignored; core/default strict all-target Clippy and
format/docs/version/diff checks pass. Hosted qualification is pending; broad
G4/P8/P13 and independent/device/release gates remain open.



## G4 evidence reconciliation - 2026-10-20

Owner-approved documentation-only reconciliation at main `919814f`.
PR #249/#250/#251/#252/#253/#254 are merged bounded qualification slices.
For #254, exact head `7c57bda39be64233ef1dfa8122b0cd47b296eee0`
passed all four triggered PR workflows (Tests `39403874324`, Security
`39403874024`, Performance `39403874196`, worker `39403874412`), with
15 successful checks and one schedule-only advisory skip; Ubuntu, Windows
and macOS Rust suites succeeded before merge `919814f7954659bed2c23a9bdc2777e2d067ddf1`.
Local identity qualification remains 221 unique tests across 18 targets.
Historical NOT_RUN evidence (including #251 local Application Control) is
preserved; later hosted success does not retroactively execute a blocked local run.

The acceptance matrix in the [P8 report](REPORT--HQL2-P8-CORE-2026-09-28.md#g4-acceptance-evidence-matrix)
separates bounded evidence from missing acceptance. G4/P8 remains PARTIAL;
independent review, device/release and P9-P13 gates are not closed.




# GenesisBlockDB Technical Architecture and Capability Composition

## Approved G4 plan identity/golden qualification - 2026-10-20

Owner approved tests/docs-only qualification on main `24d7eed`; LOW risk, C-3.
Independently specify canonical shape/hash goldens, JSON map-order stability,
parameter values versus shape, controlled node/operator/input/column changes,
and per-request normal/EXPLAIN/ANALYZE parity. Keep the existing shape hash,
planner/runtime/API/P6/WAL/schema contracts. Equal hashes do not imply equal
semantics or authorization. Local qualification passes 221/221 across 18 targets;
strict core/default Clippy, format/docs/version/diff gates pass; hosted passed in #254;
broad G4/P8/P13 and independent/device/release acceptance remain open.

PR #253 exact head `768b56a28678904e505bf0a88e149480a0dd561c` completed
four successful PR workflows (15 SUCCESS checks plus one schedule-only skip),
including Rust Ubuntu/macOS/Windows, before normal merge as
`24d7eeddac7fad3503df5d53c282105ce93f0196`. Its local runtime passed 170 tests
across 14 targets; earlier fixture failures/RCA remain historical evidence.


## Approved G4 lexical/context qualification - 2026-10-20

The owner approved the bounded tests/docs-only proposal on merged main
`7c3e5e2`. Risk LOW, complexity C-3. Qualify LexicalMatch/ContextPack exact rows,
three-mode HQL/IR parity, empty/duplicate inputs, typed unknowns, token/citation
provenance, capability/budget errors and observed-no-write EXPLAIN. Preserve
BM25, tokenizer, runtime/API/P6/WAL/schema and planner contracts. ContextPack
emits one package on empty input; token truncation is a successful package.
Local qualification passes 170/170 across 14 targets; strict core/default
all-target Clippy, format/docs/version/diff gates pass. Hosted qualification
for the lexical/context revision passed in #253. Broad G4/P8/P13,
independent/device/release gates remain open.

PR #252 exact head `25f6e4445ad0ceb11471302e0b5ec40b44d451a9` completed
four successful hosted workflows (15 SUCCESS checks plus one schedule-only
skip), including Rust on Ubuntu/macOS/Windows, then merged normally as
`7c3e5e23984e124a4ba480b8f1e22bf218372b0b`. Its local runtime passed 93 unique
tests across 11 targets. The #251 local NOT_RUN remains historical only.


## Approved G4 history/change diagnostic qualification - 2026-10-20

The owner approved the bounded test/documentation matrix on main `a3e2d4d`.
HistoryScan/ChangeScan keep existing P6/H2-D11 source, temporal/floor/ACL and
budget contracts. Qualify exact emitted rows, empty measured zero, typed unknown
source timing, HQL/IR result parity and normal/EXPLAIN/ANALYZE plan identity.
EXPLAIN remains catalog-only; data-dependent errors are compared only between
normal execution and ANALYZE. No runtime/API/WAL/schema/planner change.
Risk LOW, complexity C-3. Local qualification passes 93 tests across 11 targets;
hosted qualification passed in #252. G4/P8/P13, lexical/context,
independent review and device/release gates remain open.

PR #251 qualified head `098a71e` completed four successful hosted workflows,
15 successful checks and one schedule-only skip before merge as `a3e2d4d`.
Its three OS Rust suites passed; local runtime was NOT_RUN due to Application
Control 4551. The failed parent and fixture-grant RCA remain historical evidence.


## Approved G4 source and annotation diagnostics - 2026-10-20

The owner approved test-only source/annotation diagnostic qualification on
main `8b18a2c`: exact rows, empty input, mode parity, actor authority,
EXPLAIN no-effect and fail-closed budgets under the existing Storage/P6 contract.
See the G4 ADR and P8 checkpoint for the matrix and validation. PR #249/#250
hosted checks completed before merge. Broader G4/P8/P13 and independent,
device/release gates remain open. This slice is LOW risk, C-3.


The owner-approved G4 graph/vector diagnostic qualification follows scalar
qualification under the existing [G4 contract](adr/ADR--GENESISDB-HQL2-G4-SHARED-PIPELINE.md).
It adds fixture-based counter/result evidence for Sequence/Match and exact
KNN/Original Rerank without changing the engine architecture or capability
contracts. The [P8 report](REPORT--HQL2-P8-CORE-2026-09-28.md) records its
separate evidence and keeps broader G4/P8/P13 acceptance open.

The owner approved bounded G4 scalar and multi-input diagnostic qualification
after reviewing main `6195e85`. The [G4 ADR](adr/ADR--GENESISDB-HQL2-G4-SHARED-PIPELINE.md)
defines this test-only matrix; the [P8 checkpoint](REPORT--HQL2-P8-CORE-2026-09-28.md)
records fresh evidence separately from historical results. G4/P8 remain partial.
The additive Storage/NAPI `query_v2_contract` / `queryV2Contract` surface already
exists under [API Reference](API_REFERENCE.md); this does not establish P13
cross-surface parity. No new runtime, schema, migration or transport contract
is introduced by this qualification slice.

The owner-approved [HQL2 execution boundary](adr/ADR--GENESISDB-HQL2-EXECUTION-BOUNDARY.md)
adds a staged, explicitly selected v2 query pipeline. Existing HQL, Query IR v1,
storage and transport contracts remain the current compatibility surfaces.
P7 reference-interpreter evidence does not imply P8 production execution.
The [P8 core checkpoint](REPORT--HQL2-P8-CORE-2026-09-28.md) records the
separately approved Rust pipeline, revision-bound scans, annotation lookup,
constrained graph expansion, exact KNN/Original Rerank, lease-bound
HistoryScan/ChangeScan, root Match anchors, and structural unsigned parameters.
Approved addendum D1-D5 now has local implementation evidence: pre-parse P6
ACL denial, registered exact-scan lexical and scalar-token ContextPack profiles,
Sequence node/edge exact-JSON properties under the lease, and contextual
NULL/list/JSON literals. The focused completion target passes 10/10, including
an edge-property-before-SHORTEST regression. The approved D7 bridge accepts
differential-tested actor-scoped zero-hop HQL1 ID projections (unlabeled or
with one plain-ASCII label) and bounded unlabeled one-hop projections through
`Storage::query_v2`, including one endpoint-ID exact string equality filter;
either zero-hop projection may also include one exact same-alias string-ID
equality filter. Parser resources are preflighted and reserved before legacy
AST construction.
The differential-tested D7 extension also supports one unlabeled zero-hop
single-property projection and exact-seed depth-one `TRAVERSE` ID projection
over one physical relation or `ANY` in default/out/in/both directions. Missing
and explicit-null property values retain legacy bag semantics; traversal
excludes the seed and deduplicates projected target IDs. Other property and
traversal forms remain fail-closed. Adapter, property, ordering and
labeled-filter targets pass 13/13, 2/2, 2/2 and 2/2; root HQL2 passes 404/0/1
across 39 targets.
The focused adapter, ordering and labeled-filter tests pass 11/11, 2/2 and
2/2; the new differential covers match/miss, wrong-label exclusion and a
backslash/Unicode ID, alongside pre-parse authorization/namespace checks,
malformed and unlisted forms, and resource rejection. HQL `JOIN TABLE` now
lowers to the existing
closed RowScan/Join pipeline for Inner/Left/Semi/Anti, with bare JOIN defaulting
to Inner; HQL, typed IR and independent P7 differential results match for
duplicate, missing-property NULL and JSON-null cases. The latest root-HQL2
regression sweep passes 404/0/1 across 39 targets. Test-only optional
Sequence edge-property-miss and nested-JSON property differentials match
independent P7 for HQL and typed IR; nested node/edge object and array values
match exactly, while optional misses preserve all four input rows and
NULL-extend endpoint/edge aliases. The separate 11-target
P6/schema-v6/compatibility group passes 194/0/0. Storage-backed HQL/typed-IR
`Values`/`UnionAll` results match independent P7 for 169 nullable bag pairs
and 338 Storage executions, preserving NULL and duplicate multiplicity.
The scalar-pipeline P7 differential adds six fixtures and 12 Storage
executions across projection/arithmetic/order/offset/take and
filter/project/distinct/order/take; both frontends match P7 and each other.
The storage-backed HQL/typed-IR Node/Edge/Row source-scan bags also match
independent P7, excluding wrong labels/relations and retaining two row keys;
the focused storage-source target passes 10/10.
The HQL/typed-IR ChangeScan P7 differential also covers five exclusive-after/
inclusive-through windows, including equal empty bounds and explicit
`tx_as_of` frontiers; the focused History/Change target passes 17/17.
HistoryScan bags for Node, Edge, Row, Vector and Annotation match independent
P7 catalogs assembled from WAL-derived revision facts and captured frontiers/
valid-time windows, including endpoint, vector-owner and annotation-reference
ACL dependencies. Artifact HistoryScan remains unsupported; broad ChangeScan,
P8/P13, hosted CI and review remain open. The
H2-D11 R4/P6 annotation grant regression confirms:
Annotation subjects require explicit Annotation(Read) separately from the
Namespace(Read) query grant, while same-namespace target/evidence references
remain covered by the broad namespace grant. HQL2 and typed-IR
`tx_as_of` now select one historical frontier across source scans, graph/vector/
annotation operators, hydration and `Snapshot.tx`, with no current-state
fallback and the P6 generation/current policy still pinned. Five focused targets
pass 56/56. Independent review
of D7 remains pending; earlier read-only review found no concrete static defect,
and focused re-review confirmed the corrected D5
JSON-literal size preflight. Compact constraints remain fail-closed. Broad
exact-oracle, shared HQL1/HQL2/IR, resource/cancellation, P8 and P13 gates
remain open; these local results do not close the full HQL2/P8 gate. A separate
selected HQL2/durability/authority verification now passes 40 targets,
including schema-v6 WAL-only recovery; this is fixture evidence, not release or
consumer qualification.
The final locked/offline no-default-features Rust suite passed with no failures;
the explicitly selected `probe_vs_recall` test remains NOT_RUN. Strict all-target
Clippy passed with default and no-default features. These local results do not
close independent review, full HQL2/P8/P13, release or consumer gates.

## 1. Role of this document

This document is the authoritative technical architecture composition for GenesisBlockDB. It explains how implemented and declared product capabilities fit together.

It is not the Business Requirements Document, Product Requirements Document, or Software Requirements Specification. Those are maintained separately:

```text
BRD--GENESISBLOCKDB
  -> PRD--GENESISBLOCKDB-PLATFORM
  -> SRS--GENESISBLOCKDB
  -> MASTER-SPEC / C4 / ADR / feature specs
  -> code, tests and benchmarks
```

## 2. Abstract

GenesisBlockDB is a high-performance, embedded, local-first hybrid graph and vector database engine written in Rust. It provides a unified substrate for structured graph relationships, vector embeddings, lexical retrieval, temporal/event history, generic provenance, and durability.

GenesisBlockDB is a standalone product with multiple independent clients. GoVibe, NotiKeeper, and future applications own their own ontology, authority, workflow, and projection semantics. The database core stores and executes generic client-defined records through namespaces, schema references, typed graph records, vector collections, temporal metadata, and query contracts.

The architecture SHALL remain valid if GoVibe or NotiKeeper is removed from the ecosystem.

## 3. Product-neutral architecture boundary

```text
Client application
  -> client-owned domain/schema/authority
  -> client adapter or SDK
  -> GenesisBlockDB typed API / Query IR
  -> generic graph, vector, lexical, temporal, provenance and durability core
```

### 3.1 Core ownership

GenesisBlockDB owns:

- generic nodes, edges, labels, properties and client identifiers;
- client namespaces and schema references;
- vector collections, embeddings and retrieval indexes;
- lexical indexes;
- temporal versions, event order, supersession and generic causality references;
- WAL, snapshots, backup, restore, replay and recovery;
- query, mutation, capability and SDK contracts;
- optional generic governance-supporting and consensus primitives.

Clients own:

- atom or record taxonomy;
- relation business meaning;
- canonical identity policy;
- authority, promotion and context rules;
- planning, notification or other workflows;
- application validation and user-facing views.

GoVibe-specific GKS/MSP/planning contracts and NotiKeeper-specific notification contracts SHALL NOT become mandatory database-core ontology.

## 4. Core Architecture

### 4.1 Storage Model

GenesisBlockDB uses a **Log-Structured Merge-Friendly** architecture based on a Write-Ahead Log (WAL).

- **Primary Log:** `genesis-graph.wal` (JSONL format) stores mutation events.
- **Persistence:** high-durability append-only logic with batched group commits.
- **Unified operational boundary:** applications open, mutate, query, back up and restore GenesisBlockDB as one database. SQLite is an internal relational projection; native graph/vector indexes are not separate application-managed databases.
- **Relational projection:** embedded SQLite (`rusqlite`, bundled) stores node properties, normalized labels and U2 app-defined tables. Versioned additive schemas, idempotent typed mutation batches and bounded named joins are available through Genesis APIs. SQLite remains internal and rebuildable from the signed WAL. Unified cross-domain transactions preflight relational constraints before WAL append. Supported query reads and graph/projection publication share a reentrant commit boundary; failed durable apply requires reopen before further queries, writes or checkpoint. Concurrent reads serialize; ANN visibility still requires the existing flush barrier. See [Wave A contract and verification limits](SPEC--WAVE-A-COMMIT-CORRECTNESS.md).
- **In-memory state:**
  - `DashMap<u32, NodeOutput>`: lean primary node records; `props` are hydrated from SQLite rather than retained on the traversal path.
  - **Recovery identity:** snapshots and journal replay preserve the same public-ID-to-`u32` mapping used by vector metadata and the SQLite projection. An empty snapshot starts allocation at `0`; a populated snapshot resumes at the largest saved key plus one. ID drift can return the correct vector hit with another node's properties.
  - `DashMap<u128, EdgeOutput>`: primary edge storage. Edges are keyed by deterministic `u128 = trunc128(SHA256(id))`; the key is derived from `EdgeOutput.id` and is not client identity.
  - `Adjacency Indices`: forward (`out_idx`) and backward (`in_idx`) indexes for O(1)-class adjacency access.

Internal numeric or hashed keys are implementation details. Public contracts preserve client-provided IDs.

### 4.2 Client namespace and schema metadata

Generic client records may carry:

```yaml
client_namespace: string
schema_ref: string
schema_version: string
client_record_id: string
client_mutation_id: string | null
```

The database preserves and indexes this metadata according to the client namespace/schema contract. Validation may be performed by the client, adapter, or optional hook. The core does not hard-code one client ontology.

### 4.3 Semantic Hybrid Indexing

GenesisBlockDB bridges lexical and semantic search through:

1. **Lexical Index:** Thai-aware trigram/bigram behavior that strips combining marks for high-recall fuzzy matching.
2. **Vector Index:** named `VectorCollection`s, each with its own model, dimension, metric, arena and HNSW index. A `default` collection exists. HNSW insertion is asynchronous; `flush_index` forces a drain.
3. **Neural Bridge:** multilingual support using language centroids and mean-centering.

Embeddings and similarity are retrieval data. They do not define client canonical identity by themselves.

### 4.4 Graph Retrieval Layer

The Graph Retrieval Layer (GRL) provides generic tiered or bounded graph retrieval:

- a resolver maps configured tiers or scopes to graph expansion;
- a budget manager estimates result size/token cost and may compress results;
- an orchestrator combines vector anchors with bounded graph expansion.

A client may map these primitives to its own context policy. The GRL does not make GoVibe MSP rules mandatory for NotiKeeper or other clients.

### 4.5 GenesisRAG17 TEST integration boundary

The isolated GenesisRAG17 integration is a client adapter around this public
engine boundary. The separate `genesisrag17-worker/` process owns physical
Stage 13 graph writes, Stage 15 CPU embeddings, Stage 16 index/readback and
atomic publication. It uses one native store owner and a worker-owned SQLite
FTS5 lexical sidecar; the sidecar is an adapter implementation, not a new
engine authority. MSP authenticates and relays pipeline calls, while GKS owns
canonical decisions, Stage 14 enrichment and the Stage 17 quality verdict.

The physical sequence is graph-only write and receipt -> GKS enrichment ->
embedding -> six-lane index/readback -> quality gate -> atomic pointer update ->
publication receipt. A prepared snapshot is not query-visible, and a gate
without a matching publication receipt cannot finish a source run. This TEST
adapter does not add GKS ontology, MSP policy, source schemas or a second
database to GenesisBlockDB. The current candidate engine source and
qualification status are recorded in the
[GenesisRAG17 ADR](ADR--GENESISRAG17-SEPARATE-WORKER-PUBLICATION.md); the
historical integration baseline remains identified there. Its model and
contract pins, runtime variables, lane statuses and extension seams are recorded in the
[GenesisRAG17 ADR](ADR--GENESISRAG17-SEPARATE-WORKER-PUBLICATION.md),
[pipeline flow](FLOW--GENESISRAG17-PIPELINE.md), [extension
map](GENESISRAG17-EXTENSION-MAP.md) and [worker README](../genesisrag17-worker/README.md).

## 5. Data Model and Bitemporality

### 5.1 Generic node schema

| Field | Type | Description |
|---|---|---|
| `id` | String | Stable external/client-facing identifier. |
| `labels` | Vec<String> | Client-defined classification labels. |
| `props` | JSON | Generic client properties. |
| `impact` | f64 | Optional derived importance score. |
| `embedding` | Vec<f64> | Optional vector data; collection metadata applies. |
| `valid_from` | RFC3339 | Logical start time. |
| `valid_to` | Option<RFC3339> | Logical end time. |
| `expires_at` | Option<RFC3339> | Optional TTL expiration. |
| `caused_by` | Option<String> | Generic causality/provenance reference. |
| `clock` | LogicalClock | Lamport timestamp where CRDT behavior applies. |
| `client_namespace` | String/optional by deployment contract | Client/domain namespace. |
| `schema_ref` | String/optional by validation mode | Client-controlled schema reference. |
| `schema_version` | String/optional | Schema version metadata. |

### 5.2 Generic edge schema

A generic edge preserves:

- stable external edge ID;
- source and target external IDs;
- client-defined relation type;
- namespace and schema metadata;
- generic properties;
- provenance/causality metadata;
- temporal validity;
- internal numeric/hash key as a private optimization only.

### 5.3 Bitemporal philosophy

GenesisBlockDB follows an immutable-by-default update pattern. Updates use supersession:

1. mark the existing version with `valid_to`;
2. insert a new version with `valid_from` and updated properties;
3. link the mutation to generic causality/provenance metadata where supplied.

Clients decide whether a supersession is a semantic correction, business update, notification state change, or another domain event.

## 6. Reasoning and Autonomic Substrate

### 6.1 K-Impact Model

Node importance may be calculated through the documented K-Impact formula and inputs such as dependency depth, configured strictness/governance metadata and source stability.

The core may compute generic scores. Clients decide how or whether those scores affect authority or workflows.

### 6.2 Structural Insight Engine

The maintenance loop may perform:

- community detection;
- supernode or cluster-summary generation;
- structural gap detection;
- vector/centroid drift tracking.

Outputs are analytical candidates or derived data. They do not automatically become client canonical truth.

## 7. Governance-Supporting and Consensus Primitives

GenesisBlockDB may expose generic tiers, guards, signatures, proposals and votes. These are database/runtime primitives, not a universal application authority model.

A client may map them to:

- GoVibe canonical promotion;
- NotiKeeper notification approval;
- a future client's independent governance policy.

The core SHALL not require one mapping for all clients.

Cryptographic signatures establish event integrity against a key; peer
discovery alone does not establish membership or client authority. The accepted
[peer-membership authorization ADR](adr/ADR--GENESISDB-PEER-MEMBERSHIP-AUTHORIZATION.md)
defines the engine's closed roster, signed gossip, and role checks. Client
semantic authority remains client-owned; the engine does not infer it from a
client tier or business label.

## 8. Distributed Synchronization

Where enabled, synchronization uses documented logical-clock, reconciliation and CRDT behavior.

- Lamport timestamps provide deterministic event ordering inputs.
- LWW or other configured reconciliation behavior must be documented as a storage conflict rule, not a substitute for client semantic conflict policy.
- Local clock advancement follows the documented protocol.

Client-level semantic conflicts may require review even when storage-level
reconciliation succeeds. Enabled synchronization uses the operator-configured
roster and authenticated gossip envelope defined by the accepted
peer-membership ADR. P6 transport synchronization remains outside the
record-read ACL.

Membership authorization preflight is bounded before it accumulates retained
journal events: at most 64 MiB of uncompressed signed-event payload and 100,000
records across legacy, sealed, and active sources. Exceeding either limit fails
closed before membership activation. Membership-aware startup also fails before
replay; late configuration can occur after plain single-node `open()` has
replayed local state, which is retained while the staged roster is rolled back.
Backup restore fails before publishing its target. Source reads allow 16 bytes
of framing per record plus a 72-byte segment envelope and up to 1 MiB of LZ4
container overhead for an individual sealed segment. The declared body length
caps decompression; a maximum-size frame fits within the allowance, while a
128 MiB segment is rejected before body read. The bounds mirror the
consensus-ledger policy and do not claim measured peak RSS; see the
[journal format spec](SPEC--GENESISDB-JOURNAL-FORMAT-V1.md) and
[P2P gossip spec](SPEC--P2P-GOSSIP-PROTOCOL.md).

## 9. HQL and Typed Query IR

The accepted public-query architecture is a versioned typed Query IR. Its
partial executor/API slice implements `search`, `traverse`, linear `match_path`
and target-id `context` in core, N-API and REST. Query-vector/temporal context
and relational named-query remain unsupported or planned; see the capability
manifest and `SPEC--GENESISDB-TYPED-QUERY-IR-V1`.

GenesisBlockDB currently exposes HQL as a compatibility frontend for graph, vector and context
operations. HQL is not storage authority and is not required to grow into general-purpose SQL or
Cypher. Free-form NL interpretation belongs in a separate client-owned Agent Query Adapter, which
must produce validated Query IR before invoking the engine.

```text
NL intent -> external agent adapter -> typed Query IR -> GenesisBlockDB typed executor
HQL text -> compatibility parser --------------------^
```

### 9.1 Search

```sql
SEARCH ~target SIMILAR TO [v1, v2, ...] K 5 IN "code" LANGUAGE "th" AS OF "2026-01-01T00:00:00Z"
```

The optional `IN <collection>` clause scopes search to a named vector collection; omitted means `default`.

### 9.2 Traverse

```sql
TRAVERSE FROM seed DEPTH 2 REL INFER(depends_on) AS OF "..."
```

Relation labels are client-defined data. Query execution does not grant them universal business meaning.

### 9.3 Hybrid

```sql
MATCH target SIMILAR TO [...] ALPHA 0.4 LANGUAGE "en"
```

Query contracts should support namespace, collection, temporal/revision and bounded traversal scope where implemented.

## 10. Deployment and Connectivity

### 10.1 Deployment modes

The same Rust core supports in-process embedding and a single-node self-hosted Axum server. Multi-node HA, distributed SQL and automatic failover are not v1 claims unless separately implemented and evidenced.

### 10.2 Model Context Protocol

GenesisBlockDB may provide a native MCP server for bounded database operations.

Current tools include or may expose:

- HQL/query execution;
- bounded/tiered context retrieval;
- generic knowledge/record insertion with provenance.

MCP tools SHALL not assume GoVibe canonical authority or NotiKeeper workflow semantics.

### 10.3 Python SDK

The Python SDK provides typed generic node, edge, query and retrieval bindings. Client-specific schema wrappers belong in client packages or adapters.

### 10.4 Go SDK

The Go SDK provides concurrent-safe generic database access. References to “full mapping of GKS schemas” should be interpreted or revised as optional client adapters, not the product-neutral core contract.

## 11. Conformance and evidence

The architecture is conformant when:

- GoVibe and NotiKeeper adapters run against one unmodified core;
- a third client namespace/schema can be added without recompilation;
- public IDs survive WAL/snapshot/restore/query paths;
- internal key changes do not alter client identity;
- client-specific validation is outside mandatory core ontology;
- implemented/partial/proposed status remains evidence-backed;
- benchmark claims include workload and environment.

## 12. Document responsibility

- BRD defines business need and product independence.
- PRD defines user-facing product scope and major capabilities.
- SRS defines SHALL requirements.
- This Master Spec defines technical architecture composition.
- ADRs define significant decisions.
- Feature specs and code/tests define implementation detail and evidence.

Wave B (R-02/R-03, approved 2026-09-08) journals immutable collection definitions
including empty/default spaces and preserves calibration and exact rerank rows
through fold/recovery. Disk schema is 4; derived SQLite projection schema is 5.
Edge replacement rewires both adjacency directions and `edge_versions` selects
replica-local transaction intervals. History is available only from the reported
edge-history floor. New-reader preflight rejects unsupported complete frames;
old-engine use of a manually stripped journal-only v4 copy is unsupported.
See [Wave B contract and verification record](SPEC--WAVE-B-DURABLE-COLLECTIONS-EDGE-HISTORY.md).
These are local implementation contracts, not deployment or consumer migration evidence.

P6 (architecture correction approved 2026-09-22) is the approved target for signed generation
receipts after index flush and fail-closed snapshot validation, opaque generation-bound read
leases, explicit temporal selectors, and signed revision-checked access-policy events. Its target
disk schema is now 6 (the prior documented Wave B/P6 baselines are v4/v5); readers that do not
understand schema 6 must reject it. Fixture-backed v5-to-v6 migration and markerless WAL recovery
pass their focused tests, but no user database migration or consumer compatibility qualification
has been performed. Current graph/vector records still lack entity namespace fields, so the
P6 ACL contract does not claim migrated tenant isolation. See [P6 specification](SPEC--GENESISDB-P6-GENERATIONS-LEASES-ACL.md).

Schema-v6 selection remains authority-bearing when the snapshot marker is lost. A missing
`state.json` requires journal preflight before projection replay: schema 6 is selected only by a
valid signed local activation record and, for migrated stores, its complete migration and P6
receipt proof. Schema-5-compatible WAL remains replayable as v5; v6-only events without that
proof fail with `RECOVERY_REQUIRED` instead of silently downgrading. See H2-D11 ADR R6b and the
P6 WAL-only recovery contract. Runtime implementation is fixture-verified; no
user database was migrated, and broader P6/P8/P13 qualification remains open.

H2-D11 R5 restore validation opens staging with full retention, requires normal recovery to
reproduce the bundle's final WAL-frame frontier, then publishes or reuses a P6 generation and
independently validates a read-only lease before target exposure. A required local receipt may
advance restored live `stable_frontier`;
a valid receipt already at the manifest frontier is reused. `txn_frontier` is preserved, and the
complete return metadata is prepared before target rename. The U9 restore target passes 7/7
and the fresh HQL2 sweep passes 404/0/1 across 39 targets. Source-backed NodeScan plan-only
EXPLAIN remains partial, and
read-only/rename failure cleanup is best-effort and unverified. This does not close P14, hosted,
independent-review or broad P8/P13 gates.

## Changelog

Version diff 2.3.45b -> 2.3.46b: bound membership authorization preflight for
legacy, sealed, and active journal sources by uncompressed payload and record
count before journal-wide accumulation; preserve fail-closed context errors and
document the non-measured memory tradeoff.

Version diff 2.3.44b -> 2.3.45b: accept the closed roster, authenticated gossip,
MASTER-role, fixed-quorum, and event-bound vote contract; require an explicit
local role for multi-node configuration and synchronize implementation
entrypoints for security issues #236 and #237. Client semantic authority
remains client-owned.

Version diff 2.3.43b -> 2.3.44b: clarify that peer discovery and signatures do
not establish membership or client authority; link the candidate peer
authorization ADR. No wire, schema, or authority-policy decision is approved.

Version diff 2.3.42b -> 2.3.43b: distinguish the candidate GenesisRAG17 engine
source from its historical baseline and link qualification to the worker ADR;
no core ontology, pipeline ownership or wire-contract change is implied, and
full integration qualification remains pending.

Version diff 2.3.37b -> 2.3.38b: extend the approved D7 differential
allowlist with exact-seed depth-one HQL1 `TRAVERSE` ID projection over one
physical relation or `ANY` in default/out/in/both directions. Adapter 13/13;
HQL2 387/0/1 across 34 targets; P6/schema-v6/compatibility remains 194/0/0
across 11. Seed self-loops are excluded and parallel edges deduplicated. Other
traversal forms remain fail-closed; P6 is unchanged and independent review,
shared-runtime and broad P8/P13 gates remain open.

| Version | Date | Owner | Summary |
|---|---|---|---|
| 2.3.52b | 2026-10-20 | GenesisBlockDB Architecture | Approve bounded G4 identity/golden qualification and reconcile hosted PR253; preserve production contracts and broad gates |
| 2.3.51b | 2026-10-20 | GenesisBlockDB Architecture | Approve bounded G4 lexical/context diagnostics and reconcile hosted PR252; preserve production contracts and broader gates |
| 2.3.50b | 2026-10-20 | GenesisBlockDB Architecture | Approve test-only History/Change diagnostic matrix and reconcile PR251 hosted qualification; preserve runtime contracts and broader acceptance gates |
| 2.3.49b | 2026-10-20 | GenesisBlockDB Architecture | Owner-approved source/annotation diagnostics, exact rows and truthful unknowns; reconcile scalar/graph hosted evidence while retaining broader acceptance gates |
| 2.3.48b | 2026-10-19 | GenesisBlockDB Architecture | Link graph/vector diagnostic qualification without changing architecture, capabilities or G4/P8/P13 acceptance |
| 2.3.47b | 2026-10-19 | GenesisBlockDB Architecture | Link approved test-only G4 scalar qualification and correct additive NAPI exposure without promoting P8/P13 acceptance |
| 2.3.46b | 2026-10-19 | GenesisBlockDB Architecture | Bound membership journal-authorization preflight by uncompressed payload bytes and record count before accumulation; preserve fail-closed startup/configuration/restore errors; document memory measurement limit |
| 2.3.45b | 2026-10-16 | GenesisBlockDB Architecture | Accept closed peer roster, authenticated gossip, MASTER role checks, fixed-roster quorum, and event-bound votes for issues #236 and #237; require explicit multi-node local role and preserve client-owned semantic authority |
| 2.3.44b | 2026-10-16 | GenesisBlockDB Architecture | Clarify discovery/signature versus membership/authority; link candidate peer authorization ADR; no trust policy, schema, or wire change |
| 2.3.43b | 2026-10-06 | GenesisBlockDB Architecture | Record GenesisRAG17 candidate engine source/qualification through the worker ADR; preserve historical baseline and TEST-only boundary; no schema or wire change; full integration qualification pending |
| 2.3.42b | 2026-10-05 | GenesisBlockDB Architecture | Extend approved D7 with one unlabeled zero-hop property projection and exact-seed depth-one TRAVERSE; adapter 13/13, property/order/filter 2/2 each, HQL2 404/0/1 across 39 targets and U9 restore 7/7; sync R5 restore evidence; preserve fail-closed boundaries and broad shared-runtime/P8/P13 gates |
| 2.3.41b | 2026-10-04 | GenesisBlockDB Architecture | Extend D7 after legacy/HQL2 differential to the exact same-alias string ID predicate on labeled zero-hop scans; adapter/order/filter 11/11, 2/2, 2/2 incl. wrong-label and backslash/Unicode; HQL2 394/0/1 across 35 targets, selected P6 peers 45/0/0 across 7; PR #213 code commit checks 40 pass, 4 worker failures with cause unconfirmed, 5 skipped, 1 Windows Cargo cancellation at 15-minute job limit; no P6/schema/transport change; broad shared-runtime/P8/P13 gates open |
| 2.3.40b | 2026-10-04 | GenesisBlockDB Architecture | Add test-only HQL/typed-IR ChangeScan P7 differential for five exclusive-after/inclusive-through windows, equal empty bounds and tx_as_of frontiers; History/Change 17/17 and HQL2 392/0/1 across 34 targets; PR #210 hosted checks 11 pass, four worker markerless-identity failures, one skipped; no runtime/contract/schema/P6/transport change; broad P8/P13 and independent review remain open |
| 2.3.39b | 2026-10-04 | GenesisBlockDB Architecture | Add test-only HQL/typed-IR Node/Edge/Row source-scan P7 differential; focused target 10/10; fresh complete 34-target HQL2 sweep 391/0/1 (prior 393 count not reproduced); no runtime/contract/schema/P6/transport change; broad P8/P13 and independent review remain open |
| 2.3.38b | 2026-10-04 | GenesisBlockDB Architecture | Extend D7 after legacy/HQL2 differential for one exact same-alias string ID predicate on unlabeled zero-hop HQL1 scans; adapter 11/11, ordering 2/2, HQL2 was reported 393/0/1 across 34 targets at that checkpoint; other forms remain fail-closed; no P6/schema/transport change; independent review and broad P8/P13 gates remain open |
| 2.3.37b | 2026-10-04 | GenesisBlockDB Architecture | Record test-only nested-JSON Sequence property P7 differential for exact HQL/typed-IR node/edge object and array values; focused pattern 11/11, HQL2 392/0/1 across 34 targets; no runtime/contract/schema/P6/transport change; broad exact-oracle and P8/P13 qualification remain open |
| 2.3.36b | 2026-10-04 | GenesisBlockDB Architecture | Record the test-only optional Sequence edge-property-miss P7 differential; preserve four input rows and NULL-extend endpoint/edge aliases; HQL2 389/0/1 across 34 targets; no runtime/contract/schema/transport change; broad exact-oracle and P8/P13 qualification remain open |
| 2.3.35b | 2026-10-03 | GenesisBlockDB Architecture | Record test-only HQL/typed-IR scalar-pipeline P7 differential for six fixtures and 12 Storage executions; HQL2 383/0/1 across 33 targets, P6/compatibility 194/0/0; no runtime/contract/schema/transport change; broad exact-oracle and P8/P13 qualification remain open |
| 2.3.34b | 2026-10-03 | GenesisBlockDB Architecture | Extend D7 after legacy/HQL2 differential for a single plain-ASCII label on zero-hop HQL1 ID scans; focused adapter 10/10, HQL2 382/0/1 across 33 targets, P6/compatibility 194/0/0; retain other HQL1/shared-runtime, review and P8/P13 gates |
| 2.3.33b | 2026-10-03 | GenesisBlockDB Architecture | Record test-only HQL/typed-IR Values/UnionAll P7 differential for 169 nullable bag pairs and 338 Storage executions; HQL2 382/0/1 across 33 targets, P6/compatibility 194/0/0; no runtime/schema/transport change; retain broad exact-oracle, P8/P13 and qualification gates |
| 2.3.32b | 2026-10-03 | GenesisBlockDB Architecture | Record storage-backed P7 HistoryScan differentials for Node/Edge/Row/Vector/Annotation; History/Change 15/15, Annotation source 7/7, P7 130/130, HQL2 379/0/1 and P6/schema-v6/compatibility 194/0/0; Artifact, broad ChangeScan/P8/P13, hosted CI and review remain open |
| 2.3.31b | 2026-10-03 | GenesisBlockDB Architecture | Record independent P7 differential for H2-D11 Vector HistoryScan from WAL-derived revisions; P7 130/130, History/Change 14/14, HQL2 377/0/1 and P6/schema-v6/compatibility 194/0/0. At prior PR #194 head 9ada855, Windows cargo plus four worker checks fail and reviews are absent; hosted verification of this source revision and full acceptance/merge remain open |
| 2.3.30b | 2026-10-03 | GenesisBlockDB Architecture | Record H2-D11 R4/P6 annotation ACL conformance and local HQL2 375/0/1 plus selected P6/schema-v6 43/0/0; possible ChangeScan budget side channel, prior PR worker failures and cancelled Windows Rust, hosted checks/review and full acceptance/merge remain open |
| 2.3.29b | 2026-10-03 | GenesisBlockDB Architecture | Record PR #194 run 37083654705 at docs-only head 43cc6e8: Windows HQL/IR Join budget failure and four worker bootstrap failures; local Join reproduction 5/5, full acceptance and merge remain open |
| 2.3.28b | 2026-10-03 | GenesisBlockDB Architecture | Synchronize the explicit HQL2 regression sweep to 374/0/1 across 32 targets and current PR #194 CI state: core checks pass, four worker checks fail at fresh schema-v6 bootstrap; preserve worker-fix approval, review, broad P8/P13 and release gates. |
| 2.3.27b | 2026-10-03 | GenesisBlockDB Architecture | Implement HQL2/IR `tx_as_of` across sources, graph/vector/annotation operators, hydration and `Snapshot.tx` using one no-fallback frontier; record focused 56/56, HQL2 373/0/1 and P6/compatibility 194/0/0; retain broad P8/P13, review, worker-CI, soak, security and release gates. |
| 2.3.26b | 2026-10-03 | GenesisBlockDB Architecture | Record HQL JOIN lowering through approved RowScan/Join contract, independent HQL/IR/P7 parity and 365/0/1 across 31 HQL2 targets; retain broad P8/P13, review, worker-CI, soak, security and release gates. |
| 2.3.25b | 2026-10-02 | GenesisBlockDB Architecture | Record full local Rust regression and strict Clippy pass for HQL2/H2-D11 integration; keep `probe_vs_recall` NOT_RUN, no user migration, and independent-review/P8/P13/release gates open. |
| 2.3.24b | 2026-10-02 | GenesisBlockDB Architecture | Record fixture-verified H2-D11 R6b markerless schema-v6 recovery and upstream Query IR V1 1.0.3 linear match_path; 40 selected HQL2/durability/authority targets pass; no user DB migration, full P8/P13 gates open. |
| 2.3.23b | 2026-10-02 | GenesisBlockDB Architecture | Synchronize owner-approved H2-D11 R6b/P6 schema-v6 WAL-only recovery authority; implementation and recovery tests pending. |
| 2.3.22b | 2026-10-02 | GenesisBlockDB Architecture | Add D7's differential-proven one-hop endpoint-ID exact string filter; record 9/9 focused adapter tests, 361/0/1 across 27 HQL2 targets and 190/0/0 across 11 compatibility targets; retain shared-runtime, independent-review and broad P8/P13 gates. |
| 2.3.21b | 2026-10-02 | GenesisBlockDB Architecture | Extend D7 with differential-proven one-hop HQL1 projections and pre-parse resource reservation; record 8/8 focused tests, 361/0/1 across 27 HQL2 targets and 190/0/0 across 11 compatibility targets; retain shared-runtime, independent-review and broad P8/P13 gates. |
| 2.3.20b | 2026-10-02 | GenesisBlockDB Architecture | Record D7's initial actor-scoped HQL1 projection, 5/5 focused adapter tests and 354/0/1 across 27 HQL2 targets; retain shared-runtime, independent-review and broad P8/P13 gates. |
| 2.3.19b | 2026-10-02 | GenesisBlockDB Architecture | Record D4 edge-property-before-SHORTEST regression, 10/10 P8 completion tests and 349/0/1 across 26 HQL2 targets; independent static review has no concrete finding; retain broad P8/P13 gates. |
| 2.3.18b | 2026-10-02 | GenesisBlockDB Architecture | Truth-sync approved P8 D1-D5 runtime and 528/0/1 across 37 explicit regression targets; retain JSON-size correction review and broad P8/P13 gates. |
| 2.3.17b | 2026-10-02 | GenesisBlockDB Architecture | Register owner-approved P8 addendum D1-D6 for bounded text operators, Sequence property constraints, contextual literals and P6 ACL-fixture correction; runtime evidence pending. |
| 2.3.16b | 2026-09-30 | GenesisBlockDB Architecture | Truth-sync lease-bound HQL/typed-IR Sequence node ID/labels and 338/0/1 across 25 HQL2 targets; retain properties/Compact, broader P8 and release gates. |
| 2.3.15b | 2026-09-30 | GenesisBlockDB Architecture | Record delegated HQL/typed-IR Sequence node-ID/conjunctive-label contract under P6; runtime implementation pending, properties/Compact fail-closed. |
| 2.3.14b | 2026-09-30 | GenesisBlockDB Architecture | Truth-sync HQL root-Match P7 graph differential and 331/0/1 across 24 root HQL2 targets; retain broad HQL2/P8 and release gates. |
| 2.3.13b | 2026-09-30 | GenesisBlockDB Architecture | Truth-sync HQL/IR exact-KNN and Original Rerank differentials against independent P7 ranking and 330/0/1 across 23 root HQL2 targets; retain broad HQL2/P8 and release gates. |
| 2.3.12b | 2026-09-30 | GenesisBlockDB Architecture | Truth-sync HQL/IR exact-KNN differential against the independent P7 oracle and 329/0/1 across 23 root HQL2 targets; retain broad HQL2/P8 and release gates. |
| 2.3.11b | 2026-09-30 | GenesisBlockDB Architecture | Truth-sync checked HQL `%`/typed-IR `rem`, same-type I64/F64 semantics and 328/0/1 across 22 local targets; retain remaining HQL2/P8 and release gates. |
| 2.3.10b | 2026-09-30 | GenesisBlockDB Architecture | Truth-sync exact DecimalU64 structural HQL parameters and HQL2 NULLS LAST default; record 325/0/1 across 22 local targets; retain pattern-constraint, remaining HQL2/P8 and release gates. |
| 2.3.9b | 2026-09-30 | GenesisBlockDB Architecture | Truth-sync exact DecimalU64 structural HQL parameter binding and 323/0/1 across 22 local HQL2 targets; retain pattern-constraint, remaining HQL2/P8 and release gates. |
| 2.3.8b | 2026-09-30 | GenesisBlockDB Architecture | Truth-sync validated typed-IR root Match anchors and 319/0/1 across 22 local HQL2 targets; retain pattern-constraint, remaining HQL2/P8 and release gates. |
| 2.3.7b | 2026-09-30 | GenesisBlockDB Architecture | Truth-sync lease-bound HQL/IR HistoryScan/ChangeScan and 318/0/1 local HQL2 evidence; retain Lexical/Context, constrained-pattern, transport and P8 gates. |
| 2.3.6b | 2026-09-29 | GenesisBlockDB Architecture | Truth-sync exact HQL/IR KNN and Original Rerank, H2-D11 collection-space binding and 307/0/1 local HQL2 evidence; retain explicit history, transport and P8 gates. |
| 2.3.5b | 2026-09-29 | GenesisBlockDB Architecture | Truth-sync structural HQL/IR Sequence Expand and the 206-pass/21-target fixture matrix; keep pattern constraints, full HQL2 and release gates open. |
| 2.3.4b | 2026-09-29 | GenesisBlockDB Architecture | Truth-sync the partial P8 checkpoint for constrained HQL/IR graph expansion and the 199-pass/20-target fixture matrix; keep full HQL2 and release gates open. |
| 2.3.2b | 2026-09-28 | GenesisBlockDB Architecture | Register accepted explicit HQL2 execution boundary; P7 fixtures do not imply production completion. |
| 2.3.3b | 2026-09-28 | GenesisBlockDB Architecture | Register the approved P8 scalar/core checkpoint and remaining storage/surface gates. |
| 2.3.1b | 2026-09-22 | GenesisBlockDB Architecture | Registered the owner-approved P6 durability, generation/lease, temporal and ACL target with explicit schema-compatibility and implementation-status limits. |
| 2.3.0b | 2026-09-08 | GenesisBlockDB Architecture | Added the separate GenesisRAG17 TEST adapter boundary, ordered publication flow and extension references while retaining the client-neutral core. |
| 2.2.2 | 2026-09-08 | GenesisBlockDB Architecture | Reflected approved Wave B durable collections, schema compatibility and edge version intervals. |
| 2.2.1 | 2026-09-08 | GenesisBlockDB Architecture | Reflected approved Wave A preflight, publication and recovery-required behavior with verification limits. |
| 2.2.0 | 2026-08-14 | GenesisBlockDB Architecture | Approved typed Query IR as the primary query boundary, retained HQL compatibility, and placed NL conversion outside the engine. |
| 2.1.0 | 2026-08-03 | GenesisBlockDB Architecture | Separated BRD/PRD/SRS roles, established standalone client-neutral boundary, added client namespace/schema metadata, and removed GoVibe-specific authority from the core definition. |
| 2.0.0 | previous | GenesisBlockDB Architecture | Previous master specification. |
