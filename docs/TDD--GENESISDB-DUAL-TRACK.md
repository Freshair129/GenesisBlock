# TDD--GENESISDB-DUAL-TRACK-ARCHITECTURE

## 1. Executive Summary
GenesisDB is evolving into a **Dual-Track Hybrid Engine** designed to solve the "File System Fragmentation" problem in PKM (Obsidian) while providing enterprise-grade performance (50k+ TPS) for AI Agent systems.

## 2. Problem Statement
- **OS Exhaustion:** Standard file-based systems (like raw Obsidian) suffer from Random I/O latency when scaling to millions of small files.
- **Performance Gap:** Prototype versions are 50x slower than dedicated vector databases.
- **Rigidity vs. Flexibility:** Standard databases lack the human-readability of Markdown.

## 3. The "Two-File Rule" Philosophy
To protect hardware and optimize OS resources, GenesisDB strictly maintains only two primary files on disk regardless of data scale:
1.  **\genesis-graph.jsonl\ (WAL):** A single append-only log for all mutations, ensuring 100% Sequential I/O.
2.  **\genesis-graph.bin\ (Snapshot):** A dense binary image for sub-second cold boot.

## 4. Dual-Track Engine Strategy

### 4.1 Flex Mode (Human-Centric / PKM)
- **Target:** Personal Second Brain, Obsidian users.
- **Schema:** Markdown-first with YAML Frontmatter and JSON code blocks.
- **Addressing:** Human-readable String IDs (e.g., \project/genesis/v1\).
- **Access:** Direct editing in Obsidian with real-time sync to WAL.

### 4.2 Turbo Mode (Machine-Centric / Enterprise)
- **Target:** Big Data, Social Benchmarks (SF10+), High-QPS Agents.
- **Schema:** Aligned Binary Metadata.
- **Addressing:** ID Interning (\u32\ pointers) for RAM efficiency.
- **Access:** High-throughput Bulk APIs and Standalone HTTP Server.

## 5. Obsidian Bridge: The Shadow Layer
Instead of creating a standalone GUI, GenesisDB provides an Obsidian Plugin that acts as a "Virtual File System" (VFS).

- **On-the-fly Rendering:** The plugin fetches data from the Rust core and renders it as Markdown only when viewed.
- **Shadow Sync:** User saves a single Markdown file -> Plugin parses multiple Atomic Nodes -> Sends to GenesisDB WAL -> No new files created on disk.
- **Visual Nodes:** Custom views in Obsidian to visualize the Semantic Graph and K-Impact of notes.

## 6. Implementation Roadmap
- **Phase 9.1:** Turbo Core Overhaul (u32 Arena + DashMap Sharding).
- **Phase 9.2:** Markdown-Hybrid Parser Implementation.
- **Phase 9.5:** Obsidian Plugin (Alpha release).

## 7. Success Metrics
- **Throughput:** > 50,000 TPS in Turbo Mode.
- **Latency:** < 1ms for 3-hop traversals on 100M+ nodes.
- **Scale:** Successful SF10 (32M records) validation on a single workstation.

**Status:** APPROVED FOR IMPLEMENTATION
**Owner:** T2 Agent ARCHITECT
