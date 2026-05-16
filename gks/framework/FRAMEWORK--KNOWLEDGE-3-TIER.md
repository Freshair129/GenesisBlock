---
id: FRAMEWORK--KNOWLEDGE-3-TIER
phase: 0
type: framework
status: stable
tier: genesis
source_type: axiomatic
vault_id: default
title: Knowledge 3-Tier model — Safety / Master / Genesis (with epistemic provenance)
tags:
  - msp
  - knowledge
  - tier
  - master
  - genesis
  - safety
  - epistemic
  - provenance
crosslinks: {"references":["FRAMEWORK--MSP-ARCHITECTURE-V2","CONCEPT--KNOWLEDGE-LAYERS-V2","ADR--AGENT-WRITE-BOUNDARIES","FRAMEWORK--AUTHORITY-MATRIX"]}
created_at: 2026-05-09T14:30:00.000+07:00
---

# FRAME — Knowledge 3-Tier model

## Why this frame exists

`[[CONCEPT--KNOWLEDGE-LAYERS-V2]]` settled the **storage** question (sessions / episodes / candidates / canon). This frame settles the **knowledge-class** question on top: of all atoms in `gks/`, which are stable cross-cutting anchors agents need at session start, which are everyday knowledge fetched on demand, and which are the rails outside reasoning that prevent unsafe writes?

Without this distinction the codebase has 181 atoms flat — every session re-discovers `CLAUDE.md` rules, doc-to-code order, supersession discipline, tech-stack decisions. Putting an explicit tier on each atom turns that re-discovery into a stable preamble.

## The three tiers

### Safety Block

**Definition:** rails *outside* the agent's reasoning loop. Validators, PROTO predicates, CI gates, pre-commit / pre-push hooks, branch protection. They block actions; they do not produce thinking.

**Storage:** `src/validator/`, `gks/proto/`, `.github/workflows/`, `examples/hooks/`. Atom representations of these rules carry `tier: safety`.

**Examples:** `[[PROTO--AUTHORITY-ENFORCEMENT]]`, `[[PROTO--SUMMARY-MIN]]`, `[[PROTO--MASTER-TOKEN-CAP]]` (future), validator forbidden-fields rule, pre-commit validator gate.

### Master Block

**Definition:** stable, cross-cutting knowledge with **absolute meaning** — true regardless of session, project, or context. Like instinct passed down generation to generation: even when the origin is forgotten, the directive holds.

**How they get created:** Master atoms are **promoted from Genesis**, not authored directly. A Genesis atom that proves cross-context stability over time becomes a candidate for promotion via a `[[ADR--MASTER-PROMOTION]]-<SLUG>` evidence ADR. See PR-5 of the rollout plan and `[[PROTO--MASTER-BODY-SCHEMA]]` (future) for the body contract.

**Frontmatter contract:**

```yaml
tier: master
promoted_from: <Genesis atom ID>
promoted_at: <ISO timestamp>
promotion_adr: ADR--MASTER-PROMOTION-<SLUG>
# learned_from: NOT set — Master is "instinct"; origin tracking lives on the
# pre-promotion Genesis atom and the promotion ADR. Master itself is
# absolute and origin-less.
```

**Body contract** (enforced by future `[[PROTO--MASTER-BODY-SCHEMA]]`):

```
## Intent (1–2 sentences — what behavior this Master enforces)
## Why (rationale — for human review)
## Directives (numbered, imperative — what agent must do)
## Apply when (triggers — when this Master is relevant)
## Conflicts with (atom IDs that may contradict — for resolution)
```

**Token cap:** body ≤ 400 tokens warn / ≤ 600 error (future `[[PROTO--MASTER-TOKEN-CAP]]`). Master atoms must stay prompt-injectable.

**Loader:** `npm run msp:master compose <ID1> <ID2> ...` (future PR-6) returns concatenated bodies as a system-prompt fragment.

### Genesis Block

