---
id: AUDIT--GENESIS-GRAPH-PHASE0-POLISH
phase: 6
type: audit
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: Genesis Graph Phase 0 (TS-first backend) — coverage polish acceptance audit
tags:
  - msp
  - gks
  - graph
  - backend
  - genesis-graph
  - audit
  - cypher-v0
crosslinks: {"references":["BLUEPRINT--GENESIS-GRAPH-TS-FIRST","ADR--GENESIS-GRAPH-AS-GKS-BACKEND","SPEC--GENESIS-GRAPH-BACKEND","CONCEPT--GENESIS-GRAPH-BACKEND"]}
linked_symbols:
  - {"file":"packages/gks/src/memory/graph/genesis-graph.ts"}
  - {"file":"packages/gks/src/memory/graph/cypher-v0.ts"}
  - {"file":"packages/gks/src/memory/graph/genesis-graph-errors.ts"}
  - {"file":"packages/gks/src/memory/index.ts"}
  - {"file":"packages/gks/test/memory/genesis-graph-cypher.test.ts"}
  - {"file":"packages/gks/test/memory/memory-store-genesis-graph.test.ts"}
  - {"file":"apps/qwen/strip_fence.py"}
  - {"file":"apps/qwen/run_microtask.sh"}
created_at: 2026-05-16T14:30:00.000+07:00
aliases:
  - AUDIT
  - implementation_flow
  - Test results / quality report
cluster: implementation_flow
role: "Test results / quality report"
---

# AUDIT — Genesis Graph Phase 0 polish

## Scope

Closes the test-coverage and housekeeping gaps left after `[[BLUEPRINT--GENESIS-GRAPH-TS-FIRST]]` Phase 0 landed. Implementation of the Phase 0 backend itself (`GenesisGraphBackend`, `parseCypherV0`, error classes, MemoryStore wire-up) was already merged earlier. This milestone closes the BLUEPRINT's "every v0 shape + the unsupported-feature error path" test requirement and the wire-up loose ends.

Shipped via two squash-merge PRs against `main`:

- **PR #145** `chore(gks): close Genesis Graph Phase 0 test gaps + cleanup` → main `de55a6d`
- **PR #146** `docs(qwen): fix invocation pattern + document helpers` → main `d2fc4b2`

## Acceptance criteria from BLUEPRINT

| # | Criterion | Result |
|---|---|---|
| 1 | `genesis-graph.ts` implements full `GraphBackend` surface (`load`, `addNode`, `addEdge`, `retractEdge`, `query`, `neighbors`) | ✅ (pre-existing) |
| 2 | Adds `cypher(query)` opt-in surface, v0 subset only | ✅ (pre-existing) |
| 3 | Persists via JSONL event log + `manifest.json` (schema_version 1.0.0) | ✅ (pre-existing) |
| 4 | Uses the on-disk directory shape the future Rust crate will adopt | ✅ (pre-existing) |
| 5 | Wire-up in `memory/index.ts` — exports `createGenesisGraphBackend`, `GenesisGraphBackend`, `GenesisGraphUnsupportedCypher` | ✅ |
| 6 | `.gitignore` covers the on-disk artifact (`genesis-graph.jsonl`) | ✅ (fixed — patterns had been authored against an earlier `genesis-block.jsonl` name) |
| 7 | Full contract suite covers GraphStore + GenesisGraphBackend equivalently | ✅ `test/memory/graph.test.ts` runs both via `describe.each` |
| 8 | Cypher v0 test file covers every supported shape + unsupported-feature error path | ✅ `test/memory/genesis-graph-cypher.test.ts` — 21 tests (was 5) |
| 9 | Schema-version drift handled through `enforceSchemaCompatibility` / `SchemaVersionMismatchError` (no separate genesis-only error class) | ✅ (dead `GenesisGraphSchemaMismatchError` removed) |

## New test coverage (delta on top of pre-existing 5 Cypher tests)

