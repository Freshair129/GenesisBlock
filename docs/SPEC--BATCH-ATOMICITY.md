# Software Requirements Document (SRD): Transaction / Batch Atomicity (Mark IX, Step 2)

## 1. Introduction
Complex knowledge mutations often involve multiple interdependent steps (e.g., adding a new concept node and linking it to existing themes). Currently, GenesisDB processes these individually. **Mark IX Step 2** introduces **Batch Atomicity**, ensuring that a group of mutations either succeeds entirely or leaves the system unchanged.

## 2. Functional Requirements

### FR1: Multi-Event Batches
- The system must accept a list of events (`Node`, `Edge`, `Supersede`) and process them as a single logical unit.

### FR2: All-or-Nothing Persistence
- The entire batch must be committed to the WAL in a single atomic operation. If the write fails, none of the events should be reflected in the memory index.

### FR3: Atomic Index Updates
- In-memory indices (DashMaps, HNSW, Trigrams) must be updated only after successful WAL persistence, ensuring that a partial batch never becomes visible to queries.

---

# Technical Design Document (TDD): Batch Engine

## 1. Data Structures (`src/lib.rs`)

### 1.1 `EventBatch` Wrapper
```rust
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EventBatch {
    pub batch_id: String,
    pub events: Vec<Event>,
}
```

### 1.2 Storage Extension
```rust
impl Storage {
    pub fn execute_batch(&self, events: Vec<EventInput>) -> Result<BatchOutput>;
}
```

## 2. Implementation Logic
1.  **Validation Phase:** Loop through all events in the batch. Check governance tiers and validate IDs. If any event fails validation, reject the entire batch.
2.  **Clock Synchronization:** Increment the `LogicalClock` for the entire batch or once per event within the batch (to be decided based on ordering requirements).
3.  **WAL Phase:** Serialize the `EventBatch` as a single line in the JSONL WAL or a sequence with `BATCH_START`/`BATCH_END` markers.
4.  **Memory Phase:** Apply mutations to `self.nodes`, `self.edges`, and adjacency indices sequentially within a write-lock context (where possible) or using the current concurrent primitives.

---

## 3. Definition of Done (DoD)
1.  [ ] `execute_batch` implemented and exposed via NAPI.
2.  [ ] **Atomicity Test:** Force a failure on the 3rd event of a 5-event batch -> Verify that events 1 and 2 are not present in the graph.
3.  [ ] **Performance Check:** Benchmarking shows that 1 batch of 100 nodes is significantly faster than 100 individual `add_node` calls.
4.  [ ] Documentation updated in `MASTER-SPEC--GENESIS-DB.md`.

---
**Please review and approve this Specification. I will begin the implementation once approved.**
