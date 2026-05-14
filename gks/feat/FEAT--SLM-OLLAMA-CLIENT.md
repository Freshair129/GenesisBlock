---
id: FEAT--SLM-OLLAMA-CLIENT
phase: 2
type: feat
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: createOllamaClient + createSlmClient factory — pluggable real SLM
tags:
  - msp
  - codegen
  - slm
  - ollama
  - user-facing
crosslinks: {"implements":["ADR--SLM-OLLAMA-CLIENT"],"references":["CONCEPT--SLM-OLLAMA-CLIENT","FEAT--CODEGEN-MICROTASK-RUNNER"]}
linked_symbols:
  - {"file":"packages/msp/src/codegen/slm/ollama.ts"}
  - {"file":"packages/msp/src/codegen/slm/factory.ts"}
  - {"file":"packages/msp/src/codegen/slm/types.ts"}
created_at: 2026-05-03T16:22:31.444+07:00
---

# FEAT — Ollama SLM client

## User-facing behaviour

Pull the model once:

```sh
ollama pull qwen2.5-coder:7b
```

Then run a real microtask via the runner:

```ts
import { runTask } from '@/codegen/runner'
import { createSlmClient } from '@/codegen/slm/factory'

const result = await runTask('.brain/default/tasks/foo/T1.task.yaml', {
  slmClient: createSlmClient({ provider: 'ollama' }),
})
```

Or via env var (factory inspects):

```sh
MSP_SLM_PROVIDER=ollama npm run msp:run-task -- T1.task.yaml
```

## Acceptance criteria

- [ ] `createOllamaClient()` returns a `SlmClient` honouring `OLLAMA_HOST` + `OLLAMA_MODEL` env defaults
- [ ] Override-able via `opts: { host, model, fetchImpl, signal, timeoutMs, temperature }`
- [ ] POSTs to `<host>/api/generate` with `{model, prompt, stream:false, options:{temperature}}`
- [ ] Returns the `response` string from the JSON body
- [ ] Network error → throws `SlmError(kind:'network')` with original cause
- [ ] HTTP non-2xx → throws `SlmError(kind:'http')` with status + body
- [ ] Malformed JSON response → throws `SlmError(kind:'parse')`
- [ ] Timeout exceeded → throws `SlmError(kind:'timeout')`
- [ ] `createSlmClient({ provider:'ollama' })` returns the same Ollama client
- [ ] `createSlmClient()` with no opts and `MSP_SLM_PROVIDER=ollama` env returns Ollama client
- [ ] `createSlmClient({ provider:'mock' })` returns the runner's deterministic mock (for tests)
- [ ] Unknown provider → throws with a clear message
- [ ] Unit tests cover all error kinds + happy path with injected `fetchImpl`
- [ ] **No real network in tests** (CI must not require Ollama running)

## Surfaces

| Surface | Form |
|---|---|
| TS API | `createOllamaClient(opts?): SlmClient`, `createSlmClient(opts?): SlmClient` |
| Env vars | `OLLAMA_HOST`, `OLLAMA_MODEL`, `MSP_SLM_PROVIDER` |
| Runner integration | `runTask(path, { slmClient })` accepts the client |

## Out of scope

- Anthropic / OpenAI clients — separate FEATs when needed.
- Streaming responses.
- Model-pull automation (user runs `ollama pull` themselves).
