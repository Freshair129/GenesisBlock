# Handoff — Symbol Graph Expansion (Multi-Lang + Framework Awareness + Process Tracing)

> **Audience:** Antigravity agent (or any cognitive-layer agent) picking up the symbol-graph expansion work.
> **Status:** P2 FEAT atoms complete (✓ validated); P3 BLUEPRINTs onward = your turn.
> **Date:** 2026-05-11
> **Estimated effort:** 3 PRs, ~4-6 working days for an experienced agent.

---

## 1. Mission

Expand MSP's existing `src/symbols/` subsystem from "TypeScript-only static analysis" to **multi-language framework-aware code intelligence with process tracing**, all surfaced via MCP tools — **without** creating a new package.

This work consolidates what an earlier `packages/codex` proposal would have built into the MSP that already exists. See "Why not a new package" below.

---

## 2. Mandatory reading (cold start)

Read these in order before touching any code:

1. **`packages/msp/CLAUDE.md`** — repo conventions, doc-to-code workflow, contradiction policy, worktree gotchas
2. **`docs/gks/KNOWLEDGE-TYPES.md`** — canonical atom type taxonomy (~30 types)
3. **`packages/gks/SCOPE.md`** — what GKS is/isn't; key: **code intelligence is OUT of GKS scope** by design
4. **`docs/gks/MSP_RELATIONSHIP.md`** — GKS/MSP separation, GitNexus pattern
5. **`FRAMEWORK_MASTER_SPEC.md`** (repo root) §3.2.1 (Smart Proxy Pattern) + §4.6 (Microtasks) + §8 (Codegen)
6. **`gks/adr/ADR--MSP-INTERFACE-LAYER.md`** — interfaces/orchestrator/clients/domain split

**Existing infra to extend (not rebuild):**

```
packages/msp/src/symbols/
├── api.ts                    ← public API surface (extend)
├── cli.ts                    ← `npm run msp:graph` (extend)
├── types.ts                  ← shared types (extend)
├── util.ts
├── parser/typescript.ts      ← existing TS parser via TS Compiler API
├── store/jsonl.ts            ← JSONL backend
├── store/sqlite.ts           ← SQLite backend
└── communities/leiden.ts     ← Leiden community detection (already done!)

packages/msp/src/mcp/tools/
├── symbol-search.ts          ← existing — keyword search
├── symbol-lookup.ts          ← existing — by id
├── symbol-neighbors.ts       ← existing — 1-hop graph
├── symbol-impact.ts          ← existing — blast radius
└── symbol-community.ts       ← existing — cluster view
```

**Atoms already in place (P2 done):**

```
gks/feat/
├── FEAT--SYMBOLS-MULTI-LANG.md           ← Python + COBOL parsers
├── FEAT--SYMBOLS-FRAMEWORK-AWARENESS.md  ← Routes + ORM + MCP discovery
└── FEAT--SYMBOLS-PROCESS-TRACING.md      ← entry-point → leaf tracing

gks/blueprint/BLUEPRINT--SYMBOL-GRAPH-CORE.md   ← prior blueprint (reference)
gks/frame/FRAMEWORK--SYMBOL-GRAPH.md
gks/concept/CONCEPT--SYMBOL-GRAPH.md
gks/concept/CONCEPT--PARSER-CHOICE.md
gks/adr/ADR--SYMBOL-GRAPH-PERSISTENCE.md
gks/adr/ADR--LEIDEN-COMMUNITY-DETECTION.md
```

---

## 3. Why not a new package (decision log)

A separate `packages/codex/` was proposed but rejected because:

1. **Name collision:** "Codex" clashes with OpenAI Codex (their main coding-agent product as of 2025) → confusing branding
2. **Reinvention:** MSP already ships parser + JSONL/SQLite stores + Leiden + 5 MCP tools for the same use case
3. **Scope split is unnecessary:** code intelligence is an orchestrator concern (per `ADR-009 msp-as-orchestrator` in `docs/gks/adr/`), and MSP is the orchestrator
4. **Microtask reuse:** MSP already has `msp:run-task` for parallel codegen pipelines — the proposed 12-stage DAG maps cleanly to microtasks, no new orchestrator needed
5. **GKS rejected the work explicitly:** `packages/gks/SCOPE.md` says "Code intelligence → use GitNexus or similar"; GKS won't accept this code

