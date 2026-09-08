---
title: "ADR: GenesisRAG17 separate worker and atomic publication"
doc_id: "ADR-GENESISRAG17-SEPARATE-WORKER-PUBLICATION"
status: beta
version: "1.0.2b"
updated: "2026-09-08"
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
decision ADR-071](https://github.com/Freshair129/zuri.ai/blob/codex/ki17-integration/docs/decisions/ADR-071-GENESISRAG17-ISOLATED-EXECUTION-AND-PUBLICATION.md),
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
| 1.0.2b | 2026-09-08 | beta | Reconciled the live zuri GenesisRAG17 architecture reference to ADR-071 after the identifier collision; retained the pinned historical acceptance report. | working-tree | RWANG |
| 1.0.1b | 2026-09-08 | beta | Synced audit remediation: exact pre-commit native intents and collection checkpoint recovery, accepted graph-state ordering, Stage 16 lexical indexing, PASS-only publication and no-fallback pointer replacement. | working-tree | RWANG |
| 1.0.0b | 2026-09-08 | beta | Recorded the separate TEST worker, MSP-only relay, ordered physical execution, six-lane evidence and receipt-bound atomic publication. | working-tree | RWANG |
