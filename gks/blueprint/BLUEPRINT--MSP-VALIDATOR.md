---
id: BLUEPRINT--MSP-VALIDATOR
phase: 3
type: blueprint
status: stable
vault_id: default
title: BLUEPRINT — MSP validator implementation plan
tags:
  - msp
  - validator
  - blueprint
  - implementation
crosslinks: {"implements":["FEAT--MSP-VALIDATOR"],"references":["ADR--MSP-VALIDATOR","CONCEPT--MSP-VALIDATOR"]}
linked_symbols:
  - {"file":"src/validator/index.ts"}
  - {"file":"src/validator/types.ts"}
  - {"file":"src/validator/parse.ts"}
  - {"file":"src/validator/rules/forbidden-fields.ts"}
  - {"file":"src/validator/rules/dangling-wikilinks.ts"}
  - {"file":"src/validator/rules/id-uniqueness.ts"}
  - {"file":"src/validator/rules/id-format.ts"}
  - {"file":"src/validator/rules/future-date.ts"}
  - {"file":"src/validator/rules/summary-min.ts"}
  - {"file":"src/validator/atomic-index.ts"}
  - {"file":"src/validator/cli.ts"}
created_at: 2026-05-03T06:24:25.656Z
---

# BLUEPRINT — MSP validator implementation plan

```yaml
metadata:
  title: "MSP validator pipeline"
  parent_feat: FEAT--MSP-VALIDATOR
  parent_adr: ADR--MSP-VALIDATOR

architectural_pattern: |
  Pipeline of pure functions over a parsed-atom value object.
  Each rule is a (atom, ctx) → ValidationError[] function with no side effects.
  The runner composes rules + accumulates errors + decides exit code.
  CLI is a thin wrapper that loads the atomic index, walks files, calls the
  runner, and prints results.

data_logic: |
  Input:
    - filepath: string (absolute)
    - atomicIndex: Map<id, AtomicIndexEntry> (loaded once per CLI invocation)
  Pipeline:
    1. parse(filepath) → { fm: object, body: string, source: string }
    2. for each rule in HARD_RULES:
         errors.push(...rule(parsed, ctx))
    3. for each rule in SOFT_RULES:
         warnings.push(...rule(parsed, ctx))
    4. if errors.length > 0 → exit 1
       else if warnings.length > 0 → print + exit 0
       else → exit 0

geography:
  # Core
  - "src/validator/index.ts"           # public TS API: validate(filepath, ctx)
  - "src/validator/types.ts"           # ValidationError, AtomicIndexEntry, Severity
  - "src/validator/parse.ts"           # parseFrontmatter, extractWikilinks
  - "src/validator/atomic-index.ts"    # loadAtomicIndex(path): Map<id, entry>
  # Rules
  - "src/validator/rules/forbidden-fields.ts"
  - "src/validator/rules/id-format.ts"
  - "src/validator/rules/id-filename-match.ts"
  - "src/validator/rules/adr-monotonic.ts"
  - "src/validator/rules/dangling-wikilinks.ts"
  - "src/validator/rules/future-date.ts"
  - "src/validator/rules/summary-min.ts"
  - "src/validator/rules/phase-status.ts"
  # Entrypoint
  - "src/validator/cli.ts"             # bin entrypoint
  # Tests
  - "test/validator/forbidden-fields.test.ts"
  - "test/validator/dangling-wikilinks.test.ts"
  - "test/validator/adr-monotonic.test.ts"
  - "test/validator/id-format.test.ts"
  - "test/validator/cli.test.ts"
  - "test/fixtures/"                   # sample valid/invalid atom files

api_contracts:
  - name: validate
    signature: |
      function validate(
        filepath: string,
        ctx: ValidationContext,
      ): Promise<ValidationResult>
    types: |
      interface ValidationContext {
        atomicIndex: Map<string, AtomicIndexEntry>
        forbiddenFields?: ReadonlySet<string>  // override default for testing
        now?: Date                              // injectable clock
      }
      interface ValidationResult {
        filepath: string
        errors: ValidationError[]
        warnings: ValidationError[]
      }
      interface ValidationError {
        rule: string                  // 'forbidden-fields', 'dangling-wikilink', ...
        severity: 'error' | 'warning'
        line?: number
        column?: number
        message: string
        offending?: string            // value that triggered the error
      }
      interface AtomicIndexEntry {
        id: string
        type: string
        status: string
        path: string
        crosslinks?: Record<string, string[]>
      }

  - name: cli
    signature: |
      // src/validator/cli.ts → bin entry
      // msp-validate <file>     # exit 0/1/2
      // msp-validate --all      # walk gks/ + .brain/.../inbound/
      // msp-validate --json     # JSON output
    exit_codes: |
      0 = pass (warnings allowed)
      1 = hard-rule violations
      2 = internal error (missing index, malformed YAML, unreadable file)

verification_plan:
  - vitest unit per rule with fixtures (≥ 3 cases each: pass / hard fail / edge)
  - integration test: full validate() on the 4 promoted atoms (CONCEPT--MSP-VALIDATOR, ADR--MSP-VALIDATOR, FEAT--MSP-VALIDATOR, BLUEPRINT--MSP-VALIDATOR) → must pass
  - integration test: CLI invocation with --json on an intentionally-broken fixture → matches expected JSON shape
  - dogfood: `npm run msp:validate -- --all` after M2 ships → exit 0
```

## Implementation order (TASK chain)

T1 PARSER-FRONTMATTER
T2 ATOMIC-INDEX-LOADER
T3 RULE-FORBIDDEN-FIELDS
T4 RULE-ID-FORMAT
T5 RULE-DANGLING-WIKILINKS
T6 RULE-ADR-MONOTONIC
T7 RULE-FUTURE-DATE + RULE-SUMMARY-MIN + RULE-PHASE-STATUS
T8 CLI-RUNNER (composes rules, prints, exits)
T9 INTEGRATION-TEST + DOGFOOD

## Failure modes to handle gracefully

| Symptom | Detection | Action |
|---|---|---|
| `gks/00_index/atomic_index.jsonl` missing | `loadAtomicIndex` ENOENT | exit 2, print suggestion `npm run msp:index` |
| Frontmatter not valid YAML | parser throws | exit 2, point at line of YAML error |
| Filename has spaces / non-ASCII | regex check on basename | exit 1 with `[id-format]` |
| Empty body | parser returns `body === ''` | warn (not error — some atoms are pure metadata) |
| Body has `[[X]]` inside fenced code block | wikilink extractor skips lines between ` ``` ` and ` ``` ` | not flagged as dangling |
```