Net: **extend `packages/msp/src/symbols/`**, don't create a new package.

---

## 4. Hard constraints (validator + workflow)

Every atom/PR must satisfy these or CI will reject:

### 4.1 Frontmatter (`packages/msp/src/validator/rules/`)

| Field | Allowed values | Source of truth |
|---|---|---|
| `status` | `stub` \| `raw` \| `draft` \| `active` \| `stable` \| `deprecated` \| `superseded` \| `partial` | `src/validator/rules/phase-status.ts` |
| `phase` | integer `0..6` only | same file |
| `created_at` | ISO 8601, **must be ≤ UTC now** (not local TH time) | check `date -u` first |
| `tier` | `safety` \| `master` \| `genesis` \| `process` (strict) or `architecture` (warning, widely used) | `tier-enum` rule |
| `source_type` | `axiomatic` \| `learned` (strict) or `documented_source` (warning, widely used) | `tier-enum` rule |
| `id` | `^[A-Z][A-Z0-9_]*--[A-Z0-9][A-Z0-9_-]*$` | `id-format.ts` |
| `crosslinks.references` | all wikilinks must resolve to atoms in `atomic_index.jsonl` | `wikilink-resolve` |

### 4.2 Workflow gates (per `packages/msp/CLAUDE.md`)

```
P2 FEAT (✓ done)  →  P3 BLUEPRINT  →  P4 microtasks (T*.task.yaml)  →  P5 src/  →  P6 AUDIT
                     │
                     ├─ frontmatter validates
                     ├─ all wikilinks resolve
                     ├─ pre-commit hook passes
                     └─ pre-push hook runs verify-flow
```

**Promotion:** there is no CLI promote step. Atoms enter `gks/<type>/` **only via PR merge** (per `ADR--AGENT-WRITE-BOUNDARIES`). To write a candidate, the only path is the `msp_candidate` MCP tool which writes to `.brain/msp/projects/<ns>/candidates/`. For this handoff you author atoms directly into `gks/<type>/` because you're working from a branch that will be PR-reviewed by Boss.

### 4.3 Atom contradiction policy

If your new atom contradicts an existing `status: stable` atom of the same type:

1. Add the old atom's id to `crosslinks.supersedes` of your new atom
2. Add your new atom's id to `crosslinks.superseded_by` of the old atom
3. Flip the old atom's `status` to `superseded`

All three in the **same PR**.

### 4.4 Code constraints (per `FRAMEWORK_MASTER_SPEC §9.1`)

| File type | Hard limit | Soft |
|---|---|---|
| UI component | 500 LOC | 300 |
| Utility / parser module | 150 | 100 |
| Repository / data access | always go via `src/symbols/store/*` — never bypass |

### 4.5 Branch + PR conventions

- Branch: `claude/msp-symbols-<area>-<slug>` (e.g. `claude/msp-symbols-multi-lang-python`)
- One milestone = one branch usually
- PR open as draft; mark ready when CI green
- Squash-merge with 1-paragraph commit message
- CI must be green on **Node 20 AND Node 22**

### 4.6 Workspace gotcha (will trip you)

- `packages/msp/node_modules/` is **empty** in npm workspaces; populated tree is at the monorepo root
- When code/tests need a populated `node_modules`, walk upward (see `findWorkspaceNodeModules` in `src/codegen/acceptance/sandbox.ts` and `test/hooks/pre-push.test.ts`)

---

## 5. Implementation plan (3 PRs, sequential)

### PR 1: `FEAT--SYMBOLS-MULTI-LANG` (foundation)

**Why first:** the other two FEATs depend on parser output being multi-language.

**Atoms to author:**

