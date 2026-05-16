---
id: ADR--AGENT-WRITE-BOUNDARIES
phase: 2
type: adr
status: stable
tier: genesis
source_type: axiomatic
vault_id: default
title: Agent write boundaries — agents may write anywhere under `.brain/`; canon (`gks/`) reachable only via PR + CI
tags:
  - msp
  - agent
  - boundary
  - authority
  - decision
  - inbound-removal
crosslinks: {"references":["CONCEPT--KNOWLEDGE-LAYERS-V2","FRAMEWORK--MSP-ARCHITECTURE-V2","ADR--HUMAN-REVIEW-GATES"],"supersedes":["ADR--PROMOTION-WORKFLOW","ADR--PROMOTION-LEVELS"]}
created_at: 2026-05-08T17:01:00.000+07:00
---

# ADR — agent write boundaries

## Context

`[[CONCEPT--KNOWLEDGE-LAYERS-V2]]` proposes a 4-layer model that drops the inbound queue. This ADR fixes the **enforceable boundary**: where can an agent write at runtime, and where is it categorically forbidden?

The current `msp_spec.md` §13 Authority Matrix is mixed: `gks/blueprints/` is "T3 only — Claude" with direct write permission, while every other atom type goes through inbound. That mix exists because inbound was the gate. Once inbound is gone, the matrix simplifies.

## Decision

### One bright line

```
.brain/   ← any agent at runtime can write here, freely
gks/      ← only via PR + CI; agents have zero write capability
```

There is **no third option**. No "trusted agents that can write canon directly." No "human-approved automation that bypasses PR." If a write reaches `gks/<type>/`, it came through GitHub PR + CI. Period.

### Authority matrix (replaces `msp_spec.md` §13)

| Path | Agent (MCP / runtime) | Human via local edit | Human via PR |
|---|---|---|---|
| `.brain/msp/projects/<ns>/sessions/` | ✅ append | ✅ | n/a (gitignored) |
| `.brain/msp/projects/<ns>/memory/episodic_memory.json` | ✅ append | ✅ | n/a (gitignored) |
| `.brain/msp/projects/<ns>/candidates/<TYPE>--<SLUG>.md` | ✅ create / overwrite | ✅ | n/a (gitignored) |
| `.brain/msp/LLM_Contract/` | ❌ | ❌ (MSP maintainer only) | ✅ |
| `gks/<type>/*.md` | ❌ | ❌ | ✅ |
| `gks/00_index/atomic_index.jsonl` | ❌ (derived; `npm run msp:index`) | ✅ via index command | ✅ |
| `src/`, `test/`, `scripts/`, `web/` | ❌ | ✅ | ✅ |
| `package.json`, `tsconfig.json`, CI workflows | ❌ | ✅ | ✅ |
| `CLAUDE.md`, `ROADMAP.md`, `msp_spec.md` | ❌ | ✅ | ✅ |

### Why agents get full `.brain/` access (no sub-gates)

`.brain/` is **gitignored** and **per-user**. It never ships, never reaches another contributor, never affects canon. A misbehaving agent can corrupt only the local user's runtime memory — recoverable by deleting the directory.

Adding sub-gates inside `.brain/` (e.g., "agent can write episodes but not candidates") buys nothing because the blast radius is already bounded.

### Why canon has zero agent write paths

The inbound queue tried to be a "soft gate" — agents write, humans promote. In practice:

- Promotion was rarely done; PR was the actual review channel
- The validator had to scan inbound (race condition source)
- Two CLIs to maintain (`gks inbound list/promote`)
- Wrapper hacks (phase-6 patching) accumulated
- Agents that bypassed inbound and wrote to `gks/` directly went undetected (no enforcement)

A hard wall — "no agent write to `gks/` ever, no exceptions" — is mechanically simpler and matches what every other PR-gated repo already does.

### `gks/blueprints/` direct write removed

The current spec lets T3 (Claude as developer) edit `gks/blueprint/*.md` directly without the inbound flow. Under this ADR, that exception is removed: blueprints reach `gks/blueprint/` only via PR like every other atom type. Claude-as-developer still works — it just edits via PR, not via direct local write.

## Enforcement

### Mechanical

- **Pre-commit hook** (existing `examples/hooks/pre-commit.sh`) gates commits to `gks/<type>/` through `gks validate --files=<staged>` — already in place
- **CI** (existing `.github/workflows/ci.yml`) runs `npx tsx src/validator/cli.ts --all` on every push — already in place
- **`.gitignore`** — `.brain/` is already gitignored; verify the path matches the new candidates/ subdir

