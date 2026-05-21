# HANDOFF — Post-Phase-F work, for Gemini CLI execution

**Author:** Claude (Opus 4.7) · **Date:** 2026-05-14 · **Branch base:** `main` @ post-#135

This brief hands off the remaining cognitive_system work after the agentic
monorepo pivot (Phases D/E/F) closed out. It is written so Gemini CLI can pick
up without re-deriving context.

---

## 1. Where things stand

The agentic monorepo pivot is **done and merged**: Phase D (runtime), E
(features), F (refinements), plus closeout audits. See `ROADMAP.md` and
`AUDIT--PHASE-F-COMPLETE`.

Also merged this session: episode-path bug fix (#129), `PROTO--GENESIS-BLOCK-MEMBERSHIP`
(#130), PROTO `linked_symbols` path-drift fix for 11 PROTO atoms (#131), all 9
`PROTO--PHASE-GATES` error findings resolved (#132), `PROTO--SYMBOLS-TRACE-INVARIANTS`
wired (#134), UCF P1 CONCEPTs (#99) + P2 ADRs (#128), workspace-CI fixes (#135),
and the monorepo layout refactor (#136). #133 (UCF P2 FEATs) is the last open PR.

The PROTO validation layer now loads **14** predicates (was 2).

### Monorepo layout (post-#136)

The repo split `packages/` (libraries) from `apps/` (deployables):
- `packages/gks` — Genesis Knowledge System engine (storage, vector/graph, validation)
- `packages/msp` — Memory & Soul Passport orchestrator (atoms live in repo-root `gks/`)
- `apps/web` — Genesis UI (was `packages/ui`); `apps/qwen` (was `packages/qwen-cli`)
- `apps/cli`, `apps/mcp`, `apps/tui` — TS skeleton apps (scaffold only — `src/index.ts`
  stubs, `tsconfig.json` + no-op `test` script present so workspace CI passes)
- `apps/android`, `apps/ios`, `apps/desktop` — doc-only dirs (no `package.json`, not workspaces)

Every workspace package MUST carry `build`, `typecheck`, and `test` scripts —
the root runs each as `npm run <x> --workspaces`. A workspace missing one breaks
CI for the whole repo (this is what #135 fixed). Stub non-applicable ones with
`echo "..." && exit 0`.

---

## 2. What's left — four tiers

### Tier 1 — mechanical, hand off freely

**T1-A · `linked_symbols` path-drift sweep (64 atoms).**
64 real atoms under `gks/` still carry pre-monorepo `linked_symbols` paths
starting `src/…` instead of `packages/msp/src/…`. This is the same drift #131
fixed for the 11 PROTO atoms — now do the rest (audit/blueprint/feat/adr).
- Find them: `grep -rl '"file":"src/' gks/ --include="*.md"`
- For each `{"file":"src/X"}` → `{"file":"packages/msp/src/X"}` **only if**
  `packages/msp/src/X` actually exists on disk. Some files may have moved or
  been deleted post-monorepo — if the target doesn't exist, leave it and list
  it in the PR description for a human to triage.
- Also check `test/…` paths the same way → `packages/msp/test/…`.
- Regen index + validate after: `npm run msp:index && npm run msp:validate`.
- One PR, titled like #131. No source code changes.

**T1-B · ADRs missing a CONCEPT crosslink (~12).**
`PROTO--PHASE-GATES` emits a soft `[warning]` for every phase-2 ADR whose
`crosslinks.references` cites no `CONCEPT--`. Run `npm run msp:validate` and
read the `PROTO--PHASE-GATES` block for the current list. For each: if a
relevant `CONCEPT--` atom exists, add it to `crosslinks.references`; if none
exists, leave it (do **not** invent a CONCEPT). These are warnings, never
fail-exit — low risk, but improves the doc graph. One PR.

### Tier 2 — needs a judgment call; propose, don't unilaterally decide

**T2-A · `PROTO--TRACE-INVARIANTS` vs `PROTO--SYMBOLS-TRACE-INVARIANTS`.**
Two `type: proto` atoms, same title "PROTO — Trace Invariants", overlapping
rules (acyclic / termination / referential-integrity), both non-superseded
(`PROTO--TRACE-INVARIANTS` is `stable` Thai-prose with no predicate;
`PROTO--SYMBOLS-TRACE-INVARIANTS` is `active` with a wired predicate). This is
a likely contradiction per the atom-contradiction policy. Gemini should
**open a PR proposing** a resolution (supersede one, or merge) with reciprocal
`crosslinks.supersedes`/`superseded_by` — but flag it for human review rather
than picking unilaterally. Context in `AUDIT--WIRE-TRACE-INVARIANTS-PROTO`.

**T2-B · `trace-invariants.ts` is a stub.**
`packages/msp/src/validator/proto/trace-invariants.ts` currently returns
`{ ok: true, violations: [] }`. The real referential-integrity checks
(Rule 1/2/3 in `PROTO--SYMBOLS-TRACE-INVARIANTS`) are not implemented. This
needs design — depends on T2-A's outcome and on the symbol-graph schema.
Defer until T2-A is decided.

### Tier 3 — the big track: UCF implementation

The Universal Context Framework (`FRAMEWORK--UNIVERSAL-CONTEXT-FRAMEWORK`) has
its doc-to-code chain built up to P2:
- **P0/P1** — framework + 3 foundational CONCEPTs (#84), 4 capability CONCEPTs
  (#99: `CONCEPT--ABAC-POLICY-ENGINE`, `--RESOLUTION-GRADIENT`, `--STEP-UP-AUTH`,
  `--SUBAGENT-CONTEXT-SCOPING`)
- **P2** — 6 ADRs (#128), 5 FEATs (#133: `FEAT--POLICY-DECISION-POINT`,
  `--RESOLUTION-EXPAND-ON-DEMAND`, `--STEP-UP-AUTH-PIN`,
  `--SUBAGENT-SCOPE-FILTERING`, `--VAULT-COMPOSITION`)

**Next: P3 BLUEPRINTs, then P4/P5 code, then P6 audits** — one BLUEPRINT per
FEAT, then the implementation. This is the largest remaining body of work and
should be planned as its own milestone (a dedicated ULTRAPLAN, like
`docs/plans/ULTRAPLAN--AGENTIC-MONOREPO-PIVOT.md`). Do **not** start coding
P4 before the P3 BLUEPRINTs exist and validate — the doc-to-code workflow and
`PROTO--PHASE-GATES` both require it.

### Blocked — do not start

**`GENESIS--IDENTITY-ENGINE` + `BLUEPRINT--GENESIS-BLOCK-RUNTIME`.**
Blocked on unauthored member atoms (`COGNITIVE--EGO-DEATH-PASSPORT`,
`RUNBOOK--IDENTITY-MIGRATION`, `STACK--MSP-NODE-RUNTIME`, etc.). The contract
they must satisfy is already enforced by `PROTO--GENESIS-BLOCK-MEMBERSHIP`
(see `SPEC--GENESIS-BLOCK-MANIFEST`). Authoring the member atoms is a
prerequisite milestone of its own.

---

## 3. Repo conventions — non-negotiable

Read `CLAUDE.md`, `GEMINI.md`, `packages/msp/CLAUDE.md` first. Key points:

- **Doc-to-code order:** FRAMEWORK → CONCEPT → ADR/FEAT → BLUEPRINT → CODE →
  AUDIT. Never write code without a phase-3 BLUEPRINT covering it (or set
  `phase_override.skip_blueprint: true` with a reason — only for genuinely
  incremental work).
- **Branches:** `gemini/<milestone>-<slug>`. PRs open as **draft**; mark ready
  only when CI is green on Node 20 **and** 22. Squash-merge.
- **Before every commit:** `npm run msp:index` then `npm run msp:validate`
  (atoms must validate, crosslinks must resolve). Then `npm run typecheck` and
  `npm run test`.
- **Atom timestamps:** `created_at` in ICT (`+07:00`), never in the future.
- **PROTO `linked_symbols`:** paths are **repo-root-relative** —
  `packages/msp/src/validator/proto/<name>.ts`, not `src/…`. The loader
  silently drops PROTOs whose predicate path doesn't resolve.
- **Atom contradiction policy:** a new atom conflicting with a `stable` atom
  of the same type MUST supersede it via reciprocal `crosslinks` in the same
  PR (see `packages/msp/CLAUDE.md`).
- **Boundary rule:** `packages/gks` MUST NOT import from `packages/msp`;
  `apps/web` reads only the JSON snapshot, never imports gks/msp.
- **Workspace scripts:** every `apps/*` and `packages/*` with a `package.json`
  needs `build` + `typecheck` + `test` (stub with `echo … && exit 0` if N/A) —
  the root runs them `--workspaces` and one gap fails CI repo-wide.
- **Known local-only CI noise:** `symbol-tools` / `bin` / `validate --all`
  tests fail locally on Windows (better-sqlite3 native bindings) — these are
  green in CI. Don't chase them.

---

## 4. Recommended order for Gemini

1. **T1-A** (path-drift sweep) — biggest mechanical win, unblocks honest
   `linked_symbols` across the vault.
2. **T1-B** (ADR CONCEPT crosslinks) — quick, improves the doc graph.
3. **T2-A** (trace-invariants contradiction) — open as a *proposal* PR.
4. **Tier 3 kickoff** — write `docs/plans/ULTRAPLAN--UCF-IMPLEMENTATION.md`
   first (the plan), get human sign-off, *then* start P3 BLUEPRINTs.

Leave T2-B and the blocked GENESIS work alone until their prerequisites land.

---

## 5. Definition of done per PR

- CI green on Node 20 + 22 (draft → ready only after green).
- `npm run msp:validate` clean for touched atoms; crosslinks resolve.
- `npm run typecheck` + `npm run test` pass.
- Squash-merged with a one-paragraph summary and `Co-Authored-By`.
- An `AUDIT--*` atom for anything beyond a trivial fix.