```
gks/blueprint/BLUEPRINT--SYMBOLS-MULTI-LANG.md
gks/adr/ADR--SYMBOLS-PYTHON-PARSER.md         # tree-sitter-python vs ast vs lib choice
gks/adr/ADR--SYMBOLS-COBOL-STRATEGY.md        # regex-first, tree-sitter deferred
```

**Microtasks (`.brain/msp/projects/<ns>/tasks/SYMBOLS-MULTI-LANG/`):**

| Task | Concern | Geography |
|---|---|---|
| `T1_parser-python.task.yaml` | Tree-sitter Python parser emitting same shape as `parser/typescript.ts` | `src/symbols/parser/python.ts` |
| `T2_parser-cobol.task.yaml` | Regex-based COBOL parser (PROGRAM-ID, SECTION, PERFORM only) | `src/symbols/parser/cobol.ts` |
| `T3_parser-registry.task.yaml` | Generalize parser API so `api.ts` dispatches by file extension | `src/symbols/parser/index.ts`, `src/symbols/types.ts` |
| `T4_tests-python.task.yaml` | Fixtures + 6 happy/edge cases | `test/symbols/parser/python.test.ts` |
| `T5_tests-cobol.task.yaml` | Fixtures + 4 cases (simple program, multi-section, PERFORM chain, dead code) | `test/symbols/parser/cobol.test.ts` |
| `T6_cli-flags.task.yaml` | Add `--lang=python|cobol|ts` flag to `cli.ts` | `src/symbols/cli.ts` |

**Acceptance criteria:**

- All 6 microtasks pass their per-task acceptance test
- `npm test --workspace=packages/msp` green
- `npm run msp:graph -- packages/msp/test/fixtures/sample.py --emit=jsonl` outputs valid JSONL with at least 1 symbol per top-level def
- New parsers emit nodes/edges with the same shape as `parser/typescript.ts` (see `src/symbols/types.ts` for `SymbolNode` / `SymbolEdge` interfaces)

**AUDIT atom to write at end:** `gks/audit/AUDIT--SYMBOLS-MULTI-LANG.md` — pass/fail, coverage numbers, links to fixtures, follow-ups

### PR 2: `FEAT--SYMBOLS-FRAMEWORK-AWARENESS`

**Depends on PR 1 (parser registry).**

**Atoms to author:**

```
gks/blueprint/BLUEPRINT--SYMBOLS-FRAMEWORK-AWARENESS.md
gks/adr/ADR--FRAMEWORK-RECOGNIZER-PATTERN.md   # how to detect Next.js / FastAPI / etc. without false positives
```

**Microtasks:**

| Task | Concern | Geography |
|---|---|---|
| `T1_framework-routes.task.yaml` | Recognize `app/**/page.tsx`, `app/**/route.ts` (Next.js), `pages/**/*.ts` (legacy), FastAPI `@app.get` | `src/symbols/framework/routes.ts` |
| `T2_framework-orm.task.yaml` | Parse Prisma `schema.prisma` and Drizzle schema files; emit model nodes + relation edges | `src/symbols/framework/orm.ts` |
| `T3_framework-mcp-tools.task.yaml` | Discover MCP tool registrations (look for `registerTool({...})` patterns in TS) | `src/symbols/framework/mcp-tools.ts` |
| `T4_framework-registry.task.yaml` | Plug recognizers into main parser pipeline; emit `FRAMEWORK_NODE` and `FRAMEWORK_EDGE` types | `src/symbols/framework/index.ts`, `src/symbols/types.ts` |
| `T5_tests.task.yaml` | Fixtures for each framework + edge cases | `test/symbols/framework/*.test.ts` |

**Acceptance criteria:**

- Recognizers detect ≥ 95% of routes in a real Next.js App Router project (use `web/` if it has any, otherwise generate a fixture)
- Prisma schema parsing matches `schema.prisma` field types
- Zero false positives on plain TS files that don't use frameworks (test with `src/symbols/types.ts` as a non-framework fixture)

### PR 3: `FEAT--SYMBOLS-PROCESS-TRACING`

**Depends on PR 1 (parser registry) and PR 2 (entry-point detection).**

**Atoms to author:**

