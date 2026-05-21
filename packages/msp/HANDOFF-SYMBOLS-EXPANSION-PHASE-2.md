# Handoff — Symbol Graph Expansion (Phase 2) — Implementation + Tooling + Retrofit

> **Audience:** Antigravity agent (or any cognitive-layer agent) continuing the symbol-graph work.
> **Predecessor:** `HANDOFF-SYMBOLS-EXPANSION.md` (Phase 1 — closed by PR #77, atoms decomposition completed in a follow-up PR).
> **Date:** 2026-05-11
> **Estimated effort:** 4 PRs, ~6-10 working days total.

---

## 1. Mission

Continue the symbol-graph expansion work in **four follow-up PRs**, each independently mergeable:

| PR | Scope | Depends on |
|---|---|---|
| **PR-A** | Implement FRAMEWORK-AWARENESS recognizers (code per the 4 atoms) | atoms PR (in main) |
| **PR-B** | Decompose `FEAT--SYMBOLS-PROCESS-TRACING` (same pattern) + implement | PR-A optional |
| **PR-C** | Atom workflow scripts (`atom-date`, `scaffold-atom`, `supersede`) | independent |
| **PR-D** | Retrofit FEAT atoms that lack ADR backing (clear validator warnings) | independent |

---

## 2. Mandatory reading (cold start)

Read these in order before touching any code:

1. `packages/msp/HANDOFF-SYMBOLS-EXPANSION.md` — phase 1 handoff (provides full context, gotchas, conventions)
2. `packages/msp/CLAUDE.md` — repo conventions
3. **The 4 newly-authored atoms** for FRAMEWORK-AWARENESS (these are your spec):
   - `gks/concept/CONCEPT--SYMBOLS-FRAMEWORK-AWARENESS.md`
   - `gks/adr/ADR--SYMBOLS-FRAMEWORK-AWARENESS.md`
   - `gks/algo/ALGO--SYMBOLS-FRAMEWORK-RECOGNITION.md`
   - `gks/proto/PROTO--SYMBOLS-FRAMEWORK-INVARIANTS.md`
4. `gks/feat/FEAT--SYMBOLS-PROCESS-TRACING.md` (current — to be decomposed in PR-B)
5. `packages/msp/src/validator/proto/scaling-level-gate.ts` — see the grandfather clause and PROTO atom requirement enforcement

---

## 3. Hard constraints (re-read every PR)

Same as Phase 1 handoff §4. Especially:

- **Validator enums** for `status`/`tier`/`source_type` (validator-enforced)
- **`created_at` must be ≤ UTC now** (TH+7 ≠ UTC; run `date -u` first)
- **Phase 0-6 only** for atoms
- **PR-based merge**, squash, CI green Node 20 + 22
- **Atom contradiction policy**: if you change a `status: stable` atom's claim, supersede properly with reciprocal crosslinks
- **`PROTO--SCALING-LEVEL-GATE` is now hard-error** for atoms `created_at >= 2026-05-12T00:00:00Z`. New FEATs/decompositions MUST have ADR + CONCEPT backing or CI rejects.

---

## 4. PR-A: Implement FRAMEWORK-AWARENESS recognizers

**Goal:** Translate the 4 atoms (CONCEPT + ADR + ALGO + PROTO) into runnable code.

### 4.1 Atoms to author

```
gks/blueprint/BLUEPRINT--SYMBOLS-FRAMEWORK-AWARENESS.md   ← impl plan (geography + acceptance)
gks/audit/AUDIT--SYMBOLS-FRAMEWORK-AWARENESS.md           ← verification report at end
```

### 4.2 Code geography (BLUEPRINT will declare; you implement)

```
packages/msp/src/symbols/framework/
├── index.ts              ← registry; dispatches per recognizer
├── routes.ts             ← Next.js + FastAPI route detection
├── nextjs.ts             ← App Router refinements (Page/Layout/Loading/...)
├── runtime-tag.ts        ← 'use client' / 'use server' classification
├── data-fetching.ts      ← generateStaticParams / getServerSideProps / etc.
├── orm.ts                ← Prisma + Drizzle extraction
└── mcp-tools.ts          ← registerTool() discovery

packages/msp/src/validator/proto/
└── framework-invariants.ts  ← validator predicate for PROTO--SYMBOLS-FRAMEWORK-INVARIANTS

packages/msp/test/symbols/framework/
└── (one *.test.ts per recognizer with happy + edge case fixtures)
```

### 4.3 Microtasks (suggested split)

Author task YAMLs under `.brain/<ns>/tasks/SYMBOLS-FRAMEWORK/`:

| Task | Concern | Output |
|---|---|---|
| `T1_routes.task.yaml` | Next.js + FastAPI route detection | `routes.ts` |
| `T2_nextjs-refinements.task.yaml` | App Router node kinds (Page/Layout/Loading/Error/Template/Middleware) per FEAT §1b | `nextjs.ts` |
| `T3_runtime-tag.task.yaml` | `'use client'` / `'use server'` directive detection | `runtime-tag.ts` |
| `T4_data-fetching.task.yaml` | `generateStaticParams`, `getServerSideProps`, per-verb route handlers | `data-fetching.ts` |
| `T5_orm.task.yaml` | Prisma schema parsing + Drizzle schema files | `orm.ts` |
| `T6_mcp-tools.task.yaml` | MCP `registerTool({...})` pattern discovery | `mcp-tools.ts` |
| `T7_registry.task.yaml` | Wire all 6 into parser pipeline; emit FRAMEWORK_NODE + FRAMEWORK_EDGE types | `index.ts`, `types.ts` (update) |
| `T8_proto-validator.task.yaml` | Implement PROTO--SYMBOLS-FRAMEWORK-INVARIANTS predicate in code | `framework-invariants.ts` |
| `T9_tests.task.yaml` | Per-recognizer tests (≥ 2 cases each) + integration test on a fixture Next.js project | `test/symbols/framework/*.test.ts` |

### 4.4 Acceptance criteria

- All recognizers detect ≥ 95% of real targets in a Next.js fixture you generate
- Zero false positives on `src/symbols/types.ts` (non-framework TS file fixture)
- PROTO invariants enforced by `framework-invariants.ts`; running `npm run msp:validate` on a fixture with a Page-without-RENDERS_AT fails with `severity: error`
- Server/Client runtime tag surfaces in `msp_symbol_lookup` result via `attrs.runtime`
- 1 new MCP tool **NOT required** for this PR (PR-B's `symbol_trace` consumes this data)

### 4.5 Done

- Branch: `claude/msp-symbols-framework-impl`
- BLUEPRINT + AUDIT atoms validate clean
- All 9 microtasks pass their acceptance test
- CI green Node 20 + 22
- PR opens as draft → ready → squash-merge

---

## 5. PR-B: Decompose + implement PROCESS-TRACING (mirror PR-A pattern)

**Goal:** Apply the same decomposition pattern that was used for FRAMEWORK-AWARENESS to `FEAT--SYMBOLS-PROCESS-TRACING`, then implement.

### 5.1 Decomposition (mirror current PR's pattern)

Atoms to author:

```
gks/concept/CONCEPT--SYMBOLS-PROCESS-TRACING.md
gks/adr/ADR--SYMBOLS-PROCESS-TRACING.md
gks/algo/ALGO--SYMBOLS-CALL-GRAPH-TRAVERSAL.md      ← the tracing algorithm itself
gks/proto/PROTO--SYMBOLS-TRACE-INVARIANTS.md        ← e.g. "trace MUST terminate within depth limit"
```

Supersede:

```
FEAT--SYMBOLS-PROCESS-TRACING → status: superseded
                                superseded_by: [4 atoms above]
```

### 5.2 Implementation (after decomposition atoms)

```
packages/msp/src/symbols/tracer/
├── call-graph.ts          ← cross-file CALL edge resolution (IMPORTS already exists)
├── tracer.ts              ← entry-point → leaf walker with depth limit + cycle guard
└── types.ts               ← TracePath, TraceNode types

packages/msp/src/mcp/tools/
└── symbol-trace.ts        ← new MCP tool (the 6th in symbols suite)

packages/msp/src/validator/proto/
└── trace-invariants.ts    ← predicate for PROTO--SYMBOLS-TRACE-INVARIANTS

packages/msp/test/symbols/tracer/
└── *.test.ts
```

### 5.3 Acceptance criteria

- `symbol_trace` MCP tool returns ≥ 1 path for any entry-point detected by PR-A's recognizers (Next.js route → handler → leaf fn)
- Cycle detection prevents infinite loops (recursive-function fixture)
- Depth limit configurable; default = 8 hops (state in BLUEPRINT)
- New PROTO invariant `trace-invariants` enforces termination + acyclic guarantees at write time

### 5.4 Done

- Branch: `claude/msp-symbols-process-tracing`
- All atoms validate clean
- 6 MCP tools under symbols category total (existing 5 + new `symbol_trace`)
- CI green Node 20 + 22

---

## 6. PR-C: Atom workflow scripts (developer ergonomics)

**Goal:** Eliminate repeated manual mistakes seen in Phase 1 (date format, reciprocal supersession, frontmatter shape).

### 6.1 Scripts to add

Add to `packages/msp/package.json#scripts`:

```json
{
  "msp:atom-date": "tsx scripts/msp/atom-date.ts",
  "msp:scaffold-atom": "tsx scripts/msp/scaffold-atom.ts",
  "msp:supersede": "tsx scripts/msp/supersede.ts"
}
```

### 6.2 `scripts/msp/atom-date.ts`

Output a single line: UTC ISO timestamp suitable for `created_at`.

```ts
// Usage: npm run msp:atom-date
// Output: 2026-05-11T22:30:00.000Z
console.log(new Date().toISOString())
```

5 lines. Trivial. Eliminates the TH+7 future-date bug (seen 5× in this session).

### 6.3 `scripts/msp/scaffold-atom.ts`

```
Usage: npm run msp:scaffold-atom -- --type=<type> --slug=<SLUG> [--title=<title>] [--out=<path>]

Behavior:
  1. Validates --type is in known atom types (concept, adr, feat, frame, proto, algo, ...)
  2. Generates frontmatter with:
     - id: <TYPE>--<SLUG>  (uppercase, slug normalized)
     - phase: derived from type (concept=1, adr=2, feat=2, frame=2, proto=2, algo=2, blueprint=3, audit=6)
     - type: <type-lowercase>
     - status: draft  (canonical)
     - tier: process  (canonical default; user can edit later)
     - source_type: axiomatic  (canonical default)
     - vault_id: default
     - title: --title or fallback to SLUG humanized
     - created_at: UTC now (uses atom-date)
     - crosslinks: {}  (placeholder)
  3. Body skeleton with sections per type:
     - CONCEPT: ## Problem / ## Hypothesis / ## Scope / ## Out of scope / ## Verification
     - ADR: ## Context / ## Decision / ## Consequences / ## Alternatives / ## Source
     - FEAT: ## User-facing behaviour / ## Verification / ## Out of scope / ## Source
     - PROTO: ## Rule / ## Severity / ## Enforcement / ## Counter-example / ## Source
     - ALGO: ## Inputs / ## Algorithm / ## Complexity / ## Edge cases / ## Source
     - BLUEPRINT: ## Geography / ## Acceptance / ## Dependencies / ## Tasks / ## Source
     - AUDIT: ## Scope verified / ## Test results / ## Deviations / ## Follow-ups / ## Source
  4. Writes file at gks/<type>/<TYPE>--<SLUG>.md (or --out if specified)
  5. Echoes the path for follow-up edit
```

Constraints to enforce:

- Reject if file already exists (no overwrite without --force)
- Reject if `--slug` doesn't match `^[A-Z][A-Z0-9_-]*$`
- Print "next steps" hint at end (e.g. "Edit body sections, then run npm run msp:validate <path>")

### 6.4 `scripts/msp/supersede.ts`

```
Usage: npm run msp:supersede -- --old=<OLD-ID> --new=<NEW-ID-1>[,<NEW-ID-2>,...]
                                [--reason=<text>]

Behavior:
  1. Locate <OLD-ID> in atomic_index — fail if not found
  2. Locate each <NEW-ID> — fail if any not found
  3. Atomically update both sides:
     a. In <OLD-ID>'s file:
        - status: → 'superseded'
        - crosslinks.superseded_by: append each <NEW-ID> (dedupe)
     b. In each <NEW-ID>'s file:
        - crosslinks.supersedes: append <OLD-ID> (dedupe)
  4. Optionally append a "## Supersession note" block to <OLD-ID> body with --reason
  5. Run msp:index + msp:check-links to confirm graph consistent
  6. Echo summary
```

Constraints:

- Refuse if <OLD-ID> already has `status: superseded` (idempotency safety)
- Refuse if any <NEW-ID> file lacks `crosslinks` block (must exist for safe append)
- Always re-write atoms via YAML serializer — never regex-replace frontmatter (avoid eating newlines / quotes)

### 6.5 Acceptance criteria

- `msp:atom-date` outputs current UTC, no other lines
- `msp:scaffold-atom -- --type=concept --slug=TEST` creates a valid `gks/concept/CONCEPT--TEST.md` that **passes** `msp:validate` immediately (zero hard errors)
- `msp:supersede -- --old=CONCEPT--TEST --new=CONCEPT--TEST-NEW` updates both files and graph re-validates clean
- ≥ 3 unit tests per script covering happy path + 2 edge cases

### 6.6 Done

- Branch: `claude/msp-atom-workflow-scripts`
- 3 scripts under `scripts/msp/`
- Tests under `packages/msp/test/scripts/`
- README section in `packages/msp/README.md` documenting the scripts
- CI green

---

## 7. PR-D: Retrofit existing FEATs that lack ADR backing

**Goal:** Clear the validator-warning debt before the grandfather clause expires (target: 2026-08-01 = 90-day window).

### 7.1 Discover

```bash
# After PR-C lands you can use --strict-future flag (see §8); for now:
npx tsx packages/msp/src/validator/cli.ts --all --root=packages/msp 2>&1 | grep "missing linked ADR"
```

Expected output: list of FEAT atoms missing ADR. Count them; expect 10-25 atoms.

### 7.2 Per-atom decision (case by case)

For each FEAT without ADR:

| Decision | Action |
|---|---|
| **(a)** FEAT is L1 (Quick Task) | Add `level_override: L1` to frontmatter — exempt from rule |
| **(b)** FEAT is L2+ but never had ADR | Author retroactive `ADR--<FEAT-SLUG>` documenting the decision (even if post-hoc); link both ways |
| **(c)** FEAT is obsolete | Set `status: superseded` or `deprecated` — exempt from rule |

Boss reviews each decision before merge. Author a **single PR per cluster** of related FEATs (group by milestone or area) — not one PR per FEAT.

### 7.3 After all retrofit cleared

Remove the grandfather clause from `src/validator/proto/scaling-level-gate.ts` — promote the FEAT→ADR rule to hard error for **all** FEAT atoms regardless of `created_at`.

### 7.4 Acceptance

- Zero `[warning] (FEAT--*) ... missing linked ADR` lines in `msp:validate --all` output
- Grandfather clause removed
- New ADR atoms for ≥ 90% of retrofitted FEATs (others marked exempt via level_override)

### 7.5 Done

- Branch: `claude/msp-retrofit-feat-adr-backing`
- Tracking issue (e.g. #76+) updated with progress per FEAT
- CI green

---

## 8. Cross-cutting suggestions (nice-to-have, not gating)

### 8.1 `--strict-future` flag for validator

Promote ALL warnings to errors for atoms `created_at >= today`, regardless of rule. Use case: CI gate for new atoms only.

Implementation hint: add `--strict-future` flag to `src/validator/cli.ts` that overrides predicate severity per rule.

### 8.2 ADR-required policy as a PROTO atom

Currently the rule lives in `scaling-level-gate.ts`. Consider authoring:

```
PROTO--FEAT-REQUIRES-ADR  (new)
```

That references `FRAMEWORK--SCALING-LEVELS` and points `linked_symbols` at `scaling-level-gate.ts`. Makes the rule discoverable via atom search.

### 8.3 Pre-commit hook upgrade

When PR-C lands, update `.husky/pre-commit` (or whatever hook orchestrator) to also call `msp:check-links` (not just `msp:validate`) — catches broken crosslinks before push.

### 8.4 Atom dependency graph visualization

Not in scope of any PR above, but if you have spare cycles: a `npm run msp:graph-atoms` script that emits a D3/Cytoscape-compatible JSON of the entire atom graph (FEAT→ADR→CONCEPT chains). Helps Boss see retrofit gaps visually.

---

## 9. Process notes (avoid repeating Phase 1 mistakes)

1. **Run `date -u` before authoring atoms.** Or use `npm run msp:atom-date` after PR-C lands.
2. **After editing atoms, ALWAYS run:**

   ```
   npm run msp:index && npm run msp:validate --workspace=packages/msp -- --all --root=packages/msp && npm run msp:check-links
   ```

   In one line. Catches drift early.
3. **Per-PR sequence** is exactly the §6 checklist from `HANDOFF-SYMBOLS-EXPANSION.md` (branch → atoms → microtasks → impl → tests → AUDIT → push → PR draft → CI green → ready → squash-merge).
4. **Hand-off acknowledgement**: when you start any of PR-A through PR-D, post a comment in the corresponding tracking issue (open if not present) stating which PR you're starting and ETA.

---

## 10. Order of execution

| Priority | PR | Depends on |
|---|---|---|
| 1 (next up) | PR-A — implement FRAMEWORK recognizers | atoms in main |
| 2 (parallel ok) | PR-C — workflow scripts | none |
| 3 | PR-B — decompose + implement PROCESS-TRACING | PR-A optional |
| 4 (cleanup) | PR-D — retrofit FEAT→ADR | PR-A+B done so example exists |

PR-C can be done first if you prefer scripts before more atom authoring — both paths land the same outcome eventually.

---

## 11. Definition of "done" for Phase 2 handoff

- [ ] PR-A merged: 9 microtasks pass; recognizers detect Next.js + Prisma + MCP discovery; PROTO predicate enforces invariants
- [ ] PR-B merged: PROCESS-TRACING atoms decomposed; `symbol_trace` MCP tool live
- [ ] PR-C merged: 3 scripts (`atom-date`, `scaffold-atom`, `supersede`) live with tests + README
- [ ] PR-D merged: all FEATs have ADR backing or `level_override: L1` exemption; grandfather clause removed
- [ ] CI green on all 4 PRs on Node 20 + 22 (modulo pre-existing #75)

---

## 12. Hand-off acknowledgement template

When starting any PR, post a comment in this file or open a tracking issue:

```
Acknowledged HANDOFF-SYMBOLS-EXPANSION-PHASE-2 (date 2026-05-11).
Starting on PR-<X> (<scope>).
Branch: claude/msp-<scope-slug>
ETA: ~<N> days
```

If any constraint above contradicts actual repo state, **stop and ask Boss** before proceeding.

---

**End of Phase 2 handoff.** Phase 1 closed PR #77 successfully — the foundation is solid. The 4 PRs above complete the symbol-graph vision and pay down the technical debt that Phase 1 surfaced.
