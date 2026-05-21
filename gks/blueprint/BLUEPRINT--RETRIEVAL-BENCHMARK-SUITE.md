---
id: BLUEPRINT--RETRIEVAL-BENCHMARK-SUITE
phase: 3
type: blueprint
status: draft
vault_id: default
tier: process
source_type: axiomatic
title: BLUEPRINT — Retrieval Benchmark Suite Implementation Plan
tags: [msp, retrieval, benchmark, tuning, plan, m10c]
aliases: [BLUEPRINT, implementation_flow, Implementation plan]
cluster: implementation_flow
role: Implementation plan
crosslinks:
  references:
    - FEAT--RETRIEVAL-BENCHMARK-SUITE
    - CONCEPT--MSP-ROADMAP
linked_symbols:
  - file: packages/msp/src/orchestrator/retrieval/bench-engine.ts
  - file: packages/msp/src/orchestrator/retrieval/bench-cli.ts
created_at: 2026-05-18T15:00:00+07:00
---

# BLUEPRINT — Retrieval Benchmark Suite

## 1. Goal

Implement the technical machinery to evaluate retrieval quality (P@k, R@k, MRR) and latency, providing the data needed to tune RRF weights and verify the impact of the re-ranker.

## 2. Implementation Steps

### T1: Benchmark Data Schema (`packages/msp/test/bench/fixtures/recall_dataset.json`)

- Define the JSON structure for benchmark datasets:

    ```json
    {
      "version": "1.0.0",
      "queries": [
        {
          "query": "how do I handle expired atoms?",
          "relevant_ids": ["FEAT--DECISION-ATROPHY-GUARDS", "PROTO--VALID-UNTIL"]
        }
      ]
    }
    ```

- Collect 20 initial queries covering core system concepts.

### T2: Metric Engine (`packages/msp/src/orchestrator/retrieval/bench-engine.ts`)

- Implement functions to calculate:
  - `calculatePrecision(hits, relevantIds, k)`
  - `calculateRecall(hits, relevantIds, k)`
  - `calculateMRR(hits, relevantIds)`
- Use `recall()` for the actual retrieval calls.

### T3: CLI Implementation (`packages/msp/src/orchestrator/retrieval/bench-cli.ts`)

- Implement `msp-recall bench` command.
- Features:
  - Load dataset (T1).
  - Run sequential queries through the Metric Engine (T2).
  - Support `--rerank` toggle to compare baseline vs. enhanced search.
  - Aggregate results across the entire dataset.

### T4: Latency Profiling

- Enhance `recall()` timings to capture higher resolution stats for benchmarking.
- Collect and report p50, p90, and p99 latencies in the benchmark report.

### T5: Reporting and Integration

- Add `msp-recall` bench binary to `package.json`.
- Automatically write a Markdown report to `packages/msp/benchmarks/recall/REPORT--<SHA>.md`.

## 3. Verification Plan

### 3.1 Unit Tests

- Verify metric calculations with mock hits and relevant IDs.
- Test dataset loading and validation.

### 3.2 Manual Acceptance

- Run the benchmark with default weights.
- Intentionally degrade a weight (e.g., set `gks-vector` to 0.1).
- Verify that the benchmark metrics reflect this degradation (e.g., lower Recall@10).
- Run with re-ranking enabled and verify if Precision@3 improves.
