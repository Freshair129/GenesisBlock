# Contributing to GenesisBlock DB (Mark VIII)

## 1. Engineering Philosophy
GenesisBlock DB is developed using **Documentation-Driven Development (DDD)** and **Root Cause Analysis (RCA)**. Every contribution must prioritize technical integrity, simplicity, and empirical verification.

## 2. Core Directives

### 2.1 Documentation First (Rule 5)
Never modify code without an approved specification.
1.  Submit an **SRD (Software Requirements Document)** and **TDD (Technical Design Document)** in `docs/`.
2.  Wait for architectural approval before writing the first line of code.
3.  Update the **Master Specification** (`docs/MASTER-SPEC--GENESIS-DB.md`) if the change affects core engine behavior.

### 2.2 Root Cause Analysis (Rule 6)
Never fix a bug without identifying its root cause. Every bug fix PR must include an RCA report:
- **Symptom:** What happened?
- **Evidence:** Logs/Tests reproducing the failure.
- **Root Cause:** Why did it happen at the architectural level?
- **Prevention:** How do we ensure this never happens again?

### 2.3 Interior Mutability & Thread Safety
- Use `&self` receivers and granular locking via `DashMap` or `parking_lot::RwLock`.
- Avoid global state or heavy `Mutex` bottlenecks.
- Ensure all mutations are compatible with the **Group Commit WAL** logic.

## 3. Data Model Standards

### 3.1 Bitemporality & CRDTs
- Every mutation MUST increment the `LogicalClock`.
- Use the `supersede_node` pattern (Retract -> Re-insert) instead of in-place destructive updates.
- Ensure all new fields are reflected in both `NodeInput/Output` and `EdgeInput/Output` structs.

### 3.2 Semantic Integration (Thai-Aware)
- Contributions to indexing must respect the **Thai-aware tokenization** logic.
- Filter out `NonspacingMark` and `SpacingMark` when generating lexical trigrams to maintain fuzzy search recall.

## 4. Testing & Validation

### 4.1 Automated Tests
- Every feature must include a dedicated test file in `tests/`.
- Every bug fix must include a regression test that fails without the fix.

### 4.2 Benchmarking
- Focus on **P95 Latency** (< 30µs) and **TPS** under hardware-flushed (`fsync`) conditions.
- Run `cargo run --release --bin shadow-sync-stress` to verify no performance regressions.

## 5. Pull Request Process
1.  **Draft Spec:** Submit SRD/TDD.
2.  **Implement:** Surgical changes only (Rule 3). Match existing style.
3.  **Verify:** Pass all tests and `cargo check`.
4.  **Recap:** Finalize documentation and update `ROADMAP.md`.