| Test | Covers |
|---|---|
| `empty query rejects …` | empty + whitespace-only input |
| `missing RETURN clause rejects …` | bare MATCH pattern |
| `inverted hop range *5..3 rejects …` | parser hop-range guard |
| `unsupported keyword %s rejects …` (`it.each` × 7) | CREATE / MERGE / DELETE / SET / UNWIND / WITH / UNION |
| `WHERE predicate on seed alias a filters the seed` | predicate alias `a` (was only `b` tested) |
| `WHERE with AND combines predicates conjunctively` | multi-AND |
| `length(r) without AS uses default key length_r` | default RETURN alias |
| `rel-type union r:references\|implements walks both edge types` | multi-rel union |
| `keywords are case-insensitive` | `match` / `Where` / `return` |
| `seed label mismatch yields empty result` | label guard |
| `memory-store-genesis-graph.test.ts` × 3 | MemoryStore × GenesisGraphBackend wiring (direct instance, layout-factory, re-init persistence) |

## Test summary

```
npm test --workspace=packages/gks --run
Test Files  46 passed | 1 skipped (47)
Tests       364 passed | 13 skipped (377)
```

Files most affected:
```
test/memory/graph.test.ts                          16 tests (parametrised, both backends)
test/memory/genesis-graph.test.ts                   7 tests (legacy non-param suite)
test/memory/genesis-graph-cypher.test.ts           21 tests (5 → 21)
test/memory/memory-store-genesis-graph.test.ts      3 tests (new)
```

## Toolchain delta — reusable Qwen helpers

The bulk of the new Cypher tests (10 of 11 microtasks) were authored by the T1 tier (Qwen 2.5 Coder 14B on local Ollama) via single-shot prompts. Two reusable helpers landed:

| File | Role |
|---|---|
| `apps/qwen/strip_fence.py` | Strip surrounding ```ts / ```python fences from Qwen stdout — model adds them even when told not to. |
| `apps/qwen/run_microtask.sh` | `prompt-file → qwen.py → strip_fence.py → output-file` pipeline. Used in a single shell loop to batch all 10 prompts (~6 min wall). |

`qwen.md` was also fixed to document the correct positional-arg invocation and the four system-prompt presets (`--code` / `--review` / `--test` / `--doc`).

## Out of scope (tracked elsewhere)

- **Rust crate** at `packages/gks/native/genesis-graph/` — `[[BLUEPRINT--GENESIS-GRAPH-INTEGRATION]]` P3.1 – P3.6, beyond T1 single-shot scope.
- **Default-flip** of `MemoryStore` to `GenesisGraphBackend` — explicitly gated behind P3.5 benchmarks and a separate ADR amendment, per `[[ADR--GENESIS-GRAPH-AS-GKS-BACKEND]]` §5.
- **PgGraphBackend oracle harness** — the parametrised suite currently covers `GraphStore` + `GenesisGraphBackend`. Folding `PgGraphBackend` in is P3.2 – P3.5 work; requires Postgres in CI.

## Residual

- Cypher v0's hop range `*N..` (open upper) still defaults `maxHops` to `minHops` (`cypher-v0.ts:116`). Test coverage now treats this as "exactly N hops"; if open-ended semantics are wanted later, that's a separate parser change + ADR note.
- `qwen.py` has a hard-coded 120s read timeout. Long prompts (M13 full-file generation) hit it during this milestone; the integration test was authored manually instead. Documented in updated `qwen.md` caveats.

## Sign-off

- Implemented by: @claude-opus-4-7 (T3) with @qwen-2.5-coder-14b (T1) authoring 10 of 11 new test cases
- Verified by: 364/364 active gks tests (13 skipped, 0 failed) + typecheck
- Date: 2026-05-16

## Connections
- [[SPEC--GENESIS-GRAPH-BACKEND]]
- [[CONCEPT--GENESIS-GRAPH-BACKEND]]

