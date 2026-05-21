---
id: CONCEPT--MSP-BRIDGE-PLUGIN
phase: 1
type: concept
status: draft
vault_id: default
tier: process
source_type: axiomatic
title: CONCEPT — MSP Bridge Plugin — stable API and pgvector adapter for Obsidian
tags: [msp, obsidian, plugin, pgvector, m10a]
aliases: [CONCEPT, implementation_flow, Strategic intent / PRD]
cluster: implementation_flow
role: Strategic intent / PRD
crosslinks:
  references:
    - CONCEPT--MSP-ROADMAP
    - ADR--MSP-OBSIDIAN-INTEGRATION
created_at: 2026-05-18T14:00:00+07:00
---

# CONCEPT — MSP Bridge Plugin

## Intent

To provide a dedicated, stable integration layer between Obsidian and the cognitive_system via a custom Obsidian plugin (`msp-bridge`). This plugin will expose a reliable REST API for Smart Connections and introduce a `pgvector` adapter, ensuring high-performance semantic search and reliable connectivity as the knowledge base scales beyond the limitations of local markdown files and the default Smart Connections REST server.

## North Star

As the vault grows past 5,000 atoms or when semantic search latency exceeds 500ms, the system seamlessly transitions from local-only operations to a robust, database-backed retrieval mechanism without disrupting the existing agent workflows. The `msp-bridge` plugin acts as the authoritative gateway, abstracting the complexity of the underlying vector store (`pgvector`).

## Guiding Principles

1. **Scalability:** The architecture must effortlessly handle tens of thousands of atoms with sub-second semantic search latency.
2. **Stability:** Provide a versioned, stable API that insulates the cognitive_system from internal changes within the Smart Connections ecosystem.
3. **Pluggability:** The transition to `pgvector` should be an opt-in configuration, allowing smaller vaults to remain local-only while providing a clear upgrade path for large enterprise setups.

## Connections

- `[[CONCEPT--MSP-ROADMAP]]` §4 M10a.
- `[[ADR--MSP-OBSIDIAN-INTEGRATION]]` — the original integration decision this concept expands upon.
