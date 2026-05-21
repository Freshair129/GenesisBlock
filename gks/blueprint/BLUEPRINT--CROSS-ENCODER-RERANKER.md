---
id: BLUEPRINT--CROSS-ENCODER-RERANKER
phase: 3
type: blueprint
status: draft
vault_id: default
tier: process
source_type: axiomatic
title: BLUEPRINT — Cross-Encoder Re-ranker Implementation Plan
tags: [msp, retrieval, rerank, precision, m10c, plan]
aliases: [BLUEPRINT, implementation_flow, Implementation plan]
cluster: implementation_flow
role: Implementation plan
crosslinks:
  references:
    - CONCEPT--CROSS-ENCODER-RERANKER
    - ADR--RETRIEVAL-RRF-FUSION
linked_symbols:
  - file: packages/gks/src/memory/vector/reranker.ts
  - file: packages/msp/src/orchestrator/retrieval/index.ts
created_at: 2026-05-18T15:00:00+07:00
---

# BLUEPRINT — Cross-Encoder Re-ranker

## 1. Goal

Integrate a local Cross-Encoder re-ranking stage into the `msp_recall` pipeline to improve retrieval precision for large knowledge bases (M10c).

## 2. Implementation Steps

### T1: Reranker Adapter Interface (`packages/gks/src/memory/vector/reranker.ts`)

- Define a generic `Reranker` interface:

    ```typescript
    interface RerankItem {
      text: string;
      id: string;
    }
    interface Reranker {
      rerank(query: string, items: RerankItem[]): Promise<Array<{ id: string; score: number }>>;
      model: string;
    }
    ```

### T2: BGE-Reranker Implementation (Transformers.js)

- Implement `BgeReranker` using `@huggingface/transformers` (or similar).
- Optimize for local execution (e.g., using quantization, CPU/WebGPU threads).
- Ensure the model is cached locally.

### T3: Retrieval Orchestration Integration

- Update `packages/msp/src/orchestrator/retrieval/index.ts`.
- Insert the re-ranking pass after the RRF fusion step.
- Logic:
    1. Collect top-K (default 30) results from RRF.
    2. Pass query and candidates to the `Reranker`.
    3. Update the final results with the high-precision Cross-Encoder scores.
    4. Re-sort the Top-N results based on the new scores.

### T4: Configurability and Throttling

- Add `RecallOptions.rerank` (boolean, default: false).
- Add `RecallOptions.rerankLimit` (number, default: 30).
- Implement a "floor" check: Skip re-ranking if the top RRF hit is significantly dominant.

### T5: Performance and Latency Guard

- Add a timeout for the re-ranking pass (default: 500ms).
- If the re-ranker times out, fall back to the original RRF ordering and add a reason to `fallback_reasons`.

## 3. Verification Plan

### 3.1 Accuracy Benchmarking

- Create a benchmark script `packages/msp/test/bench/recall-precision.test.ts`.
- Compare RRF-only vs. RRF+Reranker results against a ground-truth set of complex queries.
- Measure: Precision@1, Precision@3, MRR (Mean Reciprocal Rank).

### 3.2 Latency Benchmarking

- Measure re-ranking overhead for batches of 10, 30, and 50 items.
- Verify that total `msp_recall` latency stays within acceptable bounds for interactive agent use.
