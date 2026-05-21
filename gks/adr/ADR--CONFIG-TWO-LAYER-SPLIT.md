---
id: ADR--CONFIG-TWO-LAYER-SPLIT
phase: 2
type: adr
status: draft
tier: process
source_type: axiomatic
vault_id: default
title: Config 2-Layer Split — operator-facing root + package-internal manifests
aliases: &a1
  - ADR
  - implementation_flow
  - Architecture decision record
cluster: implementation_flow
role: Architecture decision record
tags: &a2
  - msp
  - config
  - architecture
  - monorepo
crosslinks: &a3
  references:
    - CONCEPT--CONFIG-AS-SSOT
    - ADR--REGISTRY-DRIVEN-SCAFFOLDING
created_at: 2026-05-17T16:15:00.000+07:00
attributes:
  id: ADR--CONFIG-TWO-LAYER-SPLIT
  phase: 2
  type: adr
  status: draft
  tier: process
  source_type: axiomatic
  vault_id: default
  title: Config 2-Layer Split — operator-facing root + package-internal manifests
  aliases: *a1
  cluster: implementation_flow
  role: Architecture decision record
  tags: *a2
  crosslinks: *a3
  created_at: 2026-05-17T16:15:00.000+07:00
  attributes:
    id: ADR--CONFIG-TWO-LAYER-SPLIT
    phase: 2
    type: adr
    status: draft
    tier: process
    source_type: axiomatic
    vault_id: default
    title: Config 2-Layer Split — operator-facing root + package-internal manifests
    aliases: *a1
    cluster: implementation_flow
    role: Architecture decision record
    tags: *a2
    crosslinks: *a3
    created_at: 2026-05-17T16:15:00.000+07:00
    attributes:
      domain: adr
    domain: adr
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: true
    secret_type: high_entropy_string
    leak_risk: high
    encryption_level: none
  domain: adr
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: true
  secret_type: high_entropy_string
  leak_risk: high
  encryption_level: none
---

# ADR — Config 2-Layer Split (operator-facing root + package-internal manifests)

## Context

CONCEPT--CONFIG-AS-SSOT establishes that ~150 hardcoded values across the monorepo should move to YAML. The remaining architectural decision is **WHERE** those YAML files live.

We evaluated five layout options against the constraints of this repo:

| Option | Files | Industry precedent | Multi-agent merge | Package boundary | Operator discoverability |
|---|---|---|---|---|---|
| A. Single root `config.yaml` | 1 | Cargo top-level | ⚠️ all agents collide | ✗ | ⭐⭐⭐⭐ |
| B. Flat root (`config/<name>.yaml` × 10) | 10 | Rails, K8s, GitHub Actions | ✅ low collision | ⚠️ none | ⭐⭐⭐ |
| C. 1 manifest / module (`<pkg>/<pkg>.config.yaml`) | 3 | npm workspaces, Helm umbrella | ⚠️ within-module collision | ✅ | ⭐⭐ |
| D. Many files / module (`packages/<pkg>/config/*.yaml`) | 10 | Cargo workspaces, Bazel | ✅ low collision | ✅ | ⭐⭐ |
| **E. 2-layer split** (operator at root + per-package internal) | ~13 | K8s base+overlay, Helm + values, Rails + engines | ✅ low collision | ✅ | ⭐⭐⭐⭐ |

Constraints driving the decision:

1. **Multi-agent editing** (AGENT.md §4) — Claude, Gemini, Qwen, Antigravity edit concurrently. Merge conflict surface must be small.
2. **ADR--MONOREPO-STRUCTURE** — GKS MUST NOT import from MSP. Config dependency direction must respect this.
3. **GKS standalone exit** — GKS was previously a separate repo (`GksV3`) and may extract again. Its config should travel with the package.
4. **Operator discoverability** — non-engineers (PMs, reviewers, future agents) must locate tunable knobs without learning monorepo structure.
5. **Existing convention** — `policies/`, `atom_schema.yaml`, `atom_registry.yaml` already live at root. Governance lives at root in this repo.

## Decision

Adopt **Option E: 2-layer split**.

### Layer 1 — Operator-facing (root `config/`)

Values that operators, PMs, and reviewers tune frequently. Behavior-defining policy.

```
config/
├── README.md            ← index of every config file + ownership
├── validator.yaml       ← token caps, severities, required sections
├── codegen.yaml         ← forbidden patterns, forbidden imports
├── paths.yaml           ← cross-cutting paths (gks_root, brain_root, index)
└── policies/            ← ABAC classifiers (existing location, unchanged)
    ├── 60-coding-domain.yaml
    └── 70-task-management.yaml
```

### Layer 2 — Package-internal (`packages/<pkg>/config/`)

