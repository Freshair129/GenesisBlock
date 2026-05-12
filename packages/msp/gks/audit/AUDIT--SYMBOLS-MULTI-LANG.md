---
id: AUDIT--SYMBOLS-MULTI-LANG
phase: 6
type: audit
status: stable
tier: process
source_type: axiomatic
vault_id: default
title: AUDIT — Multi-language symbol parsing (Python tree-sitter + COBOL regex) verification
tags:
  - msp
  - symbol-graph
  - multi-lang
  - python
  - cobol
  - audit
  - tree-sitter
crosslinks: {"references":["FEAT--SYMBOLS-MULTI-LANG","BLUEPRINT--SYMBOLS-MULTI-LANG","ADR--SYMBOLS-PYTHON-PARSER","ADR--SYMBOLS-COBOL-STRATEGY","FRAME--SYMBOL-GRAPH"]}
linked_symbols:
  - {"file":"packages/msp/src/symbols/parser/python.ts"}
  - {"file":"packages/msp/src/symbols/parser/cobol.ts"}
  - {"file":"packages/msp/src/symbols/parser/index.ts"}
  - {"file":"packages/msp/test/symbols/parser-python.test.ts"}
  - {"file":"packages/msp/test/symbols/parser-cobol.test.ts"}
created_at: 2026-05-12T06:12:00.000+07:00
---

# AUDIT — Multi-language symbol parsing

## Scope verified

The implementation of `FEAT--SYMBOLS-MULTI-LANG` per `BLUEPRINT--SYMBOLS-MULTI-LANG`:
- TypeScript parser kept as-is (existing baseline, refactored to async)
- Python parser added via `web-tree-sitter` (WASM-loaded grammar)
- COBOL parser added as regex-based (PROGRAM-ID, SECTION/DIVISION, PERFORM/CALL)
- Parser registry that dispatches by file extension
- `--lang` flag on the `msp-graph` CLI

Pipeline upgraded to **asynchronous** end-to-end so tree-sitter WASM grammars can be loaded lazily.

## Test results

All 10 unit tests pass on Node 22 / Windows 10 / pnpm-style workspace:

| Test file | Cases | Status |
|---|---|---|
| `test/symbols/parser.test.ts` (TypeScript baseline) | 5 | ✓ all pass |
| `test/symbols/parser-python.test.ts` | 3 | ✓ all pass |
| `test/symbols/parser-cobol.test.ts` | 2 | ✓ all pass |
| **Total** | **10** | **✓ 10/10** |

Run locally:
```
npx vitest run packages/msp/test/symbols/parser*.test.ts
```

Duration: 3.36s (transform 174ms, setup 0ms, collect 646ms, tests 2.52s, prepare 406ms).

## Validator + crosslink integrity

- `npm run msp:index` → 217 atoms indexed (incl. 3 new FEATs + 1 BLUEPRINT + 2 ADRs from this work)
- `npm run msp:check-links` → PASS, all crosslinks resolve
- `npx tsx packages/msp/src/validator/cli.ts --root=packages/msp <new atom>` → 3 of 3 pass (after `created_at` UTC correction)

## Deviations from plan

| Plan item | Deviation | Reason |
|---|---|---|
| `ADR--SYMBOLS-PYTHON-PARSER` `tier: architecture` | warning (validator wants `safety \| master \| genesis \| process`) | Kept `architecture` to match the convention of existing ADRs in `packages/msp/gks/adr/`; warning-only, not blocking |
| `ADR--SYMBOLS-COBOL-STRATEGY` `tier: architecture` | same | same |
| `created_at` in 3 new atoms | initially set to local TH+7 time → tripped `future-date` validator | Corrected to UTC during this audit; root cause: handoff §4.1 warned but Antigravity agent didn't apply. Improvement for future handoffs: have agents echo `date -u` before authoring atoms |

## Anti-hallucination check

- All 3 parser implementations exercise real fixtures (Python class with imports + dunder method; COBOL hello-world + multi-section PERFORM chain; TS class with `extends`/`implements`)
- No false positives on syntactically broken input (TypeScript baseline test "returns empty arrays on syntactically broken TS without throwing" — same pattern was carried into Python/COBOL)

## Follow-ups (handled in next PRs)

- **PR 2** (`FEAT--SYMBOLS-FRAMEWORK-AWARENESS`): Next.js routing/server-client/data-fetching awareness — FEAT already extended with §1b detail
- **PR 3** (`FEAT--SYMBOLS-PROCESS-TRACING`): cross-file CALL resolution + entry-point-to-leaf tracer + new `symbol_trace` MCP tool
- **Closed by future ADR**: tree-sitter-cobol upgrade decision (deferred per `ADR--SYMBOLS-COBOL-STRATEGY`)

## Source

- `FEAT--SYMBOLS-MULTI-LANG` (intent + acceptance)
- `BLUEPRINT--SYMBOLS-MULTI-LANG` (impl plan)
- `ADR--SYMBOLS-PYTHON-PARSER`, `ADR--SYMBOLS-COBOL-STRATEGY` (decisions)
- `HANDOFF-SYMBOLS-EXPANSION.md` (process contract followed for this PR)
- Implementation by Antigravity agent (2026-05-11); PR-closure + AUDIT authored by Claude in MSP main repo
