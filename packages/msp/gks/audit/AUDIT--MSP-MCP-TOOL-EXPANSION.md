---
id: AUDIT--MSP-MCP-TOOL-EXPANSION
phase: 6
type: audit
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: M7f — MCP tool expansion (4 of 5 passport tools shipped; msp_compress deferred pending M7d)
tags:
  - msp
  - mcp
  - tools
  - m7f
  - audit
crosslinks: {"references":["FEAT--MSP-MCP-TOOL-EXPANSION","BLUEPRINT--MSP-MCP-TOOL-EXPANSION","CONCEPT--MSP-MCP-TOOL-EXPANSION","FEAT--MSP-MCP-SERVER","FEAT--CONSOLIDATOR","FEAT--RETRIEVAL-ORCHESTRATION","FEAT--IDENTITY-LAYER","FEAT--COMPRESSOR"]}
linked_symbols:
  - {"file":"src/mcp/server.ts"}
  - {"file":"src/mcp/tools/recall.ts"}
  - {"file":"src/mcp/tools/remember.ts"}
  - {"file":"src/mcp/tools/identity-get.ts"}
  - {"file":"src/mcp/tools/identity-set.ts"}
  - {"file":"test/mcp/tools/recall.test.ts"}
  - {"file":"test/mcp/tools/remember.test.ts"}
  - {"file":"test/mcp/tools/identity-get.test.ts"}
  - {"file":"test/mcp/tools/identity-set.test.ts"}
  - {"file":"test/mcp/server.test.ts"}
  - {"file":"test/mcp/bin.test.ts"}
created_at: 2026-05-05T17:35:30.812+07:00
---

# M7f — MCP tool expansion (4 of 5 passport tools shipped; msp_compress deferred)

## Scope

M7f deliverable per `FEAT--MSP-MCP-TOOL-EXPANSION` + `BLUEPRINT--MSP-MCP-TOOL-EXPANSION`: wrap M7b (consolidator), M7c (retrieval), M7d (compressor), M7e (identity) TS APIs as MCP tools so agents talking to MSP via stdio can reach them. The M6 server already shipped 6 gatekeeper-side tools (`msp_validate`, `msp_propose`, `msp_run_task`, `msp_session_append`, `msp_episode_append`, `msp_backlinks_rebuild`); M7f adds the **passport-side** tools.

## What shipped (4 of 5 planned)

| Tool | Wraps | Status |
|---|---|---|
| `msp_recall` | M7c `recall()` | shipped |
| `msp_remember` | M7b `consolidate()` + episodic writer | shipped |
| `msp_identity_get` | M7e `getIdentity()` (+ optional `prunePreferences`) | shipped |
| `msp_identity_set` | M7e `setProfile / setVoice / setPreference` (discriminated by `kind`) | shipped |
| `msp_compress` | M7d `compress()` | **DEFERRED** (M7d not yet on main) |

Tools list went from 6 → 10 (target was 11 if `msp_compress` had landed).

## msp_compress deferral

The blueprint explicitly contemplated this case (`if M7d on main, wrap; else omit and note in AUDIT`):

- At branch-creation time (commit `e5e559f` on main), `src/orchestrator/compressor/` did not exist
- M7d ships compressor atoms (P1–P3) only; the impl PR has not landed
- Per blueprint hard-constraint "do NOT add a stub", `msp_compress` is fully omitted from this PR (no file, not registered)
- A follow-up PR will add `src/mcp/tools/compress.ts` + tests once M7d's `compress()` is on main

This keeps the M7f surface honest — every shipped tool actually works end-to-end against a real wrapped API.

## Files added (5 src + 4 tests)

