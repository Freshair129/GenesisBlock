---
id: BLUEPRINT--MSP-BRIDGE-PLUGIN
phase: 3
type: blueprint
status: draft
vault_id: default
tier: process
source_type: axiomatic
title: BLUEPRINT — MSP Bridge Plugin Implementation Plan
tags: [msp, obsidian, plugin, pgvector, m10a, plan]
aliases: [BLUEPRINT, implementation_flow, Implementation plan]
cluster: implementation_flow
role: Implementation plan
linked_symbols:
  - file: apps/obsidian-mcp/msp-bridge
crosslinks:
  references:
    - CONCEPT--MSP-BRIDGE-PLUGIN
    - CONCEPT--MSP-ROADMAP
created_at: 2026-05-18T14:15:00+07:00
---

# BLUEPRINT — MSP Bridge Plugin

## 1. Goal

Implement the `msp-bridge` Obsidian plugin to expose a stable REST API for semantic search and provide an optional `pgvector` backend, addressing scalability and latency issues for large vaults (M10a).

## 2. Implementation Steps

### T1: Plugin Scaffolding

- Initialize a new Obsidian plugin project in `apps/obsidian-mcp/msp-bridge` (or a similar appropriate directory).
- Set up build tools (esbuild, TypeScript).
- Create basic plugin lifecycle methods (`onload`, `onunload`).

### T2: Stable REST API (Smart Connections Wrapper)

- Implement an Express-like router within the Obsidian plugin.
- Expose an endpoint: `POST /api/semantic-search`.
- Integrate with the existing Smart Connections API (if available) or implement a fallback mechanism to query local embeddings.
- Ensure the API response format is strictly versioned to insulate the `msp` package from upstream changes.

### T3: pgvector Adapter

- Implement a connection manager for PostgreSQL.
- Create an adapter that conforms to the GKS `Embedder` and `VectorStore` interfaces.
- Functionality:
  - Connect to a Postgres database with the `pgvector` extension.
  - Store atom embeddings and metadata.
  - Perform cosine similarity searches using `pgvector` indexing (e.g., `ivfflat` or `hnsw`).
- Provide configuration settings within the plugin to toggle between local (Smart Connections) and `pgvector` backends.

### T4: Synchronization Logic

- Implement a mechanism to keep the `pgvector` database synchronized with the local Obsidian vault.
- Listen to Obsidian's workspace events (`modify`, `create`, `delete`, `rename`).
- Queue updates and process them asynchronously to avoid blocking the UI.

### T5: GKS Integration

- Update `packages/gks/src/memory/vector/` to support configuring the `msp-bridge` endpoint.
- Add a new adapter in `packages/gks` specifically for interacting with the `msp-bridge` API.

## 3. Verification Plan

### 3.1 Unit Tests

- Test the REST API routing logic.
- Test the `pgvector` SQL query generation and execution (using a mock DB or testcontainer).
- Test synchronization event queuing and batching.

### 3.2 Integration Tests

- Spin up an Obsidian instance with the plugin loaded.
- Send requests to the `/api/semantic-search` endpoint and verify the results.
- Modify a file in Obsidian and verify that the `pgvector` database (if configured) is updated accordingly.

### 3.3 Benchmarking

- Compare semantic search latency between the default local implementation and the `pgvector` backend for vaults containing 1,000, 5,000, and 10,000 atoms.
- Target: < 500ms for 10,000 atoms.
