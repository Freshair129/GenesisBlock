---
id: BLUEPRINT--PHASE-6-CLASSIFIERS
phase: 3
type: blueprint
status: draft
tier: process
source_type: axiomatic
vault_id: default
title: "BLUEPRINT — Phase 6: Classifier plugins + auto-tagging"
tags:
  - msp
  - ucf
  - classifier
  - blueprint
crosslinks:
  implements:
    - FEAT--CLASSIFIER-PLUGINS
  references:
    - CONCEPT--ATTRIBUTE-BAG-MODEL
created_at: 2026-05-17T08:52:00+07:00
cluster: implementation_flow
role: "Implementation plan"
linked_symbols:
  - { file: "packages/msp/src/policy/classifiers/types.ts" }
  - { file: "packages/msp/src/policy/classifiers/engine.ts" }
  - { file: "packages/msp/src/policy/classifiers/path.ts" }
  - { file: "packages/msp/src/policy/classifiers/content.ts" }
  - { file: "packages/msp/src/policy/classifiers/cli.ts" }
aliases:
  - BLUEPRINT
  - implementation_flow
attributes:
  domain: blueprint
---

# BLUEPRINT — Phase 6: Classifiers

## Geography

```
packages/msp/src/
└── policy/
    └── classifiers/
        ├── types.ts          # Classifier interface (T6.1)
        ├── engine.ts         # runClassifiers() orchestration (T6.2)
        ├── frontmatter.ts    # FrontmatterClassifier (T6.3)
        ├── path.ts           # PathClassifier (T6.4)
        └── content.ts        # ContentClassifier (T6.5)
```

## Tasks

### T6.1 — Classifier Interface
- Define `Classifier` interface in `policy/classifiers/types.ts`.
- Add `Provenance` metadata types.

### T6.2 — Classification Engine
- Implement `runClassifiers(resource, classifiers[])` in `policy/classifiers/engine.ts`.
- Implement precedence rules: Frontmatter > Domain Pack > Universal.
- Implement attribute merging and provenance tracking.

### T6.3 — Frontmatter Classifier
- Create `FrontmatterClassifier` that simply returns `resource.attributes` or parses the body for FM.

### T6.4 — Path Classifier
- Create `PathClassifier` that maps directory segments to `domain` attributes.
- Support configurable prefix mapping (e.g. `gks/adr` -> `domain: adr`).

### T6.5 — Content Classifier
- Create `ContentClassifier` using Regex sets for PII detection (SSN, Email).
- Add built-in patterns for common secrets (API keys).

### T6.6 — CLI Integration
- Create `msp-tag` CLI (or add to `msp-policy`) to run auto-tagging over the whole vault.
- Support `--dry-run` to see what would be changed.

### T6.7 — Acceptance Harness
- Test case: verify an atom in `gks/adr/` is automatically tagged with `domain: adr`.
- Test case: verify an atom with an SSN in the body is tagged with `pii: true`.
- Test case: verify manual frontmatter overrides auto-tags.

## Connections
- `[[FEAT--CLASSIFIER-PLUGINS]]` — the contract implemented.
- `[[CONCEPT--ATTRIBUTE-BAG-MODEL]]` — the governing concept.
- `[[BLUEPRINT--PHASE-4-USER-ABAC]]` — predecessor phase.
