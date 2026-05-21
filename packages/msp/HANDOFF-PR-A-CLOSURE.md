# Handoff — PR-A Closure (Framework Recognizers Implementation)

> **Audience:** Antigravity agent
> **Predecessor docs:** `HANDOFF-SYMBOLS-EXPANSION-PHASE-2.md` §4 (PR-A spec)
> **Date:** 2026-05-12
> **Estimated effort:** 1-3 working hours (tests + AUDIT + PR cycle)

---

## 1. Current state — your implementation is ~80% complete locally

You (Antigravity) already wrote the implementation in a previous turn. Files sitting **untracked / uncommitted** in `main` working tree:

### Implementation (uncommitted, but typecheck-clean ✓)

```
packages/msp/src/symbols/framework/
├── index.ts              ← FrameworkRegistry orchestrator (118 lines)
├── types.ts              ← FrameworkNode + FrameworkEdge + interface
├── nextjs.ts             ← App Router (Page/Layout/Loading/Error/...)
├── routes.ts             ← Next.js + general route detection
├── runtime-tag.ts        ← 'use client' / 'use server' classification
├── data-fetching.ts      ← generateStaticParams / getServerSideProps / per-verb handlers
├── orm.ts                ← Prisma + Drizzle schema extraction
└── mcp-tools.ts          ← registerTool({...}) pattern discovery

packages/msp/src/validator/proto/
└── framework-invariants.ts  ← predicate for PROTO--SYMBOLS-FRAMEWORK-INVARIANTS

packages/msp/src/symbols/parser/index.ts (modified, +73 lines)
  └── wired frameworkRegistry into parser pipeline
```

### Verified locally

- `npm run typecheck` → PASS
- All file paths match what `ALGO--SYMBOLS-FRAMEWORK-RECOGNITION.linked_symbols` declares

---

## 2. What's missing (your remaining work)

### 2.1 Tests (mandatory — Hard PR-blocker)

Author **at minimum 6 test files** — one per recognizer:

```
packages/msp/test/symbols/framework/
├── nextjs.test.ts            ← test 7 file conventions (page/layout/loading/error/template/not-found/route)
├── routes.test.ts            ← test happy path + edge cases (catch-all, route groups)
├── runtime-tag.test.ts       ← 'use client' / 'use server' / default-server
├── data-fetching.test.ts     ← App Router + Pages Router data loaders
├── orm.test.ts               ← Prisma model fixture + Drizzle table fixture
└── mcp-tools.test.ts         ← inline registerTool + decorator + dynamic registration
```

**Each test file needs:**

