---
id: AUDIT--PROTO-LOADER
phase: 6
type: audit
status: stable
vault_id: default
title: M8a — PROTO loader implementation (foundation for M8b–f)
tags:
  - msp
  - proto
  - loader
  - audit
  - m8a
crosslinks: {"references":["FEAT--PROTO-LOADER","BLUEPRINT--PROTO-LOADER","ADR--PROTO-ATOM-TYPE","CONCEPT--PROTO-PATTERN"]}
linked_symbols:
  - {"file":"src/validator/proto/types.ts"}
  - {"file":"src/validator/proto/loader.ts"}
  - {"file":"src/validator/proto/sample.ts"}
  - {"file":"src/validator/cli.ts"}
  - {"file":"gks/proto/PROTO--SAMPLE-RULE.md"}
  - {"file":"test/validator/proto/loader.test.ts"}
  - {"file":"test/validator/proto/sample.test.ts"}
created_at: 2026-05-05T11:00:00.000Z
---

# M8a — PROTO loader (foundation)

## Scope

Implements `FEAT--PROTO-LOADER` per `BLUEPRINT--PROTO-LOADER` T1..T6. Foundation for M8b–f governance PROTOs.

## What shipped

| File | Purpose |
|---|---|
| `src/validator/proto/types.ts` | `Severity`, `ProtoStatus`, `ProtoMeta`, `PredicateContext`, `Predicate`, `PredicateResult`, `ProtoSummary` |
| `src/validator/proto/loader.ts` | `discoverProtos()`, `runProtos()`, `shouldFailExit()` |
| `src/validator/proto/sample.ts` | Trivial demo predicate (default export) |
| `src/validator/cli.ts` | Calls loader after regular rules; prints PROTO summary; exits 1 if stable + severity:error |
| `gks/proto/PROTO--SAMPLE-RULE.md` | `status: draft` demo atom |
| `test/validator/proto/loader.test.ts` | 16 tests (discover + runProtos + shouldFailExit) |
| `test/validator/proto/sample.test.ts` | 3 tests (predicate behaviour) |

## T1..T6 status

- [x] **T1 TYPES** — full surface in `types.ts`
- [x] **T2 LOADER** — `discoverProtos`, `runProtos`, `shouldFailExit` + 16 tests
- [x] **T3 SAMPLE** — `sample.ts` + `PROTO--SAMPLE-RULE.md` (draft) + 3 tests
- [x] **T4 CONTRACT** — N/A: validator's id-format regex already accepts `PROTO--*`. `severity` field not in forbidden list. No contract change needed.
- [x] **T5 CLI** — `src/validator/cli.ts` extended; prints `PROTOs: P passed, Q failed`; exit-code logic
- [x] **T6 AUDIT** — this file

## Atomic index format

Predicates receive `atomicIndex` as `AtomicIndexEntry[]` (array). The CLI converts the in-memory `Map<string, AtomicIndexEntry>` to an array via `Array.from(map.values())` before invoking. Predicates can `index.some(...)`, `index.filter(...)`, etc. without worrying about Map semantics.

## Decisions during impl

1. **Contract change skipped** — the existing id-format regex `^[A-Z][A-Z0-9_]*--...` already accepts `PROTO--*`. Adding a "proto" required-fields entry to `atomic_contract.yaml` is unnecessary because the loader itself enforces `crosslinks.enforces` + `linked_symbols`. Keeping the contract simpler.

2. **`severity` defaults to `'warning'`** — atoms that omit `severity:` from frontmatter get warnings. Conservative default; explicit `severity: error` required for hard gates.

3. **PROTO predicates receive `AtomicIndexEntry[]`** (array, not Map) — predicates are pure functions that filter / scan. Array is the natural shape; Map is an implementation detail of the loader.

4. **Dynamic `import(file://...)` for impl** — predicates are loaded via `pathToFileURL` (ESM requirement). Path resolution: `linked_symbols[0].file` is repo-relative; absolute paths also accepted.

5. **Draft PROTOs run but never fail-exit** — even `severity: error` violations from a draft don't return non-zero. This enables incremental rollout: write the predicate, ship as draft, observe, then promote to stable.

6. **Loader silently drops malformed atoms** — bad id pattern, missing `enforces`, missing `linked_symbols`, impl file not found → atom is skipped (not loaded as PROTO). The regular validator still validates the atom file itself (forbidden fields, etc.); PROTO loader is layered on top.

## Verification

- `npm test` → **498 passed** (was 478; +16 loader + 3 sample = 19 extra accounting for new tests + 1 atom)
- `npm run typecheck` → clean
- `npx tsx src/validator/cli.ts --all` → 143/143 atoms pass + 1 PROTO passed
- `npm run msp:check-links` → OK

## Counts

- Atoms 142 → 143 (+1 PROTO--SAMPLE-RULE demo)
- Tests 478 → 498 (+20)
- New PROTO directory: `gks/proto/`
- New impl directory: `src/validator/proto/`

## Unblocks

- M8b — `PROTO--PHASE-GATES`
- M8c — `PROTO--SCALING-LEVEL-GATE`
- M8d — `PROTO--ALGO-PARAM-COUPLING`
- M8e — `PROTO--AUTHORITY-ENFORCEMENT`
- M8f — Audit existing rules → promote 3 to PROTO

Each follows the same shape: atom in `gks/proto/`, impl in `src/validator/proto/`, predicate signature `(ctx) => PredicateResult`.

## Source

`FEAT--PROTO-LOADER`, `BLUEPRINT--PROTO-LOADER`, `ADR--PROTO-ATOM-TYPE`, `CONCEPT--PROTO-PATTERN`.
