---
id: CONCEPT--KNOWLEDGE-LAYERS-V2
phase: 1
type: concept
status: stable
tier: genesis
source_type: axiomatic
vault_id: default
title: Knowledge layers v2 — replace inbound queue with 4-layer model (session / episode / candidate / canon)
tags:
  - msp
  - knowledge
  - layers
  - candidate
  - inbound-removal
  - architecture
crosslinks: {"references":["FRAMEWORK--MSP-ARCHITECTURE-V2","CONCEPT--MEMORY-EPISODIC-WRITER","CONCEPT--MEMORY-SESSIONS"],"supersedes":["CONCEPT--INBOUND-QUEUE","CONCEPT--PROPOSAL-TYPES","CONCEPT--SUBMISSION-ENVELOPE"]}
created_at: 2026-05-08T17:00:00.000+07:00
aliases:
  - CONCEPT
  - implementation_flow
  - Strategic intent / PRD
cluster: implementation_flow
role: "Strategic intent / PRD"
---

# CONCEPT — knowledge layers v2

## Problem

The inbound queue (`.brain/msp/projects/<ns>/inbound/`) was designed as the single intake point for **agent-proposed atoms** awaiting human review before joining `gks/<type>/`. In practice:

- **Almost no atom in the repo went through inbound.** Every CONCEPT/ADR/BLUEPRINT in `gks/` was committed directly via PR (see `git log --diff-filter=A` over `gks/concept/`). Inbound's promotion CLI (`gks inbound promote`) is rarely invoked.
- **PR + CI already does what inbound does, better.** Validator runs on every push; reviewers thread on the diff; CODEOWNERS gate; squash-merge gives history. Inbound's "human review" step is a worse UX than GitHub PR review.
- **It costs to maintain.** `scripts/msp/propose.mjs` carries a phase-6 patching hack. `src/mcp/tools/propose.ts` had a cwd bug (PR #41). The race condition between `propose.test.ts` (writing to real inbound) and `cli.test.ts --all` (scanning inbound) flaked CI twice. Two `gks inbound *` CLI surfaces to keep working.

But there's one thing inbound DOES that PR review can't replace: **a runtime AI agent talking to a user has no GitHub auth**. It can't open a PR. If it has an insight worth keeping, it needs *somewhere* to put it that isn't canon and doesn't require human-in-the-loop right now. Inbound covered that case — but at the cost of doubling up on review infrastructure.

## Intent

Drop the inbound queue and replace it with a clean 4-layer model where each layer has unambiguous write authority and unambiguous purpose:

```
                                 agent              human
                                writes?            writes?
─────────────────────────────────────────────────────────
1. session     turn-by-turn       ✅                ✅
   .brain/.../sessions/             (append-only)
─────────────────────────────────────────────────────────
2. episode     consolidated       ✅                ✅
   .brain/.../memory/                (append-only)
─────────────────────────────────────────────────────────
3. candidate   atom-shaped        ✅                ✅
   .brain/.../candidates/            (write-or-ignore)
─────────────────────────────────────────────────────────
4. canon       reviewed atoms     ❌                ✅ (PR + CI only)
   gks/<type>/
─────────────────────────────────────────────────────────
```

Boundaries:

- **Layers 1–3 live in `.brain/msp/projects/<ns>/`** — local, gitignored, user-private. Agent has full write access.
- **Layer 4 is the only thing committed to git.** Reaches there only via PR + CI. Agent has zero write access.
- **No promotion CLI.** Human reviews candidates via Knowledge Browser, copies the worthwhile ones into `gks/<type>/`, opens a PR. CI validates. Done.

### Why "candidate" not "draft" / "hypothesis" / "proposal"

- **draft** — generic; says only "not finished," doesn't capture the path forward
- **hypothesis** — fits CONCEPT (factual claim) but not ADR (decision) or BLUEPRINT (plan)
- **proposal** — already overloaded by the GitHub PR + `msp_propose` flow
- **candidate** ✅ — analogous to "release candidate": atom-shaped, complete-format, awaiting selection. Works for every atom type.

### What replaces `msp_propose`

A new MCP tool `msp_candidate`:

```ts
msp_candidate({
  type: 'concept' | 'adr' | 'blueprint' | 'feat' | 'audit' | 'frame',
  proposed_id: 'CONCEPT--FOO',
  title: '...',
  body: '...full markdown body...',
  rationale: 'why I think this should become canon',
  confidence: 0.7,  // optional, agent's self-assessment
})
// writes: .brain/msp/projects/<ns>/candidates/CONCEPT--FOO.md
// frontmatter includes status: candidate, proposed_at, proposed_by_agent
```

No promotion CLI. Human curates via the web UI's new "Candidates" tab — picks ones worth keeping, copies into `gks/<type>/` (manually or via a one-button "promote to PR draft" action), opens a PR.

### What about agents that just want to remember mid-conversation?

They use what they already have:
- `msp_session_append` — append to session.jsonl (turn-level)
- `msp_episode_append` / `msp_remember` — write to episodic_memory.json (consolidated insight)

Only when an agent thinks "this is a *durable, structurally-shaped, citable* claim worth elevating" does it call `msp_candidate`. That's a smaller, intentional surface than the current "anything-could-go-into-inbound."

## Scope

**In scope:**

- Define the 4-layer model + write authority matrix
- Add `msp_candidate` MCP tool (writes atom-shaped markdown to `candidates/`)
- Add Knowledge Browser tab listing candidates with copy-to-canon affordance
- Delete inbound infrastructure: `msp_propose` tool, `scripts/msp/propose.mjs`, `gks inbound list/promote` calls in `package.json`, `.brain/msp/projects/<ns>/inbound/` directory, related tests
- Update `msp_spec.md` §13 Authority Matrix to reflect the 4-layer model
- Supersede `[[CONCEPT--INBOUND-QUEUE]]`

**Out of scope (deferred):**

- Two-tier candidate split (`hypothesis` vs `candidate` by confidence) — not adopted; one tier is simpler and review can still filter by `confidence` field
- Auto-promotion based on confidence threshold — never; canon entry stays human-gated
- Multi-agent candidate dedup — for later, when multiple agents actually contribute
- Candidate expiry / GC — for later

## Why this is safer than inbound

The inbound queue blurred a critical line: *who can write atom-shaped markdown that's discoverable by the validator and indexer?* Inbound said "agent can, but it sits in queue." The validator knows about inbound (`--all` walks it). That's the source of the race condition + the reason the wrapper has cwd quirks.

The 4-layer model puts a clean wall between `.brain/` (any agent can write anything; validator never reads here) and `gks/` (PR + CI only; validator's exclusive scan target). No race. No promotion CLI. No two-tier review.

## Source

- `[[CONCEPT--INBOUND-QUEUE]]` (superseded)
- `[[CONCEPT--MEMORY-SESSIONS]]`, `[[CONCEPT--MEMORY-EPISODIC-WRITER]]` (existing layers we keep)
- `[[FRAMEWORK--MSP-ARCHITECTURE-V2]]` § "Roles" (boundary table — extended here)
- PR #41 — propose.mjs cwd bug fix (becomes moot if we delete propose entirely)
- Race condition diagnosis in PR #41/#42 CI failures — direct evidence of inbound's hidden cost

## Connections
- [[CONCEPT--PROPOSAL-TYPES]]
- [[CONCEPT--SUBMISSION-ENVELOPE]]

