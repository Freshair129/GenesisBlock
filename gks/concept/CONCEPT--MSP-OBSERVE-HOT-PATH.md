---
id: CONCEPT--MSP-OBSERVE-HOT-PATH
phase: 1
type: concept
status: draft
vault_id: default
tier: genesis
source_type: learned
title: msp_observe — hot-path conversation extraction (mem0-style)
tags: &a1
  - msp
  - memory
  - extraction
  - aspirational
  - cherry-pick
crosslinks: &a2
  references:
    - FRAMEWORK--MSP-ARCHITECTURE-V2
    - CONCEPT--AGENT-AGNOSTIC
    - CONCEPT--CONSOLIDATOR
created_at: 2026-05-09T07:00:00.000+07:00
aliases: &a3
  - CONCEPT
  - implementation_flow
  - Strategic intent / PRD
cluster: implementation_flow
role: Strategic intent / PRD
attributes:
  id: CONCEPT--MSP-OBSERVE-HOT-PATH
  phase: 1
  type: concept
  status: draft
  vault_id: default
  tier: genesis
  source_type: learned
  title: msp_observe — hot-path conversation extraction (mem0-style)
  tags: *a1
  crosslinks: *a2
  created_at: 2026-05-09T07:00:00.000+07:00
  aliases: *a3
  cluster: implementation_flow
  role: Strategic intent / PRD
  attributes:
    id: CONCEPT--MSP-OBSERVE-HOT-PATH
    phase: 1
    type: concept
    status: draft
    vault_id: default
    tier: genesis
    source_type: learned
    title: msp_observe — hot-path conversation extraction (mem0-style)
    tags: *a1
    crosslinks: *a2
    created_at: 2026-05-09T07:00:00.000+07:00
    aliases: *a3
    cluster: implementation_flow
    role: Strategic intent / PRD
    attributes:
      id: CONCEPT--MSP-OBSERVE-HOT-PATH
      phase: 1
      type: concept
      status: draft
      vault_id: default
      tier: genesis
      source_type: learned
      title: msp_observe — hot-path conversation extraction (mem0-style)
      tags: *a1
      crosslinks: *a2
      created_at: 2026-05-09T07:00:00.000+07:00
      aliases: *a3
      cluster: implementation_flow
      role: Strategic intent / PRD
      attributes:
        domain: concept
      domain: concept
      language: markdown
      is_test: false
      is_entrypoint: false
      has_secret: false
      leak_risk: low
      encryption_level: none
    domain: concept
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: false
    leak_risk: low
    encryption_level: none
  domain: concept
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: false
  leak_risk: low
  encryption_level: none
---

# CONCEPT — `msp_observe` hot-path extraction

> **Status: draft (aspirational).** Not implemented. Cherry-picked from `[[SPEC--ARCHITECTURE-V2]].md` §4.2 during the 2026-05-09 architecture-doc cleanup. Belongs in roadmap, not current scope.

## Problem

Current MSP requires the agent (or human) to be disciplined: turn → decide what's worth remembering → call `msp_remember` with a structured payload. Most conversations don't get summarised because the discipline isn't there. mem0 demonstrated that an extraction layer **between** the conversation and the store closes the gap: agents dump turns, the layer figures out what's a fact, a preference, or a goal worth persisting.

## Hypothesis

A new MCP tool `msp_observe(messages, hints?)` runs a deterministic-then-LLM pipeline that extracts retainable facts from raw turns and reconciles them against existing atoms. Agents call it after every meaningful turn; MSP decides what (if anything) becomes a new atom or supersedes an existing one.

## Pipeline

```
input: { messages: Turn[], hints?: { focus: 'fact' | 'preference' | 'goal' } }

1. Filter         — drop boilerplate / greetings / pure tool calls
2. Extract (LLM)  — "what's worth remembering here?" → list of typed facts
3. For each fact:
   3a. gks.similar(fact.text, { k: 3, threshold: 0.85 })
   3b. Reconcile (rule-based first, LLM only if ambiguous):
       - No match           → ADD (new candidate atom)
       - Exact match        → NOOP
       - Near match, contradicts → SUPERSEDE (write new + flip old)
       - Near match, refines     → UPDATE (merge into existing)
4. Batch write via msp_candidate (NOT direct gks.write — respects ADR--AGENT-WRITE-BOUNDARIES)
5. Append audit entry per op
6. Return { added: AtomId[], updated: AtomId[], superseded: AtomId[], skipped: number, audit_id }
```

## Why this matters

It's the difference between "AI with good memory" (mem0 promise) and "structured note-taker" (current MSP). Agents shouldn't have to know about atom IDs / phases / types — they dump conversation, MSP figures it out.

## Constraints

- **Goes through `msp_candidate`, not direct write.** Per `[[ADR--AGENT-WRITE-BOUNDARIES]]`, agents never touch `gks/<type>/` directly. `msp_observe` writes to `.brain/msp/projects/<ns>/candidates/`; promotion stays a human PR action.
- **Reconciliation respects the contradiction policy** in `CLAUDE.md` — supersession is reciprocal (`supersedes` + `superseded_by`).
- **LLM extraction step is opt-in.** When no extractor configured, fall back to rule-based filtering only.
- **Cost budget** — every `msp_observe` call records token spend in the audit log; agents can pass `max_cost_usd` hint.

## Open questions

- What triggers `msp_observe`? Per-turn (expensive), per-N turns, on `endSession`, or agent-driven?
- Default extractor model — which Claude tier? Locally-run SLM via GKS embedder fallback?
- Schema for the typed-fact intermediate representation — reuse atom frontmatter or a smaller shape?

## Trade-offs

**Positive**

- Lowers the discipline cost of using MSP — works for "normal" agents, not just diligent ones.
- Composes naturally with `[[CONCEPT--CONSOLIDATOR]]` (which today operates on episodic memory; `msp_observe` operates on raw turns one layer up).

**Negative**

- LLM call per turn is non-trivial cost; needs per-project budget guard.
- Extraction quality is the bottleneck — bad extraction pollutes the atom graph faster than humans can clean.
- Yet-another-tool in the 16-tool MSP MCP surface; only adopt if the pipeline above proves out.

## Source

`[[SPEC--ARCHITECTURE-V2]].md` §4.2 (drafted 2026-05-07, cherry-picked here on 2026-05-09 before the original was deleted). Reference patterns: mem0 (extraction + reconciliation), langmem (procedural typology). See `[[AUDIT--ARCH-DOC-CLEANUP]]`.

## Connections

- [[FRAMEWORK--MSP-ARCHITECTURE-V2]]
- [[CONCEPT--AGENT-AGNOSTIC]]
