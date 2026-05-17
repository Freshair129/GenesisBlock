---
id: CONCEPT--CONFIG-AS-SSOT
phase: 1
type: concept
status: draft
tier: process
source_type: axiomatic
vault_id: default
title: Config as Single Source of Truth (extend atom-registry pattern to all
  behavior-driving values)
aliases: &a1
  - CONCEPT
  - implementation_flow
  - Strategic intent / PRD
cluster: implementation_flow
role: Strategic intent / PRD
tags: &a2
  - msp
  - config
  - ssot
  - governance
crosslinks: &a3
  references:
    - CONCEPT--ATOM-REGISTRY-AS-SSOT
    - ADR--REGISTRY-DRIVEN-SCAFFOLDING
created_at: 2026-05-17T16:10:00.000+07:00
attributes:
  id: CONCEPT--CONFIG-AS-SSOT
  phase: 1
  type: concept
  status: draft
  tier: process
  source_type: axiomatic
  vault_id: default
  title: Config as Single Source of Truth (extend atom-registry pattern to all
    behavior-driving values)
  aliases: *a1
  cluster: implementation_flow
  role: Strategic intent / PRD
  tags: *a2
  crosslinks: *a3
  created_at: 2026-05-17T16:10:00.000+07:00
  domain: concept
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: true
  secret_type: high_entropy_string
  leak_risk: high
  encryption_level: none
---

# CONCEPT — Config as Single Source of Truth

## Problem

The atom-registry pattern (CONCEPT--ATOM-REGISTRY-AS-SSOT) proved that externalizing taxonomy into YAML kills drift and saves tokens. But that pattern was only applied to **atom taxonomy**. The rest of the codebase still has ~150 hardcoded behavior-driving values across 30+ files:

- Validator thresholds (`TOKEN_RATIO=1.3`, `WARN=400`, `ERROR=600` in `master-token-cap.ts`)
- Codegen forbidden patterns (6 regex rules + 15 forbidden imports in `forbidden-patterns.ts`)
- Retrieval RRF weights and per-source timeouts (`orchestrator/retrieval/types.ts`)
- Memory summarizer regex and length caps (`episodic/summarisers/heuristic.ts`)
- MCP tool registration (28 tools statically imported in `mcp/server.ts`)
- Hook scope patterns (shell regex in `examples/hooks/pre-commit-validator.sh`)

Each creates a drift surface: changing behavior requires code edit → recompile → test → deploy. Multi-agent editing (Claude, Gemini, Qwen, Antigravity — see AGENT.md) increases drift risk; concurrent `.ts` edits cause merge conflicts and incident reports (`INCIDENT_REPORT--ANTIGRAVITY-*`).

## Hypothesis

The atom-registry SSOT pattern can be generalized to ALL behavior-driving config: thresholds, mappings, regex patterns, manifest lists, policy decisions. Result:

- **Operator tunability** — change behavior without TS knowledge
- **Audit trail** — `git diff config/*.yaml` shows all behavior changes in one diff
- **Lower merge conflict** — agent A edits `validator.yaml`, agent B edits `memory.yaml`, no overlap
- **Self-documenting** — YAML carries `description:` alongside values
- **Cache-safe** — proven loader pattern from `packages/msp/src/validator/utils/registry.ts`

## Scope

- Externalize ~150 hardcoded values across validator, codegen, retrieval, memory, hooks, mcp, paths, embedding, database, identity modules.
- Establish shared config loader in `packages/msp/src/config/loader.ts` (generalize the existing `registry.ts` pattern).
- Adopt the 2-layer file split decided in ADR--CONFIG-TWO-LAYER-SPLIT.
- Enforce JSON Schema validation via `npm run config:validate`.

## Out of scope

- One-shot migration scripts (`scripts/msp/migrate-*.mjs`) — leave hardcoded; they run once.
- Protocol constants (frontmatter delimiter `---`, YAML separator) — these define interop contracts, not behavior.
- Web UI config (`apps/web/`) — separate ultraplan; Vite uses code-as-config natively.
- Per-environment overrides (dev/staging/prod) — out of MVP; handled later via env-var interpolation if needed.

## Verification

- An operator changes `config/validator.yaml` → `master_tier.thresholds.warn` from `400` → `350`, restarts the MSP service, new threshold takes effect with **zero code changes**.
- `npm test --workspace=packages/msp` baseline passes after refactor (same green count as before).
- `npm run config:validate` lints every YAML against its JSON Schema; CI fails on schema violations.
- Removing a hardcoded value without adding it to YAML produces a clear loader error at boot, not a silent default.

## Source

- ADR--CONFIG-TWO-LAYER-SPLIT (the layout decision)
- BLUEPRINT--CONFIG-EXTERNALIZATION (execution plan)
- Industry precedent: 12-factor app config principle, Kubernetes Helm `values.yaml`, Cargo workspace manifests
