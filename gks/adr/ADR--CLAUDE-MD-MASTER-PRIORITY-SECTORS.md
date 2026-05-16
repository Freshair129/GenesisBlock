---
id: ADR--CLAUDE-MD-MASTER-PRIORITY-SECTORS
phase: 2
type: adr
status: stable
tier: genesis
source_type: axiomatic
vault_id: default
title: CLAUDE.md MASTER BLOCKS sector — P0–P4 priority bands, user-only P0/P1, multi-tiered triggers, constituent index
tags:
  - msp
  - master
  - claude-md
  - priority
  - sectors
  - trigger
  - constituents
  - decision
crosslinks: {"references":["FRAMEWORK--KNOWLEDGE-3-TIER","CONCEPT--MASTER-PRIORITY-SECTORS","SPEC--GENESIS-BLOCK-MANIFEST","MASTER--MSP-DOC-TO-CODE","MASTER--ATOM-CONTRADICTION-POLICY","MASTER--ROOT-CAUSE-ANALYSIS","ADR--AGENT-WRITE-BOUNDARIES"]}
created_at: 2026-05-17T02:35:00.000+07:00
---

# ADR — adopt P0–P4 sector structure in CLAUDE.md MASTER BLOCKS

## Context

`[[CONCEPT--MASTER-PRIORITY-SECTORS]]` identified four gaps in the current Master tier design:

1. No slot policy → unbounded token cost as the Master set grows
2. No "always on" vs. "context-triggered" distinction
3. No cheap relevance evaluation → every Master body must be read to decide load
4. No authority rule preventing autonomous LLM self-promotion to instinct-level

The current `CLAUDE.md` contains one ad-hoc MASTER BLOCK (Root Cause Analysis, now formalised as `[[MASTER--ROOT-CAUSE-ANALYSIS]]` per `[[ADR--MASTER-PROMOTION-ROOT-CAUSE-ANALYSIS]]`). Three Masters exist as atoms but are not referenced from `CLAUDE.md`. The token cost of inlining all three would be ≥1500 tokens for the body alone, with no scaling path beyond ~10 Masters before the preamble dominates the context window.

## Decision

Restructure `CLAUDE.md` to host a single `## MASTER BLOCKS` sector divided into priority bands P0 through P4. Each band has distinct load rules, slot budgets, and authority gates as defined in `[[CONCEPT--MASTER-PRIORITY-SECTORS]]`.

### Sector layout in CLAUDE.md

```markdown
# 🎯 MASTER BLOCKS

> Stable cross-cutting directives. Body in `gks/master/<ID>.md`.
> P0 always loaded. P1–P3 indexed; body fetched on trigger match.
> P0/P1 assignment requires explicit user permission — agents must not self-promote.

## P0 — Always loaded (foundation)

### MASTER--ROOT-CAUSE-ANALYSIS
- Apply when: bug, error, ambiguous request, failed previous attempt
- Directive: identify and confirm root cause before any fix
- → `gks/master/MASTER--ROOT-CAUSE-ANALYSIS.md`

### MASTER--MSP-DOC-TO-CODE
- Apply when: new branch, PR, file in src/|test/|scripts/|web/
- Directive: atoms before code (FRAME→CONCEPT→ADR→BP→CODE→AUDIT)
- → `gks/master/MASTER--MSP-DOC-TO-CODE.md`

### MASTER--ATOM-CONTRADICTION-POLICY
- Apply when: PR adds/edits atom in gks/<type>/
- Directive: reciprocal supersession in same PR
- → `gks/master/MASTER--ATOM-CONTRADICTION-POLICY.md`

## P1 — High-priority index (always indexed, body on first trigger)

| Master | Trigger summary | Domain |
|---|---|---|
| _(empty — no P1 promotions yet)_ | | |

## P2 — Context-triggered index

| Master | Trigger summary | Domain |
|---|---|---|
| _(empty)_ | | |

## P3 — Deep-dive only

(not listed in CLAUDE.md; fetched via explicit reference)

## P4 — Archive

(superseded / deprecated; not loaded)
```

### Frontmatter schema additions for Master atoms

Three new fields added to the Master frontmatter contract:

```yaml
priority: P0 | P1 | P2 | P3 | P4   # required for tier: master atoms

constituents:                       # required for tier: master atoms
  required:                         # at least the source CONCEPT must appear here
    framework: [FRAMEWORK--<id>, ...]
    concept:   [CONCEPT--<id>, ...]
    spec:      [SPEC--<id>, ...]
  optional:
    genesis:   [GENESIS--<id>, ...]
    adr:       [ADR--<id>, ...]
    skill:     [SKILL--<id>, ...]
    runbook:   [RUNBOOK--<id>, ...]

trigger:                            # required for P1–P3; optional for P0
  keywords: [...]                   # tier 1 — pattern match (cheapest)
  context:  [...]                   # tier 2 — semantic / conversation-state match
  llm_check: "..."                  # tier 3 — LLM-evaluated relevance prompt
```

For P0 entries, `trigger:` may be omitted (P0 = always loaded, no evaluation needed) or set to `keywords: ["*"]` for explicitness.

### Authority gate for P0/P1

The LLM/agent **MUST NOT** modify any Master's `priority:` field to `P0` or `P1` without an explicit user instruction in the same conversation that initiates the change. This applies to:

- New Master atoms (initial `priority:` assignment cannot be P0/P1 without user approval)
- Existing Master atoms (raising `priority:` from P2 or lower to P1 or P0)

Lowering from P0/P1 to P2 or below requires the same user permission — drift in either direction is gated.