Default values, internal heuristics, regex patterns. Engineers tune rarely; travels with the package on extraction.

```
packages/msp/config/
├── retrieval.defaults.yaml   ← RRF weights, source timeouts, k constants
├── memory.defaults.yaml      ← session lock TTL, decision regex, sentence caps
├── mcp.tools.yaml            ← tool manifest (enable/disable, server metadata)
├── hooks.defaults.yaml       ← pre-commit/push scope patterns
└── identity.defaults.yaml    ← default persona, namespace, voice

packages/gks/config/
├── embedding.defaults.yaml   ← chunk sizes, batch, exclude patterns
└── database.defaults.yaml    ← pg vector dim, table names, validation ranges
```

### Loader rules

- Single shared loader: `packages/msp/src/config/loader.ts` (generalized from `validator/utils/registry.ts`).
- Resolution order for a config key `K` in module M:
  1. `<repo>/config/<M>.yaml` (operator override, if the key is exposed in Layer 1)
  2. `packages/<M>/config/<name>.defaults.yaml` (package internal default)
  3. In-code fallback constant (only as last-resort during transition; removed by end of Phase 5)
- **GKS code may only load from `packages/gks/config/` and `config/` (cross-cutting).** Never from `packages/msp/config/`. Enforced by a lint rule + PR review (consistent with ADR--MONOREPO-STRUCTURE).

### Naming convention

- Operator-facing: `<name>.yaml` (e.g. `validator.yaml`)
- Package-internal default: `<name>.defaults.yaml` (e.g. `retrieval.defaults.yaml`)
- The `.defaults` suffix is a visual signal: "you usually shouldn't touch this; expose as Layer 1 if it becomes operator-tunable"

### Promotion path (Layer 2 → Layer 1)

When a value initially internal becomes operator-tunable:

1. Add the key under appropriate section in `config/<module>.yaml`
2. Loader picks up the override automatically (no code change)
3. Mention in PR description: "Promoted `retrieval.timeout_ms_total` to operator config"

## Consequences

**Positive:**

- Operator can find tunable knobs in one directory (`config/`) without learning monorepo layout.
- Each agent edits a different file → merge conflicts rare (proven pattern from policies/).
- GKS extraction stays trivial — `packages/gks/config/` travels with the package.
- Package boundary (ADR--MONOREPO-STRUCTURE) is structurally enforced — GKS code cannot accidentally read MSP config.
- 2-tier visibility matches mental model: "things I tune" vs "things I touch when rewriting code".
- Schema validation is per-file; failures are localized.

**Negative:**

- Two locations to remember when adding new config (one decision per addition: operator-facing or internal?).
- Loader resolution chain is slightly more complex than single-source pattern.
- Cross-cutting tuning (e.g. "all timeouts across the system") requires opening multiple files. Mitigated by `config/README.md` index.
- ~13 total config files (4 root + 5 MSP + 2 GKS + 2 cross-cutting/policy) — more than Option B (10) but each file is smaller and more cohesive.

**Mitigations:**

- `config/README.md` lists every config file + ownership + which module reads it.
- Loader API encapsulates the resolution chain; consumers see `loadConfig('validator')` and don't worry about Layer 1 vs Layer 2.
- JSON Schema for each file catches typos at lint time (`npm run config:validate`).
- `registry-drift`-style validator rule: PRs that change code behavior without touching corresponding YAML get flagged.

## Alternatives considered

- **Option A (single root file):** Rejected. All agents collide on one file. No package isolation. Doesn't match repo's atomic-decomposition philosophy.
- **Option B (many flat root files):** Rejected as primary. Breaks package boundary; GKS extraction becomes a config-cleanup project; doesn't scale beyond 2 packages.
- **Option C (1 manifest / module):** Rejected. Within-package multi-agent collision (e.g. Claude tunes validator while Gemini tunes retrieval, both in `msp.config.yaml`). Loses the per-concern isolation that makes the registry pattern work.
- **Option D (many files / module):** Rejected as primary. Lacks discoverability for operators — they shouldn't need to know `packages/msp/config/validator.yaml` exists. But adopted as **Layer 2** for internal defaults — that's where it shines.

## Source

- CONCEPT--CONFIG-AS-SSOT
- ADR--REGISTRY-DRIVEN-SCAFFOLDING (atom-registry precedent)
- BLUEPRINT--CONFIG-EXTERNALIZATION (execution plan; updated to reflect this decision)
- Industry precedents: Cargo workspaces (`Cargo.toml` per crate + workspace root), Helm umbrella charts (`values.yaml` per subchart + umbrella overrides), Kubernetes base + overlay (Kustomize), Rails `config/` + per-engine `engine/config/`
