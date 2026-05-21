---
id: STACK--MSP-NODE-RUNTIME
phase: 2
type: stack
status: stable
tier: process
source_type: axiomatic
vault_id: default
title: MSP Node.js Runtime — technology inventory for identity and retrieval
tags: &a1
  - msp
  - stack
  - nodejs
  - runtime
  - technical
crosslinks: &a2
  references:
    - FRAMEWORK--MSP-ARCHITECTURE-V2
created_at: 2026-05-14T20:20:00+07:00
aliases: &a3
  - STACK
  - implementation_flow
  - Technology stack inventory
cluster: implementation_flow
role: Technology stack inventory
attributes:
  id: STACK--MSP-NODE-RUNTIME
  phase: 2
  type: stack
  status: stable
  tier: process
  source_type: axiomatic
  vault_id: default
  title: MSP Node.js Runtime — technology inventory for identity and retrieval
  tags: *a1
  crosslinks: *a2
  created_at: 2026-05-14T20:20:00+07:00
  aliases: *a3
  cluster: implementation_flow
  role: Technology stack inventory
  attributes:
    id: STACK--MSP-NODE-RUNTIME
    phase: 2
    type: stack
    status: stable
    tier: process
    source_type: axiomatic
    vault_id: default
    title: MSP Node.js Runtime — technology inventory for identity and retrieval
    tags: *a1
    crosslinks: *a2
    created_at: 2026-05-14T20:20:00+07:00
    aliases: *a3
    cluster: implementation_flow
    role: Technology stack inventory
    attributes:
      id: STACK--MSP-NODE-RUNTIME
      phase: 2
      type: stack
      status: stable
      tier: process
      source_type: axiomatic
      vault_id: default
      title: MSP Node.js Runtime — technology inventory for identity and retrieval
      tags: *a1
      crosslinks: *a2
      created_at: 2026-05-14T20:20:00+07:00
      aliases: *a3
      cluster: implementation_flow
      role: Technology stack inventory
      attributes:
        domain: stack
      domain: stack
      language: markdown
      is_test: false
      is_entrypoint: false
      has_secret: false
      leak_risk: low
      encryption_level: none
    domain: stack
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: false
    leak_risk: low
    encryption_level: none
  domain: stack
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: false
  leak_risk: low
  encryption_level: none
---

# STACK — MSP Node.js Runtime

## Runtime Environment

- **Node.js**: >= 20.0.0
- **Module System**: ESM (ECMAScript Modules)
- **Language**: TypeScript >= 5.0.0

## Core Components

- **Persistence**: File-based JSONL (GKS) + SQLite (Symbol Graph).
- **Communication**: Model Context Protocol (MCP) for tool binding.
- **Serialization**: `yaml` for atoms, `zod` for schema enforcement.
- **Embeddings**: Transformers.js (ONNX Runtime) for local Nomic embeddings.
- **Auth**: `node:crypto` for nonces and hash verification.

## Dependencies

- `@freshair129/gks`: Storage engine.
- `@modelcontextprotocol/sdk`: MCP server implementation.
- `better-sqlite3`: Native bindings for high-performance symbol storage.
- `yaml`: Parser for atomic frontmatter.
- `zod`: Type-safe schema validation.

## Connections

- [[FRAMEWORK--MSP-ARCHITECTURE-V2]]