```
gks/blueprint/BLUEPRINT--SYMBOLS-PROCESS-TRACING.md
gks/adr/ADR--PROCESS-TRACE-DEPTH-LIMIT.md      # how deep to follow; cycle detection rules
```

**Microtasks:**

| Task | Concern | Geography |
|---|---|---|
| `T1_call-graph-builder.task.yaml` | Resolve cross-file call edges (`IMPORTS` already exists; add `CALLS`) | `src/symbols/tracer/call-graph.ts` |
| `T2_tracer-core.task.yaml` | Walk from entry-point node to leaf nodes; capture path; depth-limit + cycle guard | `src/symbols/tracer.ts` (top-level) |
| `T3_tracer-mcp-tool.task.yaml` | New `symbol_trace` MCP tool — input: entry-point id; output: list of trace paths | `src/mcp/tools/symbol-trace.ts` |
| `T4_tracer-tests.task.yaml` | Integration test using fixtures from PR 2 (Next.js route → handler → ORM call) | `test/symbols/tracer.test.ts` |

**Acceptance criteria:**

- `symbol_trace` returns at least 1 trace for any framework entry-point detected in PR 2
- Cycle detection prevents infinite loops on recursive functions (add a recursive fixture)
- Depth limit configurable; default = 8 hops (document in BLUEPRINT)
- New MCP tool registered in `src/mcp/server.ts` and validated by `test/mcp/server.test.ts`

---

## 6. Per-PR mechanical checklist

For each of the 3 PRs above:

```
[ ] Branch off main: claude/msp-symbols-<feat-slug>
[ ] Author atoms (BLUEPRINT + any new ADRs) — frontmatter validates
[ ] npm run msp:index → atom shows in atomic_index.jsonl
[ ] npm run msp:check-links → all crosslinks resolve
[ ] npm run msp:validate -- --all --root=packages/msp → 0 hard errors
[ ] Author microtask YAMLs in .brain/<ns>/tasks/<feat-slug>/
[ ] Implement src/ code matching BLUEPRINT.geography exactly (1:1)
[ ] Tests pass: npm test --workspace=packages/msp
[ ] Build clean: npm run build
[ ] Typecheck clean: npm run typecheck
[ ] Author AUDIT--<FEAT>.md with pass/fail + numbers
[ ] Open draft PR; mark ready when CI green on Node 20 + 22
[ ] Squash-merge with 1-paragraph summary commit message
```

---

## 7. Risks + mitigations

| Risk | Mitigation |
|---|---|
| Tree-sitter Python build fails on some platforms | Use `web-tree-sitter` (WASM) variant, not native bindings; tested in `parser/typescript.ts` already shows path |
| COBOL regex matches false positives in non-COBOL files | Restrict by file extension (`.cob`, `.cbl`, `.cpy`) at the parser registry level; never attempt COBOL parse on `.ts` |
| Process tracing explodes on deeply recursive codebases | Hard depth limit + visited-set cycle guard (cover in `ADR--PROCESS-TRACE-DEPTH-LIMIT`) |
| Framework recognizer false-positives on tutorial code or unrelated files | Require multiple signals before classifying (e.g. file path pattern AND import pattern AND export pattern) |
| Validator warnings about `tier: architecture` or `source_type: documented_source` | Acceptable — many existing ADRs use these. Don't switch to `process`/`learned` just to silence; match the convention of nearby atoms |
| `created_at` in the future (TH+7 vs UTC) | **Always** run `date -u` first; use the UTC timestamp |
| CI red on `test/hooks/pre-push.test.ts > OK chain` | Pre-existing; tracked in issue #75; ignore for this work |

---

## 8. Where NOT to touch

- `packages/gks/**` — per `SCOPE.md`, code intelligence is out of GKS scope. Do not put any of this code in `packages/gks/`.
- `gks/00_index/atomic_index.jsonl` — auto-generated; never edit by hand. Run `npm run msp:index`.
- `node_modules/` — workspace-hoisted; trust the symlinks
- `dist/` — build output; gitignored
- `.brain/msp/projects/<ns>/candidates/` — per-user staging; don't commit candidates to the repo

