# Functional Specification: Logic-Gated Context Windows (Mark V)

## 1. Objective
Provide AI Agents (LLMs) with a **Ranked Context Window**. Instead of returning raw search results, this feature filters and orders nodes based on a weighted "Reasoning Score" that combines semantic relevance (vector distance) and logical authority (K-Impact).

## 2. The Reasoning Score ($R$)
For each candidate node $n$, the Reasoning Score $R(n)$ is calculated as:

$$ R(n) = (Similarity(n) \cdot W_{sim}) + (K\_Impact(n) \cdot W_{impact}) $$

- **$Similarity(n)$:** The normalized vector similarity (1.0 - distance).
- **$K\_Impact(n)$:** The node's internal authority score (from Phase 16).
- **Default Weights:** $W_{sim} = 0.6, W_{impact} = 0.4$.

## 3. New FFI & API Endpoint: `/v1/reason/context`
A dedicated endpoint for context assembly.

### 3.1 Input Parameters
```json
{
  "query_vector": [0.1, 0.2, "..."],
  "k": 20,
  "min_impact": 0.5,
  "lang": "th"
}
```

### 3.2 Output Format
A prioritized list of text/markdown content optimized for LLM prompting.

## 4. Implementation Details

### 4.1 src/lib.rs
- Implement `pub fn get_ranked_context(&self, args: HybridSearchInput) -> Result<Vec<NeighborOutput>>`.
- This method will call `hybrid_search` but apply the $R(n)$ sorting logic.

### 4.2 src/main.rs
- Expose `POST /v1/reason/context` using the Axum router.
- Map internal `NeighborOutput` to a clean JSON response for the Obsidian plugin.

## 5. Value for AI Agents
By "gating" the context window with logic, we ensure that an LLM doesn't get distracted by high-similarity "draft" notes when low-similarity but high-authority "MASTER" axioms are available.

## 6. Implementation Roadmap
1.  **Step 1:** Implement the `ReasoningScore` formula in `src/lib.rs`.
2.  **Step 2:** Refactor `hybrid_search` to support custom weight overrides.
3.  **Step 3:** Implement the `/v1/reason/context` REST handler.
4.  **Step 4:** Add a benchmark `benches/context_assembly_stress.rs`.

Please review and approve this specification. I will generate the code once approved.