### Tool-level

- `msp_candidate` MCP tool writes only to `.brain/msp/projects/<ns>/candidates/`. The handler refuses any path outside that directory.
- `msp_propose` MCP tool is **deleted** in `[[BLUEPRINT--INBOUND-TO-CANDIDATES-MIGRATION]]` phase 3.
- All other MSP MCP tools that write (`msp_session_append`, `msp_episode_append`, `msp_remember`, `msp_identity_set`) target `.brain/` only — verified per tool in the migration audit.

### Documentation

- `CLAUDE.md` updated: §"Authority Matrix" replaced with the simpler bright-line above
- `msp_spec.md` §13 rewritten to match
- Knowledge Browser shows candidates clearly distinct from canon (different tab, different visual treatment)

## Consequences

### Positive

- **Race condition gone.** No agent runtime path writes to a directory the validator scans.
- **One mental model.** "Did it come through a PR? Then it's canon. Otherwise it's runtime memory."
- **Smaller MCP surface.** `msp_propose` deleted, replaced by simpler `msp_candidate`.
- **Spec simpler.** `msp_spec.md` §13 shrinks from 11 rows of mixed authority to 9 rows of one-of-three answer.
- **Bypass attempts visible.** If an agent ever does manage to write `gks/<type>/`, it's a security/permissions bug, not an "is this allowed?" debate.

### Negative

- **`gks/blueprint/` direct-write convenience lost.** Past pattern of hand-editing a blueprint locally and committing without PR review goes away. Mitigation: most blueprint edits in the repo's history were already done via PR; the few direct-edit cases were small enough to PR easily.
- **Migration cost.** All atoms that mention "inbound" or §13's mixed matrix need updates. ~10–15 atom edits, plus the spec rewrite. Tracked in `[[BLUEPRINT--INBOUND-TO-CANDIDATES-MIGRATION]]`.

### Neutral

- **Runtime agent UX unchanged.** Agents that today call `msp_propose` for runtime memory writes will call `msp_candidate` instead — same shape, different name.

## Alternatives considered

### A. Keep inbound, fix the race

Patch `propose.test.ts` to use tmpdirs (already done in PR #41), keep the rest of inbound as-is.

**Rejected because**: addresses the symptom, not the cause. Inbound's promotion CLI is still rarely used; phase-6 hack still lives; two write paths still confuse spec; race could re-emerge in any future test that touches inbound. PR #41 becomes a long-lived workaround for an obsolete subsystem.

### B. Delete inbound, give agents direct `gks/<type>/` write with a "candidate" status

Agent writes a file with `status: candidate` directly into `gks/concept/`. Validator skips files with that status. Human flips status to `stable` via PR.

**Rejected because**: blurs the canon line back. `git status` would show agent-generated files mixed with canon. CI rules become "validate unless candidate." `.gitignore` either gets weird patterns or commits the candidates. Adds complexity to undo what the bright-line is for.

### C. Delete inbound, no replacement at all

Agents that want to record durable knowledge ask the human in chat ("should I draft this as a CONCEPT?") and the human writes the atom themselves.

**Rejected because**: works for low-volume cases but loses async batching. An agent in a long session generating multiple insight candidates would either spam the user or drop most of them. Candidates/ provides the cheap intermediate that makes batching natural.

## What this ADR does NOT change

- `[[FRAMEWORK--MSP-ARCHITECTURE-V2]]` boundary between MSP and GKS — orthogonal axis
- Inbound queue removal mechanics (`[[BLUEPRINT--INBOUND-TO-CANDIDATES-MIGRATION]]` owns those)
- `[[ADR--HUMAN-REVIEW-GATES]]` — review is still required for canon entry; this ADR just narrows *where* the review happens to PR review

## Source

- `[[CONCEPT--KNOWLEDGE-LAYERS-V2]]` — motivation for the boundary
- `msp_spec.md` §13 Authority Matrix — the mixed table being simplified
- `[[ADR--HUMAN-REVIEW-GATES]]` — existing principle that review must precede canon
- PR #41, PR #42 CI failures — race condition evidence

## Connections
- [[ADR--PROMOTION-WORKFLOW]]
- [[ADR--PROMOTION-LEVELS]]

