---
title: "GenesisRAG17 TEST pipeline flow"
doc_id: "FLOW-GENESISRAG17-PIPELINE"
status: beta
version: "1.0.2b"
updated: "2026-09-08"
owner: "GenesisBlockDB Architecture"
source_of_truth: true
attributes:
  domain: integration
  scope: "isolated GenesisRAG17 TEST execution"
related_docs:
  - "docs/ADR--GENESISRAG17-SEPARATE-WORKER-PUBLICATION.md"
  - "docs/GENESISRAG17-EXTENSION-MAP.md"
  - "genesisrag17-worker/README.md"
---

# GenesisRAG17 TEST pipeline flow

This document describes the executable cross-repository flow for the isolated
GenesisRAG17 TEST worker. The product [GenesisRAG17 architecture decision
ADR-071](https://github.com/Freshair129/zuri.ai/blob/codex/ki17-integration/docs/decisions/ADR-071-GENESISRAG17-ISOLATED-EXECUTION-AND-PUBLICATION.md)
sets the profile; the [17-stage source
specification](https://github.com/Freshair129/zuri.ai/blob/codex/ki17-integration/docs/KNOWLEDGE-INGESTION-17-STAGE-SPEC.md)
defines the logical pipeline. The [source execution
flow](https://github.com/Freshair129/zuri.ai/blob/codex/ki17-integration/docs/KNOWLEDGE-INGESTION-17-STAGE-FLOW.md)
and [wire contract](https://github.com/Freshair129/zuri.ai/blob/codex/ki17-integration/docs/plans/GENESISRAG17-CONTRACT.md)
are the cross-repository authorities. The [pinned historical acceptance
report](https://github.com/Freshair129/zuri.ai/blob/b64b46df057d3160c659afa3c34628ee86520257/.brain/reports/GENESISRAG17-ACCEPTANCE.md)
is evidence for the synthetic isolated run, not a production quality claim.
The current approved implementation amendment is contract `1.3.0b`; the wire
schema remains `genesisrag17.v1`.

## Boundary and ownership

The logical stage number and the physical executor are deliberately separate.
GKS creates and governs the semantic decision, while the GenesisBlock worker
proves the physical write. MSP authenticates and relays every cross-repository
call; it owns no stage and no content store.

| Boundary | Owns | Does not own |
|---|---|---|
| Tier 1 zuri source | Raw, parsed and chunk versions; source mentions; run/attempt ledger; evidence cursor; source finish guard; Stages 1–8 | Canonical fact payload, native graph/vector/index files |
| Tier 2 MSP | Scope/role authentication, relay, retry-safe tool boundary and worker query proxy | Any pipeline stage, GKS decision, source content store or cursor |
| Tier 3 GKS | Stages 9–12 semantic decisions, canonical facts/ontology/temporal meaning, Stage 14 enrichment and Stage 17 quality authority | Native store access or outbound calls to GenesisBlock |
| Tier 4 GenesisBlock worker | Physical Stage 13 graph write, Stage 15 embedding, Stage 16 six-lane index/readback, candidate publication and query | GKS authority, source ledger, direct GKS/Edge access |

## End-to-end sequence

```mermaid
sequenceDiagram
    participant Z as zuri Tier 1 source
    participant M as MSP authenticated relay
    participant G as GKS passive authority
    participant W as GenesisBlock Tier 4 worker
    participant Q as Query client

    Z->>Z: 1 receive -> 2 parse -> 3 provenance -> 4 normalize
    Z->>Z: 5 classify -> 6 version/dedupe -> 7 chunk -> 8 mentions
    Z->>M: msp_pipeline_submit(batch, source grant)
    M->>G: authenticated submit
    G-->>M: batchId, decisionId, status
    M-->>Z: batch acknowledgement
    W->>M: msp_pipeline_claim(worker grant)
    M->>G: authenticated claim
    G->>G: 9 entity resolve -> 10 fact extract
    G->>G: 11 ontology map -> 12 temporal map
    G-->>M: immutable decision, decisionHash
    M-->>W: scoped decision
    W->>W: 13 fsynced intent + graph-only native transaction, flush, checkpoint, readback
    W->>M: msp_pipeline_graph_receipt
    M->>G: gks_pipeline_graph_receipt
    G->>G: verify physical 13; terminal 13; run enrich_v1 14
    G-->>M: graphReceiptHash, derived, derivedHash
    M-->>W: scoped enrichment result
    W->>W: 15 checkpoint new collection manifest; real CPU embeddings
    W->>W: 16 exact-intent derived + vectors transaction; lexical index/readback/benchmark
    W->>M: msp_pipeline_write_receipt
    M->>G: gks_pipeline_write_receipt
    W->>M: msp_pipeline_gate
    M->>G: evaluate Stage 17 dimensions and policy
    G-->>M: verdict, allowPublication
    M-->>W: bound gate verdict
    alt gate and policy allow
        W->>W: prepare snapshot; atomically replace pointer/history
        W->>M: msp_pipeline_publication_receipt
        M->>G: gks_pipeline_publication_receipt
        G-->>M: Stage 17 success evidence
    else gate failure or policy denial
        G-->>M: actual failed stage/attempt evidence
    end
    Z->>M: msp_pipeline_evidence(runId, afterCursor)
    M->>G: gks_pipeline_evidence
    G-->>M: exact terminal rows and publication receipt
    M-->>Z: evidence page
    Z->>Z: import evidence and finish or persist failure
    Q->>M: msp_pipeline_query(scope, query, topK, snapshotId?)
    M->>W: authenticated loopback /query
    W-->>M: one published generation + citations
    M-->>Q: scope-matching results
```

The important ordering is **physical Stage 13 receipt -> GKS Stage 14 -> real
Stage 15 -> Stage 16 readback -> GKS Stage 17 gate -> worker atomic publish ->
publication receipt -> source finish**. Stage 13 is not closed from decided
counts, and a gate response alone does not finish a run.

## Stage contract map

Every terminal row binds `runId`, `pipelineStageId`, `executionStepId` and
`attemptId`, and measures the six counters
`records_in`, `records_out`, `records_quarantined`, `error_count`,
`retry_count`, `duration_ms`. Delivery retries keep the same identity;
another real execution gets a new FR-071 attempt.

| Stage | Stable ID | Logical owner | TEST executor/output | Terminal evidence |
|---:|---|---|---|---|
| 1 | `DPS-KI-INGEST` | zuri | Persisted raw artifact and source receipt | Raw row and receipt |
| 2 | `DPS-KI-PARSE` | zuri | Versioned parsed text/structure | Parsed parent and parser version |
| 3 | `DPS-KI-PROVENANCE` | zuri | Source/parsed lineage digest | Parent IDs, hashes and span checks |
| 4 | `DPS-KI-NORMALIZE` | zuri | Canonical text measurement while retaining raw text | Raw and normalized hashes |
| 5 | `DPS-KI-CLASSIFY` | zuri/MSP | Scope and embedding/publication policy | Exact scope and policy flags |
| 6 | `DPS-KI-DEDUPE` | zuri | Version relationship evidence | Idempotent/revision classification |
| 7 | `DPS-KI-CHUNK` | zuri | Persisted exact-substring chunks | UTF-16 offsets and UTF-8 hashes |
| 8 | `DPS-KI-ENTITY-EXTRACT` | zuri | Typed source occurrences | Every occurrence has a source mention id |
| 9 | `DPS-KI-ENTITY-RESOLVE` | GKS | Canonical entities and occurrence links | Immutable decision identity |
| 10 | `DPS-KI-FACT-EXTRACT` | GKS | `rule_v1` candidates and held rows | Confidence, predicate and source references |
| 11 | `DPS-KI-ONTOLOGY-MAP` | GKS | `ontology_v1` verified facts or held rows | Endpoint/type validation |
| 12 | `DPS-KI-TEMPORAL-MAP` | GKS/MSP | Temporal metadata with explicit applicability | Valid-time semantics or `not_applicable` |
| 13 | `DPS-KI-GRAPH-BUILD` | GKS decision + worker physical write | Pre-commit intent with expected frontier; graph-only native transaction and graph receipt | Actual native node/edge readback; closes before 14 |
| 14 | `DPS-KI-ENRICH` | GKS | `enrich_v1` derived summaries and `derivedHash` | Separate immutable derived result |
| 15 | `DPS-KI-EMBED` | worker | Checkpointed native collection manifest; CPU E5 vectors, 384 dimensions | Verified model artifacts and vector count |
| 16 | `DPS-KI-INDEX` | worker | Checkpointed collection, exact-intent final transaction, Stage 16 lexical index, six-lane manifest and readback | Native transaction/frontier, lane evidence, benchmark |
| 17 | `DPS-KI-QUALITY-GATE` | GKS + worker publication | Five-dimension verdict, pointer and receipt | Successful only after matching publication receipt |

For the physical acknowledgement, `msp_pipeline_graph_receipt` contains the
Stage 13 transaction/frontier, actual readback and six metrics. GKS returns
`graphReceiptHash`, `derived` and `derivedHash`; the worker does not mutate the
original decision or its `decisionHash`. The final
`msp_pipeline_write_receipt` adds those hashes and execution times for 13, 15
and 16. The receipt also contains actual readback, six-lane manifest, measured
metrics and the frozen fixture benchmark.

Before either native commit, the worker writes and fsyncs the complete intent
with the exact serialized native payload, transaction id and `expected_frontier`.
The filenames are fixed under the isolated database root:

| Native phase | Exact intent filename |
|---|---|
| Graph | `genesisrag17/transactions/graph-<safeDecisionId>.json` |
| Final | `genesisrag17/transactions/final-<safeDecisionId>.json` |

`<safeDecisionId>` is the worker's sanitized filename component for the
decision id; it does not change the decision identity or hash.

The intent remains until the matching receipt is accepted and local state is
durable. A retry reads that file and passes its unchanged payload and identity
to the native commit; it never creates a second transaction payload for the
same phase.

## Lifecycle, failure and retry

The worker's durable state and outboxes make delivery and execution distinct.
The following states are observable in the isolated implementation:

| State | Meaning | Next durable action |
|---|---|---|
| `PENDING` / `CLAIMED` | Decision is available or owned by a worker attempt | Claim or resume the same attempt |
| `GRAPH_COMMITTED` | Graph-only native transaction is durable | Send/replay graph receipt |
| `GRAPH_RECEIPT_ACCEPTED` | GKS closed 13 and returned 14 derived data | Embed and materialize final candidate |
| `WRITE_RECEIPT_PENDING` | Stage 15/16 physical write exists but receipt delivery is incomplete | Replay the exact write receipt |
| `GATE_RECEIVED` | GKS returned a bound Stage 17 verdict | Publish only if verdict and policy allow |
| `PREPARED` | Candidate snapshot exists, but pointer does not publish it | Keep it invisible; resume publication decision |
| `PUBLICATION_RECEIPT_PENDING` | Pointer switched, receipt delivery is incomplete | Replay the exact publication receipt |
| `PUBLISHED` | Matching publication receipt accepted by GKS | Source may finish after evidence import |
| `FAILED` | Actual stage failure or quality/policy denial is terminal | Preserve failure evidence; no downstream success |

`msp_pipeline_stage_failure` records only the materialized failing stage
(worker-owned physical stages 13, 15 or 16). Embedding policy denial is a
Stage 15 failure and never a zero-vector success. A quality gate failure does
not produce a publication receipt. A lost reply retries the original outbox
payload; it does not create a different terminal attempt. When GKS accepts the
graph receipt, the worker persists the accepted `derived` result and graph
receipt in `state.json` before removing `graph-receipt-<safeDecisionId>.json` (and
the graph intent). `retryOutbox` follows the same ordering after a crash, so a
remote acceptance cannot be mistaken for durable local state.

## Publication and query visibility

The worker writes a prepared snapshot before it changes the pointer. It fsyncs
the temporary file and performs an operating-system atomic replacement, retrying
transient Windows `EPERM`/sharing failures. If replacement cannot complete, the
old pointer remains in place; the worker never renames the old pointer away as a
fallback. The authoritative pointer contains the current generation and the
retained `publishedSnapshotIds` history. A query reads that pointer once,
validates the requested snapshot's membership and binds one generation for the
complete query. A known prepared filename is rejected while it is outside
published history. Previous published generations remain queryable with their
original citation hashes.

The query path is:

```text
source/client -> MSP scope and role check -> worker loopback /query
              -> one pointer read -> per-generation vector + FTS5 + graph/SQLite
              -> persisted source/chunk citation -> MSP scope-checked response
```

`GENESIS_WORKER_QUERY_TOKEN` authenticates the loopback endpoint. It is
separate from the MSP pipeline credential. `snapshotId` is optional for the
current published pointer and required to select a retained historical
generation. No GKS query or direct database client is involved.

## Recovery checkpoints

| Fault point | Required visible result on restart |
|---|---|
| Native graph commit before graph-receipt response (`after-native-commit-before-receipt`, `phase=graph`) | `genesisrag17/transactions/graph-<safeDecisionId>.json` supplies the exact payload and expected frontier; restart recognizes the committed graph, reuses the same transaction id/payload and replays the graph receipt; Stage 13/14 are not duplicated |
| Final native commit before write-receipt response (`after-native-commit-before-receipt`, `phase=final`) | `genesisrag17/transactions/final-<safeDecisionId>.json` supplies the exact payload and expected frontier; the collection manifest was checkpointed before the first vector commit, so restart recovers the native final state and reuses the exact write receipt without a blind recommit |
| Graph receipt accepted before local derived state (`after-graph-receipt-accepted-before-local-state`) | The accepted graph outbox remains present; replay validates the same receipt/derived hash, saves local state first, then removes the outbox and graph intent |
| Before pointer replacement | Old published pointer remains authoritative; prepared candidate is rejected by query |
| After pointer replacement before publication outbox delivery | New pointer/history remains authoritative; exact durable publication receipt is replayed with stable timestamps/hash |
| Lost MSP reply or child-process restart | MSP request timeout kills/restarts the relay child and retries the same idempotency identity |
| Source evidence cursor interruption | Tier 1 re-reads the page and advances the cursor only after durable ledger import |

The worker exposes the two native commit and graph acknowledgement hooks named
above at the actual durability boundaries. Publication hooks are
`before-pointer-replacement` and
`after-pointer-replacement-before-publication-outbox`; process-kill acceptance
uses exit code `86` and verifies the old-pointer/new-pointer result at each
boundary.

The pinned native binding has no `close` method. The worker therefore releases a
true native store handle by stopping its dedicated child process; its lock
refuses a second live owner and does not delete a live owner's lock. `start`,
`stop` and `resume` are explicit loops and do not install an OS scheduler.

## Six-lane manifest

Stage 16 reports all six lanes with a numeric measured `objects` count. The
current TEST interpretation is:

| Lane | Required status | Evidence |
|---|---|---|
| Vector | `ready` | Native HNSW write, flush, per-scope/generation search and model/artifact verification |
| Lexical | `ready` | Stage 16 worker-owned SQLite FTS5 indexing and query; Stage 13 writes no lexical rows; manifest implementation is `worker_sqlite_fts5` |
| Graph | `ready` | Native graph commit, flush/checkpoint and node/edge readback |
| SQLite | `ready` | Engine-owned projection SQL readback; callers never open the file |
| Bitemporal | `ready` when valid-time Query IR is actually read back; `not_applicable` for fixture facts explicitly lacking valid time | Transaction timestamps alone are not valid-time evidence |
| Provenance | `ready` | Source/chunk/citation IDs, hashes, UTF-16 offsets and persisted lineage resolve after restart |

Required lanes marked `unsupported` fail the gate. `not_applicable` is an
explicit input applicability result, not a hidden implementation fallback.

## Extension pointers

Use [GENESISRAG17-EXTENSION-MAP.md](GENESISRAG17-EXTENSION-MAP.md) to choose a
stage before designing a future feature. Any extension must preserve stable
stage identity, exact scope, decision/receipt hashes, idempotency and the MSP
relay. A new semantic vocabulary belongs in GKS; a new physical capability
requires a lane-manifest and receipt change; a new model or dimension requires
a new pinned artifact set and vector generation. Do not add a logical Stage 18
for query answering without first updating the source specification.

## Explicit limits

This flow is limited to the synthetic corpus, one document per run, private
scope, rule-based extraction, local CPU embedding, separate TEST stores and
the pinned runtime/model. It does not claim production migration or deployment,
LLM extraction, new UI, multiple concurrent source ingestion, a native lexical
API, or a production temporal index.

## Changelog

| Version | Date | Status | Summary | Commit Hash | Agent |
|---|---|---|---|---|---|
| 1.0.2b | 2026-09-08 | beta | Reconciled the live zuri GenesisRAG17 architecture reference to ADR-071 after the identifier collision; retained the pinned historical acceptance report. | working-tree | RWANG |
| 1.0.1b | 2026-09-08 | beta | Synced audit remediation: pre-commit native intents and collection checkpoint recovery, graph accepted-state ordering, Stage 16 lexical indexing, PASS-only publication and no-fallback pointer replacement. | working-tree | RWANG |
| 1.0.0b | 2026-09-08 | beta | Added the stage-by-stage ownership, physical 13 -> 14 -> 15 -> 16 -> 17 sequence, receipt lifecycle, visibility rules and recovery flow. | working-tree | RWANG |
