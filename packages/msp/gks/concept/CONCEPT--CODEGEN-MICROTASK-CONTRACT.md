---
id: CONCEPT--CODEGEN-MICROTASK-CONTRACT
phase: 1
type: concept
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: Codegen microtask contract — what SLM output must obey
tags:
  - msp
  - codegen
  - slm
  - microtask
  - contract
crosslinks: {"references":["FRAME--PHASE-GOVERNANCE","CONCEPT--ATOMIC-WRITE-CONTRACT"]}
created_at: 2026-05-03T07:01:52.308Z
---

# CONCEPT — codegen microtask contract

T1 SLMs (Qwen 2.5 Coder, Llama local) execute `T*.task.yaml` files in `gks/microtasks/`. Their output is unreliable in predictable ways: extra prose, fenced markdown wrapping, hallucinated imports, `console.log` debug, `req.body` Pages-Router idioms, `// TODO` punts. The codegen contract is the deterministic post-processor + pattern-checker that catches these before the acceptance test runs.

## Pipeline

```
T*.task.yaml  ──▶  SLM (Qwen 2.5 Coder)  ──▶  raw output
                                              │
                                              ▼
                                    post-processing (strip)
                                              │
                                              ▼
                                  forbidden-imports check
                                              │
                                              ▼
                                  forbidden-patterns check
                                              │
                                              ▼
                                  required-patterns per slot
                                              │
                                              ▼
                                       acceptance test
                                              │
                              fail (≤ 3 retries)│ pass
                                       ──────────▶  src/
                                              │
                                       (all retries fail)
                                              │
                                              ▼
                                escalate to Gemini → fallback Opus review
```

## What gets stripped (post-processing)

- Markdown fences (` ``` `)
- Leading commentary before `import`/`export`/`const`
- Trailing commentary after the last `}`
- Mixed line endings → `\n`

## What gets rejected (forbidden patterns)

Hard rejects (fail acceptance test): `export default`, `req.body`, `req.tenantId`, `// TODO`, `// FIXME`, `// XXX`.
Soft warns: `console.*`, `process.env.*` at route level.

See `ADR--CODEGEN-FORBIDDEN-PATTERNS` for the full table with rationale per pattern.

## What gets required (per slot)

| Slot | Must contain |
|---|---|
| `exports` | `export const POST =` (or named export per HTTP verb) |
| `handler` | `export async function` or `export const = async` |
| `helpers` | first non-comment line is `export function` or `export const` |

See `ADR--CODEGEN-REQUIRED-PATTERNS`.

## Retry policy

≤ 3 retries with the failed test + matched forbidden pattern fed back into the next prompt. Previous attempt is stripped from context (fresh start each retry). Beyond 3 → escalate to Gemini; beyond Gemini → Opus review.

See `ADR--CODEGEN-RETRY-POLICY`.

## Why the contract is separate from the atomic contract

The atomic contract governs *frontmatter*; the codegen contract governs *generated source code*. They run at different boundaries (write to `gks/` vs write to `src/`) and have different rule shapes (YAML schema vs regex+lint).

## Source

`msp_spec.md` §5 (Codegen Micro-task Contract).
