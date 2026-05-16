---
id: STACK--MSP-NODE-RUNTIME
phase: 1
type: stack
status: stable
tier: process
source_type: axiomatic
vault_id: default
title: MSP Node.js Runtime — technology inventory for identity and retrieval
tags:
  - msp
  - stack
  - nodejs
  - runtime
  - technical
crosslinks: {"references":["FRAMEWORK--MSP-ARCHITECTURE-V2"]}
created_at: 2026-05-14T20:20:00+07:00
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

