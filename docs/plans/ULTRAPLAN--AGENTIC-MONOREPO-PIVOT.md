# ULTRAPLAN — Agentic Monorepo Pivot

> **Executor:** Gemini CLI (`gemini --approval-mode yolo`)
> **Reviewer:** Claude / human
> **Branch base:** `claude/monorepo-pivot-prep` (already created from `main`)
> **Status:** READY for autonomous execution per phase; HALT at gates ⛔
> **Last updated:** 2026-05-13T18:35:00+07:00

---

## 0. Context (read this first — full self-contained brief)

### 0.1 The strategic pivot

The `cognitive_system` monorepo was originally built with `packages/gks/` as a **standalone publishable library** (`@freshair129/gks` on npm). That design imposed constraints — each package owned its own docs, scripts, and atom vault. The result is duplication:

- `gks/` ← gks-engine atom vault (~9 atoms)
- `gks/` ← msp-orchestrator atom vault (~245 atoms)
- `docs/gks/` + `docs/msp/` ← two doc trees
- `scripts/msp/` + `packages/msp/scripts/` ← two script roots
- `packages/{gks,msp}/CLAUDE.md` ← two sub-package agent guides

The user has pivoted: **drop standalone publish**. The monorepo is the product — an agentic system that orchestrates pluggable cognitive agents (Claude Code, Gemini CLI, Qwen CLI, Hermes-like, etc.). With this constraint dropped, the canonical layout from `FRAMEWORK_MASTER_SPEC.md §4.2` applies:

```
PROJECT/
├── CLAUDE.md, GEMINI.md, qwen.md, AGENT.md     ← per-tier agent guides
├── registry.yaml, system_config.yaml           ← top-level config
├── .agents/                                     ← project-specific skills/hooks
├── gks/                                         ← SHARED BRAIN (single, root-level)
├── msp/                                         ← project governance + LLM_Contract
├── scripts/msp/                                 ← validator + codegen + compose
├── docs/                                        ← unified docs
└── src/                                         ← project source code (in user projects)
```

The monorepo itself ALSO follows this layout, since it is its own reference project.

### 0.2 Two-brain architecture

At runtime:

- **GLOBAL `~/.brain/`** (per-user, private, cross-project)
  - `~/.brain/gks/` ← user's cross-project knowledge atoms
  - `~/.brain/msp/projects/<path-encoded>/` ← per-project session/episodic/candidate state
