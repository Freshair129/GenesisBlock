# HANDOFF P2 — Ready-to-paste GKS upstream issue bodies

> Open 5 separate issues at <https://github.com/Freshair129/GksV3/issues/new> — one per section below. Each is independently mergeable. After filing, edit the corresponding `upstream/gks-proposals/0X-*.md` to bump status from `🟡 drafted` → `🔵 awaiting upstream review` and add the issue URL.

---

## Issue 1 of 5 — Accept `phase: 6` in `propose-inbound`

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

## Issue 2 of 5 — `verify-flow --through-superseded` flag

**Title**:
```

verify-flow: add --through-superseded flag to follow superseded_by chain

```

**Body**:
```markdown
## Why

`gks verify-flow <ID>` walks `crosslinks.references` to verify the chain. When a referenced atom has `status: superseded`, the walker halts — even if the supersede chain is intact via `crosslinks.superseded_by`.

Hit during MSP PR #8: `FRAMEWORK--MSP-ARCHITECTURE` v1 was marked superseded by V2; CI failed because `verify-flow` refused to walk through v1 to reach the shared ADRs. Worked around by repointing 9 atoms; correct fix is upstream.

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

## Issue 3 of 5 — Stable `gks backlinks` API

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

MSP can replace `src/memory/backlinks/` (~200 LoC) with a thin `import { deriveBacklinks } from '@freshair129/gks'` (~20 LoC). Other Memory OS layers (EVA, etc.) get the same.

## Drafted by

MSP M7-prep follow-up. Happy to open a PR.

```

---

## Issue 4 of 5 — Document Smart Connections + nomic-embed-text-v1.5 compatibility

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

## Issue 5 of 5 — Publish `@freshair129/gks@3.6.0` to npm

**Title**:

```
Publish @freshair129/gks@3.6.0 to npm (registry latest is still 3.5.6)
```

**Body**:

```markdown
## Why

`Freshair129/GksV3` repo at HEAD is `package.json: "version": "3.6.0"` and the CHANGELOG documents new features (`createNomicEmbedder`, `OBSIDIAN_URL` env convention, `.env.example` updates). But **npm registry latest is still 3.5.6** — `npm view @freshair129/gks dist-tags` returns `{ latest: '3.5.6' }`, and the 3.6.0 source is unavailable to consumers.

MSP's PR #9 / spec 2.0.1 references 3.6.0 features (locked canonical model = `nomic-embed-text-v1.5`). Consumers have to either:

- Pin a git URL (`@freshair129/gks#main`) — fragile, no dep resolution
- Wait for publish — current state, blocking M7c retrieval orchestration

Most concretely: `ADR--EMBEDDING-MODEL-PARITY` had to add a "Status note" marking the nomic claims as **aspirational** until 3.6.0 publishes. That note can be removed once this lands.

## What

Publish `3.6.0` to npm:

```bash
git checkout main           # in Freshair129/GksV3
npm version 3.6.0           # if package.json isn't already at 3.6.0 (it is)
npm publish --access public
```

Optionally tag the release on GitHub:

```bash
git tag v3.6.0
git push origin v3.6.0
gh release create v3.6.0 --notes-file CHANGELOG-3.6.0.md
```

## Compat

- Additive minor bump (per CHANGELOG): new `createNomicEmbedder()`, `.env.example` additions.
- Existing consumers on 3.5.x: `^3.5.6` semver auto-picks 3.6.0 on next `npm install`. Their existing code keeps working (Ollama / OpenAI / mock providers preserved).
- No API-shape breaks for the `MemoryStore` / `InboundQueue` / `EpisodicLayer` interfaces.

## Test (pre-publish checklist)

```bash
npm run build           # tsc clean
npm test                # vitest green
npm pack                # inspect tarball:
                        #   - dist/src/memory/vector/embedder.js exports createNomicEmbedder
                        #   - .env.example contains OBSIDIAN_URL
                        #   - package.json version: "3.6.0"
```

## Downstream impact

| Consumer | Change after 3.6.0 publishes |
|---|---|
| MSP (this project) | `npm install` auto-picks 3.6.0 via `^3.5.6`. Remove "Status note" from ADR--EMBEDDING-MODEL-PARITY. Remove fallback table. M7c can use `createNomicEmbedder` directly. |
| EVA (memory_os) | Same — auto-picks. |
| New consumers | Get nomic by default, no Ollama prerequisite. |

## Drafted by

MSP M7-prep + two-repo validation audit. Reference: [05-publish-3.6.0.md](https://github.com/Freshair129/msp/blob/main/upstream/gks-proposals/05-publish-3.6.0.md).

```

---

## Submission tracker

✅ All 5 issues filed 2026-05-07 (open, awaiting upstream review):

- [x] Issue 1 — phase: 6 acceptance: https://github.com/Freshair129/GksV3/issues/32
- [x] Issue 2 — verify-flow flag: https://github.com/Freshair129/GksV3/issues/31
- [x] Issue 3 — backlinks API: https://github.com/Freshair129/GksV3/issues/30
- [x] Issue 4 — SC compatibility doc: https://github.com/Freshair129/GksV3/issues/29
- [x] Issue 5 — publish 3.6.0: https://github.com/Freshair129/GksV3/issues/28

Status emojis in `upstream/gks-proposals/0X-*.md` titles bumped 🟡 → 🔵.

When upstream lands one, follow [`upstream/gks-proposals/README.md`](./upstream/gks-proposals/README.md) "Workflow when an upstream lands" — bump dep, replace workaround, write `AUDIT--<topic>-UPSTREAMED` atom.
