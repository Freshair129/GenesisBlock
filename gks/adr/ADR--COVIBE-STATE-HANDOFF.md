---
id: ADR--COVIBE-STATE-HANDOFF
phase: 2
type: adr
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: ADR — File-based state handoff for multi-stage micro-tasks
tags: [covibe, architecture, state, memory, context]
aliases: [ADR, implementation_flow, Architecture decision record]
cluster: implementation_flow
role: Architecture decision record
crosslinks:
  references:
    - CONCEPT--COVIBE-TASK-HIERARCHY
created_at: 2026-05-20T11:30:00+07:00
---

# ADR — File-based State Handoff

## Context

The EVA-CLI agent operates within a hard 8K context window limit. Standard agent loops that pass full conversation history quickly exceed this limit, leading to write failures, OOM errors, or high costs. We need a way for the agent to maintain technical coherence across multiple atomic actions (Micro-tasks) without relying on long-term chat history.

## Decision

We adopt a **File-based State Handoff** (also known as "Stateless Prompting with Context Hydration") for all multi-stage implementations.

### Rules

1. **History Pruning:** Conversation history is strictly limited or excluded from Micro-task prompts.
2. **Context Hydration:** Each Micro-task prompt is hydrated with:
    - The current file system tree structure.
    - The full source code of the specific target file being modified.
    - The source code or interfaces of *immediate dependencies* created in previous Micro-tasks.
3. **Atomic Writes:** Every Micro-task must result in a concrete file system change (e.g. creating a new file or patching an existing one) before the next task begins.

## Consequences

### Positive
- **Determinism:** Each step is self-contained and highly precise.
- **Infinite Scalability:** The total size of the project does not impact the reliability of individual Micro-tasks.
- **Cost/VRAM Efficiency:** Maintains sub-8K usage, allowing high-speed response on RTX 3060 hardware.

### Negative
- **Loss of Nuance:** "Implicit" intent from a long chat might be lost if not explicitly captured in a user hint or the task definition.
- **Discovery Overhead:** The system must accurately identify which dependency files to hydrate for each step.

## Alternatives considered

- **Vector-based History Retrieval:** Retrieving relevant chat segments via semantic search. *Rejected* for the initial phase to prioritize deterministic file-system-as-source reliability.

## Source

- `Implementation Architecture Document (IMP)` §5.
- `CONCEPT--COVIBE-TASK-HIERARCHY`
