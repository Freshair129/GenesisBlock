# GEMINI.md — Guidance for Gemini CLI (T2 Agent)

> Project-wide rules live in `AGENT.md` — read that first.
> This file covers Gemini CLI-specific invocation, known caveats, and context for this repo.

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

## 1. Role in This Repo

Gemini CLI is the **T2 agent** — broad-context investigation, multi-file analysis, and
subagent orchestration. It operates alongside Claude Code (T3), Qwen CLI (T1), and
Antigravity (IDE). See `AGENT.md §1` for the full agent roster and co-existence rules.

**Use Gemini for:**

- Reading and reasoning across many files simultaneously
- Drafting specs (CONCEPT--, FEAT--, BLUEPRINT--) before handing to Claude for code
- Cross-package impact analysis
- Orchestrating Qwen as a fast-codegen subagent

**Do NOT use Gemini for:**

- Committing directly to `main` — all code via PR
- Writing inside `apps/web/` without reading `apps/web/CLAUDE.md` first
- Atom authoring without running the validation gates in `AGENT.md §8`

---

## 2. Environment

- **Timezone:** UTC+07:00 (Thailand / ICT). Format: `2026-05-14T11:30:00+07:00`. No `Z`.
- **Working directory:** `C:\Users\freshair\cognitive_system` (monorepo root)
- **Packages:** `packages/gks/` · `packages/msp/` · `apps/web/` · `packages/qwen-cli/`
- **Gemini CLI version:** `gemini --version` → 0.42.0+

---

## 3. Invocation

```bash
gemini --approval-mode plan -p "<prompt>"    # read-only investigation (safe default)
gemini --approval-mode yolo -p "<prompt>"    # auto-approve file edits (use sparingly)
```

### Windows PowerShell caveats

```powershell
# BAD — here-strings get misparsed as positional arg + flag simultaneously
gemini -p @'
multi-line prompt
'@

# GOOD — use Bash heredoc or pipe via stdin
bash -c 'gemini -p "$(cat prompt.txt)"'

# GOOD — single-line with escaped quotes
gemini --approval-mode plan -p "Analyze packages/gks/src/memory/graph/genesis-graph.ts"
```

The Gemini binary on Windows is `gemini.cmd`. Code that spawns it programmatically must pass
`shell: true` (Node) or `shell=True` (Python). On PowerShell, `&&` chaining is not available —
use `;` or `if ($?) { ... }`.

### Atom proposal via MSP (non-MCP path)

Gemini CLI does not have MCP support. For proposing candidate atoms (CONCEPT, FEAT, ADR, BLUEPRINT), use the `msp-candidate` CLI which writes to the MSP candidates queue:

```bash
msp-candidate propose \
  --id=FEAT--MY-FEATURE \
  --type=feat \
  --title="My feature title" \
  --body="initial markdown body" \
  --rationale="why this atom is proposed" \
  --root=.
```

Never write directly to `gks/<type>/` — that path is human-via-PR only per `[[ADR--AGENT-WRITE-BOUNDARIES]]` and `[[ADR--MSP-CANDIDATE-CLI]]`.

---

## 4. Antigravity Coexistence — Critical

Antigravity is an IDE-embedded agent (VS Code extension) that shares this working tree.
Two incidents have already caused Antigravity crashes due to Git state left by other agents.
**Follow these rules to prevent recurrence:**

### 4.1 Git Redundancy — `.claude/worktrees/` (INCIDENT_REPORT--ANTIGRAVITY-AGENT-FAIL)

Claude Code creates Git worktrees under `.claude/worktrees/`. Each worktree contains a `.git`
file. When Antigravity scans the project at startup, it can find these nested `.git` files
before the real root, mis-identify the worktree as the project root, and crash.

**Rules:**

- Never leave stale worktrees. Prune immediately after the branch is merged:
  `git worktree remove --force .claude/worktrees/<name>`
- Verify `.gitignore` contains `.claude/` before any commit.
- Do not create worktrees inside any `packages/*` subdirectory.
- Gemini's `--approval-mode plan` mode already blocks access to `.brain/` — extend this
  caution to `.claude/worktrees/` when scanning.

```bash
# Check for nested .git files (run after any Claude worktree session)
git worktree list
git worktree prune
```

### 4.2 Git Config Inconsistency (INCIDENT_REPORT--ANTIGRAVITY-GIT-CONFIG-CONFLICT)

Setting `extensions.worktreeConfig=true` while `core.repositoryformatversion=0` causes
Antigravity's Language Server to crash on startup with:
`core.repositoryformatversion does not support extension: worktreeConfig`

