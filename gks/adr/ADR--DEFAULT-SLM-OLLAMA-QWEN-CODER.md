---
id: ADR--DEFAULT-SLM-OLLAMA-QWEN-CODER
phase: 2
type: adr
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: Default codegen SLM is Ollama + qwen2.5-coder (drop the qwen-CLI fallback in runner)
tags:
  - msp
  - codegen
  - slm
  - ollama
  - qwen
  - decision
crosslinks: {"references":["CONCEPT--CODEGEN-MICROTASK-RUNNER","CONCEPT--CODEGEN-MICROTASK-CONTRACT"]}
created_at: 2026-05-12T22:46:00.000+07:00
---

# ADR — Default codegen SLM is Ollama + qwen2.5-coder

## Context

`[[CONCEPT--CODEGEN-MICROTASK-RUNNER]]` states the runner should "Invoke a pluggable SLM (Qwen 2.5 Coder default; configurable per-task)." `FRAMEWORK_MASTER_SPEC.md` §17.3 maps the T1 tier explicitly to `Qwen 2.5-Coder 7–14B / Llama 3.x / Phi-3 (local GPU/CPU)`. The expected primary path is **local Ollama running `qwen2.5-coder:7b`** (or `:14b` on ≥16 GB VRAM).

The factory at `packages/msp/src/codegen/slm/factory.ts` already defaults to `'ollama'` when `MSP_SLM_PROVIDER` is unset, and `slm/ollama.ts` already uses `qwen2.5-coder:7b` as the default model. **But** `packages/msp/src/codegen/runner.ts:79` hard-coded the runner-side fallback to `provider: 'qwen'` — a different SLM client wrapping a standalone `qwen` binary, not Ollama. The two drifted: the documented contract said Ollama-qwen2.5-coder; the code said qwen-CLI.

This drift is the kind §14.1 calls out — **code beats spec** for runtime behaviour, but here the spec is correct and the code was wrong. The fix is at the code layer.

## Decision

The microtask runner SHALL **not** hardcode a provider. It SHALL call `createSlmClient({})` and let the factory honour `MSP_SLM_PROVIDER` (default `'ollama'`) and `OLLAMA_MODEL` (default `qwen2.5-coder:7b`).

Concretely:
- `runner.ts:79` — remove `?? 'qwen'`.
- `cli.ts` — update `--model` help text to mention `qwen2.5-coder:7b` default and `OLLAMA_MODEL=qwen2.5-coder:14b` upgrade path.
- `slm/ollama.ts` — add a leading-comment that documents the 7b/14b guidance.

The `'qwen'` provider remains in the factory for callers who explicitly want the standalone qwen-CLI wrapper. It is no longer the runner's default.

## Consequences

### Positive
- Code matches `[[CONCEPT--CODEGEN-MICROTASK-RUNNER]]` and `FRAMEWORK_MASTER_SPEC` §17.3.
- The default microtask path now resolves consistently to local Ollama, which is what every supported tier mapping in the spec assumes.
- 14B upgrade is a single env-var swap (`OLLAMA_MODEL=qwen2.5-coder:14b`) — no code change.

### Negative
- Callers that relied implicitly on the standalone `qwen` CLI must now set `MSP_SLM_PROVIDER=qwen` explicitly.

### Neutral
- The Gemini-as-escalator chain is unchanged: Ollama → Gemini (escalator) → Opus (human gate).

## Status

Draft. Promotion to `stable` requires green CI on Node 20 + 22 with the runner-default-slm test added in this same PR.

## Connections
- [[CONCEPT--CODEGEN-MICROTASK-CONTRACT]]

