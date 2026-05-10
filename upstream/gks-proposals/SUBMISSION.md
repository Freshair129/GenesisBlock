# Submission packet — for relay to `Freshair129/GksV3`

> **For the human relayer**: MSP can't push to `Freshair129/GksV3` directly. This file packages the 4 proposals under `upstream/gks-proposals/` into copy-paste-ready GitHub issue bodies. Pick one of the strategies below and ship.

## TL;DR — 3 ways to submit

| Strategy | Effort | Recommended when |
|---|---|---|
| **A. One umbrella issue** linking the 4 proposal files | ~2 min | Maintainer prefers context-first; small repo |
| **B. Four separate issues** (one per proposal) | ~8 min | Maintainer triages by topic; each can land independently |
| **C. Four draft PRs** (one per proposal, with the diff applied) | ~30 min + write access | You / maintainer collaborate directly on code |

**Recommended**: **B** — separate issues. Lets each land at its own pace; preserves single-concern hygiene that GKS's `SCOPE.md` itself models.

---

## Strategy A — Umbrella issue (copy-paste this whole block)

**Title**:
```
MSP-driven proposals: 4 small upstream improvements (Phase-6 / verify-flow / backlinks API / SC parity docs)
```

**Body**:
```markdown
Hi @<maintainer> — opening this as a single issue to collect 4 small, focused proposals discovered while building [MSP](https://github.com/Freshair129/msp) (a passport-orchestrator built on top of GksV3).

Each is independently mergeable. Happy to split into 4 issues if you'd prefer.

| # | Topic | One-line summary | Drafted PR-ready in MSP repo |
|---|---|---|---|
| 1 | `phase: 6` in `propose-inbound` | Relax phase range from 0..5 → 0..6 so post-implementation AUDIT atoms can be proposed via the standard CLI | [01-phase-6-acceptance.md](https://github.com/Freshair129/msp/blob/main/upstream/gks-proposals/01-phase-6-acceptance.md) |
| 2 | `verify-flow --through-superseded` | New flag (default off) that follows `crosslinks.superseded_by` when walking; preserves current strict default | [02-verify-flow-through-superseded.md](https://github.com/Freshair129/msp/blob/main/upstream/gks-proposals/02-verify-flow-through-superseded.md) |
| 3 | `gks backlinks` API | New CLI + TS API (`deriveBacklinks(store)`) that emits the flat edge list MSP currently re-derives in `src/memory/backlinks/` | [03-backlinks-api.md](https://github.com/Freshair129/msp/blob/main/upstream/gks-proposals/03-backlinks-api.md) |
| 4 | Smart Connections + nomic compatibility doc | Doc-only — one-pager under `docs/embedder-compatibility.md` explaining why browse-side plugins should match `createNomicEmbedder()`'s model | [04-smart-connections-parity.md](https://github.com/Freshair129/msp/blob/main/upstream/gks-proposals/04-smart-connections-parity.md) |

### Why these came up

- **#1**: GksV3 3.6.0's CLI caps `--phase` at 5; AUDIT atoms (post-implementation observability) live at phase 6 in the MSP spec. We work around it via `scripts/msp/propose.mjs` patching the file after propose, but that bypasses GKS validation.
- **#2**: Hit during MSP PR #8 — supersede chain caused `verify-flow` to halt mid-walk even though `superseded_by` was present. Repointed 9 atoms as workaround; correct fix is upstream.
- **#3**: `MSP_RELATIONSHIP.md` says backlinks derivation is the Memory OS implementer's job — but it's near-universal. ~200 LoC of duplicated logic across every Memory OS that uses GKS.
- **#4**: 3.6.0 introduced `createNomicEmbedder()` (great default!). Memory OS layers that ALSO run Smart Connections in Obsidian double-embed the same vault into incompatible spaces unless they configure SC to match. A one-page doc would prevent every implementer from rediscovering this.

### Compatibility

All 4 are additive / opt-in. None break existing CLI users. #4 is documentation only.

### Source context

The 4 drafts sit under `upstream/gks-proposals/` in [Freshair129/msp](https://github.com/Freshair129/msp), each with `Why / What (diff sketch) / Compat / Test / Atom reference` sections. They reference the MSP atoms (ADRs / CONCEPTs) that motivated them — useful if you want the long-form reasoning.

Happy to:
- Split into 4 issues if that fits your workflow better
- Open PRs for any subset (especially #1 + #4 are tiny)
- Adjust scope / API shape based on your input

Thanks!
```

---

## Strategy B — Four separate issues (recommended)

Open in this order — easiest first builds momentum.

### Issue 1 of 4 — Accept `phase: 6` in `propose-inbound`

**Title**:
```
propose-inbound: accept phase: 6 (post-implementation audit atoms)
```