| File | Purpose |
|---|---|
| `src/mcp/tools/recall.ts` | `msp_recall` handler — best-effort `tryCreateObsidianClient` + delegation to `recall()` |
| `src/mcp/tools/remember.ts` | `msp_remember` handler — `consolidate()` then bridge to `appendEpisode()` |
| `src/mcp/tools/identity-get.ts` | `msp_identity_get` handler — optional prune + `getIdentity()` |
| `src/mcp/tools/identity-set.ts` | `msp_identity_set` handler — discriminated dispatch on `kind: 'profile' \| 'voice' \| 'preference'` |
| `test/mcp/tools/recall.test.ts` | 7 tests — name + empty-repo + episodic match + top_k + weights + timeout + namespace |
| `test/mcp/tools/remember.test.ts` | 6 tests — name + missing session + emit+persist + default namespace + idempotence + error-shape |
| `test/mcp/tools/identity-get.test.ts` | 6 tests — name + default + persisted + namespace + prune + schemaVersion guard |
| `test/mcp/tools/identity-set.test.ts` | 13 tests — profile (set, set-once, missing) × voice (replace, missing) × preference (no TTL, expires_at, expires_in_ms, override, missing key, missing value) × namespace |

## Files modified (additive only)

- `src/mcp/server.ts` — register 4 new tools alongside the 6 existing M6 tools (additive only; no M6 handler changes)
- `test/mcp/server.test.ts` — assertion bumped from "exactly 6 tools" to "exactly 10 tools" with the new sorted list
- `test/mcp/bin.test.ts` — assertion bumped (server bin test for `tools/list` count)

No changes to M6 tool handlers, types, or the bin entry-point. No new runtime deps.

## Boundaries respected (hard constraints from blueprint)

- ✓ Did NOT modify any M6 tool handler (additive only)
- ✓ Did NOT add streaming output
- ✓ Did NOT add per-tool auth (M9 concern)
- ✓ Did NOT cache results
- ✓ Did NOT re-implement consolidate / recall / identity inside any tool — every handler delegates
- ✓ Did NOT add new runtime deps
- ✓ Did NOT add a stub for `msp_compress` — fully omitted

## Critical impl detail honoured: ObsidianClient lazy construction

`recall.ts` exports an inline `tryCreateObsidianClient({ root })` helper that wraps `createObsidianClient` in a try/catch and returns `undefined` on any failure. This matches the recall orchestrator's contract — when no client is supplied, the obsidian source emits `obsidian-text: skipped` in `fallback_reasons` rather than erroring. Avoids a dead probe per call when the env isn't configured.

## Episode shape adapter (remember.ts)

The consolidator emits `Episode` (sessionId, turnRange, summary, tags, score, scoreSource, createdAt) — but the episodic writer (`appendEpisode` from `src/memory/episodic/writer.ts`) expects the on-disk `MemoryEpisode` shape (episodicId, sessionId, projectId, range[], importance_score, content.summary, ...). `remember.ts` includes a small `toMemoryEpisode(ep, namespace)` adapter:

- `episodicId` = `${sessionId}-${start}-${end}` (deterministic — enables idempotence on re-call)
- `range` = `['turnIdx-N']` for single-turn or `['turnIdx-N..turnIdx-M']` for ranges (matches the existing `inRange` parser in writer.ts)
- `projectId` = namespace (existing convention)
- `importance_score` = `ep.score` (already 0..1)

Idempotence: appendEpisode dedups by `episodicId`, so calling `msp_remember` twice on the same session produces the same on-disk file, not duplicates.

## SLM provider scope

`createSlmClient` currently exposes `'mock' | 'ollama'`. The blueprint mentioned `'auto' | 'openai'` aspirationally, but the factory hasn't been extended — `remember.ts` accepts `'mock' | 'ollama'` (matching `run-task.ts`) and defaults to `'mock'` (safe for any agent caller without a configured Ollama).

## Verification

```
npm test                            → 429 passed (was 397 → +32)
npm run typecheck                   → clean
npx tsx src/validator/cli.ts --all  → 130/130 passed
npm run msp:check-links             → OK (130 atoms scanned)
```

Test count delta: 397 → 429 (+32 — within blueprint target of ~32 with msp_compress omitted; full target was ~40 with all 5 tools).

## Acceptance criteria from `FEAT--MSP-MCP-TOOL-EXPANSION`

