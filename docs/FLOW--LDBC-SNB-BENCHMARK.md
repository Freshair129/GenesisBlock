# FLOW--LDBC-SNB-BENCHMARK

## 1. Process Workflow
The end-to-end process from initiation to certification.

| Step | Actor | Action |
|---|---|---|
| 1 | Commander | Start Phase 8 Benchmark |
| 2 | Agent | Trigger SNB Datagen (SF0.1) |
| 3 | GenesisDB | Parallel Bulk Ingestion |
| 4 | GenesisDB | Build HNSW & CSR Indices |
| 5 | Auditor | Generate P8 Certification |

## 2. Data Flow Architecture
How data moves through the hybrid storage engine.

- Ingestion: CSV -> Parallel Rust Parser -> Transaction Buffer
- Indexing: Vector -> Aligned Arena -> HNSW Graph; Social -> CSR Adjacency List
- Query: Input -> Hybrid Resolver -> SIMD Dot Product -> K-Impact Blending

## 3. Entity Relationship Diagram (ERD)
Mapping the SNB Social Schema to GenesisDB Atomic structures.

- PERSON (Node): { firstName, gender, embedding: Interests }
- POST (Node): { content, creationDate, embedding: Semantics }
- KNOWS (Edge): { since, weight: K-Impact }
