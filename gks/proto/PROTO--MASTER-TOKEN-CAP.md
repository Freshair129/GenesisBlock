---
id: PROTO--MASTER-TOKEN-CAP
phase: 2
type: proto
status: draft
severity: error
vault_id: default
tier: safety
source_type: axiomatic
title: PROTO--MASTER-TOKEN-CAP — keep Master atom bodies prompt-injectable (warn
  400 / error 600 tokens, PR-5)
tags: &a1
  - msp
  - proto
  - master
  - token-budget
  - 3-tier
crosslinks: &a2
  enforces:
    - FRAMEWORK--KNOWLEDGE-3-TIER
  references:
    - FRAMEWORK--KNOWLEDGE-3-TIER
    - ADR--MASTER-PROMOTION-DOC-TO-CODE
    - ADR--MASTER-PROMOTION-CONTRADICTION-POLICY
linked_symbols: &a3
  - file: packages/msp/src/validator/proto/master-token-cap.ts
created_at: 2026-05-09T15:06:00.000+07:00
aliases: &a4
  - PROTO
  - implementation_flow
  - Machine-enforced invariant
cluster: implementation_flow
role: Machine-enforced invariant
attributes:
  id: PROTO--MASTER-TOKEN-CAP
  phase: 2
  type: proto
  status: draft
  severity: error
  vault_id: default
  tier: safety
  source_type: axiomatic
  title: PROTO--MASTER-TOKEN-CAP — keep Master atom bodies prompt-injectable (warn
    400 / error 600 tokens, PR-5)
  tags: *a1
  crosslinks: *a2
  linked_symbols: *a3
  created_at: 2026-05-09T15:06:00.000+07:00
  aliases: *a4
  cluster: implementation_flow
  role: Machine-enforced invariant
  attributes:
    id: PROTO--MASTER-TOKEN-CAP
    phase: 2
    type: proto
    status: draft
    severity: error
    vault_id: default
    tier: safety
    source_type: axiomatic
    title: PROTO--MASTER-TOKEN-CAP — keep Master atom bodies prompt-injectable (warn
      400 / error 600 tokens, PR-5)
    tags: *a1
    crosslinks: *a2
    linked_symbols: *a3
    created_at: 2026-05-09T15:06:00.000+07:00
    aliases: *a4
    cluster: implementation_flow
    role: Machine-enforced invariant
    attributes:
      id: PROTO--MASTER-TOKEN-CAP
      phase: 2
      type: proto
      status: draft
      severity: error
      vault_id: default
      tier: safety
      source_type: axiomatic
      title: PROTO--MASTER-TOKEN-CAP — keep Master atom bodies prompt-injectable (warn
        400 / error 600 tokens, PR-5)
      tags: *a1
      crosslinks: *a2
      linked_symbols: *a3
      created_at: 2026-05-09T15:06:00.000+07:00
      aliases: *a4
      cluster: implementation_flow
      role: Machine-enforced invariant
      attributes:
        domain: proto
      domain: proto
      language: markdown
      is_test: false
      is_entrypoint: false
      has_secret: true
      secret_type: high_entropy_string
      leak_risk: high
      encryption_level: none
    domain: proto
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: true
    secret_type: high_entropy_string
    leak_risk: high
    encryption_level: none
  domain: proto
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: true
  secret_type: high_entropy_string
  leak_risk: high
  encryption_level: none
---

# PROTO — MASTER-TOKEN-CAP

## Rule

Every atom with `tier: master` keeps its body within a token budget so it
remains cheap to inject into an agent's system prompt:

| body tokens | severity |
|---|---|
| ≤ 400 | OK |
| > 400 | `warning` |
| > 600 | `error` |

The token count is a heuristic — whitespace-split words multiplied by
`1.3` (a conservative ratio for BPE-style tokenisers that most production
LLMs use):

```
token_count = body.split(/\s+/).filter(Boolean).length * 1.3
```

## Schema

Reads `atomicIndex: AtomicIndexEntry[]` from the `PredicateContext`. For
each entry whose `tier === 'master'`, reads the file at
`<repoRoot>/gks/<entry.path>`, strips the leading YAML frontmatter, and
counts whitespace-separated words. The frontmatter does NOT count toward
the budget — only the body that the loader will inject.

## Predicate

```ts
for (const entry of atomicIndex) {
  if (entry.tier !== 'master') continue
  const body = stripFrontmatter(await readFile(resolve(gksRoot, entry.path)))
  const tokens = body.split(/\s+/).filter(Boolean).length * 1.3
  if (tokens > 600) violations.push({ severity: 'error', ... })
  else if (tokens > 400) violations.push({ severity: 'warning', ... })
}
```

Implementation: `src/validator/proto/master-token-cap.ts`.

## Trigger

`msp:validate --all` (runs after the regular validator rules, alongside
other PROTO predicates).

## Severity

`error` — exceeding 600 tokens fails the build because Master atoms are
loaded into every relevant session and over-budget entries blow up
prompt cost across the whole MSP fleet. A warning at 400 nudges authors
to rewrite before they cross the hard cap.

## Status

`draft` — gradual rollout. Once the first batch of Master atoms (PR-5
onward) is live and the loader (PR-6) confirms the budget holds in
practice, this PROTO promotes to `stable`.

## Source

`[[FRAMEWORK--KNOWLEDGE-3-TIER]]` (Master Block § "Token cap"); PR-5 of the
3-tier rollout plan.

## Connections

- [[ADR--MASTER-PROMOTION-DOC-TO-CODE]]
- [[ADR--MASTER-PROMOTION-CONTRADICTION-POLICY]]
