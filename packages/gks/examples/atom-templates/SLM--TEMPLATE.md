---
id: SLM--TEMPLATE
tier: genesis
created_at: 2026-05-13T12:00:00.000+07:00
phase: 5
type: slm
status: active
vault_id: <YOUR-PROJECT>
title: <Model Name & Version>
tags: [ai, inference, slm, codegen]
domain: ai-inference
crosslinks:
  enforces: []                  # GUARD-- for codegen markers
  references: []                # BLUEPRINT-- this model is assigned to
linked_symbols: []              # Pointer to the local runner code
---

# SLM — <Model Name>

## Role & Tier

- **Intelligence Tier:** Execution / Codegen / Summarization / Formatting
- **Preferred Phase:** P5 (Code), P3.5 (Microtasks)

## Configuration

- **Model ID:** `e.g., qwen2.5-coder-7b`
- **Runner:** <Ollama / vLLM / Local / API>
- **Quantization:** <e.g. Q4_K_M>
- **Temperature:** <e.g. 0.1 for precise coding>

## Specialized Domain

- **Languages:** [Typescript, Python, Rust, etc.]
- **Frameworks:** [Next.js, Prisma, etc.]

## Performance

- **Tokens/sec (TPS):** <Value>
- **Context Window:** <e.g. 32k>
- **Memory Footprint:** <e.g. 8GB VRAM>

## Verification

- **Benchmark:** <Reference to HumanEval / similar result>
- **Audit ID:** AUDIT--<performance-report>
