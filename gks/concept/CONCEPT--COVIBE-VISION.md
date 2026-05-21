---
id: CONCEPT--COVIBE-VISION
phase: 1
type: concept
status: draft
vault_id: default
tier: process
source_type: axiomatic
title: CONCEPT — CoVibe — autonomous music synchronization for riders
tags: [covibe, vision, music, sync, riders]
aliases: [EVA-CLI, vibe-coding]
cluster: implementation_flow
role: Strategic intent / PRD
crosslinks:
  references:
    - FRAMEWORK--MSP-ARCHITECTURE-V2
created_at: 2026-05-21T23:30:00+07:00
---

# CONCEPT — CoVibe Vision

## Intent

To revolutionize the music listening experience for motorcycle riders (Rider and Passenger) through a synchronized, low-latency, and autonomous "Vibe Coding" agent framework (**EVA-CLI**). CoVibe enables seamless shared listening using YouTube IFrame and WebSockets, while the EVA-CLI agent manages the complex technical implementation via a hierarchical task model.

## North Star

A "Zero Cost, High Control" autonomous system where a local LLM (Qwen2.5-Coder) acts as a project manager and implementer, allowing solo developers or small teams to build stable, synchronized applications on consumer-grade hardware with zero cloud dependency.

## Guiding Principles

1. **Local-First (Zero Cost):** All AI processing happens on the user's hardware (RTX 3060 baseline).
2. **Hierarchical Task Decomposition:** Task -> Sub-task -> Micro-task to manage the 8K context window.
3. **Quiet Luxury & High Control:** The UI focuses on a clean Planning (Roadmap) view and an Active Execution (Kanban) view, keeping the human as the authoritative reviewer.

## Connections
- `[[CONCEPT--COVIBE-ROADMAP]]` — the execution timeline.
- `[[CONCEPT--COVIBE-TASK-HIERARCHY]]` — the structural method for implementation.
