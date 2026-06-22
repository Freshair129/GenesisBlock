---
title: {{TITLE}}
summary: {{SUMMARY}}
doc_id: {{DOC_ID}}
created: "{{TIMESTAMP}},{{USER}}"
updated: "{{TIMESTAMP}},{{USER}}"
version: "1.0.0-beta"
status: active
state: active
type: genesis
vault_id: default
source_type: axiomatic
tags:
  - genesis
  - cognitive-engine
  - manifest
block_manifest:
  core:
    module:
      id: [[MOD::{{MODULE_ID}}]]
      version: "1.0"
      masterplan: "{{MP_ID}}"
      roadmap: "{{RM_ID}}"
      phase: "{{PH_ID}}"
      epic: "{{EP_ID}}"
      sprint: "{{SP_ID}}"
      task: "{{TASK_ID}}"
      context_scaling_tier: "H3"
      cluster: "{{CLUSTER}}"
      domain: "{{DOMAIN}}"
      layer: "Module"
      role: "orchestrator"
      status: "ACTIVE"

---

# {{MOD_NAME}} [L1-Module] {{MODULE_ID}}

## DESCRIPTION
{{MOD_DESCRIPTION}}

## EXECUTION FLOW
```mermaid
graph TD
    A[Trigger] --> B[Process]
    B --> C[Outcome]
```

## ATOMS

### FEAT: {{FEAT_NAME}} [L2-Feature] {{FEAT_ID}}
- **Metadata:**
  ```yaml
  id: [[FEAT::{{FEAT_ID}}]]
  context_scaling_tier: "H2"
  role: "worker"
  ```
- **Logic:**
  {{FEAT_LOGIC}}

### ALGO: {{ALGO_NAME}} [L3-Logic] {{ALGO_ID}}
- **Metadata:**
  ```yaml
  id: [[ALGO::{{ALGO_ID}}]]
  context_scaling_tier: "H1"
  role: "calculator"
  ```
- **Logic:**
  {{ALGO_LOGIC}}

### ENTITY: {{ENTITY_NAME}} [L3-Storage] {{ENTITY_ID}}
- **Metadata:**
  ```yaml
  id: [[ENTITY::{{ENTITY_ID}}]]
  context_scaling_tier: "H2"
  role: "repository"
  ```
- **Schema:**
  {{ENTITY_SCHEMA}}

## Changelog
| Version | Date | Summary |
|---|---|---|
| 0.1.0 | 2026-06-15 | Initial template scaffold aligned with document versioning governance. |