- [x] **5 tools registered** — *partial: 4 shipped, msp_compress deferred (documented above)*
- [x] Each tool follows M6 handler pattern: `({ root, ... }) => async (args) => result`
- [x] `msp_recall` wraps M7c `recall()`; constructs `createObsidianClient` lazily, passes `undefined` on failure
- [x] `msp_remember` wraps M7b `consolidate()` + appends emitted episodes via `appendEpisode` (the existing episodic writer)
- [ ] `msp_compress` — deferred (depends on M7d)
- [x] `msp_identity_get` wraps M7e `getIdentity()`; optionally calls `prunePreferences()` first when `prune: true`
- [x] `msp_identity_set` dispatches on `kind: 'profile' | 'voice' | 'preference'` to the right setter
- [x] All tools accept `namespace?` defaulting to `'evaAI'` (matches M6 convention)
- [x] All tools handle `root` via the existing context (`ctx.root`) + optional `args.root` override
- [x] Tool tests under `test/mcp/tools/` (4 new files, mirroring M6 pattern)
- [x] Server registration test verifies all 10 tools (6 M6 + 4 M7f) via sorted `REGISTERED_TOOL_NAMES`
- [x] `bin.test.ts` server-spawn smoke test bumped to assert all 10 tools via real `tools/list`

## Decisions during impl

1. **Inline `tryCreateObsidianClient` in recall.ts** rather than a shared helper — only one consumer; keeps surface small.

2. **`msp_remember` derives `episodicId` deterministically.** `${sessionId}-${start}-${end}` so that re-running the tool on the same session produces stable IDs and `appendEpisode`'s upsert semantics dedupe naturally. This matches the consolidator's idempotence contract.

3. **`msp_remember` defaults provider to `'mock'`** (matches `msp_run_task`). Any agent can call it without an Ollama dependency. To activate tier-2 LLM consolidation, callers explicitly pass `provider: 'ollama'`.

4. **`msp_remember`'s `llm_calls` counter** counts episodes whose `scoreSource === 'tier2'`. Tier-2-default (i.e. LLM was called but bailed) is not counted as a successful call. Matches the spirit of the FEAT example output.

5. **`msp_identity_set` validates the discriminated union at runtime** rather than relying on Zod's union type. Reason: the wire shape needs to be flat (`{ kind, partial?, voice?, key?, value?, ... }`) so that MCP clients can supply it via plain JSON Schema; Zod discriminated unions don't survive that translation cleanly. Each `case` block validates the required field for that kind and returns a structured error otherwise.

6. **`msp_identity_set` returns the full identity post-write** (per FEAT) — saves the caller a follow-up `msp_identity_get`.

7. **`msp_identity_get` does NOT auto-prune by default.** Pruning writes to disk, and the read-only invariant says reads should not mutate. Callers opt in via `prune: true` when they actually want eager cleanup.

8. **No JSON Schema for `value` in `msp_identity_set` preferences.** The spec says preferences are "JSON-serialisable any shape" — Zod `z.unknown()` is the right primitive here. The handler verifies `'value' in args` for explicit-undefined safety.

9. **`process.cwd()` is NOT swapped during tool calls** (unlike `msp_run_task` which `chdir`s for the codegen runner). The wrapped APIs (`recall`, `consolidate`, `getIdentity`, etc.) all take an explicit `root` option, so no cwd dance is needed.

## Public API surface

```jsonc
// MCP tools/list now includes:
{ "name": "msp_recall",       "description": "..." }
{ "name": "msp_remember",     "description": "..." }
{ "name": "msp_identity_get", "description": "..." }
{ "name": "msp_identity_set", "description": "..." }
// + the 6 existing M6 tools
```

TS handler imports (for advanced direct callers):

```ts
import { handler as recallHandler }       from '@/mcp/tools/recall'
import { handler as rememberHandler }     from '@/mcp/tools/remember'
import { handler as identityGetHandler }  from '@/mcp/tools/identity-get'
import { handler as identitySetHandler }  from '@/mcp/tools/identity-set'
```

## Out of scope (deferred)

- `msp_compress` — M7f follow-up once M7d's `src/orchestrator/compressor/` lands
- Per-tool auth — M9
- Streaming tool outputs — out of MCP-stdio scope
- Tool versioning / capability negotiation — current MCP spec doesn't need it
- Tool discovery beyond MCP's standard `tools/list` — caller uses the protocol directly

## Sign-off

- Implemented by: @claude-opus-4-7
- Verified by: 32 new tests + validator (130/130) + check-links OK + typecheck clean
- Branch: `claude/msp-m7f-mcp-tools-impl`
- Follow-up: file a small PR for `msp_compress` once M7d is merged to main
