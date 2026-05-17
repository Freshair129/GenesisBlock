---
id: FEAT--CLASSIFIER-PLUGINS
phase: 2
type: feat
status: active
tier: process
source_type: axiomatic
vault_id: default
title: FEAT — Classifier plugins for automatic attribute tagging
tags:
  - msp
  - ucf
  - classifier
  - tagging
  - attributes
crosslinks:
  implements:
    - CONCEPT--ATTRIBUTE-BAG-MODEL
  references:
    - FRAMEWORK--UNIVERSAL-CONTEXT-FRAMEWORK
created_at: 2026-05-17T08:50:00+07:00
cluster: implementation_flow
role: "Feature spec"
aliases:
  - FEAT
  - implementation_flow
attributes:
  domain: feat
---

# FEAT — Classifier Plugins

## Context

UCF Attribute-Based Access Control (ABAC) relies on resources carrying metadata (Attribute Bags). Manual tagging is error-prone and doesn't scale. Classifier plugins automate this by deriving attributes from file paths, content patterns, or graph topology.

## Requirements

1. **Standard Interface:** All classifiers must implement a common async `classify(resource): Promise<AttributeBag>` method.
2. **Provenance Tracking:** Every derived attribute should ideally carry source metadata (which classifier, version, confidence).
3. **Pluggable Architecture:** The system must allow adding new domain-specific classifiers without core modifications.
4. **Precedence Rules:** Human-authored tags (frontmatter) must always override auto-generated ones.
5. **Efficiency:** Batch classification must be performant enough to run during re-indexing.

## API Contract

```ts
interface Classifier {
  id: string
  /** Attribute names this classifier is capable of producing. */
  outputs: string[]
  /**
   * Derive attributes for a resource.
   * Returns a bag where values are JSON-serializable.
   */
  classify(resource: { id: string, path: string, body: string, attributes?: Record<string, any> }): Promise<Record<string, any>>
}
```

## Built-in Classifiers

- **PathClassifier:** Tags `domain` based on directory structure (e.g. `gks/adr/` -> `domain: adr`).
- **ContentClassifier:** Regex-based tagging for sensitive patterns (SSN, Email, Keys).
- **FrontmatterClassifier:** Passthrough for explicit `attributes:` in markdown.

## Verification Criteria

- A new classifier can be registered and invoked by the tagging engine.
- Conflict resolution correctly prioritizes frontmatter over auto-tags.
- Provenance metadata correctly identifies the source of each tag.
