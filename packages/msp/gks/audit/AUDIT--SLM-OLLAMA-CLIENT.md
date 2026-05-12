---
id: AUDIT--SLM-OLLAMA-CLIENT
phase: 6
type: audit
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: M4b — Ollama SLM client acceptance audit
tags:
  - msp
  - m4
  - m4b
  - audit
  - codegen
  - slm
  - ollama
crosslinks: {"references":["FEAT--SLM-OLLAMA-CLIENT","BLUEPRINT--SLM-OLLAMA-CLIENT","ADR--SLM-OLLAMA-CLIENT","FEAT--CODEGEN-MICROTASK-RUNNER"]}
linked_symbols:
  - {"file":"src/codegen/slm/ollama.ts"}
  - {"file":"src/codegen/slm/factory.ts"}
  - {"file":"src/codegen/slm/errors.ts"}
  - {"file":"src/codegen/slm/types.ts"}
created_at: 2026-05-03T17:24:58.170+07:00
---

# AUDIT — Ollama SLM client

## Scope

Closes FEAT--SLM-OLLAMA-CLIENT. Closes P0 item #1 (real SLM) from the M3 production-readiness backlog.

## Acceptance criteria from FEAT

| # | Criterion | Result |
|---|---|---|
| 1 | `createOllamaClient()` honours OLLAMA_HOST + OLLAMA_MODEL env defaults | ✅ test |
| 2 | `opts: { host, model, fetchImpl, signal, timeoutMs, temperature }` overrides | ✅ test |
| 3 | POSTs to `<host>/api/generate` with correct body shape | ✅ test |
| 4 | Returns `response` string from JSON | ✅ test |
| 5 | Network error → `SlmError(kind:'network')` | ✅ test |
| 6 | Non-2xx → `SlmError(kind:'http')` | ✅ test |
| 7 | Malformed JSON → `SlmError(kind:'parse')` | ✅ test |
| 8 | Missing `response` field → `SlmError(kind:'parse')` | ✅ test |
| 9 | Timeout exceeded → `SlmError(kind:'timeout')` | ✅ test |
| 10 | `createSlmClient({ provider:'ollama' })` returns Ollama client | ✅ test |
| 11 | `createSlmClient({ provider:'mock' })` returns mock | ✅ test |
| 12 | `MSP_SLM_PROVIDER` env honoured | ✅ test |
| 13 | Unknown provider → SlmError(config) | ✅ test |
| 14 | **No real network in tests** | ✅ all paths injected fetchImpl |

## Test summary

```
test/codegen/slm/ollama.test.ts:  10/10 passing
test/codegen/slm/factory.test.ts:  4/4 passing
total: 14/14
```

## How to actually use it

```sh
# 1. Install Ollama, pull the model
brew install ollama   # or your platform equivalent
ollama serve &
ollama pull qwen2.5-coder:7b

# 2. Run a microtask via the real SLM
MSP_SLM_PROVIDER=ollama npx msp-run-task .brain/default/tasks/foo/T1.task.yaml
```

Or programmatic:

```ts
import { runTask } from '@/codegen/runner'
import { createOllamaClient } from '@/codegen/slm/factory'

await runTask(taskPath, { slmClient: createOllamaClient({ model: 'qwen2.5-coder:14b' }) })
```

## Residual

- **Anthropic / OpenAI clients** — separate FEATs when needed. Factory branch ready.
- **Streaming responses** — rejected for M4b per ADR; revisit if latency becomes a concern.
- **Auth** — Ollama is local-by-default. Remote Ollama with bearer token is a future opt extension.
- **`runTask` CLI doesn't yet honour `--provider` flag** — for now users plug via the TS API or set `MSP_SLM_PROVIDER`. Wiring CLI flag is a small follow-up.

## Sign-off

- Implemented by: @claude-opus-4-7
- Verified by: 14/14 unit tests + manual smoke (factory + mock path)
- Date: 2026-05-03