**Rules:**

- Do not set `extensions.worktreeConfig` without also setting `repositoryformatversion=1`.
- Prefer keeping the repo at `repositoryformatversion=0` with no extensions unless strictly needed.
- After any worktree-related git operations, verify: `git config --list | grep extension`

```bash
# Safe state check
git config core.repositoryformatversion   # expect: 0 (or 1 if extensions intentionally used)
git config extensions.worktreeConfig      # expect: (empty — not set)
```

### 4.3 Lock File Hygiene

Do not create `package-lock.json` inside individual `packages/*` subdirectories.
Antigravity's dependency analysis reads lock files and multiple lock files at different
levels cause incorrect resolution.

- One `package-lock.json` at repo root — that's it.
- If a sub-package auto-generates one: delete it and re-run from root.

---

## 5. Monorepo Workflow (Doc-Before-Code)

Every implementation follows this phase order. Gemini typically operates at P1–P3.

| Phase | Artifact prefix | Location |
|---|---|---|
| P1 | `CONCEPT--` | `gks/concept/` |
| P2 | `ADR--` or `FEAT--` | `gks/adr/` or `gks/feat/` |
| P3 | `BLUEPRINT--` | `gks/blueprint/` |
| P5 | Code | `packages/*/src/` |
| P6 | `AUDIT--` | `gks/audit/` |

**Before handing off to Claude (T3) for code:**

- [ ] `FEAT--` exists with `status: stable/active`
- [ ] `BLUEPRINT--` exists with `status: stable/active`
- [ ] All referenced `ADR--` are `status: stable/active`

---

## 6. Packages Overview (for context-loading)

| Package | Name | Purpose | Key entry point |
|---|---|---|---|
| `packages/gks/` | `@freshair129/gks` | GKS engine library | `src/index.ts` |
| `packages/msp/` | `@freshair129/msp` | MSP orchestrator | `src/index.ts`, `msp_spec.md` |
| `apps/web/` | `@freshair129/genesis-ui` | Genesis UI frontend | `src/App.tsx`, `CLAUDE.md` |
| `packages/qwen-cli/` | — | Qwen T1 subagent | `qwen.py` |

**When working in `apps/web/`:** read `apps/web/CLAUDE.md` before any edit.
The UI reads GKS data via `apps/web/src/data/gksData.json` (JSON snapshot) — it never
imports from `packages/gks` directly.

---

## 7. MSP CLI Tools (UCF & Governance)

The following tools are available for system orchestration and policy enforcement:

| Command | Purpose | Usage Example |
|---|---|---|
| `msp-auth` | Step-up auth management (PIN) | `msp-auth set-pin` |
| `msp-tag` | Automatic attribute tagging | `msp-tag gks/ --verbose` |
| `msp-atrophy` | Identify expired/stale knowledge | `msp-atrophy scan --root=.` |
| `msp-genesis-exec` | Execute a Genesis Block unit | `msp-genesis-exec IDENTITY-ENGINE` |
| `msp-validate` | Atom and protocol validation | `npm run msp:validate` |

---

## 8. Atom Taxonomy (v2.3)

See `AGENT.md §7` for the full table. Key prefixes Gemini handles:

| Prefix | When Gemini authors it |
|---|---|
| `CONCEPT--` | Problem definition, intent, north-star |
| `FEAT--` | Feature spec with API contract |
| `ADR--` | Decision record (present to Boss for approval) |
| `BLUEPRINT--` | Step-by-step implementation plan for T1/T3 |
| `AUDIT--` | Post-implementation review |

Atom ID format: `^[A-Z][A-Z0-9_]*--[A-Z0-9][A-Z0-9_-]*$`

---

## 8. Git Branch Convention

```
gemini/msp-<milestone>-<slug>

Examples:
  gemini/msp-phase-g-episode-search
  gemini/msp-p2-feat-inbox-pipeline
```

Commit style: `type(scope): summary` — present tense, ≤72 chars.
Never commit directly to `main`. Open PR, Boss squash-merges.

---

## 9. Validation Before Committing

```bash
# Atoms
npm run msp:index
npm run msp:validate
npm run msp:check-links

# Code (run in affected package)
npm run typecheck --workspace=packages/<name>
npm test --workspace=packages/<name>
```

---

*Last updated: 2026-05-14. For project-wide rules: `AGENT.md`. For Claude: `CLAUDE.md`. For Qwen: `qwen.md`.*