- **PROJECT `<project>/gks/`** (committed to project's git)
  - The canonical, reviewed atom vault for that project

MSP orchestrates between the two. The cognitive_system monorepo's own `gks/` (at root, post-pivot) is the canonical seed brain that ships with the framework template.

### 0.3 Agent tier stack

- **T1 (fast codegen)** = Qwen CLI — uses `qwen.md` in project root
- **T2 (mid-tier reasoning)** = Gemini CLI — uses `GEMINI.md`
- **T3 (architectural)** = Claude Code (Opus/Sonnet) — uses `CLAUDE.md`
- **Project-wide rules** = `AGENT.md` — parent that all three tiers inherit

### 0.4 Decisions already locked

- Naming: keep `.brain/` (not `.mind`/`.cognitive`/`.conscious`)
- Atom-vault layout: **flat type-based** under root `gks/` (matches ADR-013, no per-package split)
- Standalone-publish: DROPPED; `packages/gks/` becomes an internal sub-workspace (no longer `npm publish`-able)
- Sub-package CLAUDE.md: KEEP but SHRINK to "scope-specific overrides + pointer to root"
- Genesis Block prefix: `GENESIS--` (confirmed; FRAME-- placeholder retired in PR #100)

---

## 1. Target Architecture

```
cognitive_system/                          # monorepo root
├── AGENT.md                               # project-wide rules (T*-agnostic)
├── CLAUDE.md                              # T3 — Claude Code
├── GEMINI.md                              # T2 — Gemini CLI
├── qwen.md                                # T1 — Qwen CLI (NEW)
├── FRAMEWORK_MASTER_SPEC.md
├── ROADMAP.md                             # moved from packages/msp/
├── CHANGELOG.md                           # (NEW or moved)
├── package.json                           # workspaces: ["packages/*"]
├── package-lock.json
├── tsconfig.base.json                     # NEW — shared TS compilerOptions
├── registry.yaml                          # NEW (skeleton; ROLES only, no business config)
├── system_config.yaml                     # NEW (skeleton)
├── .agents/                               # NEW dir (empty placeholder for project-specific hooks)
│
├── gks/                                   # 🧠 UNIFIED shared brain
│   ├── 00_index/atomic_index.jsonl       # single index
│   ├── concept/                           # all merged atoms by type
│   ├── adr/  feat/  blueprint/  algo/  flow/  entity/
│   ├── framework/  genesis/               # NEW dir: gks/genesis/ for GENESIS-- atoms
│   ├── audit/  proto/  master/  mod/  protocol/  spec/
│   ├── cognitive/  stack/  safety/  runbook/  params/  guard/
│   ├── skill/  policy/  persona/  fr/  nfr/  constraint/
│   ├── inc/  risk/  runbook/  slo/  issues/  task/  hotfix/
│   └── (one folder per atom type)
│
├── msp/                                   # 🛡️ project governance
│   ├── ARCHITECTURE_OVERVIEW.md          # (NEW; can be a pointer to FRAMEWORK--MSP-ARCHITECTURE-V2)
│   ├── LLM_Contract/
│   │   └── atomic_contract.yaml          # moved from msp/LLM_Contract/
│   └── rules/                             # (NEW empty dir for project-specific agent rules)
│
├── scripts/                               # unified tooling
│   ├── msp/                              # was scripts/msp/
│   │   ├── re-indexer.ts
│   │   ├── validator-cli.ts (entry shim)
│   │   ├── migrate-genesisblock-to-genesisgraph.mjs   # historical, keep
│   │   ├── migrate-frame-to-framework.mjs             # historical, keep
│   │   ├── migrate-monorepo-pivot.mjs                 # NEW (this plan's PR-B)
│   │   ├── atom-date.ts  scaffold-atom.ts  supersede.ts
│   │   └── ...
│   └── (gks engine scripts merged into scripts/msp/ since validator IS msp's; gks scripts are minor)
│
├── docs/                                  # unified documentation
│   ├── gks/                              # was docs/gks/
│   │   ├── ARCHITECTURE.md  KNOWLEDGE-TYPES.md  TECHNICAL-OVERVIEW.md
│   │   ├── WORKFLOW.md  ONBOARDING.md  MSP_RELATIONSHIP.md
│   │   ├── MIGRATIONS.md  OBSERVABILITY.md  BENCHMARKS.md  ULTRAPLAN.md
│   │   ├── embedder-compatibility.md
│   │   └── adr/  (the numbered legacy ADRs — 001–015)
│   ├── msp/                              # was docs/msp/
│   │   ├── AGENT-INTEGRATION.md
│   │   └── UNIVERSAL-CONTEXT-FRAMEWORK_spec.md
│   └── plans/
│       └── docs/plans/ULTRAPLAN--AGENTIC-MONOREPO-PIVOT.md  ← THIS FILE moves here after PR-B
│
└── packages/
    ├── gks/                              # @freshair129/gks (workspace-only, not published)
    │   ├── src/   (TypeScript engine — unchanged)
    │   ├── test/  (unchanged)
    │   ├── dist/  (build output)
    │   ├── package.json   (workspace-only — no main/types/exports needed for publish)
    │   ├── tsconfig.json  (extends ../../tsconfig.base.json)
    │   └── CLAUDE.md      (shrunk to ≤ 30 lines — scope-specific overrides + pointer to root)
    ├── msp/                              # msp orchestrator (workspace-only)
    │   ├── src/   (TypeScript orchestrator — unchanged)
    │   ├── test/  (unchanged)
    │   ├── dist/  (build output)
    │   ├── package.json
    │   ├── tsconfig.json  (extends ../../tsconfig.base.json)
    │   └── CLAUDE.md      (shrunk)
    └── qwen-cli/                         # NEW workspace location for the Python subagent
        ├── package.json   (workspace metadata; Python project)
        ├── qwen.py  setup.py  ...
        └── (was at repo root)
```

**Removed from `packages/<x>/`:**
- `gks/`     → merged into root `gks/`
- `docs/gks/`    → moved to root `docs/gks/`
- `scripts/msp/` → moved to root `scripts/msp/` (consolidated)
- `gks/`     → merged into root `gks/`
- `docs/msp/`    → moved to root `docs/msp/`
- `packages/msp/scripts/` → moved to root `scripts/msp/`
- `packages/msp/.brain/`  → only LLM_Contract is canonical (moved to root `msp/`); rest is runtime-generated and stays gitignored

---

## 2. Execution Phases

### PHASE A — Pivot prep (LOW RISK, ~30 min)

**Goal:** Document the strategic decision, prepare workspace, add foundational config.

#### A.1 New ADR — pivot documentation

Author atom: `gks/adr/ADR--AGENTIC-MONOREPO-PIVOT.md` (BEFORE atom moves; will move with PR-B).

Required frontmatter (validator-compliant):

```yaml
---
id: ADR--AGENTIC-MONOREPO-PIVOT
phase: 2
type: adr
status: stable
tier: genesis
source_type: axiomatic
vault_id: default
title: Drop standalone publish; cognitive_system monorepo IS the product (agentic, agent-pluggable)
tags: [msp, gks, monorepo, agentic, pivot, decision, foundation]
crosslinks: {"references":["ADR--MONOREPO-STRUCTURE","FRAMEWORK--MSP-ARCHITECTURE-V2","CONCEPT--AGENT-AGNOSTIC"],"supersedes":[]}
created_at: 2026-05-13T18:35:00+07:00
---
```

Body sections to include:
- §1 Context (the standalone-publish requirement and its costs)
- §2 Decision (drop publish; embrace agentic-monorepo)
- §3 Consequences (positive: shared resources; negative: not separately installable)
- §4 What this enables (PR-B brain unification, PR-C tooling centralize)
- §5 What's NOT affected (engine separation of concerns; ADR-008 storage-engine scope still holds within the monorepo)

#### A.2 Amend existing ADRs

- `docs/gks/adr/008-gks-storage-engine-scope.md` — append a "Post-2026-05-13 note" stating that GKS remains a storage-engine in scope but is no longer published standalone; production deployment is via the agentic monorepo.
- `gks/adr/ADR--MONOREPO-STRUCTURE.md` — append "Post-2026-05-13 amendment: pivot to agentic monorepo per `ADR--AGENTIC-MONOREPO-PIVOT`; canonical layout per FRAMEWORK_MASTER_SPEC §4.2 will be materialised in PR-B."

#### A.3 Add `qwen.md` skeleton at root

File: `qwen.md` (root). ≤ 60 lines. Same structure as `GEMINI.md`:

- Header explaining T1 role
- Environment Rules (UTC+07:00 ICT, working dir)
- Invocation patterns (qwen CLI binary path — see `packages/qwen-cli/` after A.4)
- Caveats (Python-based; not yet wired as MSP_SLM_PROVIDER alternative)
- Atom Taxonomy reference (same as AGENT.md §"Atom taxonomy")
- Pointer to root `AGENT.md` for project-wide rules

#### A.4 Move `qwen-cli/` → `packages/qwen-cli/`

```bash
git mv qwen-cli packages/qwen-cli
```

Update `qwen.md` path references to point at the new location.

#### A.5 Add `tsconfig.base.json` at root

Content:

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "NodeNext",
    "moduleResolution": "NodeNext",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "resolveJsonModule": true,
    "forceConsistentCasingInFileNames": true,
    "noUncheckedIndexedAccess": true,
    "lib": ["ES2022"]
  }
}
```

Refactor `packages/gks/tsconfig.json` and `packages/msp/tsconfig.json` to extend it:

```jsonc
{
  "extends": "../../tsconfig.base.json",
  "compilerOptions": {
    /* package-specific overrides only */
  },
  "include": ["src", "test"]
}
```

#### A.6 Verify

```bash
npm install
npm run typecheck                           # must pass on all workspaces
npm test --workspace=packages/gks           # smoke check
npm test --workspace=packages/msp -- --testNamePattern "validator"   # quick subset
```

#### A.7 ⛔ GATE A — human review

**STOP.** Open PR, request review. Do NOT proceed to PHASE B until reviewer approves.

---

### PHASE B — Brain unification (HIGH RISK, ~2 hours)

**Goal:** Materialise the canonical `gks/` at root by merging both per-package atom vaults. Update every path reference.

> ⚠️ This phase touches ~250 atoms + hundreds of path references. **Run the migration script with `--dry-run` first**, review the change set with a human, then apply.

#### B.1 Write the migration script

File: `scripts/msp/migrate-monorepo-pivot.mjs` (NEW, after `scripts/` is created in A or here).

Script responsibilities (mirror the structure of `scripts/msp/migrate-genesisblock-to-genesisgraph.mjs` from PR #100 — a working template):

1. **Atom file moves** — for each file matching `packages/{gks,msp}/gks/<type>/<ID>.md`:
   - Determine target: `gks/<type>/<ID>.md` (root)
   - Create target dir if needed
   - Move file (preserve content; do not rewrite frontmatter beyond what step 4 demands)
   - Delete empty source directories afterwards
2. **Index move** — `gks/00_index/atomic_index.jsonl` is regenerated, not moved. The script SHOULD delete both old indexes after move completes.
3. **`atomic_contract.yaml` move** — `msp/LLM_Contract/atomic_contract.yaml` → `msp/LLM_Contract/atomic_contract.yaml`. Update any path constants in `packages/msp/src/validator/contract.ts` (or wherever loaded — search for the string).
4. **Frontmatter `linked_symbols` rewrites** — every `linked_symbols.file` that says `packages/gks/...` or `packages/msp/...` STAYS unchanged (those still point at src/). But any `linked_symbols.file` that says `gks/...` or relative paths from the atom file's POV needs updating since the atom file moved. Audit each manually if the script can't safely rewrite.
5. **Body path-string rewrites** — across ALL markdown files in the repo, rewrite:
   - `gks/<type>/<ID>.md` → `gks/<type>/<ID>.md`
   - `gks/<type>/<ID>.md` → `gks/<type>/<ID>.md`
   - `msp/LLM_Contract/` → `msp/LLM_Contract/`
   - `scripts/msp/` → `scripts/msp/`
   - `scripts/msp/` → `scripts/msp/` (consolidated)

#### B.2 Run dry-run + review

```bash
node scripts/msp/migrate-monorepo-pivot.mjs --dry-run > /tmp/pivot-dry.log 2>&1
```

Inspect `/tmp/pivot-dry.log`. Expected ballpark:
- Atom files moved: ~250 (between both packages)
- Path-string rewrites: ~500 across ~150 markdown files
- Frontmatter rewrites: ~50 (linked_symbols within atoms that referenced sibling atoms by path)

#### B.3 Apply migration

```bash
node scripts/msp/migrate-monorepo-pivot.mjs
```

#### B.4 Update source paths

Files known to hardcode paths (search-replace):

- `scripts/msp/msp/re-indexer.ts` — `GKS_ROOT`, `INDEX_PATH` constants
- `packages/msp/src/validator/cli.ts` — default `--root` (was `packages/msp`, becomes `.`)
- `packages/msp/src/validator/contract.ts` — `atomic_contract.yaml` resolver
- `packages/gks/src/memory/index.ts` (`gksLayout()` function) — atomic-index path under `gks/00_index/`
- `package.json` (root) — npm scripts: `msp:index`, `msp:check-links`, `msp:validate` should all `--root=.` now
- `.gitignore` — review excluded paths (`gks/00_index/atomic_index.jsonl` was excluded; now `gks/00_index/atomic_index.jsonl`)

#### B.5 Regen index + validate

```bash
npm run msp:index
npx tsx scripts/msp/validator-cli.ts --root=. --all
npm run msp:check-links --workspace=packages/msp
npm run typecheck
npm test
```

**Expected**: 251 (or current) atoms pass, 0 fail. Test suite unchanged. Typecheck clean.

#### B.6 Cleanup empty dirs

```bash
rmdir gks/00_index gks/{type,...} packages/gks/gks
rmdir gks/{00_index,concept,adr,feat,...} packages/msp/gks
# (script should do this; double-check)
```

#### B.7 ⛔ GATE B — human review

**STOP.** Open PR. Reviewer must verify:
- Atom IDs unchanged
- No crosslinks broken
- `gks/00_index/atomic_index.jsonl` is the only index file
- All tests pass on CI
- `packages/{gks,msp}/gks/` directories no longer exist

Do NOT proceed to PHASE C until merged.

---

### PHASE C — Tooling + docs consolidation (MEDIUM RISK, ~1 hour)

**Goal:** Finish the canonical layout — move docs/scripts to root, shrink sub-package CLAUDE.md.

#### C.1 Move scripts

```bash
git mv packages/msp/scripts/msp scripts/msp-tmp
mkdir -p scripts/msp
git mv scripts/msp-tmp/* scripts/msp/
# any gks-specific scripts merge into scripts/msp/ or scripts/gks/ at reviewer discretion
git mv scripts/msp/* scripts/msp/   # gks has scaffold helpers; consolidate
rmdir packages/{gks,msp}/scripts
```

Update root `package.json` workspaces scripts — replace `--workspace=packages/msp` script invocations with direct calls:

```json
"msp:index": "tsx scripts/msp/re-indexer.ts",
"msp:check-links": "tsx scripts/msp/check-links.ts || gks validate --links --root=.",
"msp:validate": "tsx scripts/msp/validator-cli.ts --all"
```

#### C.2 Move docs

```bash
mkdir -p docs/gks docs/msp docs/plans
git mv docs/gks/* docs/gks/
git mv docs/msp/* docs/msp/
git mv docs/plans/ULTRAPLAN--AGENTIC-MONOREPO-PIVOT.md docs/plans/
```

#### C.3 Move `LLM_Contract` (if not done in B.1)

```bash
mkdir -p msp/LLM_Contract msp/rules
git mv msp/LLM_Contract/atomic_contract.yaml msp/LLM_Contract/
# .brain at packages/msp/ is gitignored runtime state — leave alone
```

#### C.4 Shrink sub-package CLAUDE.md

Replace `packages/gks/CLAUDE.md` (currently ~250 lines) with a ≤ 30-line shim:

```markdown
# GKS package — scope-specific overrides

> Project-wide rules: see root `AGENT.md` and root `CLAUDE.md`.
> This file documents GKS sub-system specifics only.

## Quick commands (run from packages/gks)
```sh
npm run build                   # tsc -p tsconfig.build.json
npm test                        # vitest run
```

## Scope reminder
GKS is the storage-engine sub-system. See `../../docs/gks/SCOPE.md`.
Do not let MSP-layer concerns leak into `packages/gks/src/`.

## Boundary
- May NOT import from `packages/msp/`
- May export to `@freshair129/gks` (workspace-internal only, post-pivot)
```

Same treatment for `packages/msp/CLAUDE.md` — shrink to ≤ 40 lines.

#### C.5 Add `registry.yaml`, `system_config.yaml`, `.agents/` skeletons

Top-level skeletons per canonical spec. Empty placeholders are fine for now.

#### C.6 Final verify

```bash
npm install                                 # workspaces still resolve
npm run typecheck
npm test
npm run msp:validate
npm run msp:check-links
```

#### C.7 ⛔ GATE C — human review

**STOP.** Open PR. Final review.

---

## 3. Verification Protocol (every phase)

Run before opening PR at each gate:

```bash
# 1. Workspace resolution
npm install

# 2. TypeScript health
npm run typecheck

# 3. Atom integrity
npm run msp:index
npx tsx scripts/msp/validator-cli.ts --root=. --all
# Expected: ≥ 251 passed, 0 failed (count depends on phase)

# 4. Crosslink integrity
npm run msp:check-links
# Expected: status: OK

# 5. Test suite
npm test
# Expected: previous green count maintained or improved (Windows-skips OK)

# 6. Build
npm run build
```

If any of (2)–(5) fail, HALT phase and report to reviewer.

---

## 4. Rollback Strategy

Each phase is on its own branch. Rollback is `git reset --hard <pre-phase-commit>` or `gh pr close` without merge.

**Phase B** has the largest blast radius. The migration script must accept `--inverse` to undo atom moves and path rewrites. The script's `--dry-run` output (saved to `/tmp/pivot-dry.log`) doubles as a rollback log.

The previous repo state is fully recoverable from git as long as PR-B has not been squash-merged. After merge, rollback requires either:
- A revert PR (clean if no follow-up commits)
- A new migration in the inverse direction (if other PRs landed on top)

---

## 5. Decision points reserved for human review

Gemini MUST NOT decide these autonomously. If unclear, HALT and ask the reviewer.

1. **In B.1**, when the migration script encounters an atom whose `linked_symbols.file` references a path that no longer exists post-move — DO NOT silently rewrite. Log a warning and let the reviewer triage.
2. **In C.1**, the boundary between `scripts/msp/` and a possible `scripts/gks/` is ambiguous if there are gks-specific scaffolders. Default to consolidating into `scripts/msp/`; flag any non-trivial split for the reviewer.
3. **In C.4**, the exact content of the shrunk CLAUDE.md is editorial. Use the template here as a starting point but flag the diff for review.
4. Any unexpected typecheck or test failure that isn't fixed by a path update — HALT.

---

## 6. Out of scope (do NOT do)

- Renaming any existing atom IDs (we just did GENESIS-- rename in PR #100; no more taxonomy churn)
- Adding new atoms beyond `ADR--AGENTIC-MONOREPO-PIVOT` in Phase A
- Rewriting `FRAMEWORK_MASTER_SPEC.md` body (only Changelog + section pointers)
- Touching `~/.brain/` or any global state on the executor's machine
- Changing the validator's required-fields contract
- Implementing the `members.*` validator predicate proposed in `SPEC--GENESIS-BLOCK-MANIFEST` §5

---

## 7. Output reporting

After each phase, Gemini SHOULD post (as PR description or in `docs/plans/AUDIT--MONOREPO-PIVOT-PHASE-<A|B|C>.md`):

- Files moved (count + first 10 as sample)
- Path rewrites applied (count + sample)
- Verification command output (typecheck, validator totals)
- Any HALT points encountered + the resolution

---

## 8. Critical files (reviewer should pre-read)

Before approving any PR:

- `gks/concept/CONCEPT--TAXONOMY-V2-3.md` — current taxonomy
- `gks/spec/SPEC--GENESIS-BLOCK-MANIFEST.md` — Genesis Block contract
- `FRAMEWORK_MASTER_SPEC.md` §4.2 — the canonical layout being materialised
- `scripts/msp/migrate-genesisblock-to-genesisgraph.mjs` — template for the new migration script
- `docs/gks/adr/008-gks-storage-engine-scope.md` — the ADR being amended

---

End of ULTRAPLAN.
