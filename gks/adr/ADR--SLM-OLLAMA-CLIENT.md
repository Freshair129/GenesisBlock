---
id: ADR--SLM-OLLAMA-CLIENT
phase: 2
type: adr
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: Ollama via fetch with env-var config + injectable transport
tags:
  - msp
  - codegen
  - slm
  - ollama
  - decision
crosslinks:
  references:
    - CONCEPT--SLM-OLLAMA-CLIENT
    - ADR--CODEGEN-RETRY-POLICY
created_at: 2026-05-03T16:22:30.900+07:00
aliases:
  - ADR
  - implementation_flow
  - Architecture decision record
cluster: implementation_flow
role: Architecture decision record
attributes:
  domain: adr
---

# ADR — Ollama client shape

## Context

We need a real SLM provider. Three live options:

1. **Ollama** — local HTTP, no key, supports Qwen 2.5 Coder.
2. **Anthropic API** — production-grade, costs money + key.
3. **OpenAI / Mistral via OpenRouter** — one API surface, multiple models, costs.

For first cut, Ollama wins on friction (zero auth, free, runs Qwen). We need a shape that lets the other two land later without churn.

## Decision

### One concrete client + one factory

- `src/codegen/slm/ollama.ts` exports `createOllamaClient(opts?): SlmClient`.
- `src/codegen/slm/factory.ts` exports `createSlmClient(opts?): SlmClient` that picks based on env or explicit `provider`. For M4b: only `ollama` is implemented; future clients land next to it.
- `src/codegen/slm/types.ts` re-exports `SlmClient` from the runner (keeps import paths short).

### Transport via global `fetch`

- Node 22 + Node 20 both ship `fetch`. No `node-fetch`, no `axios`, no extra deps.
- Tests inject a custom `fetch` via `opts.fetchImpl` so we never hit network in CI.

### Env-driven defaults, override-friendly

```
OLLAMA_HOST   default http://127.0.0.1:11434
OLLAMA_MODEL  default qwen2.5-coder:7b
```

`createOllamaClient({ host, model, fetchImpl, signal })` overrides any of the above. Useful for parallel runners targeting different models.

### Error shape

```ts
class SlmError extends Error {
  constructor(message: string, public readonly kind: 'network' | 'http' | 'parse' | 'timeout') { ... }
}
```

`runner.ts` doesn't try to recover (any SLM failure → record on the attempt + retry). The error class exists for callers / observability.

### Request body

```json
{
  "model":  "<model>",
  "prompt": "<full prompt>",
  "stream": false,
  "options": { "temperature": 0.2 }
}
```

Single round-trip; collect `response` field from JSON. Streaming (`stream: true`) is rejected for now — adds complexity for marginal latency win at our scale.

### Timeout

Default 120 s via `AbortSignal.timeout(120_000)`. Tunable via `opts.timeoutMs`. SLM failures surface as `SlmError(kind: 'timeout')`; runner records and retries.

## Consequences

**Positive**
- Zero new runtime deps (`fetch` is built in).
- Pluggable — adding Anthropic later is `createAnthropicClient` + factory branch.
- Tests inject `fetchImpl`; no network in CI.
- Same exit-code contract from the runner; users don't see a difference between mock and Ollama paths.

**Negative**
- Ollama-only for M4b. Anthropic / OpenAI users must wait or wire their own client (the `slmClient` opt accepts any `SlmClient`).
- Default model `qwen2.5-coder:7b` requires the user to `ollama pull` it before first use. README must document.

## Alternatives considered

1. **Bundle multiple providers in one client.** Rejected — fat module, hard to tree-shake.
2. **Use the official `ollama` npm package.** Considered. It's a thin wrapper over the same HTTP API; using `fetch` directly avoids a dep + version-skew risk.
3. **Streaming responses.** Rejected for M4b — single round-trip is simpler and the runner consumes the full output anyway.

## Source

`[[CONCEPT--SLM-OLLAMA-CLIENT]]` + Ollama HTTP API docs (`/api/generate` endpoint).

## Connections
- [[ADR--CODEGEN-RETRY-POLICY]]

