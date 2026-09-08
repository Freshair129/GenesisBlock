---
doc_id: SPEC--WAVE-E-QUERY-CONTEXT-CLIENT-CAPABILITY
owner: GenesisBlockDB Engineering
version: "0.1.0b"
created_at: "2026-09-08T21:34:45+07:00,ATHER"
last_update: "2026-09-08T21:44:00+07:00,ATHER"
status: beta
superseded_by: null
attributes:
  domain: query-ir-graphrag-clients
  scope: wave-e-r10-r12
  complexity: C-3
  risk: MEDIUM-HIGH
---

# Wave E — Typed context contract and client capability conformance

## 1. Decision requested

Wave E follows the verified Wave D checkpoint `6e8d4ea` on
`codex/wave-c-query-correctness`. It proposes a small, evidence-gated slice for
R-10 and R-12:

1. close the planned Query IR `context` operation over the existing bounded
   Graph Retrieval Layer (GRL) packet;
2. disclose the exact filtered-hybrid and context limits instead of implying
   unsupported GraphRAG behavior; and
3. make the Python and Go clients, FFI/JNI callers, N-API, REST and MCP
   surfaces prove the same capability and error semantics.

This document was approved for local implementation on
`codex/wave-e-client-query-contracts`. It authorizes the scoped local code and
verification below, but no merge, push, release or deployment. The current main
worktree contains unrelated dirty security-audit files; implementation remains
isolated in the Wave E worktree.

Complexity is **C-3** because one public query contract crosses the Rust core,
N-API, REST, FFI/JNI and client adapters. Risk is **MEDIUM-HIGH**: a partial
surface can cause callers to believe they received a filtered or temporally
complete GraphRAG result when the engine did not provide that guarantee.

## 2. Assumptions and success criteria

### Assumptions

1. Wave D query budgets and admission controls remain the shared resource
   boundary for context and search requests.
2. The existing `Storage::retrieve_context` packet and `CoverageReport` are the
   first GraphRAG context implementation; this wave does not invent a second
   context assembler.
3. Filter-aware HNSW pruning and lexical BM25+dense fusion require workload
   evidence and remain fail-closed capabilities until a later decision.
4. SDK compatibility matters: existing Python and Go constructors and HQL
   methods remain usable without forcing a breaking migration.

### Success criteria

- A valid Query IR context request has one normalized response shape across the
  core, REST, N-API, FFI and JNI paths.
- Capability output says exactly which context, temporal, filter and lexical
  features are implemented, partial, planned or unsupported.
- Python, Go and MCP fixtures exercise success, typed rejection, API-key
  propagation, finite timeout behavior and budget/coverage metadata.
- The existing HQL context and `retrieveContext` compatibility surfaces keep
  their current behavior.
- No filtered hybrid or lexical claim is added without a measured workload
  artifact.

## 3. Parent and peer alignment

- Wave C established temporal visibility, collection correctness and filtered
  ANN eligibility.
- Wave D established shared query budgets, REST admission and per-index quality
  evidence. Context must consume the same budget vocabulary and expose
  truncation/coverage rather than hiding it.
- `SPEC--GENESISDB-TYPED-QUERY-IR-V1` reserves `context` but records it as
  planned. This wave turns only the target-id context slice into an implemented
  operation.
- `ADR--GENESISDB-TYPED-QUERY-IR-AGENT-BOUNDARY` keeps NL/model behavior
  outside the engine. No prompt or provider field is added here.
- The existing REST, N-API, FFI and JNI context methods remain compatibility
  fronts over one core implementation.

## 4. Confirmed findings

### R-10 — Context and filtered hybrid semantics are not fully contracted

- Query IR currently implements `search` and `traverse`; `context` is still
  reported as `planned`.
- `Storage::retrieve_context` already returns nodes, edges, super-nodes,
  `token_estimate`, `reasoning_path` and `CoverageReport`, but that packet is
  not available through Query IR.
- HQL `MATCH ... SIMILAR` is vector plus K-Impact blending. It is not a
  lexical BM25+dense fusion contract.
- Typed metadata predicates, vector candidate selection and temporal context
  are not one frozen pipeline. Adding a `filters` field now would risk claiming
  semantics that are only post-filter behavior.

### R-12 — Client and distribution behavior is incomplete

- Python has raw HQL/context helpers but no timeout, API-key header or typed
  Query IR method.
- Go has a fixed 30-second HTTP timeout and raw HQL/context helpers but no
  API-key option or typed Query IR method.
- N-API already exposes Query IR and context. FFI/JNI expose each separately,
  but cross-surface conformance is not covered by one fixture.
- MCP exposes HQL/context tools and does not expose the typed Query IR boundary.
- Source parity tests do not prove the client-visible response and error
  behavior.

