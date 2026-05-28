---
id: LLM--TEMPLATE
tier: genesis
created_at: 2026-05-13T12:00:00.000+07:00
phase: 2
type: llm
status: active
vault_id: <YOUR-PROJECT>
title: <Model Name & Version>
tags: [ai, inference, llm, reasoning]
domain: ai-inference
crosslinks:
  enforces: []                  # GUARD-- or PROTO-- this model must follow
  references: []                # ADR-- that selected this model
linked_symbols: []              # Pointer to the adapter code
---

# LLM — <Model Name>

## Role & Tier

- **Intelligence Tier:** Reasoning / Planning / Complex Mapping
- **Preferred Phase:** P1 (Concept), P2 (ADR), P3 (Blueprint)

## Configuration

- **Model ID:** `e.g., gemini-1.5-pro`
- **Temperature:** <e.g. 0.0 for deterministic planning>
- **Top P:** <Value>
- **Max Tokens:** <Value>

## System Prompt / Personality

- **Instruction:** <Brief summary of the core personality/role>
- **Safety Constraints:** <Reference to SAFETY-- or GUARD-->

## Known Capabilities

- [ ] Multilingual reasoning
- [ ] Long-context window (1M+ tokens)
- [ ] Tool use (MCP)
- [ ] Image/Video multimodal

## Limitations

- Cost per 1M tokens: $<Value>
- Latency: <High/Medium>
- Tokens per minute (TPM): <Value>
