---
id: ADR--MASTER-PROMOTION-ROOT-CAUSE-ANALYSIS
phase: 2
type: adr
status: stable
tier: genesis
source_type: axiomatic
vault_id: default
title: Promote root-cause-analysis discipline to MASTER--ROOT-CAUSE-ANALYSIS
tags:
  - msp
  - master
  - promotion
  - rca
  - 3-tier
  - decision
crosslinks:
  references:
    - FRAMEWORK--KNOWLEDGE-3-TIER
    - CONCEPT--ROOT-CAUSE-ANALYSIS
    - MASTER--ROOT-CAUSE-ANALYSIS
    - ADR--MASTER-PROMOTION-DOC-TO-CODE
created_at: 2026-05-17T02:05:00.000+07:00
aliases:
  - ADR
  - implementation_flow
  - Architecture decision record
cluster: implementation_flow
role: Architecture decision record
attributes:
  domain: adr
---

# ADR — promote RCA discipline to Master

## Context

`[[FRAMEWORK--KNOWLEDGE-3-TIER]]` defines the Master tier as stable cross-cutting knowledge loaded as instinct preamble. Master atoms cannot be authored directly — they are promoted from a Genesis atom via an evidence ADR.

The Root Cause Analysis mandate has lived as a free-form narrative block at the top of `CLAUDE.md` since the doc-to-code workflow was introduced. It is not yet a Master atom, which means:

- Non-Claude agents (Gemini CLI, Qwen CLI) that do not consume `CLAUDE.md` cannot receive this instinct
- Drift between the narrative and any future MASTER body is unconstrained
- The discipline has no `promoted_from` evidence chain

Cross-context stability evidence:

- The RCA mandate has applied unchanged to every milestone since the doc-to-code workflow was introduced
- Multiple prior incidents (audit atoms, incident reports under `gks/audit/`) trace reactive fixes that bypassed RCA — the discipline holds across validator, frontend, and GKS internals
- No ADR has ever proposed a domain-specific exception
- This session's own conversation (2026-05-17) surfaced the gap when the agent created `msp-candidate` CLI without first writing the governing ADR — a direct instance of the failure mode RCA exists to prevent

These three lines of evidence — written discipline (`CLAUDE.md`), a stable Genesis CONCEPT (the companion `[[CONCEPT--ROOT-CAUSE-ANALYSIS]]`), and a multi-milestone audit trail — meet the bar set by `[[FRAMEWORK--KNOWLEDGE-3-TIER]]` for "true regardless of session, project, or context."

## Decision

Promote the RCA discipline to `[[MASTER--ROOT-CAUSE-ANALYSIS]]` with:

```yaml
tier: master
promoted_from: CONCEPT--ROOT-CAUSE-ANALYSIS
promoted_at: 2026-05-17T02:10:00.000+07:00
promotion_adr: ADR--MASTER-PROMOTION-ROOT-CAUSE-ANALYSIS
```

The Master body follows the 5-section schema defined in `[[FRAMEWORK--KNOWLEDGE-3-TIER]]`: Intent / Why / Directives / Apply when / Conflicts with. Body stays within the 400-token warn / 600-token error budget.

The pre-promotion `[[CONCEPT--ROOT-CAUSE-ANALYSIS]]` remains `status: stable`. The Master is an additive distillation, not a supersession. `CLAUDE.md` continues to host a P0 entry pointing to the Master, replacing the current free-form mandate.

## Consequences

- Gemini CLI and Qwen CLI sessions that load Master atoms via `msp master compose` (future PR-6) will receive the RCA directive as instinct rather than re-discovering it from `CLAUDE.md`
- The narrative block at the top of `CLAUDE.md` can be reduced to a single-row P0 index entry pointing at `gks/master/MASTER--ROOT-CAUSE-ANALYSIS.md`, freeing context budget
- Drift mitigation: `[[MASTER--ATOM-CONTRADICTION-POLICY]]` governs any future divergence between `CLAUDE.md` and the Master body
- The atom does not yet carry `constituents:` or `trigger.multi_tiered` fields — those are introduced by `[[ADR--CLAUDE-MD-MASTER-PRIORITY-SECTORS]]` (companion ADR in this session) and will be back-filled when that schema lands

## Alternatives considered

### A. Keep RCA as `CLAUDE.md` narrative only

**Rejected:** non-Claude agents miss it entirely; no atom-level promotion trail; cannot be loaded into non-Claude system prompts.

### B. Promote a bundled "agent instincts" Master containing RCA + doc-to-code + write-boundaries

**Rejected:** violates the single-directive principle from `[[FRAMEWORK--KNOWLEDGE-3-TIER]]`. Each Master should encode one enforceable directive so it can be cited, contradicted, or superseded independently.

### C. Wait until the priority-sector refactor lands, then promote with the new schema

**Rejected:** RCA promotion is independent of sector design. Promoting under the current Master schema is unblocked and demonstrates the workflow. Schema migration is a follow-up edit, not a new promotion.

## Source

- `[[FRAMEWORK--KNOWLEDGE-3-TIER]]` § "Master Block — How they get created"
- `[[CONCEPT--ROOT-CAUSE-ANALYSIS]]` — the pre-promotion Genesis atom
- `[[ADR--MASTER-PROMOTION-DOC-TO-CODE]]` — the precedent ADR for Master promotion
- `CLAUDE.md` § "MASTER BLOCK: ROOT CAUSE ANALYSIS MANDATE" — the narrative being formalised
- This session's own RCA failure (2026-05-17, agent wrote `msp-candidate` CLI before authoring `ADR--MSP-CANDIDATE-CLI`) — additional evidence that the discipline holds across contexts

## Connections

- [[FRAMEWORK--KNOWLEDGE-3-TIER]]
- [[CONCEPT--ROOT-CAUSE-ANALYSIS]]
- [[MASTER--ROOT-CAUSE-ANALYSIS]]
- [[ADR--MASTER-PROMOTION-DOC-TO-CODE]]
