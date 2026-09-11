---
title: "ADR: GenesisRAG17 separate worker and atomic publication"
doc_id: "ADR-GENESISRAG17-SEPARATE-WORKER-PUBLICATION"
status: beta
version: "1.0.4b"
updated: "2026-09-11"
owner: "GenesisBlockDB Architecture"
source_of_truth: true
attributes:
  domain: integration
  scope: "GenesisRAG17 isolated TEST pipeline"
  engine_commit: "e15e35b0093394e0a8880af7f4e6f63cf81223b7"
  model_revision: "614241f622f53c4eeff9890bdc4f31cfecc418b3"
related_docs:
  - "docs/MASTER-SPEC--GENESIS-DB.md"
  - "docs/C4--GENESISDB-ARCHITECTURE.md"
  - "docs/FLOW--GENESISRAG17-PIPELINE.md"
  - "docs/GENESISRAG17-EXTENSION-MAP.md"
  - "genesisrag17-worker/README.md"
---

# ADR: GenesisRAG17 separate worker and atomic publication

## Status and scope

This is the GenesisBlockDB-side architecture decision for the isolated
GenesisRAG17 TEST integration. It records the boundary that the worker package
and the cross-repository acceptance run implement. It is not a production
deployment approval and it does not change the client-neutral native engine.

