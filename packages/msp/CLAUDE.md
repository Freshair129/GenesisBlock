# CLAUDE.md — repo guidance for Claude Code (and contributors)

This is the project-internal contract for how Claude Code (and human contributors driving it) should work in this repo. Read top-to-bottom on a new session.

## What this repo is

**MSP** (Memory & Soul Passport) — a passport-orchestrator that travels with an AI agent, carrying memory + soul + retrieval + identity. Built on top of `@freshair129/gks` (Genesis Knowledge System) which provides atomic markdown storage + vector / graph backends.

Authoritative docs:
- `gks/frame/FRAME--MSP-ARCHITECTURE-V2.md` — top-level architecture
- `msp_spec.md` — full spec (currently 2.0.2)
- `gks/concept/CONCEPT--MSP-ROADMAP.md` — milestone plan + execution order
- `ROADMAP.md` — public summary

## Doc-to-code workflow (mandatory)

Every milestone follows this phase order. Don't skip.

```
P0 FRAME       (architecture)        gks/frame/FRAME--*.md
  ↓
P1 CONCEPT     (problem + intent)    gks/concept/CONCEPT--*.md
  ↓
P2 ADR/FEAT    (decision + API)      gks/adr/ADR--*.md, gks/feat/FEAT--*.md
  ↓
P3 BLUEPRINT   (impl plan)           gks/blueprint/BLUEPRINT--*.md
  ↓
P5 CODE        (actual src/ + tests) src/, test/
  ↓
P6 AUDIT       (what shipped)        gks/audit/AUDIT--*.md
```

**Phase 4 (TASK)** is for orchestrator handoff — usually skipped for single-developer slices. See `ADR--PROMOTION-LEVELS` for when it's required.

### Gates between phases

- Atoms must validate (`npx tsx src/validator/cli.ts --all`) before commit
- Crosslinks must resolve (`npm run msp:check-links`)
- Pre-commit hook enforces validator
- Pre-push hook runs `gks verify-flow` on touched FEATs

## Atom contradiction policy

The validator catches structural problems (schema, links, ID format, write-time anti-hallucination). It does **not** catch semantic contradiction — two atoms with valid schema can still claim opposite things about the same scope and both pass.

**Rule (Layer 0 of `BLUEPRINT--CONTRADICTION-DETECTION-IMPL`):** if a new atom claims something that conflicts with an existing `status: stable` atom of the same type, the conflicting atom MUST be explicitly superseded in the same PR:

1. Add the old atom's id to the new atom's `crosslinks.supersedes`
2. Add the new atom's id to the old atom's `crosslinks.superseded_by`
3. Flip the old atom's `status` to `superseded`

The PR reviewer must verify the above before approving any PR that adds or modifies an atom in `gks/<type>/`. The PR template (`.github/pull_request_template.md`) carries an explicit checklist for this.

**Why this is human-enforced first:** a few mechanical layers will follow (`PROTO--RECIPROCAL-SUPERSESSION`, `PROTO--DOMAIN-UNIQUENESS`, embedding similarity hint, opt-in LLM judge — all specified in `ADR--CONTRADICTION-DETECTION-STACK`), but the cheapest place to catch a contradiction is at the moment the author is writing it. Mechanical layers are scaffolding around this rule, not replacements for it.

### Mandatory Context Tracing (Crosslinks Rule)

Before modifying any existing atom or writing code based on an atom, agents (and humans) **MUST** read the related atoms listed in the `crosslinks.references` or `Source` sections of that file. 

Atoms in this architecture are not isolated; they form a dependency graph. Modifying an ADR without reading its parent CONCEPT, or implementing a feature without checking its related ADRs, leads to semantic contradictions that schema validators cannot catch. Optimize for accuracy over speed: always trace the context first.


## Useful commands

```bash
npm run msp:index                            # regen atomic_index.jsonl
npm run msp:check-links                      # verify all crosslinks resolve
npx tsx src/validator/cli.ts --all           # full atom validation
npm test                                     # vitest run
npm run typecheck                            # tsc --noEmit
```

Atom proposals at runtime go through the `msp_candidate` MCP tool (writes to
`.brain/msp/projects/<ns>/candidates/`); promotion to `gks/<type>/` is a
human PR action — there is no `gks inbound promote` or `msp:promote` CLI.
See `ADR--AGENT-WRITE-BOUNDARIES`.

### Inbound queue removed (Phase 3)

As of Phase 3 of `BLUEPRINT--INBOUND-TO-CANDIDATES-MIGRATION` (2026-05-09):

