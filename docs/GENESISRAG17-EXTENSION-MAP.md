---
title: "GenesisRAG17 extension map"
doc_id: "MAP-GENESISRAG17-EXTENSIONS"
status: beta
version: "1.0.1b"
updated: "2026-09-08"
owner: "GenesisBlockDB Architecture"
source_of_truth: true
attributes:
  domain: integration
  scope: "GenesisRAG17 isolated TEST pipeline"
related_docs:
  - "docs/ADR--GENESISRAG17-SEPARATE-WORKER-PUBLICATION.md"
  - "docs/FLOW--GENESISRAG17-PIPELINE.md"
  - "genesisrag17-worker/README.md"
---

# GenesisRAG17 extension map

Use this map before extending a stage. It separates the logical seventeen-stage
product contract from the current physical implementation and identifies the
documents and evidence that must move together. The [17-stage source
specification](https://github.com/Freshair129/zuri.ai/blob/codex/ki17-integration/docs/KNOWLEDGE-INGESTION-17-STAGE-SPEC.md)
remains the product-level authority. The [17-stage execution
flow](https://github.com/Freshair129/zuri.ai/blob/codex/ki17-integration/docs/KNOWLEDGE-INGESTION-17-STAGE-FLOW.md)
is the cross-repository flow reference.

## Ownership map

The GenesisBlock worker owns physical substrate work only. GKS remains the
passive semantic and quality authority reached through MSP. Tier 1 owns source
lineage and finish evidence. A future extension must keep this ownership split
unless a new architecture decision explicitly changes it.

| Stage | Stable ID | Current owner / executor | Current TEST seam | What a safe extension must update |
|---:|---|---|---|---|
| 1 | `DPS-KI-INGEST` | zuri source | Raw receipt and immutable source version | Source connector contract, scope grant, raw hash fixture and recovery evidence |
| 2 | `DPS-KI-PARSE` | zuri source | Versioned exact parsed text/structure | Parser version, raw-to-parsed lineage, format-specific error policy and offsets |
| 3 | `DPS-KI-PROVENANCE` | zuri source | Parent IDs and source/parsed digests | Provenance schema, citation resolution after restart and all downstream receipt references |
| 4 | `DPS-KI-NORMALIZE` | zuri source | Canonical measurement while raw text remains authoritative | Normalization rules, hash semantics and chunk mapping; never replace raw evidence silently |
| 5 | `DPS-KI-CLASSIFY` | zuri + MSP | Exact six-field private scope and embedding/publication flags | MSP grants, policy propagation to stages 15/17/query and security negative cases |
| 6 | `DPS-KI-DEDUPE` | zuri source | Immutable version/revision classification | New FR-071 attempt, snapshot lineage, correction semantics and old citation retention |
| 7 | `DPS-KI-CHUNK` | zuri source | Persisted exact-substring chunks with UTF-16 offsets | Chunk identity/version, parser-specific anchors, mention offsets and benchmark fixture |
| 8 | `DPS-KI-ENTITY-EXTRACT` | zuri source | All typed occurrences with distinct `sourceMentionId` | Recognizer coverage, occurrence payload, resolution-key behavior and scope tests |
| 9 | `DPS-KI-ENTITY-RESOLVE` | GKS | Immutable canonical decision and `decisionHash` | GKS policy/identity contract, MSP schema, endpoint references and replay evidence |
| 10 | `DPS-KI-FACT-EXTRACT` | GKS | `rule_v1`, explicit/structured candidates and HELD rows | Extractor version, confidence floor, predicate provenance and negative/HELD fixtures; worker stays LLM-free |
| 11 | `DPS-KI-ONTOLOGY-MAP` | GKS | `ontology_v1` aliases and endpoint validation | Ontology version, endpoint schema, verified/held distinction, decision hash and gate reasons |
| 12 | `DPS-KI-TEMPORAL-MAP` | GKS (parity against pinned MSP source) | Mapped valid time or explicit `not_applicable` | Temporal semantics, parity fixture, native temporal readback capability and applicability manifest |
| 13 | `DPS-KI-GRAPH-BUILD` | GKS decision + worker physical write | Fsynced `genesisrag17/transactions/graph-<safeDecisionId>.json` intent with expected frontier; graph-only native transaction, flush/checkpoint, readback and graph receipt | Node/edge classes, physical count formula, graph receipt schema, idempotent frontier and provenance |
| 14 | `DPS-KI-ENRICH` | GKS | `enrich_v1` derived objects with `derivedHash` | Derived schema, source-reference arrays, enrichment counts, graph receipt response and final receipt |
| 15 | `DPS-KI-EMBED` | worker | Checkpointed native collection manifest; real CPU E5 embeddings, 384 dimensions, pinned artifacts | New model revision/dimension, artifact hashes, generation/collection identity, policy denial and benchmark |
| 16 | `DPS-KI-INDEX` | worker | Checkpointed vector collection; fsynced `genesisrag17/transactions/final-<safeDecisionId>.json` intent; native graph/vector/SQLite readback plus Stage 16 worker FTS5 | Lane capability, manifest status/reason/objects, per-generation retrieval, transaction frontier and readback |
| 17 | `DPS-KI-QUALITY-GATE` | GKS authority + worker publication | PASS-only policy result, atomic pointer/history replacement and publication receipt | Thresholds, gate evidence, `allowPublication` policy, historical visibility and source finish guard |

The post-stage publication step is part of the current Stage 17 completion
contract even though it has no new logical stage number:

| Post-stage boundary | Current owner | Extension seam |
|---|---|---|
| Candidate snapshot and pointer | worker | Fsynced prepared snapshot plus atomic pointer/history replacement, transient `EPERM` retry, no rename-away fallback, prepared-snapshot rejection and one-generation query binding |
| Publication receipt | worker -> MSP -> GKS | Exact receipt hash, snapshot/generation, model revision, transaction frontier and stable replay timestamps |
| Source finish | zuri source | Evidence import and cursor transaction; successful finish requires all 17 terminal successes plus publication receipt |
| Query | client -> MSP -> worker | Scope, loopback bearer, published-history membership, per-generation six-lane fusion and citations |

The worker's storage and recovery paths use the exact intent filenames shown in
the Stage 13 and Stage 16 rows. Each file retains the exact serialized native
payload, transaction id and `expected_frontier` until its matching receipt and
local state are durable. After graph-receipt acceptance, the worker saves the
accepted receipt and `derived` result before removing the graph outbox or graph
intent; `retryOutbox` follows that order. A newly created vector collection is
checkpointed with `saveState` before its first vector transaction, preserving
model, dimension and metric metadata through WAL replay. Stage 13 writes no
lexical rows; worker FTS5 indexing belongs to Stage 16.

In those filenames, `<safeDecisionId>` is the worker's sanitized filename
component for the decision id; the decision identity and hash remain unchanged.

Publication uses a temp-file fsync and operating-system atomic replacement. If
replacement fails after transient retries, the old pointer remains
authoritative. Historical snapshot files and their citations remain available
after later generations publish.

## Extension protocol

1. Start with the stage row and identify its owning authority. Do not place
   semantic decisions in the worker or physical writes in GKS.
2. Read the [separate-worker/publication ADR](ADR--GENESISRAG17-SEPARATE-WORKER-PUBLICATION.md),
   the [execution flow](FLOW--GENESISRAG17-PIPELINE.md) and the frozen wire
   contract before changing a message.
3. Preserve `schemaVersion: genesisrag17.v1`, stable stage IDs, exact six-field
   scope, stage identity `{runId,pipelineStageId,executionStepId,attemptId}`,
   decision/receipt hashes and idempotency. A meaning change needs an additive
   contract version and coordinated owner review; a new field must not silently
   change an old field's meaning.
4. Update the source specification, execution flow, ADR, this map, the worker
   README and the owning repository's contract/tests together. Add a fixture
   that contains expected results independently of the implementation when the
   change affects retrieval or citations.
5. Keep actual evidence separate from decided counts. New physical lanes must
   report real readback and a truthful `ready`, `not_applicable` or
   `unsupported` status. `laneManifest.objects` remains a measured numeric
   count.
6. Run the isolated acceptance path with real native writes, CPU/model checks,
   restart/reply-loss cases and scope/security negatives. Record exact commits,
   runtime versions and artifact hashes. Do not turn a TEST result into a
   production claim.

## Examples for future design

### New source formats and evidence

PDF, OCR, HTML, tables and page/cell citations extend Stages 2, 3, 7 and 8
together. The new parser must retain a recoverable raw artifact, define exact
offset/anchor semantics, carry source references through GKS and keep query
citations resolvable after restart. Adding a parser without updating provenance
and chunk fixtures is incomplete.

### New semantic extraction and ontology

New relation patterns begin at Stage 10, then update Stage 11 endpoint rules,
Stage 12 temporal handling, Stage 13 graph projection and Stage 17 gate
evidence. A new extractor version or an LLM-assisted profile must be explicit
in the GKS contract and policy. The current worker does not run extraction and
must not silently grow an LLM path.

### New temporal capability

A native temporal Query IR index may replace the current optional/readback
boundary only after capability reporting, persisted valid-time fixtures, parity
with the pinned MSP temporal implementation and gate/readback changes. JSON
properties or transaction timestamps alone do not prove the lane is ready.

### New embedding model or dimension

Treat a model or dimension change as a new physical generation and artifact
manifest. Pin the upstream revision, verify every artifact hash, use a separate
vector collection, rerun the frozen benchmark and bind the model revision to
write/publication receipts. Never overwrite the current collection or fall back
to a different model when the pinned artifact is unavailable.

### New retrieval lane or native lexical support

If a future native API supplies lexical search, add a capability and parity
contract, then compare it with the current worker-owned
`implementation: worker_sqlite_fts5` path. Change the lane manifest only after
actual native query/readback evidence. A new lane or a replacement must remain
scoped by tenant and generation; global top-k followed by post-filtering cannot
establish complete historical recall.

### New query or answer layer

Reranking, context assembly and answer generation extend the published query
path after Stage 17. They must read one published generation, preserve scope
and citations, and define their own evaluation contract. They do not become a
logical Stage 18 merely by being added to a client.

## Boundaries that require a new ADR

The following are outside the current TEST baseline and cannot be introduced
by editing only the worker README or a lane flag:

- multiple concurrent source documents or cross-run batching;
- production deployment, shared stores or migrations;
- direct GKS, MSP database or Edge store access from the worker;
- caller-managed SQLite/graph/vector stores or a second durability authority;
- changing source/worker grants or relay credential ownership;
- automatic scheduler ownership;
- publication semantics other than one atomic pointer and retained history;
- a new logical stage, including any proposed Stage 18;
- replacing rule-based extraction with LLM extraction;
- claiming native lexical or temporal readiness without an actual API and
  readback proof.

## Version and evidence pins

- Native engine checkout: `e15e35b0093394e0a8880af7f4e6f63cf81223b7`.
- Embedding: `intfloat/multilingual-e5-small`, revision
  `614241f622f53c4eeff9890bdc4f31cfecc418b3`, dimension 384, cosine.
- Wire: `genesisrag17.v1`, contract `1.3.0b`.
- Historical isolated acceptance: [pinned report](https://github.com/Freshair129/zuri.ai/blob/b64b46df057d3160c659afa3c34628ee86520257/.brain/reports/GENESISRAG17-ACCEPTANCE.md).

## Changelog

| Version | Date | Status | Summary | Commit Hash | Agent |
|---|---|---|---|---|---|
| 1.0.1b | 2026-09-08 | beta | Synced audit remediation: exact native intent filenames/frontiers and collection checkpoint recovery, graph accepted-state ordering, Stage 16 lexical indexing, PASS-only publication and no-fallback pointer replacement. | working-tree | RWANG |
| 1.0.0b | 2026-09-08 | beta | Added stage ownership, safe extension seams, coordinated evidence requirements and boundaries requiring a new ADR. | working-tree | RWANG |
