---
id: BLUEPRINT--DEEP-REASONING-RECALL
phase: 3
type: blueprint
status: draft
tier: process
source_type: axiomatic
vault_id: default
title: "BLUEPRINT — Deep Reasoning via Genesis Graph integration"
tags:
  - msp
  - ucf
  - graph
  - reasoning
  - blueprint
crosslinks: {"implements":["BLUEPRINT--GENESIS-GRAPH-INTEGRATION"],"references":["CONCEPT--GENESIS-GRAPH-BACKEND"]}
created_at: 2026-05-17T21:00:00+07:00
cluster: implementation_flow
role: "Implementation plan"
---

# BLUEPRINT — Deep Reasoning Recall

## Objective

Enhance the orchestrator's `recall()` logic to support multi-hop structural reasoning using the Genesis Graph (Cypher v0). This transitions retrieval from "find similar text" to "understand related knowledge".

## Geography

```
packages/msp/src/
└── orchestrator/
    └── retrieval/
        └── sources/
            └── graph.ts       # NEW — Genesis Graph reasoning source (T1)
```

## Tasks

### T1 — Implement Graph Source
- Create `packages/msp/src/orchestrator/retrieval/sources/graph.ts`.
- Implement `graphSource(opts)` using `GenesisGraphBackend.cypher()`.
- Define default reasoning query (e.g., "Find all stable atoms referenced by the candidate set within 3 hops").

### T2 — Wire into Fusion
- Add `graph` to `SourceName` in `retrieval/types.ts`.
- Update `recall()` in `retrieval/index.ts` to invoke `graphSource` in Phase B (replacing or augmenting the existing JSONL `backlinksSource`).
- Adjust RRF weights to prioritize structural hits.

### T3 — Genesis Runtime Integration
- Update `msp-genesis-exec` to use graph-aware recall for resolving dimension members.
- Enable automatic "dependency inclusion" for Genesis Blocks (e.g., if a Block uses an Algorithm, automatically recall the referenced Concept).

### T4 — Verification
- Create `packages/msp/test/orchestrator/graph-recall.test.ts`.
- Test case: Verify that recalling an ADR also pulls in its parent CONCEPT via graph traversal.
- Test case: Verify that 2-hop implementations are correctly surfaced.

## Connections
- `[[BLUEPRINT--GENESIS-GRAPH-INTEGRATION]]` — provides the underlying backend.
- `[[CONCEPT--GENESIS-GRAPH-BACKEND]]` — the intent for Cypher-based reasoning.
