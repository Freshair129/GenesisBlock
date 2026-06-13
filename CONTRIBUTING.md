# Contributing to GenesisBlock DB

## 1. Engineering Philosophy
GenesisBlock DB is developed using **Documentation-Driven Development (DDD)** and **Root Cause Analysis (RCA)**. Every contribution must prioritize technical integrity, simplicity, and empirical verification.

## 2. Documentation Entrypoints

Before changing code, read the relevant docs in this order:

1. Architecture index / SSOT map: `docs/C4--GENESISDB-ARCHITECTURE.md`
2. Parent technical specification: `docs/MASTER-SPEC--GENESIS-DB.md`
3. Same-level specs, TDDs, ADRs, API docs, and SDK docs linked from the C4 map
4. Source files and tests for the touched component

If code changes alter architecture, public API, SDK behavior, agent workflow, or persistence behavior, the related docs must change in the same work item or include an explicit approved waiver.

## 3. Core Directives

### 3.1 Documentation First (Rule 5)
Never modify code without an approved specification.
1.  Submit an **SRD (Software Requirements Document)** and **TDD (Technical Design Document)** in `docs/`.
2.  Wait for architectural approval before writing the first line of code.
3.  Update the **Master Specification** (`docs/MASTER-SPEC--GENESIS-DB.md`) if the change affects core engine behavior.
4.  Update `docs/C4--GENESISDB-ARCHITECTURE.md` if the change affects containers, components, code anchors, or known architecture drift.

### 3.2 Root Cause Analysis (Rule 6)
Never fix a bug without identifying its root cause. Every bug fix PR must include an RCA report:
- **Symptom:** What happened?
- **Evidence:** Logs/Tests reproducing the failure.
- **Root Cause:** Why did it happen at the architectural level?
- **Prevention:** How do we ensure this never happens again?

### 3.3 Interior Mutability & Thread Safety
- Use `&self` receivers and granular locking via `DashMap` or `parking_lot::RwLock`.
- Avoid global state or heavy `Mutex` bottlenecks.
- Ensure all mutations are compatible with the **Group Commit WAL** logic.

## 4. Data Model Standards

### 4.1 Bitemporality & CRDTs
- Every mutation MUST increment the `LogicalClock`.
- Use the `supersede_node` pattern (Retract -> Re-insert) instead of in-place destructive updates.
- Ensure all new fields are reflected in both `NodeInput/Output` and `EdgeInput/Output` structs.

### 4.2 Semantic Integration (Thai-Aware)
- Contributions to indexing must respect the **Thai-aware tokenization** logic.
- Filter out `NonspacingMark` and `SpacingMark` when generating lexical trigrams to maintain fuzzy search recall.

## 5. Testing & Validation

### 5.1 Automated Tests
- Every feature must include a dedicated test file in `tests/`.
- Every bug fix must include a regression test that fails without the fix.

### 5.2 Benchmarking
- Focus on **P95 Latency** (< 30µs) and **TPS** under hardware-flushed (`fsync`) conditions.
- Run `cargo run --release --bin shadow-sync-stress` to verify no performance regressions.

## 6. Pull Request Process
1.  **Draft Spec:** Submit SRD/TDD.
2.  **Implement:** Surgical changes only (Rule 3). Match existing style.
3.  **Verify:** Pass all tests and `cargo check`.
4.  **Recap:** Finalize documentation and update `ROADMAP.md`.
5.  **Drift Check:** Confirm the C4 map, API docs, SDK docs, and agent context still match the changed code.
