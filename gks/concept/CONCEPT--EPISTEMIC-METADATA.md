---
id: CONCEPT--EPISTEMIC-METADATA
phase: 1
type: concept
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: Epistemic metadata — confidence + source_type + duration on every claim
tags:
  - msp
  - epistemic
  - metadata
  - anti-hallucination
crosslinks:
  references:
    - CONCEPT--ATOMIC-WRITE-CONTRACT
created_at: 2026-05-03T14:01:54.801+07:00
aliases:
  - CONCEPT
  - implementation_flow
  - Strategic intent / PRD
cluster: implementation_flow
role: Strategic intent / PRD
attributes:
  domain: concept
---

# CONCEPT — epistemic metadata

When an atom makes a substantive claim — especially an ADR, a protocol, or any inference about how the system behaves — it carries an `epistemic` block declaring *how well-founded the claim is*. This forces agents to mark their guesses as guesses and prevents inference rot.

## Block shape

```yaml
epistemic:
  confidence: 0.85                    # 0.0..1.0
  source_type: documented_source      # see table below
  duration: permanent                 # permanent | temporary | deprecated
```

If `duration: temporary`, the atom must also carry a `valid_until: <ISO-date>` — the validator rejects temporary atoms without an expiry.

## `source_type` enum

| Value | Meaning |
|---|---|
| `direct_experience` | author observed the behaviour first-hand (saw the test pass) |
| `documented_source` | author cited a doc, ADR, or external spec (and the wikilink resolves) |
| `inferred` | author reasoned from other atoms — `confidence` should reflect uncertainty |
| `hypothesis` | author is proposing without verification yet — usually `confidence < 0.5` |
| `external` | source is outside the repo (vendor docs, RFC, paper) — body must include URL |
| `axiom` | foundational; needs no source (e.g. "the validator runs as a CLI") |

## When the validator requires it

| `type` | Required? |
|---|---|
| `adr` | yes |
| `protocol` | yes |
| `concept` | optional but recommended |
| `feat`, `blueprint`, `frame` | optional |
| `microtask`, `audit`, `idea` | not required |

(Conditional-fields rule, currently soft warning in M2 → hard rule in M3 once `atomic_contract.yaml` is loaded at runtime.)

## Why per-atom, not per-statement

Per-statement would be cleaner but unworkable — each ADR would balloon with markers. Per-atom puts the burden on the author to make atoms *small enough* that one confidence rating is honest. Atoms larger than ~500 words usually deserve splitting anyway.

## Source

`msp_spec.md` §4.6 (Epistemic & Crosslinks Block).

## Connections
- [[CONCEPT--ATOMIC-WRITE-CONTRACT]]