**Body**:
```markdown
## Why

Memory OS layers above GKS often define a Phase 6 = "post-implementation audit / observability". Currently `gks propose-inbound` rejects `--phase 6` (caps at 0..5) so consumers (e.g. [MSP](https://github.com/Freshair129/msp)) work around it by patching inbound files after propose — losing GKS's validation pipeline + audit log entry on the propose action.

## Proposed change

Relax range to `0..6` in `src/memory/inbound.ts`:

```diff
-    if (!Number.isInteger(phase) || phase < 0 || phase > 5) {
-      throw new Error(`invalid phase ${phase}, must be integer 0..5`)
+    if (!Number.isInteger(phase) || phase < 0 || phase > 6) {
+      throw new Error(`invalid phase ${phase}, must be integer 0..6`)
     }
```

GKS doesn't have to assign semantics to phase 6 — only stop rejecting it.

## Compat

- Existing atoms (phases 0–5): unaffected
- Existing callers passing `--phase 6`: previously errored, now succeeds
- Persistence schema: phase already stored as integer; no migration

## Test

Add to `tests/memory/inbound.test.ts`:

```ts
it('accepts phase: 6 (post-implementation audit)', async () => {
  const result = await store.proposeInbound({ proposed_id: 'AUDIT--TEST', phase: 6, type: 'audit', content: 'x' })
  expect(result.path).toMatch(/AUDIT--TEST/)
})

it('still rejects out-of-range', async () => {
  await expect(store.proposeInbound({ ..., phase: 7 })).rejects.toThrow()
  await expect(store.proposeInbound({ ..., phase: -1 })).rejects.toThrow()
})
```

## Drafted by

[MSP M7-prep follow-up](https://github.com/Freshair129/msp/blob/main/upstream/gks-proposals/01-phase-6-acceptance.md) — happy to open a PR if useful.
```

---

### Issue 2 of 4 — `verify-flow --through-superseded` flag

**Title**:
```
verify-flow: add --through-superseded flag to follow superseded_by chain
```

**Body**:
```markdown
## Why

`gks verify-flow <ID>` walks `crosslinks.references` to verify the chain. When a referenced atom has `status: superseded`, the walker halts — even if the supersede chain is intact via `crosslinks.superseded_by`.

Hit during MSP PR #8: `FRAME--MSP-ARCHITECTURE` v1 was marked superseded by V2; CI failed because `verify-flow` refused to walk through v1 to reach the shared ADRs. Worked around by repointing 9 atoms; correct fix is upstream.

## Proposed change

Add `--through-superseded` flag (default OFF — preserves current strict behaviour):

```diff
 export interface VerifyFlowOptions {
   id: string
   verbose?: boolean
+  throughSuperseded?: boolean
 }

 // inside walker:
 if (atom.status === 'superseded') {
+  if (options.throughSuperseded && atom.crosslinks?.superseded_by?.length) {
+    const successor = atom.crosslinks.superseded_by[0]
+    if (visited.has(successor)) break  // cycle guard
+    visited.add(successor)
+    queue.push(successor)
+    continue
+  }
   errors.push(`atom ${atom.id} is superseded; cannot continue (use --through-superseded to follow)`)
   break
 }
```

CLI: `.option('through-superseded', { type: 'boolean', default: false })`.

## Compat

- Default behaviour unchanged — flag is opt-in
- Output shape unchanged
- Cycle guard prevents infinite loops if `superseded_by` ever forms a cycle

## Test

Three cases: halts by default at superseded; walks through with flag; detects supersede cycles. Full sketches in [the draft](https://github.com/Freshair129/msp/blob/main/upstream/gks-proposals/02-verify-flow-through-superseded.md).

## Drafted by

MSP M7-prep follow-up. Happy to open a PR.
```

---

### Issue 3 of 4 — Stable `gks backlinks` API

**Title**:
```
Add stable backlinks derivation API (gks backlinks --emit=jsonl + TS API)
```

**Body**:
```markdown
## Why

`SCOPE.md` declares atomic graph traversal as in-scope for GKS, and the building blocks exist (`atomic_index.jsonl`, `ObsidianAdapter.backlinksOf`). But there's no stable CLI / TS API that derives a flat backlinks JSONL from the index alone.

Memory OS layers reimplement this independently (MSP has ~200 LoC under `src/memory/backlinks/`; `MSP_RELATIONSHIP.md` says it's the implementer's job). Universal need + duplicated logic + drift risk.

## Proposed change

### CLI
```
gks backlinks [--emit=jsonl|json] [--out=<path>] [--filter-type=<predicate>]
```

### TS API
```ts
export interface BacklinkEdge { from: string; to: string; type: string }
export async function deriveBacklinks(store: MemoryStore, opts?: { filterTypes?: string[]; sort?: boolean }): Promise<BacklinkEdge[]>
export async function emitBacklinksJsonl(store: MemoryStore, outPath: string, opts?): Promise<{ edgeCount: number; bytes: number }>
```

### Implementation sketch
Walk `atomic_index.jsonl`, emit one edge per `crosslinks.<predicate>` entry, sort by `(from, to, type)` for git-diff stability. ~50 LoC.

## Compat

Additive — new CLI subcommand + new module. No existing call sites change.

## Test

Three cases: edge emission, type filtering, deterministic sort. Full sketches in [the draft](https://github.com/Freshair129/msp/blob/main/upstream/gks-proposals/03-backlinks-api.md).

## What lands on the consumer side

MSP can replace `src/memory/backlinks/` (~200 LoC) with a thin `import { deriveBacklinks } from '@evaai/gks'` (~20 LoC). Other Memory OS layers (EVA, etc.) get the same.

## Drafted by

MSP M7-prep follow-up. Happy to open a PR.
```

---

### Issue 4 of 4 — Document Smart Connections + nomic-embed-text-v1.5 compatibility

**Title**:
```
docs: add embedder-compatibility note for browse-side plugins (Smart Connections, etc.)
```

**Body**:
```markdown
## Why

3.6.0's `createNomicEmbedder()` (using `nomic-ai/nomic-embed-text-v1.5`) is a great default. But Memory OS layers above GKS often pair it with **Smart Connections** in Obsidian for human browse — and Smart Connections lets the user pick the embedding model in a GUI dropdown.

If GKS embeds with model A and Smart Connections embeds with model B, the same vault is embedded twice into incompatible vector spaces. 2× compute + 2× storage + cross-surface incompatibility.

GKS doesn't have to enforce this — it's a Memory OS concern. But a one-page doc would help every implementer make the same decision instead of discovering the divergence the hard way.

## Proposed change

Documentation only — add `docs/embedder-compatibility.md` covering:

1. Why double-embedding is a trap (dimension / tokenizer / normalisation differences → vectors not comparable)
2. Smart Connections: where to set model in GUI to match GKS's default
3. Other browse plugins: same principle
4. Headless / non-Obsidian setups: GKS vector store is independent
5. What if the project deliberately picks a different model: document in the project's ADRs; configure both sides
6. Re-embedding after a model swap: `npm run gks re-embed` + plugin re-index
7. Why GKS doesn't enforce this (storage engine vs Memory OS scope)

Full body draft in [the file](https://github.com/Freshair129/msp/blob/main/upstream/gks-proposals/04-smart-connections-parity.md).

## Compat

Doc-only. No code change.

## Test

N/A.

## Drafted by

MSP M7-prep follow-up. Happy to open a PR with the markdown drop-in.
```

---

### Issue 6 of 6 — Update `docs/MSP_RELATIONSHIP.md` (post-MSP-cleanup)

> **Drafted 2026-05-10**, not yet filed. Open this one only after the maintainer has digested issues 1–4 (or in parallel — it's purely doc-side, no code conflict).

**Title**:
```
docs: update MSP_RELATIONSHIP.md — MSP migrated off inbound queue (Phase 3) and is now agent-agnostic
```

**Body**:
```markdown
Hi @<maintainer> — `docs/MSP_RELATIONSHIP.md` is now ~6 weeks behind the actual MSP repo (`Freshair129/msp`). Three concrete drift points + one architectural reframe.

## Drift 1 — `/submit-memory` and inbound queue removed

**Current text** (under "What MSP brings on top → Workflow"):
> Agent → /submit-memory → inbound queue → human review → promote → gks/

**Reality**: `Freshair129/msp` completed `BLUEPRINT--INBOUND-TO-CANDIDATES-MIGRATION` in Phase 3 (commit `7eff62b feat: delete msp_propose + inbound infrastructure (Phase 3)`). The inbound queue, `msp_propose` MCP tool, and `scripts/msp/propose.mjs` are all gone. The new path is:

```
Agent → msp_candidate (MCP tool) → .brain/msp/projects/<ns>/candidates/ → human PR → gks/
```

EVA's MSP-v9.1 (Python) may still use the inbound shape; the doc should mark inbound as a legacy path and the candidates pipeline as the new default for `Freshair129/msp`.

## Drift 2 — `gksLayout()` defaults still mention `inbound`

The "Design intent baked into GKS" table cites:
> Default paths under `.brain/msp/projects/<path>/{inbound,session,memory,audit}/`

The `inbound` subdir is no longer written by `Freshair129/msp`. `candidates/` is the new entry. Worth noting that GKS-default `gksLayout()` still includes `inbound` for backwards compat with non-MSP consumers — the doc should call out that MSP-the-consumer doesn't use it anymore.

## Drift 3 — `MSP-IMP-` / `MSP-TSK-` / `MSP-WKT-` are EVA process artifacts, not MSP

Current text:
> MSP-IMP- (P3 plan) → MSP-TSK- (P4 task) → MSP-ACT- (P5 action) → MSP-WKT- (P6 walkthrough)

These IDs come from EVA's `FRAMEWORK_MASTER_SPEC.md` (per GKS `SCOPE.md` line 135's own reference). The MSP repo just declared itself **agent-agnostic** and removed `CORE_FRAMEWORK_MASTER_SPEC.md` (EVA's spec) from its repo (see [Freshair129/msp#65](https://github.com/Freshair129/msp/pull/65), now merged). Suggest moving the `MSP-IMP-` etc. discussion into a separate "EVA-on-GKS" example doc, or qualifying it as "EVA's MSP-v9.1 process IDs" rather than "MSP's".

## Reframe — there are now two MSPs

The current doc conflates:

- **MSP-v9.1** — Python, EVA's biological consolidation Memory OS (RI levels, RMS affect, Session→Core→Sphere cascade)
- **MSP-this-repo** (`Freshair129/msp`) — TypeScript, passport-orchestrator, agent-agnostic, plugs into Claude Code / Gemini CLI / Antigravity / EVA / Hermes / openclaw

The doc reads as if there's only one MSP (the v9.1 Python one). A short "Which MSP?" section near the top would help downstream readers pick the right one.

## Proposed change

Doc-only PR. I can draft if useful:

1. Section "Which MSP?" near top with the two-MSP table
2. Remove `/submit-memory` + inbound queue from the workflow bullet; replace with `msp_candidate` → candidates/ → PR
3. Update the CLI bullets to current scripts (`msp:validate`, `msp:check-links`, `msp:run-task`, `msp:master`, `msp:graph`, `msp:hotfix:*`)
4. Move `MSP-IMP-`/`MSP-TSK-`/`MSP-WKT-` into a "EVA-specific process artifacts" subsection or out to an EVA example
5. Add a "What MSP-this-repo owns" agent-agnostic mini-table

## Source

Drift discovered while auditing MSP architecture docs. See [`AUDIT--ARCH-DOC-CLEANUP`](https://github.com/Freshair129/msp/blob/main/gks/audit/AUDIT--ARCH-DOC-CLEANUP.md) and the new [`CONCEPT--AGENT-AGNOSTIC`](https://github.com/Freshair129/msp/blob/main/gks/concept/CONCEPT--AGENT-AGNOSTIC.md) + [`CONCEPT--AGENT-INTEGRATION-PATTERNS`](https://github.com/Freshair129/msp/blob/main/gks/concept/CONCEPT--AGENT-INTEGRATION-PATTERNS.md).

Happy to open the doc PR if you'd prefer code over discussion.
```

---

## Strategy C — Four draft PRs

If the maintainer prefers code over discussion:

1. Fork `Freshair129/GksV3` (you'll need write to your fork)
2. For each proposal, create a branch + apply the diff sketch from the corresponding draft file
3. Open 4 draft PRs, one per branch
4. Cross-link to the original draft files in the MSP repo (preserves the "why" context)

Suggested branch names:
- `gks/accept-phase-6`
- `gks/verify-flow-through-superseded`
- `gks/backlinks-api`
- `gks/embedder-compatibility-docs`

Each PR body: same as the issue bodies in Strategy B, plus a "## Implementation" section with the actual diff applied.

---

## Submission checklist

- [ ] Choose strategy (A / B / C)
- [ ] Open issues / PRs in `Freshair129/GksV3`
- [ ] Record issue numbers in `upstream/gks-proposals/<file>.md` (replace 🟡 status with 🔵 awaiting review + the issue URL)
- [ ] Subscribe to upstream notifications
- [ ] When something lands → bump file status to 🟢 merged + write `AUDIT--<topic>-UPSTREAMED` atom + remove MSP-side workaround

## After upstream lands — MSP-side cleanup

| Upstream | MSP cleanup |
|---|---|
| #1 phase: 6 accepted | Delete `scripts/msp/propose.mjs`'s phase-6 special-case patching block |
| #2 verify-flow flag | Add `.mspconfig.json` opt-in; pre-commit hook calls `gks verify-flow --through-superseded` |
| #3 backlinks API | Replace `src/memory/backlinks/` with `import { deriveBacklinks } from '@evaai/gks'`; supersede `FEAT--MEMORY-BACKLINKS-INDEXER` → `superseded_by: GKS native API` |
| #4 docs landed | Update MSP's `CONCEPT--EMBEDDING-STRATEGY` to link upstream doc instead of restating |
