---
id: CONCEPT--MASTER-PRIORITY-SECTORS
phase: 1
type: concept
status: draft
tier: genesis
source_type: axiomatic
vault_id: default
title: Master priority sectors — P0–P4 context budget partition for CLAUDE.md
tags:
  - msp
  - master
  - priority
  - sectors
  - context-budget
  - claude-md
  - foundation
crosslinks:
  references:
    - FRAMEWORK--KNOWLEDGE-3-TIER
    - SPEC--GENESIS-BLOCK-MANIFEST
    - CONCEPT--TAXONOMY-V2-3
    - MASTER--MSP-DOC-TO-CODE
    - MASTER--ATOM-CONTRADICTION-POLICY
    - MASTER--ROOT-CAUSE-ANALYSIS
created_at: 2026-05-17T02:30:00.000+07:00
aliases:
  - CONCEPT
  - implementation_flow
  - Strategic intent / PRD
cluster: implementation_flow
role: Strategic intent / PRD
attributes:
  domain: concept
---

# CONCEPT — Master priority sectors (P0–P4)

## Why this concept exists

`[[FRAMEWORK--KNOWLEDGE-3-TIER]]` introduced the Master tier as instinct-level knowledge loaded as a system-prompt preamble. It established:

- Master atoms cannot be authored directly — they are promoted from Genesis
- Body cap: 400 tokens warn / 600 tokens error
- The first three Masters exist: `[[MASTER--MSP-DOC-TO-CODE]]`, `[[MASTER--ATOM-CONTRADICTION-POLICY]]`, `[[MASTER--ROOT-CAUSE-ANALYSIS]]`

What the framework does **not** specify:

1. **How many Masters can be loaded at once.** Without a slot policy, the system either loads all Masters (token cost explodes as the set grows) or loads none (instinct preamble is empty).
2. **Which Masters are "always on" vs. "context-triggered."** Today the loader (future PR-6) would treat all Masters equivalently.
3. **How agents discover relevant Masters cheaply.** Free-form "Apply when" prose forces the agent to read every Master body to decide relevance — defeating the token-budget rationale.
4. **Who decides priority.** Without an authority rule, an autonomous agent could silently promote its own preferred Masters to "always-loaded" status.

This concept fills those gaps by introducing **priority sectors** that partition the Master set into bands with distinct load rules, slot budgets, and authority gates.

## Definition

A **priority sector** is a named band that groups Master atoms by load behaviour:

| Sector | Load behaviour | Slot policy | Authority to assign |
|---|---|---|---|
| **P0** | Always loaded into every session's system prompt | Foundation-only, ≤7 typical (no hard cap; each entry must justify "always needed") | **User only** — LLM cannot self-promote without explicit user permission |
| **P1** | Always loaded as **index entry** (id + 1-line directive); body fetched on first trigger match | Larger budget, ~10–20 typical | **User only** |
| **P2** | Index entry only when trigger-pattern indicates likely relevance for the project area | Soft cap ~30 | Agent may propose; user approves |
| **P3** | Not in CLAUDE.md preamble; fetched only on explicit reference or deep dive | Unbounded | Agent may propose; user approves |
| **P4** | Archive — superseded, deprecated, or experimental Masters retained for traceability | Unbounded | Automatic on `status: superseded` |

Sector membership is a property of the Master atom (frontmatter field `priority:`), not of the loader configuration.

## Multi-tiered triggering

To decide whether a P1–P3 Master should load its body into the active context, the loader applies trigger evaluation in three increasing-cost tiers:

```yaml
trigger:
  keywords: ["bug", "error", "fix", "broken", "พัง", "ผิด"]   # tier 1: pattern match (cheapest)
  context:  ["user mentions failed attempt",                  # tier 2: semantic context match
             "agent encountered known-invariant violation"]
  llm_check: "uncertain whether root-cause analysis applies"  # tier 3: LLM judgement (most expensive)
```

Evaluation order:

1. **Tier 1 (keywords)** — regex / substring match on user turn and recent agent output. If match, load. Stop.
2. **Tier 2 (context)** — semantic match on conversation state (e.g. recent tool failures, active branch name, file paths touched). If match, load. Stop.
3. **Tier 3 (llm_check)** — one-shot LLM evaluation prompt asking whether this Master is relevant given the conversation. Used only when tiers 1–2 are inconclusive.

A Master with only tier-1 triggers costs zero LLM calls to evaluate. A Master with only tier-3 triggers costs one LLM call per turn — reserve for high-value, hard-to-pattern directives.

## Constituent indexing

Master atoms must declare their `constituents:` — the atoms and Genesis blocks they distill or aggregate. This makes Master a true **index** rather than a self-contained directive:

```yaml
constituents:
  required:
    framework: [FRAMEWORK--KNOWLEDGE-3-TIER]
    concept:   [CONCEPT--ROOT-CAUSE-ANALYSIS]
  optional:
    genesis:   [GENESIS--FRONTEND-CORE]
    skill:     [SKILL--PR-REVIEW]
    adr:       [ADR--MASTER-PROMOTION-DOC-TO-CODE]
```

When an agent needs deeper context than the Master body provides, it follows the constituent links. The recommended deepening order is **Genesis first, then atoms** — a Genesis block aggregates five-dimensional context (Cognitive / Algo / Runbook / Concept / Params) in one fetch, whereas atoms require separate reads.

This makes Master a hierarchical context anchor:

```
Always loaded:    CLAUDE.md index entry (~30 tokens)
On trigger match: Master body (~400 tokens)
On deep dive:     Genesis block (~500 tokens) → 5 member atoms (~variable)
```

## The user-authority rule for P0/P1

`[[FRAMEWORK--KNOWLEDGE-3-TIER]]` already requires evidence ADRs for Master promotion. This concept adds a stricter rule for the top two sectors:

**The LLM/agent may NOT modify a Master's `priority:` field to P0 or P1 without explicit user permission in the same conversation that initiates the change.**

Rationale: P0 entries consume the every-session token budget. P1 entries are guaranteed-loaded index. A drifting agent that self-promotes its own preferred rules to P0 corrupts the instinct preamble for every future session. The cost of an erroneous P0 is paid forever; the cost of confirming with the user is one round-trip.

P2–P4 changes follow standard promotion ADR flow (agent proposes via candidate, user approves via PR).

## What this concept does NOT decide

- **The MSP loader implementation.** A separate BLUEPRINT will spec `msp master compose --sector=P0,P1` and the trigger evaluation engine.
- **The validator rule for `priority:`.** A future PROTO will enforce that only P4 can hold `status: superseded`; that P0 entries pass a stricter completeness check (constituents declared, multi-tiered trigger or "always-on" flag set, etc.).
- **Migration mechanics for the three existing Masters.** Handled by the companion ADR.
- **Cross-project portability of Masters.** Whether a P0 Master in MSP can load in another repo is deferred — for now, sectors are per-repo.

## Source

- `[[FRAMEWORK--KNOWLEDGE-3-TIER]]` § "Master Block" — the tier this concept refines
- `[[SPEC--GENESIS-BLOCK-MANIFEST]]` § "Membership rules" — the constituent pattern this concept inherits
- This session's dialogue (2026-05-17) — the priority-sector and user-authority requirements

## Connections

- [[FRAMEWORK--KNOWLEDGE-3-TIER]]
- [[SPEC--GENESIS-BLOCK-MANIFEST]]
- [[MASTER--MSP-DOC-TO-CODE]]
- [[MASTER--ATOM-CONTRADICTION-POLICY]]
- [[MASTER--ROOT-CAUSE-ANALYSIS]]
