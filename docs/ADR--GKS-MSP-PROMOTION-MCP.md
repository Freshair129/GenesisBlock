---
title: "ADR: Typed GKS MCP promotion for MSP"
status: "accepted"
version: "1.0.0"
updated: "2026-08-10"
owner: "Boss (CEO)"
---

# ADR: Typed GKS MCP promotion for MSP

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
