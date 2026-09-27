---
title: "ADR: Typed GKS MCP promotion for MSP"
status: "accepted"
version: "1.0.1"
updated: "2026-09-08"
owner: "Boss (CEO)"
---

# ADR: Typed GKS MCP promotion for MSP

> **Compatibility boundary:** This accepted ADR describes the legacy
> `gks_knowledge_promote` MCP compatibility path. It is separate from the
> current isolated GenesisRAG17 worker flow, which is documented in
> [ADR--GENESISRAG17-SEPARATE-WORKER-PUBLICATION.md](ADR--GENESISRAG17-SEPARATE-WORKER-PUBLICATION.md).
> The legacy tool does not define the GenesisRAG17 graph receipt, six-lane
> write receipt, Stage 17 gate or publication receipt.

## Decision

GenesisBlock adds `gks_knowledge_promote` for the MSP-only promotion path. It
uses a deterministic graph node ID derived from the idempotency key, stores the
source snapshot hash and provenance as properties, and returns structured MCP
content with a `gks:knowledge/` reference, matching source hash, and idempotent
flag.

The tool rejects a key that maps to a different source hash. It does not replace
`add_knowledge`, expose a direct GoVibe API, or create a second persistence
store. GenesisBlock's engine-owned graph and WAL remain the durable authority.

```mermaid
flowchart LR
  MSP["MSP only"] --> MCP["gks_knowledge_promote"] --> DB["GenesisBlockDB WAL graph"]
  DB --> MCP --> MSP
```

## Verification

Cover first write, same-key retry, conflicting retry, missing fields, and the
structured response shape through the real MCP stdio server.

## Changelog

| Version | Date | Status | Summary | Commit Hash | Agent |
|---|---|---|---|---|---|
| 1.0.1 | 2026-09-08 | accepted | Clarified that this is the legacy compatibility path and linked the separate GenesisRAG17 publication ADR. | working-tree | RWANG |
| 1.0.0 | 2026-08-10 | accepted | Added the typed `gks_knowledge_promote` MCP path for MSP-only promotion. | working-tree | Boss |
