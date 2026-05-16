---
id: ADR--CODEGEN-RETRY-POLICY
phase: 2
type: adr
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: Codegen retry policy — max 3 SLM then Gemini then Opus
tags:
  - msp
  - codegen
  - retry
  - slm
  - escalation
crosslinks: {"references":["CONCEPT--CODEGEN-MICROTASK-CONTRACT","ADR--CODEGEN-FORBIDDEN-PATTERNS"]}
created_at: 2026-05-03T14:08:42.580+07:00
---

# ADR — codegen retry policy

## Context

When SLM output fails the codegen contract (forbidden pattern, missing required pattern, failing acceptance test), we have to decide:

- How many retries before giving up?
- What context to feed the SLM on retry?
- Where to escalate when the SLM clearly can't solve it?

Without an explicit policy, agents either retry forever (cost overrun) or give up after one failure (high false-give-up rate).

## Decision

```yaml
max_retries: 3
include_failed_test_in_next_prompt: true
include_forbidden_pattern_match: true
strip_previous_attempt_from_ctx: true
```

### Per-retry behaviour

1. **Strip previous attempt** from context before the next prompt. Keeping it in increases temptation to repeat the same mistake (autoregression).
2. **Include the failure signal**: "Your previous output failed on test `X` with error `Y`" or "Your previous output contained forbidden pattern `Z`".
3. **Don't include the original prompt verbatim**; re-render with the failure injected as a new acceptance criterion.

### Escalation ladder

```
SLM (Qwen 2.5 Coder)
   │  fail × 3
   ▼
Gemini (T2)            ← single attempt; bigger context window, can reason about why SLM failed
   │  fail
   ▼
Opus (T3) review       ← human-in-the-loop; either fix or restate the task
```

Escalation is invoked via `scripts/msp/escalate-to-gemini.mjs <task_id>` (script TBD in M3).

### Why exactly 3 retries

- 1 retry: too tight. Many SLM failures are formatting glitches the model self-corrects with one feedback round.
- 5+ retries: cost dominates. Empirically, retries beyond 3 succeed < 10% of the time on the task families we see.
- 3 is the empirical sweet spot per the source spec.

## Consequences

**Positive**
- Bounded cost per task (≤ 4 SLM calls + 1 Gemini call + optional Opus).
- Escalation produces a paper trail (`escalate-to-gemini` writes an audit entry).
- The fresh-context retry breaks autoregression failure modes.

**Negative**
- Stripping previous attempts means the SLM doesn't learn within a session — but SLM in-context learning is unreliable anyway.
- 3 retries × multiple tasks per microtask file can rack up costs. Monitor via per-task cost tracker.

## Alternatives considered

1. **Adaptive retry count.** Considered. Better SLMs (when we upgrade to Qwen 3) might warrant 2 retries; weaker ones, 4. Defer until we have empirical data per model.
2. **Skip Gemini, escalate straight to Opus.** Rejected. Gemini's 2M context window handles large blueprint geographies cheaper than Opus.
3. **Include all previous attempts.** Rejected — see autoregression note above.

## Source

`msp_spec.md` §5.5 (Retry Policy).

## Connections
- [[CONCEPT--CODEGEN-MICROTASK-CONTRACT]]
- [[ADR--CODEGEN-FORBIDDEN-PATTERNS]]