The product-level source documents are the [GenesisRAG17 architecture
decision ADR-073](https://github.com/Freshair129/zuri.ai/blob/codex/ki17-integration/docs/decisions/ADR-073-GENESISRAG17-ISOLATED-EXECUTION-AND-PUBLICATION.md),
the [17-stage source
specification](https://github.com/Freshair129/zuri.ai/blob/codex/ki17-integration/docs/KNOWLEDGE-INGESTION-17-STAGE-SPEC.md),
the [17-stage execution
flow](https://github.com/Freshair129/zuri.ai/blob/codex/ki17-integration/docs/KNOWLEDGE-INGESTION-17-STAGE-FLOW.md),
and the [approved wire contract](https://github.com/Freshair129/zuri.ai/blob/codex/ki17-integration/docs/plans/GENESISRAG17-CONTRACT.md).
The [pinned historical acceptance
report](https://github.com/Freshair129/zuri.ai/blob/b64b46df057d3160c659afa3c34628ee86520257/.brain/reports/GENESISRAG17-ACCEPTANCE.md)
is evidence for the isolated test only.

The approved implementation contract is `1.3.0b`; its wire schema remains
`genesisrag17.v1`. This repair does not change the native engine pin, model
revision, receipt schema or stage identities.

## Context

GenesisRAG17 is a logical seventeen-stage pipeline whose ownership crosses
four repositories. Tier 1 persists raw, parsed and chunk lineage; MSP is the
authenticated transport and owns no stage; GKS owns canonical decisions and
the quality authority; Tier 4 writes and queries the physical retrieval
substrate. A gate result without physical write evidence or a publication
receipt cannot establish that a run finished.

GenesisBlockDB therefore needs a separate integration worker with one native
store owner. The worker must use the public native boundary while retaining the
engine's generic node, edge, vector, lexical, temporal, SQLite and provenance
semantics. It must not turn GKS vocabulary, MSP policy, or a source application's
schema into native core ontology.

## Decision

### 1. Separate worker and neutral engine

`genesisrag17-worker/` is a separate TEST package and process. It owns one
GenesisBlock native store process, a worker-owned SQLite FTS5 lexical sidecar,
durable worker state/outboxes and the loopback query endpoint. The native
engine remains pinned to
`e15e35b0093394e0a8880af7f4e6f63cf81223b7`; no GenesisRAG17 domain logic is
added to the Rust core.

The worker uses the CPU `intfloat/multilingual-e5-small` model at revision
`614241f622f53c4eeff9890bdc4f31cfecc418b3`, dimension 384, cosine metric, with
the executable artifact hash manifest. A missing, mismatched or fallback model
is an error. The worker's lexical manifest says
`implementation: worker_sqlite_fts5` because the pinned native binding has no
usable standalone lexical query method; it does not invent a native API.

### 2. MSP is the only pipeline transport

The worker claims decisions, reports physical receipts, requests the quality
gate, reports stage failures and submits publication receipts through the MSP
pipeline tools. MSP authenticates the exact scope and role, relays to GKS, and
relays query requests to the worker's loopback endpoint. The worker never opens
the GKS store, calls GKS directly, reads the Edge store, or owns a caller's
database.

The grant separation is part of the boundary:

| Principal | Allowed TEST operations | Credential boundary |
|---|---|---|
| Tier 1 source | `msp_pipeline_submit`, evidence pull and query | A source grant configured in `MSP_PIPELINE_PRINCIPALS` with `role: source`; it is never reused as the worker grant. |
| Tier 4 worker | claim, graph receipt, write receipt, gate, publication receipt, stage failure and query | `GENESIS_WORKER_CREDENTIAL`, configured in the MSP principal list with `role: worker` and the exact private scope. |
| MSP to GKS | relayed provider calls | `MSP_GKS_PIPELINE_CREDENTIAL` is checked by GKS as `GKS_PIPELINE_RELAY_CREDENTIAL`; neither value is supplied by a caller. |
| Worker query client | authenticated loopback `/query` | `GENESIS_WORKER_QUERY_TOKEN` / `MSP_PIPELINE_WORKER_TOKEN`; this is a query bearer, not a pipeline principal. |

Runtime examples use synthetic TEST credentials only. Production credentials
must stay in the owning process environment or secret manager and must not be
written into this repository.

### 3. Physical execution order

GKS may return a decision containing the logical Stage 13 graph decision, but
Stage 13 is not terminal until the worker has written and read back the graph.
The physical order is fixed:

1. Tier 1 executes and durably records Stages 1–8, then sends one source batch
   for the Stage 9 attempt through MSP.
2. GKS performs Stages 9–12 and returns an immutable decision hash. The worker
   claims it through MSP.
3. The worker commits a graph-only candidate for Stage 13: source, parsed,
   chunks, mentions, entities, verified facts and held records; it writes no
   vectors or derived summaries in this transaction. It first fsyncs the graph
   transaction intent with its exact payload and expected frontier, then
   commits, flushes, checkpoints and reads back the native graph/SQLite state.
4. The worker sends `msp_pipeline_graph_receipt`. MSP relays it to GKS. GKS
   validates the physical counts, identity, scope and six Stage 13 metrics,
   closes Stage 13 once, runs `enrich_v1` Stage 14, and returns immutable
   `derived` data plus `derivedHash`.
5. The worker keeps the enrichment result separate from the original decision,
   performs real CPU Stage 15 embeddings, and commits the derived records and
   vectors for the candidate generation. Before that first vector transaction,
   it checkpoints a newly created native collection manifest with `saveState`.
   Stage 16 then performs worker-owned lexical indexing, flush/checkpoint,
   native and lexical readback and the frozen benchmark before sending the
   Stage 15/16 write receipt.
6. GKS verifies the matching graph receipt, derived hash and final physical
   receipt. The worker asks for the Stage 17 quality gate through MSP. GKS
   evaluates data, graph, knowledge, security and retrieval dimensions together
   with the explicit policy.
7. Only a `PASS` verdict with `allowPublication: true` in both the verdict and
   decision policy permits the worker to write a prepared snapshot. `WARN` is
   held fail-closed even when its allow flag is true. The worker atomically
   replaces the publication pointer and its retained published-history
   membership, then sends
   `msp_pipeline_publication_receipt`. GKS emits successful Stage 17 evidence
   only after that exact receipt is accepted. Tier 1 can finish only after the
   matching evidence and publication receipt are imported.

The detailed message sequence, failure transitions and query path are in
[FLOW--GENESISRAG17-PIPELINE.md](FLOW--GENESISRAG17-PIPELINE.md).

### 4. Durable publication and recovery

Each native phase has one durable, exact transaction-intent file below the
isolated worker database root. The fixed filenames are:

| Native phase | Exact intent filename |
|---|---|
| Graph | `genesisrag17/transactions/graph-<safeDecisionId>.json` |
| Final | `genesisrag17/transactions/final-<safeDecisionId>.json` |

`<safeDecisionId>` is the worker's sanitized filename component for the
decision id; it does not change the decision identity or hash.

The worker fsyncs the intent before `commitTransaction`. It contains the exact
serialized native payload, transaction id, `expected_frontier`, scope/decision
identity and payload hash. On restart or an uncertain native reply, the worker
validates and reuses that same payload and transaction id. The intent is
removed only after the corresponding receipt and local state are durable.
The native collection declaration is also checkpointed with `saveState` before
the first vector transaction, so WAL replay cannot auto-provision a collection
with fallback model or metric metadata.

The candidate snapshot file is not publication. Queries accept a snapshot only
when one authoritative pointer read includes its id in
`publishedSnapshotIds`; a file existing on disk is insufficient. The pointer
binds one query to one generation, and retained historical generations keep
their citations addressable for correction and audit.

Graph receipt, final write receipt, stage failure and publication receipt
requests are durable outbox records with deterministic idempotency identity.
After GKS accepts a graph receipt, the worker saves the accepted graph receipt
and `derived` result to local state before removing the graph outbox or graph
intent. `retryOutbox` uses the same ordering, including after a process crash.
Reply loss retries the original request. A crash after native commit does not
blindly recommit; a crash before pointer replacement leaves the old pointer
visible; a crash after pointer replacement resumes the exact stored
publication receipt from its outbox. Receipt hashes, transaction frontier,
snapshot id, generation, model revision and timestamps are not rewritten for a
replayed terminal result.

Prepared snapshots and pointer files are written through a temp-file fsync and
an operating-system atomic replacement. Transient Windows `EPERM`/sharing
errors may be retried. If replacement still fails, the old pointer remains
authoritative; the worker never renames the old pointer away as a fallback.
The pointer retains historical snapshot ids, and those snapshots and citations
remain queryable after later generations publish.

The worker enforces one store owner with a lock and uses a dedicated child
process for true native-handle restart because the pinned N-API binding does
not expose `close`. `start`, `stop` and `resume` are explicit process-owned
loops; no OS scheduler is part of this TEST path.

The acceptance fault hooks are placed at the actual durability boundaries:

| Hook | Boundary |
|---|---|
| `after-native-commit-before-receipt` | Immediately after native commit returns and before native flush/state completion or a graph/write receipt; `phase` is `graph` or `final`. |
| `after-graph-receipt-accepted-before-local-state` | After MSP/GKS accepts the graph receipt and before the accepted derived state is saved; the graph outbox remains. |
| `before-pointer-replacement` | After the prepared snapshot is durable and before pointer replacement. |
| `after-pointer-replacement-before-publication-outbox` | After pointer/history replacement and before publication outbox creation. |

The process-kill acceptance harness terminates at these hooks with exit code
`86` and verifies recovery from the durable files above.

### 5. Honest six-lane evidence

Stage 16 reports a manifest for all six lanes. `objects` is a measured numeric
count, and status is not inferred from a JSON field or a successful write:

| Lane | TEST status rule | Current implementation |
|---|---|---|
| Vector | `ready` after actual N-API write, flush and per-generation readback | Native HNSW, 384-dimension CPU embeddings, one collection per scope/generation |
| Lexical | `ready` after Stage 16 actual indexed and queried rows | Worker-owned SQLite FTS5; Stage 13 writes no lexical rows; `implementation: worker_sqlite_fts5` |
| Graph | `ready` after native graph commit, flush/checkpoint and readback | Native graph projection and receipt-bound physical counts |
| SQLite | `ready` after native projection SQL readback | Engine-owned internal SQLite projection; caller never opens it |
| Bitemporal | `ready` only for actual native temporal Query IR readback; `not_applicable` for explicit no-valid-time fixture data | Transaction stamps do not become fabricated valid-time support |
| Provenance | `ready` after persisted source/chunk/citation IDs, hashes and UTF-16 spans verify | Native source/chunk lineage and citation readback |

An unsupported required lane fails the gate. A bitemporal `not_applicable`
row is valid only when the input has no applicable valid-time assertion; it is
not evidence of a production temporal index.

### 6. Legacy compatibility path remains distinct

[`docs/ADR--GKS-MSP-PROMOTION-MCP.md`](ADR--GKS-MSP-PROMOTION-MCP.md) remains
the accepted legacy `gks_knowledge_promote` MCP compatibility path. It is not
the GenesisRAG17 worker protocol, does not authorize direct GKS or Tier 1
substrate access, and does not replace graph receipts, the six-lane write
receipt, the Stage 17 gate or the publication receipt. Its history remains
unchanged.

## Consequences and non-goals

- The cross-repository flow has one reviewable physical write owner and one
  publication boundary while the core remains reusable by other clients.
- Source, worker and MSP credentials can be checked independently and exact
  scope mismatches fail before provider invocation.
- Real native persistence, CPU embedding, FTS5 search, readback, recovery and
  receipt semantics are test evidence; no in-memory fixture can substitute for
  them.
- This decision does not add LLM extraction, a new UI, production deployment,
  multi-source concurrency, a native lexical API, or a native temporal index.
- New semantic stages belong in GKS or the source application. New physical
  lanes or model revisions require an additive contract/ADR and new evidence;
  they are not silent worker configuration changes.

## Accepted extension — structured-record profile, contract revision 2 (2026-09-11)

**This is a docs-only acceptance note. No worker code changes accompany it.**
It records the GenesisBlock worker owner's acceptance of contract revision 2
of the GenesisRAG17 structured-record ingestion profile (the SmartGift
catalog use case), one of the four acceptance notes required before Phase 2
implementation may begin (source of truth: the owner decision record dated
2026-09-11, `phase2-contract-decisions.md`, itself built on the proposal at
`zuri.ai:.brain/proposals/2026-09-11-genesisrag17-structured-record-profile.md`,
branch `docs/adr-075-phase2-gate`). The facts below were re-verified against
this repository's `origin/main` at `5dc75ff522ee4cc06961fb239afb1c20976d7e7a`
before this note was written.

### Qualifier shape: Option A accepted, Option B deferred

The owner chose **Option A** — a tier-qualified price is represented as a
distinct entity (`PRICE_TIER`, resolutionKey `{productCode}:{tier}:{price}`,
e.g. `PM-BOTTLE-LED:qty100:<satang>`), not as a new field. This keeps the
`facts[]`/`graph.edges[]` object shape the worker already validates
byte-for-byte unchanged — no new key crosses the Stage 13 claim boundary for
pricing.

**Option B** (an optional `qualifiers: Record<string,string>` field on
facts/edges) is **deferred as a future option, not rejected**. Adopting it
later changes the frozen decision shape and every `decisionHash` input, so it
needs its own contract revision and its own four-repo gate — accepting it
now, bundled with this revision, is explicitly out of scope for this note.

### C-2 vocabulary and the shared predicate → endpoint table

`ontology_v2` is a **superset** of `ontology_v1`. The worker and GKS must
carry the *same* table content — this worker currently encodes the table as
two independent hard-coded ternaries (see below) plus an independent
predicate allowlist, none of which is table-driven:

| Predicate | Subject | Object | Status |
|---|---|---|---|
| `WORKS_FOR` | `PERSON` | `ORGANIZATION` | unchanged from v1 |
| `PURCHASED` | `PERSON` or `ORGANIZATION` | `PRODUCT` | unchanged from v1 |
| `HAS_COMPONENT` | `PACKAGE` | `PRODUCT` | new |
| `PRICED_AT` | `PRODUCT` or `PACKAGE` | `PRICE_TIER` | new (Option A) |
| `IN_CATEGORY` | `PRODUCT` or `PACKAGE` | `CATEGORY` | new |

`PACKAGED_AS` (the reverse of `HAS_COMPONENT`) and `OFFER` (no predicate uses
it; SmartGift `BundleOffer` records map to `PACKAGE`) are **not** in v2.
Endpoint types in v2: `PERSON`, `ORGANIZATION`, `PRODUCT`, `PACKAGE`,
`CATEGORY`, `PRICE_TIER`.

### Supported-version set and accept-before-produce rollout

The worker's Stage 13 claim check and GKS's Stage 17 gate both move from
accepting the single literal `'ontology_v1'` to accepting the fixed set
`{ontology_v1, ontology_v2}`, each decision validated against the table for
its own version. Rollout is strictly ordered and **the worker is step 1**:

1. the worker accepts both versions (this repository's change);
2. GKS accepts both and starts producing `ontology_v2`;
3. zuri-ai starts sending parser-2 catalog batches.

The worker must ship its accept-both change before GKS is allowed to produce
a single `ontology_v2` decision — a worker that still hard-rejects anything
but `ontology_v1` would fail every such decision at the Stage 13 claim, not
at Stage 17, which is a worse failure mode (a claimed-but-unwritable
decision) than declining to claim it at all.

### C-9 required worker implementation, with verified anchors

Re-verified by direct reading at `origin/main` `5dc75ff`, not by memory of
the earlier review:

| Change | File : line(s) | Current behavior |
|---|---|---|
| Version check → supported set | `genesisrag17-worker/src/worker.mjs:280` | `if (decision.ontologyVersion !== 'ontology_v1' \|\| decision.pipelineVersion !== SCHEMA_VERSION) fail('DECISION_VERSION_INVALID');` — the only reference to `ontologyVersion` in the file |
| `validateFact` endpoint ternary → shared table | `genesisrag17-worker/src/worker.mjs:303-305` | `const endpointsValid = row.predicate === 'WORKS_FOR' ? entityKind(subject)==='Person' && entityKind(object)==='Organization' : ['Person','Organization'].includes(entityKind(subject)) && entityKind(object)==='Product';` |
| Graph-build endpoint ternary → shared table | `genesisrag17-worker/src/worker.mjs:1156-1160` | The same two-branch shape, duplicated independently in the Stage 13 graph-build loop (`validEndpoints = predicate === 'WORKS_FOR' ? … : predicate === 'PURCHASED' ? … : false`) |
| `entityKind()` gains package/category/price_tier | `genesisrag17-worker/src/worker.mjs:758-763` | Recognizes only `person` → `Person`, `organization`/`company` → `Organization`, `product` → `Product`; anything else passes through as the raw `semanticType` string |
| Test fixtures | `genesisrag17-worker/test/worker.test.mjs` (1090 lines) | No `ontology_v2` fixtures exist yet |

**One additional hard-coded gate found during re-verification, not named in
the C-9 list above and needing the same table-driven fix:** `validateFact`
also carries an independent predicate allowlist at
`genesisrag17-worker/src/worker.mjs:296` —
`if (!['WORKS_FOR', 'PURCHASED'].includes(row.predicate)) fail('FACT_PREDICATE_NONCANONICAL', row.predicate);`
— which runs *before* the endpoint ternary and would reject
`HAS_COMPONENT`/`PRICED_AT`/`IN_CATEGORY` outright regardless of endpoint
types. Implementation must extend this allowlist (or replace it with a
lookup against the same shared table) alongside the two ternaries; fixing
only the ternaries and missing this check would leave `ontology_v2` facts
failing with `FACT_PREDICATE_NONCANONICAL` instead of succeeding.

Confirmed **unaffected** by this profile, so no change is required at those
points: `generation` keying (`worker.mjs:1059`, `:1912` —
`` `g17-${hashObject({decisionId, decisionHash}).slice(0,32)}` ``, no
`ontologyVersion` input) and snapshot/pointer keying, both version-agnostic;
Stage 14 enrichment and Stage 15/16 embedding/indexing, both driven by
generic counts and chunk text with no `semanticType`/predicate branch (the
only four `semanticType` reads in the file are the two `typeof` validation
checks, `entityKind()` itself, and generic property pass-through into node
storage).

### Worker tests required (C-8)

Per the contract-decision list, `genesisrag17-worker/test/worker.test.mjs`
needs, in addition to existing coverage: `ontology_v2` decisions exercising
each new predicate/endpoint-type pair (including a `PRICE_TIER` object for
`PRICED_AT`); a `ontology_v1` decision still accepted unchanged (regression);
and the mixed-temporal bitemporal case described below. None of these tests
are added by this PR — this note only records that they are required before
Phase 2 implementation lands.

### Open verification item — the bitemporal lane on a mixed generation

The contract-decision list carries this open item: "the bitemporal lane must
handle a generation mixing dated facts and `not_applicable` facts (GKS
core:410 expects an object for every fact)." Read-only finding from
`verifyTemporalLane()` and its helpers at `origin/main` `5dc75ff`
(`genesisrag17-worker/src/worker.mjs:1464-1469` for `temporalRows`,
`:1471-1531` for `verifyTemporalLane`, feeding `laneManifest()`'s
`bitemporal` entry at `:1557-1561`):

- `temporalRows(decision)` (`:1464-1469`) collects every `fact` and `held`
  row into one list — it does not separate dated from `not_applicable` rows
  up front.
- `temporalClassification()` (`:243-260`) classifies each row independently
  as `mapped`, `not_applicable`, or `unsupported`, based only on that row's
  own `temporal` field.
- `verifyTemporalLane()` then **filters to the `mapped` subset** (`:1480`)
  and runs the real native Query IR temporal readback (`executeQueryIr`,
  `:1506-1524`) only against those rows; `not_applicable` rows are excluded
  from that readback and from the reported `objects` count (`:1530`,
  `objects: mapped.length`) but do **not** cause the lane to fail.
- The lane fails (`status: 'unsupported'`) only if a row's classification is
  itself `unsupported`, or a `mapped` row is missing a parseable
  `validFrom`, or the native readback disagrees with expected visibility for
  a `mapped` row. A generation that is a genuine mix of dated and
  `not_applicable` facts — the case this open item asks about — hits none of
  those conditions on the `not_applicable` side: those rows are silently
  excluded from readback, not treated as an error.

**Finding: the worker already handles a mixed generation without failing or
crashing.** The bitemporal lane reaches `ready` based solely on the dated
(`mapped`) subset; `not_applicable` facts in the same generation are
correctly excluded from the lane's `objects` count and from native temporal
verification, consistent with ADR §5's description of `not_applicable` as
valid "only when the input has no applicable valid-time assertion" — here
that test is applied per-row rather than per-generation, which is what makes
the mix safe. This is a read-only observation; it required no code to
satisfy the open item, only reading how the existing per-row classification
already resolves it. It does not by itself confirm GKS's own expectation
("an object for every fact" at GKS `core:410`) — that is a GKS-side
question, out of scope for this worker-side note.

### Summary

The worker accepts: Option A for tier-qualified pricing; the C-2 vocabulary
and shared predicate→endpoint table; the supported-version set
`{ontology_v1, ontology_v2}` with worker-first accept-before-produce
rollout; the C-9 implementation list above (plus the one additional
`FACT_PREDICATE_NONCANONICAL` gate found during re-verification); and the
C-8 test obligations. **No worker code changes accompany this PR** — this is
acceptance of the contract, to be implemented in a follow-up change once all
four repositories' acceptance notes are merged per the ADR-075 Phase 2 gate
rule.

## Verification evidence

The worker setup, exact model artifact manifest, runtime variables, lifecycle,
query, fault points and focused proof are documented in
[genesisrag17-worker/README.md](../genesisrag17-worker/README.md). The frozen
historical acceptance report records the isolated synthetic corpus, actual
native operations, 17-stage terminal evidence, six metrics, recovery cases and
its explicit non-production limits.

## Changelog

| Version | Date | Status | Summary | Commit Hash | Agent |
|---|---|---|---|---|---|
| 1.0.4b | 2026-09-11 | beta | Docs-only acceptance of GenesisRAG17 structured-record profile contract revision 2 (ADR-075 Phase 2 gate): Option A tier-qualified pricing, the C-2 predicate/endpoint table, the {ontology_v1, ontology_v2} supported-version set with worker-first accept-before-produce rollout, the C-9 worker implementation list with verified file:line anchors (plus one additional FACT_PREDICATE_NONCANONICAL gate found on re-verification), the C-8 worker tests required, and a read-only finding that the bitemporal lane already handles a mixed dated/not_applicable generation. No worker code changed. | working-tree | Claude Opus 5 |
| 1.0.2b | 2026-09-08 | beta | Reconciled the live zuri GenesisRAG17 architecture reference to ADR-071 after the identifier collision; retained the pinned historical acceptance report. | working-tree | RWANG |
| 1.0.1b | 2026-09-08 | beta | Synced audit remediation: exact pre-commit native intents and collection checkpoint recovery, accepted graph-state ordering, Stage 16 lexical indexing, PASS-only publication and no-fallback pointer replacement. | working-tree | RWANG |
| 1.0.0b | 2026-09-08 | beta | Recorded the separate TEST worker, MSP-only relay, ordered physical execution, six-lane evidence and receipt-bound atomic publication. | working-tree | RWANG |

## Reference version diff — 2026-09-08

"1.0.2b → 1.0.3b: follow zuri's pre-merge ADR-071 → ADR-073 collision repair because published main owns ADR-071 for CRM. Historical revision rows and pinned acceptance reports retain their original identifiers. Protocol and runtime behavior are unchanged.
