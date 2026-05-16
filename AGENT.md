# AGENT.md — Project-wide rules for all AI Agents

> **Read this first.** Every AI agent operating in this repository — regardless of which tool —
> must follow the rules in this file. Tool-specific rules live in separate files:
> `CLAUDE.md` (Claude Code) · `GEMINI.md` (Gemini CLI) · `qwen.md` (Qwen CLI)

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

## P1–P4
See `CLAUDE.md` § MASTER BLOCKS for the full sector layout. Gemini-relevant Masters are listed here when promoted.

---

## 1. Who is Working in This Repo

This repository is operated by **multiple AI agents and one human (Boss)**. All agents share the
same git working tree, the same npm workspace, and the same atom store. Co-existence is possible
only if every agent respects the boundaries below.

| Agent | Tool | Role | Config file |
|---|---|---|---|
| **Claude Code** | Anthropic CLI / Desktop | T3 — architecture, deep refactors, atom authoring, design | `CLAUDE.md` |
| **Gemini CLI** | Google CLI (`gemini`) | T2 — broad context, investigation, multi-file analysis, subagent orchestration | `GEMINI.md` |
| **Qwen CLI** | Python script (`apps/qwen/`) | T1 — fast codegen, small targeted edits | `qwen.md` |
| **Antigravity** | IDE extension (VS Code) | IDE-embedded code assistant — inline completions, local context, quick fixes | _(this file §4)_ |
| **Human (Boss)** | Terminal / IDE | Owner, final approver, PR merger | — |

**No agent may push directly to `main`.** All code lands via PR, squash-merged by Boss.

---

## 2. Monorepo Layout

```
cognitive_system/                    ← monorepo root (git root)
  packages/                          ← shared libraries (imported by apps)
    gks/          @freshair129/gks   ← GKS engine library
    msp/          @freshair129/msp   ← MSP orchestrator
    skill-creator/                   ← Skill authoring tool
  apps/                              ← deployable clients (NOT imported as libraries)
    web/          @freshair129/genesis-ui  ← React web app (Vite + TS)
    qwen/         @freshair129/qwen-cli   ← Qwen subagent CLI
    cli/          @freshair129/gks-cli    ← GKS CLI for humans + AI agents (SKELETON)
    mcp/          @freshair129/gks-mcp    ← MCP server for AI agents (SKELETON)
    tui/          @freshair129/gks-tui    ← Terminal UI (SKELETON)
    desktop/                         ← Tauri desktop app (PLACEHOLDER)
    ios/                             ← Swift iOS app (PLACEHOLDER)
    android/                         ← Kotlin Android app (PLACEHOLDER)
  gks/                               ← atom store (knowledge files, NOT source code)
  scripts/                           ← repo-level automation
  AGENT.md        ← THIS FILE — read by all agents
  CLAUDE.md       ← Claude Code rules + monorepo workflow
  GEMINI.md       ← Gemini CLI rules
  qwen.md         ← Qwen rules
```

**packages/ vs apps/ rule:**
- `packages/` = things other packages/apps **import** (libraries)
- `apps/` = things that are **deployed or run** directly (executables, servers, apps)
- Never import from `apps/` — if code is shared, it belongs in `packages/`

**CLI is also an agent interface:**
- `apps/cli/` serves both humans (`gks search "..."` in terminal) and AI agents
  (Claude/Gemini invoke it via Bash tool to query GKS without importing TypeScript)
- `apps/mcp/` serves AI agents only via Model Context Protocol

**Boundary rules (ADR--MONOREPO-STRUCTURE):**
- `packages/gks/` MUST NOT import from `packages/msp/` or any `apps/*`
- `packages/msp/` depends on `packages/gks/` via `workspace:*`
- `apps/web/` reads GKS data via a **JSON snapshot only**, never imports gks/msp directly
- `apps/cli/` and `apps/mcp/` import `packages/gks` + `packages/msp` directly (same Node.js runtime)
- `apps/ios/` and `apps/android/` communicate via `server/api/` REST (cannot run Node.js)
- Every `apps/*` directory has its own `CLAUDE.md` — read it before working in that directory

---

## 3. Environment Rules (All Agents)

- **Timezone:** UTC+07:00 (Thailand / ICT) for all human-readable timestamps.
  Format: ISO 8601 with offset — `2026-05-14T11:30:00+07:00`. **Never** use `Z` unless you computed UTC yourself.
- **Working directory:** `C:\Users\freshair\cognitive_system` (Windows path) or `/c/Users/freshair/cognitive_system` (Git Bash)
- **Node:** ≥20 required. Use `npm` workspaces — run from repo root unless instructed otherwise.
- **Shell:** PowerShell or Bash both work. PowerShell: use backtick for line continuation, no `&&` chaining.

---

## 4. Antigravity — Coexistence Rules

Antigravity is a VS Code extension that embeds an AI coding assistant. It scans the project
at startup and maintains a Language Server. It is the **most fragile** agent in this setup —
two known failure modes have already caused production outages (see `INCIDENT_REPORT--ANTIGRAVITY-*.md`).

### Rules every other agent must follow to keep Antigravity healthy

