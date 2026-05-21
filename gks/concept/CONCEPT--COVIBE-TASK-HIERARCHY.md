---
id: CONCEPT--COVIBE-TASK-HIERARCHY
phase: 1
type: concept
status: draft
vault_id: default
tier: process
source_type: axiomatic
title: CONCEPT — CoVibe Task Hierarchy — Tree decomposition for 8K context
tags: [covibe, architecture, tasks, context, scaling]
aliases: [tree-decomposition, context-isolation]
cluster: implementation_flow
role: Strategic intent / PRD
crosslinks:
  references:
    - CONCEPT--COVIBE-VISION
created_at: 2026-05-20T11:55:00+07:00
---

# CONCEPT — CoVibe Task Hierarchy

## 1. Goal

To implement a hierarchical task decomposition model that allows the EVA-CLI agent to operate within a hard 8K context window limit without suffering from hallucination or information loss.

## 2. The Hierarchy

| Level | Scope | Responsibility | UI Representation |
|---|---|---|---|
| **L1: Task** | Project Roadmap | Strategic Features (e.g. "Login System") | Roadmap List |
| **L2: Sub-task** | Kanban Card | Technical Components (e.g. "API Route") | Kanban Board |
| **L3: Micro-task** | Single LLM Call | Atomic Code Action (e.g. "Write Button.tsx") | Background Metric |

## 3. Principles

1. **Tree Decomposition:** Large tasks are recursively broken down into smaller units until each unit fits comfortably into an 8K token window.
2. **Context Isolation:** Each Micro-task receives only the strictly necessary context (System Prompt, File Tree, Target File).
3. **Stateless Prompting:** Micro-tasks do not rely on chat history. Instead, they use a "File-based State" handoff mechanism where the output of one task is read as the input for the next.

## Connections
- `[[CONCEPT--COVIBE-VISION]]` — the intent for high control.
- `[[ADR--COVIBE-STATE-HANDOFF]]` — the technical decision for memory management.
