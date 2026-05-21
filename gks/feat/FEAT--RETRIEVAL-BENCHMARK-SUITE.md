---
id: FEAT--RETRIEVAL-BENCHMARK-SUITE
phase: 2
type: feat
status: draft
vault_id: default
tier: process
source_type: axiomatic
title: FEAT — Retrieval Benchmark Suite — empirical evaluation for search tuning
tags: [msp, retrieval, benchmark, metrics, tuning, m10c]
aliases: [FEAT, implementation_flow, Feature specification]
cluster: implementation_flow
role: Feature specification
crosslinks:
  references:
    - CONCEPT--MSP-ROADMAP
    - ADR--RETRIEVAL-RRF-FUSION
    - PARAMS--RETRIEVAL-WEIGHTS
created_at: 2026-05-18T14:30:00+07:00
---

# FEAT — Retrieval Benchmark Suite

## 1. Summary

The Retrieval Benchmark Suite is a specialized testing and evaluation framework designed to measure the effectiveness of the `msp_recall` orchestration. It provides objective metrics (Precision@k, Recall@k, MRR) to guide the tuning of RRF weights and verify the performance impact of new retrieval sources or re-ranking stages.

## 2. Motivation

"Retrieval quality" is often subjective and difficult to track as the knowledge base grows. Without empirical data, tuning the RRF weights (`PARAMS--RETRIEVAL-WEIGHTS`) is based on intuition rather than evidence. As we introduce more complex components like the Cross-Encoder re-ranker and pgvector backends, we need a standard "litmus test" to ensure we are actually improving search results without regressing on latency or basic recall.

## 3. Requirements

### 3.1 Ground Truth Data

- **Query Fixtures:** Maintain a set of representative natural language queries.
- **Expected Results (Labels):** For each query, define a set of "relevant" atoms that should be retrieved.
- **Versioning:** Benchmark datasets must be versioned to allow for consistent comparison over time.

### 3.2 Metric Calculation

- **Precision@k:** The fraction of retrieved atoms that are relevant among the top-k results.
- **Recall@k:** The fraction of all relevant atoms that were successfully retrieved in the top-k results.
- **MRR (Mean Reciprocal Rank):** Measures how high the first relevant result appears in the ranked list.
- **Latency:** Measure and report the p50, p90, and p99 latency for the entire retrieval pipeline.

### 3.3 CLI Interface

- Provide a command `msp-recall bench [--dataset <path>] [--json]`.
- Support running benchmarks against different configurations (e.g., re-ranking ON vs. OFF).
- Output a summary table of metrics and a comparison report.

### 3.4 Automated Reporting

- Integrate with the project's benchmarking infrastructure (e.g., `packages/msp/benchmarks/`).
- Generate a Markdown report after each run, including a git SHA for traceability.

## 4. Acceptance Criteria

- [ ] A suite of at least 20 test queries and labeled ground-truth results exists.
- [ ] The `msp-recall bench` command successfully calculates and reports P@k, R@k, and MRR.
- [ ] The benchmark can clearly differentiate between a "good" and "bad" RRF weight configuration.
- [ ] Results are stable and reproducible across multiple runs on the same hardware.

## 5. Connections

- `[[CONCEPT--MSP-ROADMAP]]` §4 M10c.
- `[[PARAMS--RETRIEVAL-WEIGHTS]]` — the parameters this suite helps tune.
- `[[ADR--RETRIEVAL-RRF-FUSION]]` — the algorithm being evaluated.
