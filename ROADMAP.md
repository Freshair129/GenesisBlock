# GENESISDB ROADMAP (MARK III)
**Positioning:** Local Hybrid Knowledge Engine for AI Agents

## Current Status
- **Engineering Quality:** 8/10 (Serious Engineering Project)
- **Production Readiness:** 5/10 (Missing critical enterprise features)
- **Core Architecture:** Interior Mutability, Lock Sharding, HNSW Semantic Indexing, Single-WAL Event Sourcing.
- **Verified Benchmark Limit:** 139 TPS (True durable NVMe fsync under thread contention). 

## Phase 13: Reproducible Enterprise Infrastructure
*The focus shifts from chasing QPS to building verifiable, production-ready database internals.*

### 1. WAL Group Commit & Crash Recovery
- **Problem:** True \sync\ on every single write drops throughput to ~139 TPS due to NVMe IOPS limits and file descriptor contention.
- **Solution:** Implement a WAL Group Commit mechanism. Batch concurrent write requests in memory and flush them to disk via a single \sync\ call.
- **Evidence Required:** Automated "Kill -9" Recovery Test harness in CI/CD demonstrating zero data loss and state consistency.

### 2. Query Engine & Planner
- **Problem:** HQL execution is currently a hardcoded Regex dispatcher.
- **Solution:** Build a proper Query Planner that parses HQL into an Abstract Syntax Tree (AST), estimates costs, and executes via a unified physical plan.
- **Evidence Required:** Source code of the AST parser and planner.

### 3. CI/CD Benchmark Harness
- **Problem:** Benchmark results rely on manual execution and reporting.
- **Solution:** Integrate \scientific_audit.rs\ into GitHub Actions. Provide flamegraphs, memory profiles (jemalloc), and raw latency histograms automatically on every PR.

