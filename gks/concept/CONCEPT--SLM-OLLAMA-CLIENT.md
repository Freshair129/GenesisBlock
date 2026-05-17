---
id: CONCEPT--SLM-OLLAMA-CLIENT
phase: 1
type: concept
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: Ollama SLM client — real codegen via Ollama HTTP API
tags: &a1
  - msp
  - codegen
  - slm
  - ollama
  - runtime
crosslinks: &a2
  references:
    - FEAT--CODEGEN-MICROTASK-RUNNER
    - ADR--CODEGEN-RETRY-POLICY
created_at: 2026-05-03T16:22:30.407+07:00
aliases: &a3
  - CONCEPT
  - implementation_flow
  - Strategic intent / PRD
cluster: implementation_flow
role: Strategic intent / PRD
attributes:
  id: CONCEPT--SLM-OLLAMA-CLIENT
  phase: 1
  type: concept
  status: stable
  vault_id: default
  tier: genesis
  source_type: axiomatic
  title: Ollama SLM client — real codegen via Ollama HTTP API
  tags: *a1
  crosslinks: *a2
  created_at: 2026-05-03T16:22:30.407+07:00
  aliases: *a3
  cluster: implementation_flow
  role: Strategic intent / PRD
  attributes:
    id: CONCEPT--SLM-OLLAMA-CLIENT
    phase: 1
    type: concept
    status: stable
    vault_id: default
    tier: genesis
    source_type: axiomatic
    title: Ollama SLM client — real codegen via Ollama HTTP API
    tags: *a1
    crosslinks: *a2
    created_at: 2026-05-03T16:22:30.407+07:00
    aliases: *a3
    cluster: implementation_flow
    role: Strategic intent / PRD
    attributes:
      domain: concept
    domain: concept
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: false
    leak_risk: low
    encryption_level: none
  domain: concept
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: false
  leak_risk: low
  encryption_level: none
---

# CONCEPT — Ollama SLM client

## Problem

`[[FEAT--CODEGEN-MICROTASK-RUNNER]]` works end to end with a mock SLM that returns either a stub or whatever the prompt's `// MOCK_OUTPUT:` hint specifies. The mock is enough for testing the pipeline mechanics but cannot produce real code for production microtasks. Until a real SLM is wired, M3c-4 is a scaffold, not a usable feature.

## Hypothesis

If the runner accepts a pluggable `SlmClient` and we ship one concrete implementation that talks to Ollama (`POST /api/generate` with `model` + `prompt`), then anyone with a local Ollama install + Qwen 2.5 Coder can run real microtasks end to end. No API key required. No network call in tests (mock `fetch`).

## Scope

In:
- HTTP client targeting Ollama's `/api/generate` endpoint (single-turn, no chat history).
- Honour `OLLAMA_HOST` (default `http://127.0.0.1:11434`) + `OLLAMA_MODEL` (default `qwen2.5-coder:7b`) env vars.
- Implements the `SlmClient` interface from `src/codegen/types.ts`.
- Surface errors clearly: connection refused, model not pulled, timeout.
- Pluggable factory `createSlmClient(opts?)` so future Anthropic / OpenAI clients drop in next to it.

Out:
- Streaming responses (we collect the full output before returning).
- Chat-history mode (the codegen retry policy strips previous attempts anyway).
- Authentication / TLS — Ollama is local-by-default.
- Anthropic / OpenAI / vLLM clients — separate atoms when needed.

## Source

Closes M3c-4's residual ("real Qwen 2.5 Coder integration is a future task"). P0 item #1 from production-readiness backlog.

## Connections
- [[ADR--CODEGEN-RETRY-POLICY]]

