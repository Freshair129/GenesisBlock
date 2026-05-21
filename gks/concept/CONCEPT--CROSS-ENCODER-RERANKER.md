---
id: CONCEPT--CROSS-ENCODER-RERANKER
phase: 1
type: concept
status: draft
vault_id: default
tier: process
source_type: axiomatic
title: CONCEPT — Cross-Encoder Re-ranker — second-stage high-precision retrieval
tags: [msp, retrieval, rerank, cross-encoder, precision, m10c]
aliases: [CONCEPT, implementation_flow, Strategic intent / PRD]
cluster: implementation_flow
role: Strategic intent / PRD
crosslinks:
  references:
    - CONCEPT--MSP-ROADMAP
    - ADR--RETRIEVAL-RRF-FUSION
created_at: 2026-05-18T14:45:00+07:00
---

# CONCEPT — Cross-Encoder Re-ranker

## Intent

To introduce a second-stage re-ranking pass in the `msp_recall` pipeline using a **Cross-Encoder** model. While the first stage (RRF fusion of vector/text/episodic/graph) prioritises **recall** (finding all relevant candidates), the Cross-Encoder pass focuses on **precision** (ensuring the most relevant items are at the very top of the list) by performing deep semantic comparison between the query and each candidate.

## North Star

As the vault scales and RRF produces larger candidate sets with similar scores, the Cross-Encoder pass ensures that the Top-3 results consistently contain the "perfect" answer for complex queries, even when semantic similarity scores from the bi-encoder (first-stage) are near-identical.

## Guiding Principles

1. **Selective Application:** Only re-rank the Top-N (e.g., Top-30) candidates from the first stage to maintain total latency budgets.
2. **Deterministic Thresholds:** Establish a "re-ranking floor" — if the first-stage RRF scores are significantly high and unambiguous, the expensive re-ranking pass may be skipped.
3. **Model Sovereignty:** Support local Cross-Encoder models (e.g., BGE-Reranker via Transformers.js or Ollama) to maintain the zero-cloud-exposure promise.

## Expected Benefits

- **Improved Precision@1 and Precision@3.**
- **Robustness to "Vector Noise":** Better handling of queries where simple cosine similarity fails to distinguish between closely related concepts.
- **Improved Context Window Utilization:** Ensuring the most relevant knowledge is prioritised for the LLM's prompt.

## Connections

- `[[CONCEPT--MSP-ROADMAP]]` §4 M10c.
- `[[ADR--RETRIEVAL-RRF-FUSION]]` — the first-stage pipeline this concept enhances.