## 5. Requirements

### E-01 — Typed context operation

1. **WHEN** a request has `contract_version: "query-ir.v1"`, a non-empty
   `request_id`, `operation.kind: "context"`, a valid `target_id`, a tier in
   `H0`–`H6`, and an optional positive token budget **THEN** the engine SHALL
   return `status: "ok"`, `operation_kind: "context"`, and the existing
   `ContextPackage` data including `coverage`.
2. **WHEN** context expansion reaches the tier boundary or token budget
   **THEN** the response SHALL preserve `coverage.ceiling_hit` or
   `coverage.truncated` and SHALL add a deterministic warning code in
   `meta.warnings` when compression occurred.
3. **IF** `target_id` is empty, the tier is invalid, or the budget is zero or
   otherwise invalid **THEN** the engine SHALL return a typed
   `QUERY_IR_VALIDATION_FAILED` error before graph work begins.
4. **IF** the target does not resolve **THEN** the engine SHALL return
   `QUERY_TARGET_NOT_FOUND` without returning a fabricated empty success.
5. **IF** a context request includes `query_vector`, namespace scope, or
   temporal selectors that the first slice does not implement **THEN** the
   engine SHALL return `QUERY_CAPABILITY_UNSUPPORTED` and SHALL not silently
   drop the field.

### E-02 — Capability disclosure and filtered-hybrid gate

1. **WHEN** `/v1/query/ir/capabilities`, N-API, FFI or JNI capabilities are
   read **THEN** `context` SHALL be reported as `implemented` with its accepted
   input and output limits.
2. **WHEN** a caller inspects search capabilities **THEN** the manifest SHALL
   state that vector and K-Impact hybrid modes are implemented, lexical fusion
   is planned, and typed metadata predicates are unsupported in this slice.
3. **IF** a caller asks for a capability marked unsupported or planned **THEN**
   the request SHALL fail with `QUERY_CAPABILITY_UNSUPPORTED`; no post-filter
   reinterpretation or silent downgrade is allowed.
4. **WHEN** a representative workload artifact is absent **THEN** this wave
   SHALL not change HNSW defaults, quantizer parameters or recall floors.

### E-03 — Cross-surface conformance

1. **WHEN** the same canonical context, search, traverse and unsupported
   capability fixtures run through core, REST, N-API, FFI and JNI **THEN** the
   normalized envelope SHALL agree on operation kind, error code, budget
   accounting, coverage and deterministic fields.
2. **WHEN** a REST request carries the configured API key **THEN** Python and
   Go clients SHALL send `Authorization: Bearer <key>`; absent or wrong keys
   SHALL surface the server code and HTTP status.
3. **WHEN** a client request exceeds its configured timeout **THEN** Python and
   Go clients SHALL terminate the request and return a typed client error; the
   default timeout SHALL be finite and documented.
4. **WHEN** MCP invokes the typed query tool **THEN** it SHALL return structured
   content for success and a structured error for validation/capability failure;
   the existing `query_hql` and `retrieve_tiered_context` tools SHALL remain
   backward compatible.

### E-04 — Documentation and evidence

1. The API reference, Query IR spec and client capability matrix SHALL identify
   the implemented context slice and unsupported filter/lexical semantics.
2. Tests SHALL use a shared fixture set and record the exact engine/client
   versions under test.
3. Local verification SHALL distinguish implemented, locally tested,
   externally gated and production-unverified states.

## 6. Proposed design

### 6.1 Query IR context shape

The first slice is deliberately target-id based:

```json
{
  "contract_version": "query-ir.v1",
  "request_id": "ctx-001",
  "operation": {
    "kind": "context",
    "target_id": "entity:42",
    "tier": "H1",
    "budget": 512,
    "fuzzy": false
  }
}
```

The response keeps the existing packet as `data` and adds only the common
Query IR envelope:

```json
{
  "contract_version": "query-ir.v1",
  "request_id": "ctx-001",
  "status": "ok",
  "operation_kind": "context",
  "data": {
    "nodes": [],
    "edges": [],
    "super_nodes": [],
    "token_estimate": 0,
    "reasoning_path": "...",
    "coverage": {
      "hops_requested": 1,
      "hops_served": 1,
      "ceiling_hit": false,
      "truncated": false
    }
  },
  "meta": {
    "capability_version": "...",
    "index_lag": 0,
    "budget": {},
    "warnings": []
  }
}
```

The operation must call the existing context assembler under the Wave D
blocking/admission boundary. It must not duplicate GRL traversal or expose
embeddings in the packet.

### 6.2 Capability manifest

Keep the existing `operations` string map backward compatible and add a closed
`operation_details` object. The first implementation advertises:

