---
id: FEAT--MSP-VALIDATOR
phase: 2
type: feat
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: msp:validate — schema, ID, wikilink, anti-hallucination gate
tags:
  - msp
  - validator
  - cli
  - user-facing
crosslinks: {"implements":["ADR--MSP-VALIDATOR"],"references":["CONCEPT--MSP-VALIDATOR"]}
linked_symbols:
  - {"file":"src/validator/index.ts"}
  - {"file":"src/validator/rules/forbidden-fields.ts"}
  - {"file":"src/validator/rules/dangling-wikilinks.ts"}
  - {"file":"src/validator/rules/id-uniqueness.ts"}
  - {"file":"src/validator/cli.ts"}
created_at: 2026-05-03T06:24:25.043Z
---

# FEAT — msp:validate

## User-facing behaviour

Given an MSP-managed repo with `gks/00_index/atomic_index.jsonl` populated, when an agent or developer runs

```sh
npm run msp:validate -- <file.md>
# or
npx msp-validate <file.md>
```

the validator parses the frontmatter + body, applies every hard rule from `ADR--MSP-VALIDATOR`, and either:

- prints `✓ <file>` and exits 0, or
- prints one or more `✗ <file>:<line> [rule-id] <reason>` lines and exits 1.

For whole-tree mode:

```sh
npm run msp:validate -- --all
```

walks every atom under `gks/` and every candidate under `.brain/msp/projects/evaAI/inbound/`. Reports one summary line per file, then a totals footer.

## Acceptance criteria

- [ ] Single-file mode rejects a frontmatter that contains `commit_hash` (forbidden field) with exit-1 and prints `[forbidden-fields]`
- [ ] Single-file mode rejects an ADR file numbered `ADR-005` when atomic index has `ADR-001..ADR-007` already (must be `ADR-008`) with `[adr-monotonic]`
- [ ] Single-file mode rejects an atom whose body contains `[[FEAT--ghost]]` not in the atomic index with `[dangling-wikilink]`
- [ ] Single-file mode rejects an atom whose `crosslinks.references` entry doesn't resolve with `[dangling-wikilink]`
- [ ] Single-file mode rejects an atom with `created_at: 2099-01-01T00:00:00Z` with `[future-date]`
- [ ] Single-file mode rejects an atom with `id: foo-bar` (wrong format) with `[id-format]`
- [ ] Single-file mode rejects an atom whose `id` doesn't match the filename basename with `[id-filename-match]`
- [ ] Single-file mode rejects a `summary` field of length < 10 or containing `TBD` with `[summary-min]`
- [ ] Whole-tree mode prints one line per atom + a `Total: X passed, Y failed` footer
- [ ] Exit-2 returned when atomic index is missing or unreadable
- [ ] `--json` flag emits machine-readable `{ file, errors: [{rule, line, message}] }[]`
- [ ] No false positives on the four atoms in `gks/concept/`, `gks/adr/`, `gks/feat/`, `gks/blueprint/` after they're promoted (dogfood test)

## Surfaces

| Surface | Form |
|---|---|
| TS API | `validate(filepath, atomicIndex): ValidationResult` |
| CLI | `msp-validate <file>` / `msp-validate --all` / `msp-validate --json` |
| npm script | `npm run msp:validate -- <file>` |
| Pre-commit hook | wrapper script under `examples/hooks/pre-commit-validator.sh` (out of scope for M2) |

## Out of scope (this FEAT)

- Auto-fixing violations
- LLM-assisted suggestions for rejections
- Loading the rule set from `atomic_contract.yaml` at runtime — for M2, rules are TypeScript constants
- Vector / semantic similarity checks
