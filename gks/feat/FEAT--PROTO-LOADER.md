---
id: FEAT--PROTO-LOADER
phase: 2
type: feat
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: PROTO loader — discovers PROTO-- atoms and runs their predicates as part of msp:validate
tags:
  - msp
  - proto
  - validator
  - loader
  - m8a
  - user-facing
crosslinks: {"implements":["ADR--PROTO-ATOM-TYPE"],"references":["CONCEPT--PROTO-PATTERN","FEAT--MSP-VALIDATOR"]}
linked_symbols:
  - {"file":"packages/msp/src/validator/proto/loader.ts"}
  - {"file":"packages/msp/src/validator/proto/types.ts"}
  - {"file":"packages/msp/src/validator/proto/sample.ts"}
created_at: 2026-05-05T16:18:00.000+07:00
---

# FEAT — PROTO loader

## User-facing behaviour

```bash
# Existing validator now also runs PROTOs
npx tsx src/validator/cli.ts --all
# → output:
# ... existing rule output ...
# PROTO--SAMPLE-RULE (status: stable, severity: warning):
#   ✓ all checks pass
# PROTO--SOME-OTHER-RULE (status: draft):
#   (skipped — draft)
# Total: N atoms passed, M failed; PROTOs: P passed, Q failed
```

PROTO predicates that fail with `severity: error` cause `--all` to exit 1. `severity: warning` exits 0 with a warning printed.

## Acceptance criteria

- [ ] **New atom type `proto`** registered in `src/validator/contract.ts` (or equivalent contract loader)
- [ ] **`proto` ID pattern** `^PROTO--[A-Z][A-Z0-9-]*$` enforced
- [ ] **PROTO required-fields** check: `crosslinks.enforces` (≥1 FRAME), `linked_symbols` (≥1 src file)
- [ ] **`gks/proto/` directory** is scanned by the loader
- [ ] **Loader** reads each atom, dynamically imports the predicate from `linked_symbols[0].file`, calls it with the atom set
- [ ] **`status: draft` PROTOs** are loaded but their results don't fail CI (logged as info)
- [ ] **`status: stable` PROTOs** with `severity: error` violations cause exit 1
- [ ] **`status: stable` PROTOs** with `severity: warning` violations log a warning, exit 0
- [ ] **`PROTO--SAMPLE-RULE`** (a trivial demo) ships and is `status: draft` — proves loader works end-to-end without fail-blocking CI
- [ ] **CLI summary** shows `PROTOs: P passed, Q failed`
- [ ] Test target ~470 → ~485 (+15)

## Surfaces

| Surface | Form |
|---|---|
| Validator | `npx tsx src/validator/cli.ts --all` (extended) |
| Loader | `src/validator/proto/loader.ts` |
| Types | `src/validator/proto/types.ts` (Predicate, PredicateResult, Severity) |
| Sample | `src/validator/proto/sample.ts` (trivial demo) |
| Atom | `gks/proto/PROTO--SAMPLE-RULE.md` (status: draft) |
| Tests | `test/validator/proto/{loader,sample}.test.ts` |

## Out of scope

- Specific governance PROTOs (PHASE-GATES / SCALING-LEVEL-GATE / etc.) — M8b–f
- PROTO discovery via Obsidian — filesystem scan only
- Async predicates — predicates are sync pure functions for M8a (async + I/O can be added later)
- PROTO sandboxing — same Node process, no isolation
