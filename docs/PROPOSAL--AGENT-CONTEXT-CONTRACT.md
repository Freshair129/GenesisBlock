---
version: "0.2.0b"
created_at: "2026-07-20T18:52:13+07:00,ATHER,working-tree"
last_update: "2026-08-14T04:01:51+07:00,ATHER"
status: superseded
superseded_by: "SPEC--GENESISDB-TYPED-QUERY-IR-V1.md"
attributes:
  domain: "agent-context"
  scope: "GRL typed API, SDK, and MCP contract"
  complexity: "C-2"
  risk: "MEDIUM"
---

# Proposal - Agent Context Contract

> Superseded on 2026-08-14 by the accepted Typed Query IR V1 specification. Its hard-budget,
> relevant-compression and evidence requirements remain inputs to the `context` operation design.

## Decision request

Separate agent-context assembly from HQL language development.

The context capability should be a typed engine contract exposed through Rust, NAPI,
REST, SDKs, and MCP. The existing HQL `CONTEXT` command remains as a compatibility
wrapper, but no new HQL grammar is justified by context-packet needs.

This proposal supersedes the earlier working-tree direction that defined HQL itself as
an agent-context language. HQL's separate market-gap proposal is
`PROPOSAL--HQL-CAUSAL-SEMANTIC-EVOLUTION.md`.

[ASSUMPTIONS]

1. The existing HQL `CONTEXT` command remains temporarily for compatibility rather than
   being removed in the first implementation slice.
2. Agent callers benefit more from a versioned typed packet than from additional query
   syntax.
3. Final transport field names and tokenizer policy remain approval-gated follow-up
   decisions.

## 1. Boundary

Agent context owns:

- target/query resolution;
- semantic seeding and bounded graph expansion;
- token/byte budgeting and compression;
- evidence packaging for agent consumption;
- MCP and SDK ergonomics;
- context-specific ranking defaults.

HQL owns:

- general graph/vector query semantics;
- temporal and historical state selection;
- semantic evolution and drift queries;
- causal event and distributed-conflict queries;
- model-space correctness.

The context layer may call HQL or lower-level engine methods, but HQL must not inherit
agent-only concepts such as prompt budgets, packet compression, or reasoning-path text.

## 2. Proposed typed contract

```rust
pub struct ContextRequest {
    pub target_id: Option<String>,
    pub query_vector: Option<Vec<f64>>,
    pub collection: Option<String>,
    pub tier: ScalingTier,
    pub rels: Vec<String>,
    pub direction: Direction,
    pub valid_at: Option<String>,
    pub max_bytes: u32,
    pub evidence: EvidenceLevel,
}

pub struct ContextPacketV1 {
    pub contract_version: u16,
    pub nodes: Vec<NodeOutput>,
    pub edges: Vec<EdgeOutput>,
    pub evidence: Vec<ContextEvidence>,
    pub budget: BudgetReport,
    pub omissions: OmissionReport,
    pub index_lag: u32,
}
```

The final field names remain design-gated. The important decision is that this is a
versioned typed API, not an expanding query-language grammar.

## 3. Requirements

### C1 - Hard budget

WHEN a byte budget is supplied THEN the serialized packet SHALL fit the budget or
return an explicit envelope-too-small error. It SHALL NOT silently overflow.

### C2 - Relevant compression

WHEN atom-level results exceed the budget THEN compression SHALL select only relevant
SuperNodes. It SHALL NOT return every SuperNode in the database.

### C3 - Evidence

WHEN a node is returned THEN its context evidence SHALL identify selection reason,
supporting path, temporal validity, and `caused_by` where present.

### C4 - Honest index state

WHEN semantic search participates THEN the packet SHALL report index lag. A stable mode
may flush the index before retrieval; an eventual mode must disclose its lag.

### C5 - HQL separation

WHEN the context contract evolves THEN additions SHALL land in typed request/response
types and public APIs. New HQL keywords require an independent HQL use case.

## 4. Current gaps

- `retrieve_context` performs exact/fuzzy target BFS but no semantic seed fusion.
- The budget is `props` character count divided by four, not a hard packet limit.
- Budget overflow clears atoms/edges and returns all global SuperNodes.
- `reasoning_path` is an unstructured string rather than machine-verifiable evidence.
- Surface behavior is not yet versioned as a packet contract.

## 5. Implementation sequence after approval

1. Freeze `ContextRequest` and `ContextPacketV1` in a normative spec.
2. Add failing GRL tests for hard budget, relevant compression, and evidence.
3. Implement typed core behavior without changing HQL grammar.
4. Wire NAPI/REST/TypeScript/Python/Go/MCP parity.
5. Keep HQL `CONTEXT` as a compatibility adapter over the typed API.

## 6. Non-goals

- Query-language differentiation.
- General graph pattern matching.
- Historical vector indexing or peer conflict queries.
- Exact model-token counting without a named tokenizer.

## 7. Risk

**Risk: MEDIUM.** The contract crosses public surfaces but does not change storage or
query-language architecture. The main risk is packet-version drift between transports.

## Version diff

| From | To | Change |
|---|---|---|
| none | `0.1.0b` | Extracted agent-context assembly from HQL into a typed GRL/API track. |

## CHANGELOG

| Version | Date | Status | Summary | Commit Hash | Agent |
|---|---|---|---|---|---|
| `0.1.0b` | 2026-07-20 | candidate | Initial separated agent-context contract proposal; no implementation authorized. | working-tree | ATHER |
| `0.2.0b` | 2026-08-14 | superseded | Replaced by the accepted Typed Query IR V1 contract and agent-adapter boundary. | working-tree | ATHER |
