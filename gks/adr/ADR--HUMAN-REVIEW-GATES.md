---
id: ADR--HUMAN-REVIEW-GATES
phase: 2
type: adr
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: Human review gates — who reviews which atom type
tags:
  - msp
  - review
  - governance
  - decision
crosslinks: {"references":["ADR--AGENT-WRITE-BOUNDARIES","FRAMEWORK--AUTHORITY-MATRIX","FRAMEWORK--SCALING-LEVELS","CONCEPT--MASTER-PROMOTION"]}
created_at: 2026-05-03T17:36:08.623+07:00
---

# ADR — human review gates

## Context

`ADR--PROMOTION-WORKFLOW` defines the third gate as "human review" without saying *who* is the human for which atom type. `FRAMEWORK--AUTHORITY-MATRIX` covers who can write where but not who approves the promotion. We need an explicit per-type → reviewer mapping so promotions don't stall waiting for an unspecified human.

## Decision

| Atom type | Reviewer | Notes |
|---|---|---|
| `ADR--`            | **Boss** (project owner) | Decisions that bind future code; only the owner approves |
| `FEAT--`           | Boss                      | Defines acceptance criteria the project commits to |
| `BLUEPRINT--`      | Boss OR T3 (Claude/Opus) | Implementation plan; T3 may approve when Boss is unavailable, but Boss may revoke retroactively |
| `CONCEPT--`        | T3 self-review            | Concepts are framing; reviewer is the author's own next-day reread |
| `FRAME--`          | Boss                      | Foundational frames bind everything downstream |
| `MOD--` / `FLOW--` | Boss                      | Architectural; same weight as FEAT |
| `API--` / `ENDPOINT--` / `ENTRYPOINT--` | Boss | Public surface, contract-bearing |
| `ENTITY--`         | T3 self-review            | Domain modelling; correctness is technical, not architectural |
| `ALGO--` / `PROTO--` | T3 + Boss skim          | Two pairs of eyes for algorithmic claims |
| `IDEA--`           | none required             | Capture-only; no acceptance |
| `AUDIT--`          | none required             | Records observed reality; reviewer is the next reader |
| `MICROTASK` / `T*.task.yaml` | Implementer self + acceptance test | Light-tier; passing tests = approval |
| `TASK--` (orchestrator state) | none — mutable        | Live state; not in `gks/` (per `ADR--TASK-TRACKING-AT-ORCHESTRATOR`) |
| `HOTFIX--`         | none at open; Boss at close | 48h emergency window; close needs sign-off |

## Default routing

- If reviewer column says **Boss**, the promote workflow refuses without an explicit `--reviewer=@boss-handle` flag (or env var). Future M6+ work: PR-comment-driven approval.
- If reviewer column says **T3 self-review**, promote works without flag.
- If a project's CLAUDE.md / GEMINI.md overrides `Boss` to `T3 alone`, that override is recorded as a project-local ADR; this ADR is the default.

## Consequences

**Positive**
- Promotes are no longer ambiguous. CI / orchestrator can detect "atom waiting for Boss" vs "ready to merge".
- T3 has explicit autonomy on CONCEPT/ENTITY/AUDIT — reduces Boss bottleneck on low-risk artifacts.
- Hotfix open is fast (no review); close is gated (real recovery audit).

**Negative**
- Boss is the single point of failure for ADR/FEAT/BLUEPRINT. Mitigation: BLUEPRINT carries the "OR T3" clause for momentum during Boss absence.
- The mapping is a default; per-project overrides need their own ADR. Adds ceremony.

## Alternatives considered

1. **Two-tier (Boss vs anyone).** Rejected — too coarse; CONCEPT shouldn't need Boss.
2. **N-of-M voting.** Rejected — too much process for a small team.
3. **Auto-approve everything; rely on validator.** Rejected — validator can't judge "is this a good ADR" (per `CONCEPT--MSP-VALIDATOR`).

## What this ADR does NOT change

- The validator's hard rules — see `ADR--ANTI-HALLUCINATION-RULES`.
- The promotion levels themselves — see `ADR--PROMOTION-LEVELS`.
- Authority over file paths — see `FRAMEWORK--AUTHORITY-MATRIX`.

## Source

`msp_spec.md` §9 (Human Review Gates) — referenced from `ADR--PROMOTION-WORKFLOW` but not previously written as an atom.