- `msp_propose` MCP tool — **removed**. Use `msp_candidate` instead.
- `npm run msp:propose` / `msp:list` / `msp:promote` scripts — **removed**.
- `scripts/msp/propose.mjs` — **deleted**.
- `.brain/msp/projects/<ns>/inbound/` — no longer written; remove the empty
  directory if present (it's gitignored). Atom supersession of
  `CONCEPT--INBOUND-QUEUE` happens in Phase 4.

## Branching + PR conventions

- Branch name: `claude/msp-<milestone>-<slug>` (e.g. `claude/msp-m7c-retrieval`)
- One milestone = one branch usually; bigger milestones split (atoms PR + impl PR)
- PRs open as **draft**; mark ready when CI is green
- Squash-merge with a 1-paragraph summary commit message
- Never merge to main without CI green on both Node 20 + 22

## Environment Rules
- **Timezone**: Use **UTC+07:00** (Indochina Time / ICT — Thailand) for human-readable timestamps in ISO 8601 offset format. Validator uses UTC absolute internally; `Date.parse()` handles offset correctly. Authoring rule: write `created_at: 2026-05-12T11:55:00.000+07:00` (TH wall-clock) — NOT `Z` suffix unless you've computed UTC yourself.


## Working with subagents (Agent tool)

The doc-to-code workflow + isolated branches make subagents an effective fan-out tool. Patterns we've used successfully:

- **M7-prep follow-up**: parallel subagents on M7a/M7e/M7-examples (independent file areas)
- **M7b**: sequential — atoms PR first, then implementation subagent reading the BLUEPRINT

### Subagent prompt template

When dispatching a subagent for milestone work, the prompt MUST include:

1. **Branch name** to create from `main`
2. **Atom paths** to read first (CONCEPT, ADR, FEAT, BLUEPRINT) — these define the contract
3. **Hard constraints** (MUST NOT) — what NOT to do (don't mutate sessions, don't add new LLM bundle, etc.)
4. **Style match** — point to a recent similar PR / module
5. **Verification commands** the agent must run before pushing
6. **Done criteria** — green CI, draft PR open, atoms validated, AUDIT atom committed

### ⚠️ Worktree mode caveat (`isolation: "worktree"`)

When you spawn a subagent with `isolation: "worktree"`, it gets a fresh git worktree with the branch checked out — but **`node_modules/` is NOT copied**. Tests that depend on installed binaries via `npx` (e.g. `test/hooks/pre-push.test.ts` symlinks the worktree's `node_modules/` into a fixture and calls `npx gks`) will fail in the worktree even though they pass on `main`.

**Symptom**: subagent reports "1 unrelated pre-existing flake" — but `npm test` on `main` shows the test passes cleanly. Don't trust the "pre-existing" framing without verifying.

**Workarounds**, in order of preference:

1. **Trust CI** — GitHub Actions runs `npm ci` on a fresh checkout, so the test runs correctly there. If subagent CI is green, the impl is fine even if local worktree run shows the failure.
2. **Add `npm ci` to subagent prompts** — instruct the agent to run it before `npm test`. Adds ~30s. Worth it to avoid false alarms.
3. **Symlink node_modules into the worktree** — quick fix:
   ```bash
   ln -s /home/user/msp/node_modules /home/user/msp/.claude/worktrees/agent-*/node_modules
   ```
   But subagents can't see the parent repo path reliably; this needs harness support.

**Reviewer's checklist when subagent reports a failing test**:

- Run the same test on `main` in isolation — if it passes there, the failure is environmental
- Check if the worktree has `node_modules/` (`ls .claude/worktrees/agent-*/node_modules/`)
- Trust CI as the source of truth

This caveat was discovered during M7b (PR #16, 2026-05-04). Keeping it documented here so future sessions don't waste time chasing the same red herring.

## Two-repo sync rule

MSP atoms sometimes reference `@freshair129/gks` features. Before claiming a GKS API exists:

1. Check `node_modules/@freshair129/gks/dist/src/memory/index.d.ts` for the symbol
2. If absent: mark the atom with a **Status note** ("aspirational pending GKS X.Y") and add an upstream proposal under `upstream/gks-proposals/`
3. Never assume CHANGELOG-documented features in unreleased GKS source are usable

`AUDIT--TWO-REPO-VALIDATION` (2026-05-04) is the canonical example.

## Out of scope (don't add to MSP)

- Atomic graph traversal logic (it's GKS scope per `ADR--GRAPH-IS-GKS-DOMAIN`)
- New embedder bundles (use `createSlmClient` / `createEmbedder` from GKS)
- New persistence layers without an ADR
- Workflow / governance enforcement that belongs in CI lints rather than runtime code

## Reading order for new sessions / new contributors

1. This file (CLAUDE.md)
2. `ROADMAP.md` (where we are)
3. `gks/frame/FRAME--MSP-ARCHITECTURE-V2.md` (architecture)
4. `gks/concept/CONCEPT--MSP-ROADMAP.md` (full milestone plan)
5. `msp_spec.md` (full spec — long; reference, not cover-to-cover)
6. The CONCEPT/ADR/FEAT/BLUEPRINT atoms for the milestone you're working on
