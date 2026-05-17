---
id: BLUEPRINT--COST-TRACKING
phase: 3
type: blueprint
status: draft
vault_id: default
tier: genesis
source_type: axiomatic
title: BLUEPRINT — Cost Tracking — pricing math + USAGE-atom recorder + dispatch
  wiring
scale_level: feature
tags:
  - msp
  - agents
  - cost
  - usage
  - blueprint
crosslinks:
  references:
    - CONCEPT--COST-TRACKING
    - SPEC--USAGE-ATOM
    - SPEC--EPISODE-ATOM
    - BLUEPRINT--AGENT-DISPATCHER
    - ADR--AGENT-TIER-COST-POLICY
linked_symbols:
  - file: packages/msp/src/agents/cost-tracker.ts
  - file: packages/msp/src/agents/usage-recorder.ts
  - file: packages/msp/src/agents/dispatch.ts
created_at: 2026-05-14T03:46:00.000+07:00
aliases:
  - BLUEPRINT
  - implementation_flow
  - Implementation plan
cluster: implementation_flow
role: Implementation plan
attributes:
  domain: blueprint
---

# BLUEPRINT — Cost Tracking

Concrete plan to ship the cost-tracking layer described in `[[CONCEPT--COST-TRACKING]]` and the `[[USAGE--DAILY]]-*` atom contract from `[[SPEC--USAGE-ATOM]]`. Phase E3 of the MSP roadmap.

## File layout

```
packages/msp/src/agents/
├── cost-tracker.ts          # NEW — pure pricing math
├── usage-recorder.ts        # NEW — daily-bucket USAGE atom writer
├── dispatch.ts              # EDIT — populate cost_usd + call recordUsage()
├── result-recorder.ts       # unchanged
└── ...
```

## `cost-tracker.ts` — pure pricing math

```typescript
export interface PricingRow {
  tier: Tier
  per_million_input_usd: number
  per_million_output_usd: number
}

export const PRICING: PricingRow[] = [
  { tier: 'T1', per_million_input_usd: 0,     per_million_output_usd: 0 },
  { tier: 'T2', per_million_input_usd: 0.075, per_million_output_usd: 0.30 },
  { tier: 'T3', per_million_input_usd: 3.00,  per_million_output_usd: 15.00 },
]

export function estimateCost(tier: Tier, input_tokens: number, output_tokens: number): number
export function estimateTokens(text: string): number
```

- **No dependencies.** Pricing table is a const literal, dated by code-comment at top of file.
- **`estimateTokens`** uses `Math.ceil(text.length / 4)` — the standard rough heuristic. Cheap, predictable, off by ~10-20% from real tokenizers — acceptable for ballpark.
- **`estimateCost`** is `(input_tokens * row.per_million_input_usd + output_tokens * row.per_million_output_usd) / 1_000_000`. Returns 0 for T1 always.

## `usage-recorder.ts` — daily-bucket atom

Public surface:

```typescript
export async function recordUsage(
  episode: { tier: Tier; cost_usd: number; episode_id?: string },
  root: string,
): Promise<string>
```

Implementation outline:

1. Compute today's ISO date in UTC (`new Date().toISOString().slice(0, 10)`).
2. Target path: `<root>/gks/usage/[[USAGE--DAILY]]-<isoDate>.md`.
3. If file does not exist:
   - `mkdir -p` the parent.
   - Build frontmatter per `[[SPEC--USAGE-ATOM]]` §3.
   - Build body with summary block (markers + initial JSON) per `[[SPEC--USAGE-ATOM]]` §4.
   - Write file.
4. If file exists:
   - Read it.
   - Find the JSON block between `<!-- USAGE-SUMMARY-START -->` and `<!-- USAGE-SUMMARY-END -->`.
   - Parse, mutate totals + by_tier + top_episodes (keep top 5), bump `updated_at`.
   - Rewrite the JSON block in place. Frontmatter stays.
5. Return absolute path.

**Errors propagate.** `dispatch.ts` wraps the call in try/catch and treats it as best-effort, matching the `recordEpisode` pattern.

## `dispatch.ts` — wiring

Additive edit only. After building `runResult` and before constructing the `DispatchResult`:

```typescript
const inputTokens = estimateTokens(task.prompt)
const outputTokens = estimateTokens(runResult.output)
const cost_usd = estimateCost(tier, inputTokens, outputTokens)
```

Add `cost_usd` to the result object. Then, parallel to `recordEpisode()`:

```typescript
try {
  await recordUsage({ tier, cost_usd }, process.cwd())
} catch (err) {
  process.stderr.write(`[dispatch] recordUsage failed: ...\n`)
}
```

`DispatchTask` / `DispatchResult` public shapes are NOT changed — `cost_usd` is already optional on `DispatchResult`.

## Validator changes

`msp/LLM_Contract/atomic_contract.yaml` — add `usage` to `required_fields.by_type`:

```yaml
usage:
  - id
  - phase
  - type
  - status
  - title
  - created_at
  - tags
```

## Tests

- `test/agents/cost-tracker.test.ts` — `estimateCost` / `estimateTokens` math, T1=0, T2 ratio, T3 ratio.
- `test/agents/usage-recorder.test.ts` — first-call creates file with frontmatter; second-call same day updates same file; JSON summary block is parseable.
- `test/agents/dispatch.test.ts` — new cases: T2 cost > 0; T1 cost === 0.

## Out of scope

- Hourly / weekly buckets (SPEC reserves the name space; no code yet).
- Real tokenizers per tier.
- Cost-based escalation policy (that would belong in `cost-policy.ts`, not here).
- Cross-vault aggregation.

## Connections
- [[SPEC--EPISODE-ATOM]]
- [[BLUEPRINT--AGENT-DISPATCHER]]
- [[ADR--AGENT-TIER-COST-POLICY]]