P2–P4 changes follow the standard candidate-propose-merge flow per `[[ADR--AGENT-WRITE-BOUNDARIES]]`.

### Token budget

| Sector | Per-entry budget | Sector cap | Always loaded? |
|---|---|---|---|
| P0 | ~80 tokens (id + 3-line summary + path) | soft ~600 tokens | yes |
| P1 | ~30 tokens (index row only) | soft ~600 tokens | yes (index only) |
| P2 | ~30 tokens (index row only) | soft ~900 tokens | yes (index only) |
| P3 | 0 | — | no |
| P4 | 0 | — | no |

Body fetched on trigger match is bounded by the existing 400-token warn / 600-token error cap from `[[FRAMEWORK--KNOWLEDGE-3-TIER]]`.

### Migration plan for the three existing Masters

| Master | Initial priority | Rationale |
|---|---|---|
| `MASTER--ROOT-CAUSE-ANALYSIS` | P0 | Applies to every interaction with any error or ambiguity |
| `MASTER--MSP-DOC-TO-CODE` | P0 | Applies to every code-touching PR |
| `MASTER--ATOM-CONTRADICTION-POLICY` | P0 | Applies to every atom-editing PR |

All three are assigned P0 with explicit user approval recorded in this ADR's source dialogue (2026-05-17). The migration adds `priority:` and `constituents:` to each Master's frontmatter; `trigger:` is omitted (P0 default = always loaded).

The `CLAUDE.md` narrative-form "MASTER BLOCK: ROOT CAUSE ANALYSIS MANDATE" is replaced by the P0 index entry above. The narrative becomes redundant — the Master atom is the SSOT.

## Consequences

### Positive

- **Bounded token cost.** P0 body load is capped at ~600 tokens regardless of total Master count. Adding the 100th Master costs no more preamble tokens than adding the 4th, as long as it lands in P2 or below.
- **Single instinct surface for all agents.** Non-Claude agents (Gemini, Qwen) load the same Master atoms via `msp master compose --sector=P0`; CLAUDE.md is the convenience surface for Claude Code specifically but no longer the SSOT.
- **Cheap relevance evaluation.** Tier-1 keyword matching avoids LLM calls for the common case; tier-3 LLM eval reserved for high-value Masters where pattern matching is insufficient.
- **Drift-resistant instinct preamble.** P0/P1 user-only authority prevents an autonomous agent from silently corrupting the always-loaded directive set.
- **Constituent links make Masters into indexes.** Agents fetch Genesis blocks for breadth or specific atoms for depth, on demand — no need to inline aggregate knowledge into the Master body.

### Negative

- **Three existing Masters require frontmatter migration.** `constituents:` and `priority:` must be back-filled. Mechanical edit, no semantic change.
- **Validator gains new rules.** A future PROTO must enforce: P0/P1 require user-approval in promotion ADR; `constituents.required` must include at least one entry; `trigger:` required for P1–P3.
- **Loader implementation deferred.** This ADR specifies the schema; a separate BLUEPRINT will spec the `msp master compose` and trigger-evaluation engine.

### Neutral

- **Existing Master body content unchanged.** All three current Masters' 5-section body (Intent / Why / Directives / Apply when / Conflicts with) is preserved. Sector design is additive frontmatter, not a body rewrite.

## Alternatives considered

### A. Flat Master list, no sectors

Load all Masters at session start; rely on body brevity (the 400-token cap) to bound cost.

**Rejected:** does not scale. At 20 Masters × 400 tokens = 8 000 tokens of preamble, dominating most context windows. Provides no relevance signal for selective loading.

### B. Pure trigger-based loading (no priority)

Every Master has a trigger; loader evaluates all triggers each turn and loads matches.

**Rejected:** "instinct" Masters that apply to every turn (RCA, doc-to-code) would need a trigger like `keywords: ["*"]`, which is degenerate. Priority makes the "always on" semantics first-class instead of a special case.

### C. Priority bands but no user-authority gate

Let the LLM propose P0/P1 promotions; gate only at PR review time.

**Rejected:** PR review may miss subtle drift if many Masters change at once. Requiring same-conversation user approval keeps the change traceable to a single intent point. The cost (one round-trip) is small relative to the blast radius of an erroneous P0 (every future session affected).

### D. Use `tags:` for sector membership instead of new `priority:` field

Reuse existing taxonomy infrastructure (`tags: [p0, master]`).

**Rejected:** tags are descriptive metadata, not authority-gated state. Mixing them invites accidental priority changes via tag edits. A dedicated `priority:` field has a single, clear semantic.

## Source

- `[[CONCEPT--MASTER-PRIORITY-SECTORS]]` — the design this ADR adopts
- `[[FRAMEWORK--KNOWLEDGE-3-TIER]]` — Master tier definition this ADR extends
- `[[SPEC--GENESIS-BLOCK-MANIFEST]]` — `members:` pattern that `constituents:` mirrors
- `[[ADR--AGENT-WRITE-BOUNDARIES]]` — the authority model this ADR layers on top of
- This session's dialogue (2026-05-17) — user-authority requirement and multi-tiered trigger specification

## Connections

- [[CONCEPT--MASTER-PRIORITY-SECTORS]]
- [[FRAMEWORK--KNOWLEDGE-3-TIER]]
- [[SPEC--GENESIS-BLOCK-MANIFEST]]
- [[ADR--AGENT-WRITE-BOUNDARIES]]
- [[MASTER--MSP-DOC-TO-CODE]]
- [[MASTER--ATOM-CONTRADICTION-POLICY]]
- [[MASTER--ROOT-CAUSE-ANALYSIS]]