| Capability | Status | Boundary |
|---|---|---|
| `search.vector` | implemented | per-collection vector search with Wave D budgets |
| `search.hybrid` | implemented | vector plus K-Impact blend; no lexical fusion |
| `search.filters` | unsupported | no typed metadata predicate in this slice |
| `search.lexical` | planned | requires a separate FTS5/RRF decision |
| `traverse` | implemented | bounded direction/relation traversal |
| `context.target_id` | implemented | H0–H6, token budget, coverage packet |
| `context.query_vector` | unsupported | target-id slice only |
| `context.temporal` | unsupported | current-view context only |
| `match_path` | planned | existing HQL compatibility does not imply typed parity |
| `relational_named_query` | planned | remains the relational contract slice |

### 6.3 Client adapters

- **Python:** preserve `GenesisClient(base_url=...)`; add optional finite
  `timeout` and `api_key`, a shared request helper, `execute_query_ir`, and
  `query_ir_capabilities`. Raise `QueryError` with `status`, `code` and
  `message` when the server returns the structured error envelope.
- **Go:** preserve `NewClient(baseURL)`; add `NewClientWithOptions` for timeout
  and API key, `ExecuteQueryIR`, `QueryIRCapabilities`, and a typed HTTP error
  carrying status/code/message. Existing context cancellation remains honored.
- **N-API/FFI/JNI:** route context through the same core `execute_query_ir`
  implementation and update declarations/generated parity checks.
- **MCP:** add one `query_ir` tool with a closed request schema. HQL tools stay
  compatibility paths and must not be used as a fallback for unsupported typed
  capabilities.

### 6.4 Fixture and verification layout

Use one JSON fixture directory with:

- valid context, search and traverse requests;
- invalid tier/zero budget/unknown target requests;
- unsupported filters, lexical mode and temporal context requests;
- expected normalized success/error fields;
- API-key and timeout cases for Python/Go;
- MCP structured-content/error cases.

The fixture runner may normalize nondeterministic IDs and timings, but it must
not normalize away status codes, error codes, coverage flags or budget fields.

## 7. Work plan

- [ ] 1. Freeze the Wave E fixture schema and capability manifest shape.
  - Add requirements and peer links to the API/Query IR docs.
  - _Requirements: E-02, E-04_
- [ ] 2. Add the core typed `context` operation and capability details.
  - Reuse `retrieve_context`; carry Wave D budget state and warnings.
  - Add focused RED/GREEN tests for validation, target errors, truncation and
    response serialization.
  - _Requirements: E-01, E-02_
- [ ] 3. Wire REST, N-API, FFI and JNI parity.
  - Preserve existing context methods and route errors through the common
    Query IR taxonomy.
  - _Requirements: E-01, E-03_
- [ ] 4. Harden Python and Go client transport contracts.
  - Add timeout/API-key options without breaking current constructors.
  - Add typed Query IR/capabilities methods and mocked-server tests.
  - _Requirements: E-03_
- [ ] 5. Add the MCP `query_ir` tool and shared consumer fixtures.
  - Verify structured success/errors and backward-compatible existing tools.
  - _Requirements: E-03, E-04_
- [ ] 6. Run the full verification gate and update evidence.
  - Rust focused/full tests, N-API/MCP, Python/Go tests, mobile/FFI checks,
    format/lint and fixture report.
  - _Requirements: E-04_

## 8. Explicit non-goals

- Filter-aware HNSW graph pruning, ACORN-style indexes or a new metadata index.
- BM25/FTS5, RRF or any lexical+dense score fusion.
- Full Cypher/GQL, branching/optional paths, aggregations or arbitrary SQL.
- Query-vector seeded GRL context or temporal context reconstruction.
- HNSW/quantizer tuning to make the six Wave D quality rows pass.
- NL interpretation, prompts, provider credentials or model selection in the
  engine.
- Production deployment, package publishing, merge to main or hosted acceptance.

## 9. Exit criteria

Wave E is complete only when:

1. E-01 through E-04 have focused regression evidence.
2. Capability output and public docs agree across all supported surfaces.
3. Python/Go/MCP fixtures prove auth, timeout, success and typed errors.
4. Full relevant verification passes, with failures recorded rather than hidden.
5. A workload artifact explicitly decides whether filtered hybrid work is
   ready for a later wave; no unsupported recall claim is promoted.

## CHANGELOG

| Version | Date | Status | Summary | Commit Hash | Agent |
|---|---|---|---|---|---|
| 0.1.0b | 2026-09-08 | beta | Approved typed context operation and cross-client capability conformance for R-10/R-12 | 6e8d4ea | ATHER |

Please review and approve this documentation. I will generate the code once approved.
