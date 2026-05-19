# AGENTS.md — Monorepo guidance for Codex

> **Multi-agent repo.** This project is worked on by Codex, Gemini CLI, Qwen CLI, and Antigravity (IDE).
> Project-wide rules and co-existence requirements are in `AGENT.md` — read that first.
> This file covers Codex-specific rules only.

---

# 🎯 MASTER BLOCKS

> Stable cross-cutting directives. Body in `gks/master/<ID>.md`.
> P0 always loaded. P1–P3 indexed; body fetched on trigger match.
> P0/P1 assignment requires explicit user permission — agents must not self-promote.

## P0 — Always loaded (foundation)

### MASTER--ROOT-CAUSE-ANALYSIS
- **Apply when:** bug, error, ambiguous request, failed previous attempt
- **Directive:** identify and confirm root cause before any fix
- → `gks/master/MASTER--ROOT-CAUSE-ANALYSIS.md`

### MASTER--MSP-DOC-TO-CODE
- **Apply when:** new branch, PR, file in `src/|test/|scripts/|web/`
- **Directive:** atoms before code (FRAME→CONCEPT→ADR→BP→CODE→AUDIT)
- → `gks/master/MASTER--MSP-DOC-TO-CODE.md`

### MASTER--ATOM-CONTRADICTION-POLICY
- **Apply when:** PR adds/edits atom in `gks/<type>/`
- **Directive:** reciprocal supersession in same PR
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

(not listed in AGENTS.md; fetched via explicit reference)

## P4 — Archive

(superseded / deprecated; not loaded)

---

This is the project-internal contract for how Codex (and human contributors driving it) should work in this monorepo.

## What this repo is

**MSP Monorepo** — collapses the GKS storage engine, the MSP orchestrator, and the Genesis UI frontend into a single workspace.

- **`packages/gks/`** — Genesis Knowledge System engine. Atomic storage, vector/graph logic, and validation.
- **`packages/msp/`** — Memory & Soul Passport orchestrator. Uses GKS to manage agent memory and identity.
- **`apps/web/`** — Genesis UI frontend. Vite + React 19 + TypeScript knowledge graph explorer. Deployed to https://genesis-ui-eight.vercel.app/. See `apps/web/AGENTS.md` for full frontend guide.

> **History note.** GKS used to live in a separate repo `Freshair129/GksV3`. As of the 2026-05-11 monorepo migration (`ADR--MONOREPO-STRUCTURE`), that repo is **archived (read-only) on GitHub** and the canonical source lives here at `packages/gks/`. Do not push to `GksV3`; do not look there for current code or docs.

### Authoritative docs:
- `gks/framework/FRAMEWORK--MSP-ARCHITECTURE-V2.md` — Top-level architecture
- `packages/msp/msp_spec.md` — Full MSP spec
- `gks/concept/CONCEPT--TAXONOMY-V2-3.md` — atomic-knowledge prefix taxonomy (v2.3)
- `gks/spec/SPEC--GENESIS-BLOCK-MANIFEST.md` — FRAME-- frontmatter contract
- `packages/gks/README.md` — GKS engine documentation
- `packages/msp/ROADMAP.md` — Execution plan

> **Naming disambiguation**: "Genesis Block" appears with two distinct meanings in this repo. (1) **Genesis Graph Backend** = the embedded graph DB at `packages/gks/src/memory/graph/genesis-graph.ts` (Cypher v0, JSONL log). (2) **Genesis Block** = the composite knowledge unit declared by a `GENESIS--<NAME>` manifest atom per v2.3 taxonomy (contract: `SPEC--GENESIS-BLOCK-MANIFEST`).

## Monorepo Workflow

### Boundary Rules (ADR--MONOREPO-STRUCTURE)
- **GKS** (`packages/gks/`) MUST NOT import from **MSP** (`packages/msp/`).
- **MSP** depends on **GKS** via the workspace protocol (`"@freshair129/gks": "workspace:*"`).
- GKS is still publishable as a standalone library.

### Useful Commands
Run these from the repo root:
```bash
npm run msp:index           # Regen atomic_index.jsonl in packages/msp
npm run test                # Run tests across all packages
npm run typecheck           # Run tsc --noEmit across all packages
npm run build               # Build all packages
```

Per-package commands:
```bash
npm test --workspace=packages/gks
npm run msp:validate --workspace=packages/msp
npm run dev --workspace=packages/ui       # Genesis UI dev server → localhost:5173
npm run build --workspace=packages/ui     # build Genesis UI
```

## Doc-to-code workflow (mandatory)

> Alias: **Block Assembly** (bottom-up half of the Genesis Block Cycle) — see `docs/gks/PRD--GENESIS-BLOCK-CYCLE.md` for the unified vocabulary and its `Block Decomposition` counterpart.

Every milestone follows this phase order:
1. **FRAMEWORK** (architecture / governance) -> `gks/framework/` (was `FRAME--` pre-v2.3; see `ADR--TAXONOMY-V2-3-MIGRATION`)
2. **CONCEPT** (intent) -> `gks/concept/`
3. **ADR/FEAT** (decision) -> `gks/adr/` or `gks/feat/`
4. **BLUEPRINT** (plan) -> `gks/blueprint/`
5. **CODE** (actual src) -> `packages/msp/src/` or `packages/gks/src/`
6. **AUDIT** (what shipped) -> `gks/audit/`

## Atom Integrity
- Atoms must validate (`npm run msp:validate`) before commit.
- Crosslinks must resolve (`npm run msp:check-links`).
- Context tracing is mandatory: read related atoms before coding.

## Branching + PR conventions
- Branch name: `Codex/msp-<milestone>-<slug>`
- Squash-merge with a 1-paragraph summary.
- CI must be green on both Node 20 + 22.

## Environment Rules
- **Timezone**: Use **UTC+07:00** (Indochina Time / ICT — Thailand) for human-readable timestamps in ISO 8601 offset format. Validator uses UTC absolute internally; `Date.parse()` handles offset correctly. Authoring rule: write `created_at: 2026-05-12T11:55:00.000+07:00` (TH wall-clock) — NOT `Z` suffix unless you've computed UTC yourself.

## Antigravity Coexistence
Antigravity (IDE agent) shares this working tree. Codex's worktrees have previously crashed it.
- Always clean up worktrees: `git worktree prune` after any branch work.
- `.Codex/` must stay in `.gitignore` — verify before committing.
- Never set `extensions.worktreeConfig` in git config without `repositoryformatversion=1`.
- Full details and incident post-mortems: `AGENT.md §4` + `INCIDENT_REPORT--ANTIGRAVITY-*.md`