---

## 9. Open questions to confirm with Boss before starting

These were raised in the original "Codex" proposal and partially resolved here. **Re-confirm answers before PR 1 lands.**

| # | Question | Current answer | Confidence |
|---|---|---|---|
| Q1 | Storage backend for code graph? | Reuse existing `src/symbols/store/{jsonl,sqlite}.ts`; don't touch GKS graph | High |
| Q2 | COBOL parser approach? | Regex first; defer tree-sitter to a future ADR if needed | Medium — confirm scope of COBOL needs |
| Q3 | Community detection algorithm? | **Leiden** (already implemented in `communities/leiden.ts`) | High |
| Q4 | Trace depth default? | Suggested 8 hops, configurable | Low — confirm with Boss |
| Q5 | Output format for `symbol_trace` MCP tool? | Suggest JSON: `{ entry, paths: [{ nodes: [id], edges: [type] }] }` | Low — propose in BLUEPRINT, get approval |

---

## 10. Useful commands (copy-paste reference)

```bash
# Atom integrity
npm run msp:index                                                # regen atomic_index.jsonl
npm run msp:validate --workspace=packages/msp -- --all --root=packages/msp
npm run msp:check-links --workspace=packages/msp
npm run msp:backlinks --workspace=packages/msp

# Codegen pipeline (existing — extend, don't reinvent)
npm run msp:run-task --workspace=packages/msp -- <feat-id> <task-id>

# Build / test / typecheck
npm run build                       # both packages
npm run typecheck                   # both packages
npm test --workspace=packages/msp   # MSP tests only
npm test --workspace=packages/gks   # GKS tests only

# Symbol graph (existing CLI you're extending)
npm run msp:graph --workspace=packages/msp -- <file-or-dir>

# Date for created_at (UTC, not local TH time!)
date -u +"%Y-%m-%dT%H:%M:%S.000Z"

# Time-of-handoff snapshot
# - main HEAD: 87e60ab (docs+code: drop "v3" from GKS name)
# - main contains: PR #73 (docs sweep + ADR--MSP-INTERFACE-LAYER), PR #76 (v3 rename)
# - 3 FEAT atoms in place (this handoff document follows from them)
```

---

## 11. Definition of "done" for the whole handoff

The handoff is complete when:

- [ ] 3 PRs merged to `main` (PR 1 → 2 → 3 sequentially)
- [ ] All 3 FEAT atoms have `status: stable` (currently `active`)
- [ ] 3 BLUEPRINT atoms exist (1 per FEAT), `status: stable`
- [ ] 3 AUDIT atoms exist (1 per FEAT), `status: stable`
- [ ] At least 1 new ADR per FEAT (decision records for parser/framework/trace choices)
- [ ] `src/symbols/parser/` covers `.ts`, `.py`, `.cob`
- [ ] `src/symbols/framework/` recognizes Next.js + Prisma + MCP tool registrations
- [ ] `src/symbols/tracer.ts` + `mcp/tools/symbol-trace.ts` live and exercised by ≥ 1 integration test
- [ ] Total new MCP tool count: existing 5 + new `symbol_trace` = 6 under symbols category
- [ ] CI green on Node 20 + 22 across all 3 PRs (modulo pre-existing #75)
- [ ] `gks verify-flow FEAT--SYMBOLS-MULTI-LANG` (and the other two) returns clean

---

## 12. Hand-off acknowledgement

When you (the agent) start work, post a comment in this file or open an issue tagged `handoff-ack`:

```
Acknowledged HANDOFF-SYMBOLS-EXPANSION (date 2026-05-11).
Starting on PR 1 (FEAT--SYMBOLS-MULTI-LANG).
Branch: claude/msp-symbols-multi-lang
ETA: ~2 days for atoms + tests + impl.
```

If any constraint above is unclear or appears to contradict actual repo state, **stop and ask Boss** before proceeding. Don't guess.

---

**End of handoff.** Good luck. The infrastructure to do this well already exists — your job is to extend it carefully, not to re-architect it.
