# Master Specification: Mark IV (Global Scale & Reasoning)

## 1. Objective
Transition GenesisDB into the **Mark IV** era. The primary goals are to optimize the O(N) fuzzy search implemented for Obsidian, enforce strict data governance (Axiomatic Guards), and introduce the first layer of graph-based reasoning.

## 2. Trigram Performance Standard (Optimization)
The current `find_fuzzy_id` iterates through all node IDs ($O(N)$). As Obsidian vaults grow to 100k+ notes, this will exceed the <10ms latency target.

### 2.1 Implementation: Trigram Index
- **Structure:** `trigram_index: DashMap<String, HashSet<u32>>`.
- **Logic:**
  1.  During `add_node`, split the ID into 3-character shingles (e.g., `"Note"` -> `["not", "ote"]`).
  2.  Index these shingles to the node's `u32` internal ID.
  3.  In `find_fuzzy_id`:
      - Extract trigrams from the search term.
      - Retrieve candidate IDs from the `trigram_index`.
      - Only perform Jaro-Winkler similarity on the **intersection of candidates**.
- **Target:** Constant-time ($O(1)$ candidate retrieval) fuzzy matching.

## 3. Axiomatic Guards (Data Governance)
Enforce tier-based integrity rules to ensure the "Brain" doesn't ingest corrupt reasoning.

### 3.1 Tier Enforcement
- **MASTER Tier:** Read-only for external agents. Only derived through internal consensus.
- **SPEC Tier:** Requires valid `valid_from` and `recorded_at` timestamps.
- **Rules:** Any mutation to a MASTER-labeled node without a system-signed signature will return `403 Forbidden`.

## 4. Derived Reasoning (Inference Hooks)
Support "Virtual Edges" based on path logic.

### 4.1 Transitive Inference
- If `A -> [REPORTS_TO] -> B` and `B -> [REPORTS_TO] -> C`, then HQL should optionally resolve `A -> [IN_ORG_CHART] -> C`.

## 5. Implementation Roadmap (Phase 16)
1.  **Step 1:** Implement the `TrigramIndex` in `src/lib.rs`.
2.  **Step 2:** Refactor `find_fuzzy_id` to use the index.
3.  **Step 3:** Implement the `Guard` trait for mutation interceptors.
4.  **Step 4:** Add `INFERENCE` keyword to HQL grammar.

Please review and approve this Mark IV Master Specification. I will generate the code once approved.
