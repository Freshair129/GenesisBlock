# Functional Specification: Cross-Lingual Knowledge Mapping (Mark V)

## 1. Objective
Enable **Thai-English Neural Retrieval**. This feature allows GenesisDB to resolve semantic queries across different languages, ensuring that a query performed in Thai (e.g., "การเรียนรู้ของเครื่อง") can correctly retrieve relevant nodes stored in English (e.g., "Machine Learning") and vice-versa.

## 2. Technical Approach: The Canonical Space
Embeddings from different languages often occupy different regions in the high-dimensional space. We will implement a **Semantic Normalizer** to bridge this gap.

### 2.1 Language Metadata
- Add an optional `lang` field to `NodeInput` and `NodeMetadata`.
- Supported values: `th`, `en`, `auto` (default: `en`).

### 2.2 Vector Normalization (Centering)
To improve cross-lingual retrieval without a dedicated translation layer, we will implement **Mean-Centering Normalization**:
1.  Calculate the global mean vector $\mu_{en}$ and $\mu_{th}$.
2.  Shift incoming query vectors toward the target language's centroid.
3.  In Step 1, we will provide an FFI method to `set_language_centroid(lang, vector)` to allow the host (Node.js) to provide the translation offsets from models like `multilingual-e5`.

## 3. HQL Expansion: `LANGUAGE` Keyword
Allow users to specify the query language for better accuracy.

**Proposed Syntax:**
- `SEARCH Node SIMILAR TO [vector] K 10 LANGUAGE "th"`
- This signals the engine to apply the Thai-to-English translation matrix/offset before searching the HNSW index.

## 4. Proposed Changes
- **src/lib.rs:**
    - Update `NodeMetadata` struct with `lang: [u8; 2]`.
    - Implement `lang_centroids: DashMap<String, Vec<f32>>`.
    - Implement `GenesisDatabase::set_language_centroid(lang: String, centroid: Vec<f64>)`.
- **src/query/ast.rs:**
    - Update `HqlCommand::Search` and `Hybrid` with `lang: Option<String>`.

## 5. Implementation Roadmap
1.  **Step 1:** Update `NodeMetadata` and FFI to store language centroids.
2.  **Step 2:** Refactor `hybrid_search` to apply vector shifting based on the `LANGUAGE` keyword.
3.  **Step 3:** Update `hql.pest` to support the `LANGUAGE` keyword.
4.  **Step 4:** Add a benchmark `benches/cross_lingual_audit.rs` verifying Thai-English recall.

Please review and approve this specification. I will generate the code once approved.
