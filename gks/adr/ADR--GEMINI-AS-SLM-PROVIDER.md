---
id: ADR--GEMINI-AS-SLM-PROVIDER
phase: 2
type: adr
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: Gemini CLI is a first-class SLM provider, not only an escalator
tags:
  - msp
  - codegen
  - slm
  - gemini
  - decision
crosslinks: {"references":["ADR--DEFAULT-SLM-OLLAMA-QWEN-CODER","CONCEPT--CODEGEN-MICROTASK-RUNNER"]}
created_at: 2026-05-12T22:47:00.000+07:00
aliases:
  - ADR
  - implementation_flow
  - Architecture decision record
cluster: implementation_flow
role: "Architecture decision record"
---

# ADR — Gemini CLI as a first-class SLM provider

## Context

`packages/msp/src/codegen/escalator/gemini.ts` invokes `gemini -p <prompt> -y` after Ollama exhausts retries. The same pattern is useful as the **primary** SLM for callers who prefer Gemini's hosted model over a local Ollama install — e.g. cloud CI runners without GPU, or T2/T3-tier work per `FRAMEWORK_MASTER_SPEC` §17.3.

Today the factory at `slm/factory.ts` knows three providers: `ollama` / `qwen` / `mock`. Gemini is conspicuously absent from this list despite the escalator already containing the subprocess wrapper.

## Decision

Add `'gemini'` to the `SlmFactoryOpts.provider` union. The new `slm/gemini.ts` exports two functions:

- `createGeminiClient(opts)` — returns an `SlmClient` for the codegen runner.
- `runGeminiCli(prompt, opts)` — shared subprocess wrapper (env-var aware, timeout/maxBuffer caps, error mapping). The existing escalator is refactored to consume this helper so both call paths stay in sync.

Selection knobs:
- `MSP_SLM_PROVIDER=gemini` — promote Gemini to the primary SLM.
- `GEMINI_BIN=<path>` — override the binary path (default `gemini`).
- `GEMINI_MODEL=<id>` — passed to the CLI via `-m`.

The fallback chain post-PR:

```
attempt 1-3  : MSP_SLM_PROVIDER     (default ollama / qwen2.5-coder)
escalator    : Gemini CLI           (gemini -p ... -y)
human gate   : Opus layer           (exit code 4)
```

## Consequences

### Positive
- No new tooling required — `gemini-cli` (Google's official CLI) is the only dependency, already available in most Claude/Gemini multi-agent setups.
- Symmetric with the Ollama path — both providers go through `createSlmClient({})`, so `runTask` doesn't care which one resolves.
- One subprocess wrapper, two callers — removes the drift risk between primary and escalator paths.

### Negative
- Hosted Gemini introduces a token cost; not appropriate for high-volume T1 microtasks. Default stays on local Ollama (per `[[ADR--DEFAULT-SLM-OLLAMA-QWEN-CODER]]`).
- CLI dependency: `gemini --version` must exist on PATH when the provider is enabled. The new `SlmError('config')` makes the failure mode explicit.

### Neutral
- The escalator still defaults to Gemini even when Gemini is also the primary SLM. If Gemini-as-primary fails 3 times, escalating to Gemini-again is fine — the spec leaves this knob to the caller; we don't try to be clever.

## Status

Draft. Promotion to `stable` requires green CI on Node 20 + 22 with the new `test/codegen/slm/gemini.test.ts` passing.

## Connections
- [[CONCEPT--CODEGEN-MICROTASK-RUNNER]]