**Definition:** everyday knowledge — concepts, decisions, frameworks, parameters — that an agent needs *contextually* (when working on the relevant subject). Most atoms in `gks/` belong here.

**Two sub-classes by provenance:**

1. **Authored Genesis (axiomatic).** Atoms written top-down through the doc-to-code workflow (`source_type: axiomatic`). They have no `learned_from:` because they were never learned — they were specified. All 181 existing atoms in this repo are axiomatic Genesis (or Process / Safety, see classification below). This is grandfathered as a one-time decision; the repo started authored, not learned.

2. **Learned Genesis.** Future atoms that emerge from session learning carry `source_type: learned` plus a `learned_from:` block pointing back to the source episode:

   ```yaml
   tier: genesis
   source_type: learned
   learned_from:
     sessionId: "MSP-SESS-..."
     episodicId: "ep_..."
     turnId: 23
     msgId: "m_023"
   ```

The two sub-classes coexist. The promotion path to Master is the same for both — what matters for promotion is cross-context stability, not how the Genesis was originally formed.

## Classification rules (for the existing 181 atoms — applied in PR-4)

| Atom shape | tier | source_type |
|---|---|---|
| `PROTO--*` (predicates) | `safety` | `axiomatic` |
| `AUDIT--*`, `BLUEPRINT--*`, `FEAT--*` (process artifacts — what shipped, what to ship, contracts) | `process` | `axiomatic` |
| Strong cross-cutting frames + concepts | `genesis` (candidate for future Master) | `axiomatic` |
| Everything else (most CONCEPT, ADR, FRAME) | `genesis` | `axiomatic` |

**No atom in PR-4 is tagged `tier: master` directly** — Master only appears via the promotion ADR flow in PR-5.

## Frontmatter additions (cross-cutting)

These fields are added to the runtime contract (`atomic_contract.yaml`) as permitted optional fields. None are forbidden; none are required by the validator's required-fields rule for existing types.

| Field | Where it appears | Required when |
|---|---|---|
| `tier` | every atom (after PR-4 bulk-tag) | gradual — warn-level validator rule introduced in PR-4 |
| `source_type` | every atom (after PR-4) | always paired with `tier` |
| `learned_from` | only when `source_type: learned` | conditional |
| `promoted_from` | only when `tier: master` | always for Master |
| `promoted_at` | only when `tier: master` | always for Master |
| `promotion_adr` | only when `tier: master` | always for Master |

## What this frame does NOT decide

- **Master atom auto-injection** into MCP clients (Claude Code / Gemini CLI / Codex). PR-6 ships a CLI loader; downstream auto-inject hook is post-roadmap.
- **Conflict resolution between Master atoms.** When two Masters loaded together contradict, no rule yet. Defer; flag in `MASTER--*` body §"Conflicts with".
- **Cross-project Master portability.** Whether a Master tagged in MSP can be loaded in `gitnexus`, `eva`, or other projects. Test post-roadmap.
- **CLAUDE.md migration.** CLAUDE.md remains the human contract; Master atoms describe rules that overlap with it. Drift is mitigated by `[[MASTER--ATOM-CONTRADICTION-POLICY]]` (PR-5) declaring Master atoms SSOT for the rules they distill.

## Source

- `[[CONCEPT--KNOWLEDGE-LAYERS-V2]]` — storage layer model (sibling)
- `[[ADR--AGENT-WRITE-BOUNDARIES]]` — write boundary that 3-tier sits on top of
- `[[FRAMEWORK--AUTHORITY-MATRIX]]` — tier mapping (T1/T2/T3 agent authority — different "tier" axis; not to be confused with this frame's Safety / Master / Genesis)
- `[[BLUEPRINT--INBOUND-TO-CANDIDATES-MIGRATION]]` — migration that closed the gap that 3-tier exists to fill
- User design dialogue (2026-05-09 session) — the polymorphism + axiomatic + manual+ADR-evidence promotion + instinct framing decisions

## Connections
- [[FRAMEWORK--MSP-ARCHITECTURE-V2]]