**Git hygiene — most critical:**
- Never leave `.git` files inside subdirectories that Antigravity might scan.
  Claude Code worktrees at `.claude/worktrees/` each contain a `.git` file — these have previously
  crashed Antigravity's Language Server (Ref: `INCIDENT_REPORT--ANTIGRAVITY-AGENT-FAIL.md`).
- The `.gitignore` at repo root must include `.claude/` — verify before any commit.
- Do not create nested worktrees inside `packages/*` directories.

**Git config consistency:**
- `core.repositoryformatversion` must be `0` unless worktreeConfig extensions are also in use.
  If both are set inconsistently, Antigravity's Language Server crashes on startup
  (Ref: `INCIDENT_REPORT--ANTIGRAVITY-GIT-CONFIG-CONFLICT.md`).
- Check: `git config core.repositoryformatversion` — should return `0` or `1` (never mixed state).

**Lock file hygiene:**
- Keep ONE `package-lock.json` at repo root. Do NOT create `package-lock.json` inside individual
  `packages/*` subdirectories. Duplicate lock files confuse Antigravity's dependency graph analysis.

**What Antigravity does NOT do:**
- Antigravity does not read `AGENT.md`, `CLAUDE.md`, or any guidance file. It operates on code context only.
- Antigravity does not commit or push. It only suggests inline edits.
- Antigravity does not run npm scripts or shell commands.

### If Antigravity crashes

1. Check `.claude/worktrees/` — if non-empty, run `git worktree list` and prune dead ones.
2. Check `git config core.repositoryformatversion` — correct to `0` or `1` consistently.
3. Kill all background Antigravity processes (Windows: Task Manager → filter "antigravity" or "node").
4. Restart VS Code cleanly.

---

## 5. Git Collaboration Rules

All agents share one git working tree. To prevent conflicts:

| Rule | Detail |
|---|---|
| Branch naming | `claude/msp-<milestone>-<slug>` · `gemini/msp-<milestone>-<slug>` · `qwen/<slug>` |
| Never push to `main` | All merges via PR, squash-merge by Boss |
| One agent per branch | Do not have two agents committing to the same branch simultaneously |
| Commit message style | `type(scope): summary` — present tense, under 72 chars |
| Worktree cleanup | Remove worktrees immediately after branch is merged or abandoned |
| `.claude/` in .gitignore | Always — protects Antigravity from nested .git files |

---

## 6. Doc-Before-Code Workflow (Mandatory)

No code is written without a preceding spec. Phase order:

| Phase | Artifact | Location |
|---|---|---|
| P1 | `CONCEPT--` | `gks/concept/` |
| P2 | `ADR--` or `FEAT--` | `gks/adr/` or `gks/feat/` |
| P3 | `BLUEPRINT--` | `gks/blueprint/` |
| P5 | Code | `packages/*/src/` |
| P6 | `AUDIT--` | `gks/audit/` |

Before implementing: confirm `FEAT--` is `status: stable/active` and a `BLUEPRINT--` exists.

---

## 7. Atom Taxonomy (v2.3)

Canonical ref: `gks/concept/CONCEPT--TAXONOMY-V2-3.md`

| Prefix | Role |
|---|---|
| `GENESIS--` | Block Manifest — runtime entry-point of a Genesis Block (v2.3+) |
| `FRAMEWORK--` | Governance / architectural framework (prior `FRAME--` meaning) |
| `CONCEPT--` | Strategic intent |
| `FEAT--` | Feature specification |
| `ADR--` | Architectural Decision Record |
| `BLUEPRINT--` | Implementation plan |
| `GUARD--` | Structural data-integrity invariants (renamed from `GUARDRAIL--`) |
| `SPEC--` | Data shape specification |
| `AUDIT--` | Post-implementation report |
| `STACK--` | Technology stack inventory |
| `MOD--` | Module manifest |
| `EPISODE--` | Time-bounded event log (route to project store, not gks/) |

**Atom ID regex:** `^[A-Z][A-Z0-9_]*--[A-Z0-9][A-Z0-9_-]*$` — no dots, no lowercase.

---

## 8. Validation Gates

Before any atom commit:
```bash
npm run msp:index                                                # regen atomic_index.jsonl
npx tsx packages/msp/src/validator/cli.ts --root=packages/msp --all  # 0 failed required
npm run msp:check-links                                          # no dangling crosslinks
```

Before any code commit in `packages/*`:
```bash
npm run typecheck --workspace=packages/<name>
npm test --workspace=packages/<name>
```

---

## 9. Known Incident Patterns — Do Not Repeat

| Incident | Root cause | Prevention |
|---|---|---|
| Antigravity Language Server crash | `.git` file inside `.claude/worktrees/` subdirectory | Keep `.claude/` in `.gitignore`; prune worktrees immediately |
| Antigravity crash on startup | `repositoryformatversion=0` + `extensions.worktreeConfig=true` mixed | Never set `extensions.worktreeConfig` without also setting `repositoryformatversion=1` |
| Qwen/Gemini wrong output path | Relative paths resolved from wrong cwd | Always pass absolute paths or verify cwd before file writes |
| Stale atom crosslinks | Atom deleted without updating references | Run `msp:check-links` before every commit |

---

*Last updated: 2026-05-14. Owner: Boss. For tool-specific rules: `CLAUDE.md` · `GEMINI.md` · `qwen.md`*
