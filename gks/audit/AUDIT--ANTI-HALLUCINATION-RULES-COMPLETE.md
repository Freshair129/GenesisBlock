---
id: AUDIT--ANTI-HALLUCINATION-RULES-COMPLETE
phase: 6
type: audit
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: M5c — anti-hallucination rules complete (3/6 → 6/6)
tags: &a1
  - msp
  - m5
  - m5c
  - audit
  - validator
  - anti-hallucination
crosslinks: &a2
  references:
    - ADR--ANTI-HALLUCINATION-RULES
    - FEAT--MSP-VALIDATOR
linked_symbols: &a3
  - file: packages/msp/src/validator/rules/no-invented-versions.ts
  - file: packages/msp/src/validator/rules/evidence-for-decisions.ts
  - file: packages/msp/src/validator/rules/cite-or-mark-inferred.ts
  - file: packages/msp/src/validator/index.ts
created_at: 2026-05-03T18:01:45.959+07:00
aliases: &a4
  - AUDIT
  - implementation_flow
  - Test results / quality report
cluster: implementation_flow
role: Test results / quality report
attributes:
  id: AUDIT--ANTI-HALLUCINATION-RULES-COMPLETE
  phase: 6
  type: audit
  status: stable
  vault_id: default
  tier: process
  source_type: axiomatic
  title: M5c — anti-hallucination rules complete (3/6 → 6/6)
  tags: *a1
  crosslinks: *a2
  linked_symbols: *a3
  created_at: 2026-05-03T18:01:45.959+07:00
  aliases: *a4
  cluster: implementation_flow
  role: Test results / quality report
  attributes:
    id: AUDIT--ANTI-HALLUCINATION-RULES-COMPLETE
    phase: 6
    type: audit
    status: stable
    vault_id: default
    tier: process
    source_type: axiomatic
    title: M5c — anti-hallucination rules complete (3/6 → 6/6)
    tags: *a1
    crosslinks: *a2
    linked_symbols: *a3
    created_at: 2026-05-03T18:01:45.959+07:00
    aliases: *a4
    cluster: implementation_flow
    role: Test results / quality report
    attributes:
      id: AUDIT--ANTI-HALLUCINATION-RULES-COMPLETE
      phase: 6
      type: audit
      status: stable
      vault_id: default
      tier: process
      source_type: axiomatic
      title: M5c — anti-hallucination rules complete (3/6 → 6/6)
      tags: *a1
      crosslinks: *a2
      linked_symbols: *a3
      created_at: 2026-05-03T18:01:45.959+07:00
      aliases: *a4
      cluster: implementation_flow
      role: Test results / quality report
      attributes:
        domain: audit
      domain: audit
      language: markdown
      is_test: false
      is_entrypoint: false
      has_secret: true
      secret_type: high_entropy_string
      leak_risk: high
      encryption_level: none
    domain: audit
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: true
    secret_type: high_entropy_string
    leak_risk: high
    encryption_level: none
  domain: audit
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: true
  secret_type: high_entropy_string
  leak_risk: high
  encryption_level: none
---

# AUDIT — anti-hallucination rules complete

## Scope

Closes the 3-of-6 gap from `[[ADR--ANTI-HALLUCINATION-RULES]]`. M2 shipped `dangling-wikilinks`, `adr-monotonic`, `no-future-dates`. M5c lands the remaining three.

## Rules added

| Rule | Severity | What |
|---|---|---|
| `no-invented-versions` | error | semver only; first draft must be `0.1.0`; existing atoms can bump |
| `evidence-for-decisions` | error | ADR body must contain `## Context`, `## Decision`, `## Consequences` headings (case-insensitive) |
| `cite-or-mark-inferred` | warning | body claims about `src/`, `lib/`, `app/`, `scripts/` paths require either matching `linked_symbols` OR `epistemic.source_type=inferred` + `confidence < 1.0` |

`cite-or-mark-inferred` was added as a **soft** rule (warning) per the spec — `SOFT_RULES` list runs after hard rules; outputs always recorded as warnings even if the rule's own emit marks them `error`.

## Test summary

```
test/validator/rules/no-invented-versions.test.ts:    8/8 passing
test/validator/rules/evidence-for-decisions.test.ts:  5/5 passing
test/validator/rules/cite-or-mark-inferred.test.ts:   9/9 passing
total: 22/22
```

## Bugs found during dogfood

1. **Greedy regex** in `cite-or-mark-inferred`'s PATH_RE wouldn't match `src/foo.ts` (extension consumed by greedy `.+`). Replaced with `[A-Za-z0-9_-]+(/[A-Za-z0-9_-]+)*\.ext` that excludes the extension from the path part.

2. **Strip-suffix regex was eating `.ts` extension**. The `:fn|:line` strip used `[:.]` character class — accidentally matched extension dot. Fixed to `(?::fn)?(?::line)?`.

Both fixed during the same M5c session; recorded here per the M2/M4a/M5a precedent of in-line "Bugs found" sections.

## Coverage

All 6 rules from `[[ADR--ANTI-HALLUCINATION-RULES]]` now implemented:

| Rule | Status |
|---|---|
| dangling-wikilinks | ✅ M2 |
| adr-monotonic | ✅ M2 |
| no-future-dates | ✅ M2 |
| no-invented-versions | ✅ M5c |
| evidence-for-decisions | ✅ M5c |
| cite-or-mark-inferred | ✅ M5c (soft) |

## Sign-off

- Implemented by: @claude-opus-4-7
- Verified by: 22/22 unit tests + 78/78 atoms still validate
- Date: 2026-05-03

## Connections

- [[FEAT--MSP-VALIDATOR]]