- ≥ 2 cases per recognizer (happy path + 1 edge case)
- Use `mkdtempSync` + `writeFileSync` to create file fixtures (don't depend on real repo state)
- Assert exact `FrameworkNode.kind` + `FrameworkEdge.type` emitted

**Plus 1 integration test:**

```
packages/msp/test/symbols/framework/registry.test.ts
```

That exercises `FrameworkRegistry.processFile()` end-to-end on a fixture file. Verify all recognizers fire correctly + outputs merge into combined nodes/edges.

**Plus 1 PROTO predicate test:**

```
packages/msp/test/validator/proto/framework-invariants.test.ts
```

Test each of the 5 invariants in PROTO--SYMBOLS-FRAMEWORK-INVARIANTS:

- Page MUST have RENDERS_AT edge → emit violation when missing
- Route MUST have HANDLES edge → emit violation when missing
- Server Component MUST NOT carry runtime: 'client' → emit violation when conflict
- Entity MUST have orm attr → emit violation when missing
- Tool MUST have name attr → emit violation when missing

### 2.2 Wire validator predicate into loader

Check that `framework-invariants.ts` is automatically discovered by `src/validator/proto/loader.ts`. If not (likely it auto-discovers any file in `proto/` directory), no change needed — but verify by running `npm run msp:validate` and ensuring the new PROTO appears in the output.

### 2.3 AUDIT atom

Write `gks/audit/AUDIT--SYMBOLS-FRAMEWORK-AWARENESS.md` following the pattern of `AUDIT--SYMBOLS-PROCESS-TRACING.md` (most recent reference):

- Cite CONCEPT + ADR + ALGO + PROTO atoms
- List test results (X / Y tests passing)
- Document deviations from plan
- Note any follow-ups

### 2.4 PR cycle

1. Branch: `claude/msp-symbols-framework-impl`
2. `git add` only the files for this PR (see §3 below)
3. Commit with message referencing the 4 spec atoms
4. `git push` + `gh pr create`
5. CI must be green on Node 20 + 22 (modulo pre-existing #75)
6. After review → `gh pr merge --squash --delete-branch`

---

## 3. Exact files to commit in PR-A

**Include:**

```
packages/msp/src/symbols/framework/**/*.ts       (8 files — your impl)
packages/msp/src/symbols/parser/index.ts          (your modification)
packages/msp/src/validator/proto/framework-invariants.ts
packages/msp/test/symbols/framework/**/*.test.ts  (you author — 6+ files)
packages/msp/test/validator/proto/framework-invariants.test.ts  (you author)
gks/audit/AUDIT--SYMBOLS-FRAMEWORK-AWARENESS.md  (you author)
```

**Exclude (other agents' / Boss's WIP):**

```
packages/msp/msp_spec.md           ← Boss editing separately
gks/audit/AUDIT--CORE-FRAMEWORK-RECONCILE-V1.md
gks/concept/CONCEPT--CODEGEN-MICROTASK-RUNNER.md
gks/frame/FRAMEWORK--PHASE-GOVERNANCE.md
AGENT.md, GEMINI.md  ← user/agent personal config
```

---

## 4. Validator gotchas (please learn from past iterations)

1. **`created_at` timezone**: use `+07:00` offset (Thailand ICT). NEVER `Z` unless you computed UTC yourself. NEVER `+08:00` (Singapore time, wrong for Thailand). Run `npm run msp:atom-date` once it lands (PR-C — Claude is implementing in parallel) OR manually compute current TH wall-clock + add `+07:00`.

2. **Severity**: validator-enforced enums:
   - `status`: `stub | raw | draft | active | stable | deprecated | superseded | partial`
   - `tier`: `safety | master | genesis | process` (other values warn but pass)
   - `source_type`: `axiomatic | learned` (other values warn but pass)
   - `phase`: integer 0..6

3. **PR-blocker rule** (post-2026-05-12 cutoff): new FEATs/ADRs MUST have CONCEPT + ADR backing per `PROTO--SCALING-LEVEL-GATE`. Validator emits `severity: error` for missing chain on atoms ≥ cutoff date.

4. **MCP tool count tests**: `test/mcp/server.test.ts`, `test/mcp/bin.test.ts`, `test/mcp/tools/candidate.test.ts` count exactly 20 tools. **If your work adds a new MCP tool, update these 3 tests to 21**. (This is what tripped PR #79 — same will happen if you add e.g. a `framework_*` tool.)

5. **Workspace node_modules**: `packages/msp/node_modules/` is empty (npm workspace hoists). When tests need it, walk upward — see `findWorkspaceNodeModules` in `test/hooks/pre-push.test.ts` for the established pattern.

6. **Local Windows + Node 25 `better-sqlite3` failures**: pre-existing environmental issue — CI Linux is unaffected. Don't waste time chasing.

---

## 5. Acknowledgement template

When you start work, post a comment in this file or open a tracking issue:

```
Acknowledged HANDOFF-PR-A-CLOSURE (2026-05-12).
Starting on tests for framework recognizers.
Branch: claude/msp-symbols-framework-impl
ETA: ~3 hours.
```

If your local implementation doesn't actually work (you'll discover via tests), **fix forward**, don't unwind. The atoms (CONCEPT + ADR + ALGO + PROTO) are the contract — code must conform.

---

## 6. Definition of done

- [ ] All 6 recognizer tests pass (≥ 12 cases total)
- [ ] Integration test on registry composition passes
- [ ] PROTO predicate test (5 invariants) passes
- [ ] `npm run typecheck` clean
- [ ] `npm run msp:validate --all` clean (no new warnings/errors)
- [ ] AUDIT atom validates + lists test results
- [ ] PR opens against `main`, CI green on Node 20 + 22 (excl. #75)
- [ ] Squash-merge with single-paragraph commit message

---

**End of PR-A closure handoff.** You wrote good code in the previous turn — finishing tests + AUDIT closes the loop. Take your time on tests; they're the difference between "code that runs" and "code we can trust."
