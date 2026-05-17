---
id: BLUEPRINT--SLM-OLLAMA-CLIENT
phase: 3
type: blueprint
scale_level: L2
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: BLUEPRINT — Ollama SLM client implementation plan
tags:
  - msp
  - codegen
  - slm
  - ollama
  - blueprint
  - implementation
crosslinks: {"implements":["FEAT--SLM-OLLAMA-CLIENT"],"references":["ADR--SLM-OLLAMA-CLIENT"]}
linked_symbols:
  - {"file":"packages/msp/src/codegen/slm/ollama.ts"}
  - {"file":"packages/msp/src/codegen/slm/factory.ts"}
  - {"file":"packages/msp/src/codegen/slm/types.ts"}
  - {"file":"packages/msp/src/codegen/slm/errors.ts"}
created_at: 2026-05-03T16:22:31.964+07:00
aliases:
  - BLUEPRINT
  - implementation_flow
  - Implementation plan
cluster: implementation_flow
role: "Implementation plan"
---

# BLUEPRINT — Ollama SLM client

```yaml
metadata:
  title: "Ollama SLM client"
  parent_feat: FEAT--SLM-OLLAMA-CLIENT

architectural_pattern: |
  Three pure modules:
    - errors.ts   SlmError class + kinds enum
    - ollama.ts   createOllamaClient(opts?) factory; closes over config
    - factory.ts  createSlmClient(opts?) — provider switch
    - types.ts    re-exports SlmClient from runner for short import paths

  Each `createXClient` returns a closure of type SlmClient. Runner stays
  oblivious to which provider it has.

data_logic: |
  createOllamaClient({ host?, model?, fetchImpl?, timeoutMs?, temperature? }):
    resolves config:
      host  = opts.host  ?? process.env.OLLAMA_HOST  ?? 'http://127.0.0.1:11434'
      model = opts.model ?? process.env.OLLAMA_MODEL ?? 'qwen2.5-coder:7b'
      f     = opts.fetchImpl ?? globalThis.fetch
      timeoutMs   = opts.timeoutMs   ?? 120_000
      temperature = opts.temperature ?? 0.2
    returns:
      async ({ prompt }: SlmCall) => {
        const ac = AbortSignal.timeout(timeoutMs)
        const body = JSON.stringify({ model, prompt, stream: false,
                                       options: { temperature } })
        let resp
        try {
          resp = await f(`${host}/api/generate`, {
            method: 'POST',
            headers: { 'content-type': 'application/json' },
            body, signal: ac,
          })
        } catch (err) {
          if (ac.aborted) throw new SlmError('timeout', 'timeout')
          throw new SlmError(`ollama network: ${err.message}`, 'network', err)
        }
        if (!resp.ok) {
          const text = await resp.text().catch(() => '')
          throw new SlmError(`ollama http ${resp.status}: ${text}`, 'http')
        }
        let json
        try { json = await resp.json() }
        catch (err) { throw new SlmError('ollama parse', 'parse', err) }
        if (typeof json.response !== 'string') {
          throw new SlmError('ollama response missing string field', 'parse')
        }
        return json.response
      }

  createSlmClient({ provider? }):
    p = provider ?? process.env.MSP_SLM_PROVIDER ?? 'ollama'
    switch (p) {
      case 'ollama': return createOllamaClient(...)
      case 'mock':   return defaultMock from runner
      default:       throw new Error(`unknown SLM provider: ${p}`)
    }

geography:
  - "packages/msp/src/codegen/slm/errors.ts"
  - "packages/msp/src/codegen/slm/ollama.ts"
  - "packages/msp/src/codegen/slm/factory.ts"
  - "packages/msp/src/codegen/slm/types.ts"
  - "packages/msp/test/codegen/slm/ollama.test.ts"
  - "packages/msp/test/codegen/slm/factory.test.ts"

api_contracts:
  - name: createOllamaClient
    signature: |
      function createOllamaClient(opts?: OllamaOpts): SlmClient
    types: |
      interface OllamaOpts {
        host?: string             // default OLLAMA_HOST or http://127.0.0.1:11434
        model?: string            // default OLLAMA_MODEL or qwen2.5-coder:7b
        fetchImpl?: typeof fetch  // injectable transport for tests
        timeoutMs?: number        // default 120_000
        temperature?: number      // default 0.2
      }
      type SlmErrorKind = 'network' | 'http' | 'parse' | 'timeout'
      class SlmError extends Error {
        constructor(message: string, kind: SlmErrorKind, cause?: unknown)
      }

  - name: createSlmClient
    signature: |
      function createSlmClient(opts?: SlmFactoryOpts): SlmClient
    types: |
      interface SlmFactoryOpts {
        provider?: 'ollama' | 'mock'
        ollama?: OllamaOpts
      }

verification_plan:
  - vitest with injected fetchImpl returning 200 + valid JSON → expected response string
  - vitest with injected fetchImpl returning 500 → SlmError kind 'http'
  - vitest with injected fetchImpl that throws → SlmError kind 'network'
  - vitest with fetchImpl that returns malformed JSON → SlmError kind 'parse'
  - vitest with timeout (very small timeoutMs + slow fetchImpl) → SlmError kind 'timeout'
  - factory: 'ollama' returns ollama client; 'mock' returns mock; unknown throws
  - env-var fallbacks: OLLAMA_HOST/OLLAMA_MODEL/MSP_SLM_PROVIDER honoured
  - integration via runner: runTask(path, { slmClient: ollama }) with mock fetch
    returning a known-good string → exit 0
  - **no real network in any test** (CI doesn't require Ollama)
```

## Implementation order

T1 OLLAMA-HTTP-CLIENT (ollama.ts + errors.ts)
T2 FACTORY (factory.ts switch + types.ts re-exports)
T3 ENV-CONFIG (env-var defaults; tested via process.env mocking)

## Connections
- [[FEAT--SLM-OLLAMA-CLIENT]]
- [[ADR--SLM-OLLAMA-CLIENT]]

