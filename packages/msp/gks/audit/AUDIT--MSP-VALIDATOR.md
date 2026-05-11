---
id: AUDIT--MSP-VALIDATOR
phase: 5
type: audit
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: MSP validator M2 acceptance audit
tags:
  - msp
  - validator
  - audit
  - dogfood
crosslinks: {"references":["FEAT--MSP-VALIDATOR","BLUEPRINT--MSP-VALIDATOR","ADR--MSP-VALIDATOR"]}
linked_symbols:
  - {"file":"src/validator/index.ts"}
  - {"file":"src/validator/cli.ts"}
  - {"file":"src/validator/parse.ts"}
  - {"file":"src/validator/atomic-index.ts"}
  - {"file":"src/validator/rules/forbidden-fields.ts"}
  - {"file":"src/validator/rules/id-format.ts"}
  - {"file":"src/validator/rules/id-filename-match.ts"}
  - {"file":"src/validator/rules/adr-monotonic.ts"}
  - {"file":"src/validator/rules/dangling-wikilinks.ts"}
  - {"file":"src/validator/rules/future-date.ts"}
  - {"file":"src/validator/rules/summary-min.ts"}
  - {"file":"src/validator/rules/phase-status.ts"}
created_at: 2026-05-03T06:34:05.276Z
---

# AUDIT — MSP validator M2 acceptance

## Scope

Closes the doc-to-code loop on `FEAT--MSP-VALIDATOR`, implementing every
acceptance criterion declared in that atom against the geography in
`BLUEPRINT--MSP-VALIDATOR`. Note: GKS 3.5.6 caps `phase` at 5, so this
audit is filed at P5 even though the master-spec phase is P6 — an
upstream alignment task tracked in M3+.

## Acceptance criteria from FEAT--MSP-VALIDATOR

| # | Criterion | Result |
|---|---|---|
| 1 | Forbidden field `commit_hash` rejected with `[forbidden-fields]` | ✅ unit + CLI test |
| 2 | ADR colliding with index rejected with `[adr-monotonic]` | ✅ `adr-monotonic.test.ts` |
| 3 | Body wikilink `[[FEAT--ghost]]` rejected with `[dangling-wikilink]` | ✅ unit + CLI test |
| 4 | `crosslinks.references` to unknown ID rejected | ✅ `dangling-wikilinks.test.ts` |
| 5 | `created_at: 2099-...` rejected with `[future-date]` | ✅ `future-date.test.ts` |
| 6 | Bad ID format rejected with `[id-format]` | ✅ `id-format.test.ts` |
| 7 | ID/filename mismatch rejected with `[id-filename-match]` | ✅ `id-filename-match.test.ts` |
| 8 | Short / placeholder summary rejected with `[summary-min]` | ✅ `summary-min.test.ts` |
| 9 | `--all` walks `gks/` + inbound and emits `Total: X passed, Y failed` | ✅ CLI test |
| 10 | Exit-2 when atomic index missing | ✅ CLI test |
| 11 | `--json` flag emits `{file, errors[]}[]` | ✅ CLI test |
| 12 | Dogfood: no false positives on the 4 promoted MSP atoms | ✅ `--all` exit-0 |

## Test summary

```
9 test files, 49 tests, 49 passed (5.42s)
```

Files:

- `test/validator/parse.test.ts` (8) — frontmatter parser + wikilink extractor
- `test/validator/rules/forbidden-fields.test.ts` (4)
- `test/validator/rules/id-format.test.ts` (9)
- `test/validator/rules/id-filename-match.test.ts` (3)
- `test/validator/rules/adr-monotonic.test.ts` (5)
- `test/validator/rules/dangling-wikilinks.test.ts` (5)
- `test/validator/rules/future-date.test.ts` (4)
- `test/validator/rules/summary-min.test.ts` (5)
- `test/validator/cli.test.ts` (6) — end-to-end CLI invocation

## Bugs found during M2 dogfood

1. **id-format regex used lowercase slug.** GKS's canonical
   `ATOMIC_ID_PATTERN` (`/^[A-Z][A-Z0-9_]*--[A-Z0-9][A-Z0-9_-]*$/`) is
   uppercase. Spec said `[a-z0-9-]` — I followed the spec literally and
   got bitten. Aligned to GKS's pattern (extended with the numeric
   `ADR-NNN` form). Documented inline.

2. **Inline-code wikilinks falsely flagged.** The original parser only
   skipped fenced code blocks, but the master spec body has `` `[[X]]` ``
   inline-code examples (e.g. in CONCEPT — "Reference `[[FEAT--ghost]]`
   that doesn't exist"). Added `stripInlineCode()` to mask backtick
   spans before applying the wikilink regex. Column offsets preserved.

## Performance

- `npm run msp:validate -- --all --root=.` over 4 atoms + 1 inbound: ≈ 700 ms wall
  (including tsx startup). Comfortable for a pre-commit hook.

## Residual risks / known gaps

- **No `epistemic.confidence` enforcement** for ADR/protocol atoms (spec
  §4.2). Soft rule, deferred to M3.
- **`cite_or_mark_inferred` warning** for code-path claims is also
  deferred — needs `linked_symbols` cross-referencing.
- **The forbidden-fields list is hardcoded** in `src/validator/rules/forbidden-fields.ts`.
  Loading it from `.brain/msp/LLM_Contract/atomic_contract.yaml` at
  runtime is queued for M3 per BLUEPRINT.
- **`phase: 6` is rejected by GKS 3.5.6**. AUDIT atoms in master-spec terms
  are P6; we file at P5 here. Upstream patch needed before P6 is real.
- **No pre-commit hook installed** — the validator is wired but the hook
  install script (`examples/hooks/pre-commit-validator.sh`) is M3 work.

## Sign-off

- Implemented by: @claude-opus-4-7
- Verified by: `gks verify-flow FEAT--MSP-VALIDATOR` (OK), `gks validate --links` (OK), `npm test` (49/49)
- Date: 2026-05-03

## References

- `FEAT--MSP-VALIDATOR` — acceptance criteria source
- `BLUEPRINT--MSP-VALIDATOR` — geography + verification plan
- `ADR--MSP-VALIDATOR` — rule semantics + exit-code contract
- `msp_spec.md` §4 — atomic write contract that this validator enforces
