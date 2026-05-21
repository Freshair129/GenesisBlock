---
id: BLUEPRINT--CONFIG-EXTERNALIZATION
phase: 3
type: blueprint
status: draft
tier: process
source_type: axiomatic
vault_id: default
title: ULTRAPLAN — Config externalization to YAML (script behavior driven by
  config, not code)
aliases: &a1
  - BLUEPRINT
  - implementation_flow
  - Implementation plan
cluster: implementation_flow
role: Implementation plan
tags: &a2
  - msp
  - config
  - yaml
  - refactor
  - governance
crosslinks: &a3
  references:
    - CONCEPT--CONFIG-AS-SSOT
    - ADR--CONFIG-TWO-LAYER-SPLIT
    - CONCEPT--ATOM-REGISTRY-AS-SSOT
    - ADR--REGISTRY-DRIVEN-SCAFFOLDING
  parent_blueprint:
    - ADR--CONFIG-TWO-LAYER-SPLIT
linked_symbols: &a4
  - file: packages/msp/src/config/loader.ts
  - file: packages/msp/src/validator/proto/master-token-cap.ts
  - file: packages/msp/src/validator/proto/master-body-schema.ts
  - file: packages/msp/src/validator/proto/phase-gates.ts
  - file: packages/msp/src/validator/rules/summary-min.ts
  - file: packages/msp/src/codegen/forbidden-patterns.ts
  - file: packages/msp/src/codegen/post-process.ts
  - file: packages/msp/src/orchestrator/retrieval/types.ts
  - file: packages/msp/src/memory/sessions/lock.ts
  - file: packages/msp/src/memory/episodic/writer.ts
  - file: packages/msp/src/memory/episodic/summarisers/heuristic.ts
  - file: packages/msp/src/memory/backlinks/walk.ts
  - file: packages/msp/src/mcp/server.ts
  - file: packages/msp/src/identity/profile.ts
  - file: packages/msp/src/identity/voice.ts
  - file: packages/msp/examples/hooks/pre-commit-validator.sh
  - file: scripts/msp/re-embed.ts
  - file: scripts/msp/pg-migrate.ts
created_at: 2026-05-17T16:20:00.000+07:00
attributes:
  id: BLUEPRINT--CONFIG-EXTERNALIZATION
  phase: 3
  type: blueprint
  status: draft
  tier: process
  source_type: axiomatic
  vault_id: default
  title: ULTRAPLAN — Config externalization to YAML (script behavior driven by
    config, not code)
  aliases: *a1
  cluster: implementation_flow
  role: Implementation plan
  tags: *a2
  crosslinks: *a3
  linked_symbols: *a4
  created_at: 2026-05-17T16:20:00.000+07:00
  attributes:
    id: BLUEPRINT--CONFIG-EXTERNALIZATION
    phase: 3
    type: blueprint
    status: draft
    tier: process
    source_type: axiomatic
    vault_id: default
    title: ULTRAPLAN — Config externalization to YAML (script behavior driven by
      config, not code)
    aliases: *a1
    cluster: implementation_flow
    role: Implementation plan
    tags: *a2
    crosslinks: *a3
    linked_symbols: *a4
    created_at: 2026-05-17T16:20:00.000+07:00
    attributes:
      id: BLUEPRINT--CONFIG-EXTERNALIZATION
      phase: 3
      type: blueprint
      status: draft
      tier: process
      source_type: axiomatic
      vault_id: default
      title: ULTRAPLAN — Config externalization to YAML (script behavior driven by
        config, not code)
      aliases: *a1
      cluster: implementation_flow
      role: Implementation plan
      tags: *a2
      crosslinks: *a3
      created_at: 2026-05-17T16:00:00.000+07:00
      domain: blueprint
      language: markdown
      is_test: false
      is_entrypoint: false
      has_secret: true
      secret_type: high_entropy_string
      leak_risk: high
      encryption_level: none
    domain: blueprint
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: true
    secret_type: aws_secret
    leak_risk: high
    encryption_level: none
  domain: blueprint
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: true
  secret_type: aws_secret
  leak_risk: high
  encryption_level: none
---

# ULTRAPLAN — Config Externalization

> **Goal:** Every script and runtime module reads behavior from YAML. Editing YAML changes behavior without code changes. Extends the pattern proven with `atom_schema.yaml` ↔ `atom_registry.yaml`.

---

## ๑. Vision

```
Before:   [hardcoded values in .ts] → recompile, retest, redeploy
After:    [config/*.yaml]           → edit, reload, done
```

**Properties we want:**

- **Single Source of Truth** — each policy/threshold/mapping lives in exactly one place
- **Auditable** — changes show up in `git diff` of YAML
- **Hot-tunable** — operators can tune without TypeScript knowledge
- **Self-documenting** — YAML carries description/why fields alongside values
- **Cache-safe** — shared loader pattern (module-level cache + invalidation)

---

## ๒. Current State Audit

**Scanned:** `scripts/msp/`, `packages/msp/src/{validator,codegen,memory,orchestrator,identity,mcp,policy}/`, `packages/msp/examples/hooks/`, `policies/`.

**Findings:** ~150 hardcoded values across 30+ files that drive behavior. Grouped by cohesion:

| Cohesion group | Hardcodes | Example |
|---|---|---|
| Validator rules | ~20 | `TOKEN_RATIO=1.3`, `WARN=400`, `ERROR=600` in `master-token-cap.ts` |
| Codegen policies | ~25 | 6 forbidden patterns + 15 forbidden imports in `forbidden-patterns.ts` |
| Retrieval fusion | ~12 | RRF weights, per-source timeouts in `orchestrator/retrieval/types.ts` |
| Memory thresholds | ~15 | Decision regex, summary length caps in `episodic/summarisers/heuristic.ts` |
| Classifier patterns | ~30 | Lang extensions, entrypoint files, test paths in `policy/classifiers/coding.ts` |
| Hook scopes | ~6 | Atom path patterns in `examples/hooks/pre-commit-validator.sh` |
| MCP tool manifest | ~30 | 28 statically imported tools in `mcp/server.ts` |
| Paths / dirs | ~20 | `.brain/msp/projects/{ns}/...` across many files |
| Embedding params | ~10 | Chunk size 512, overlap 64, batch 1 in `re-embed.ts` |
| DB tuning | ~8 | Vector dim 1024, table `gks_vector` in `pg-migrate.ts` |

**Already externalized:**

- ✅ `atom_schema.yaml` — taxonomy, schema_spec, decision_rule
- ✅ `atom_registry.yaml` — atom_list
- ✅ `policies/60-coding-domain.yaml`, `policies/70-task-management.yaml` — ABAC classifiers

---

## ๓. Target File Layout (2-Layer per ADR--CONFIG-TWO-LAYER-SPLIT)

```
cognitive_system/
├── atom_schema.yaml                          ← already done
├── atom_registry.yaml                        ← already done
│
├── config/                                   ← LAYER 1: operator-facing (root)
│   ├── README.md                             ← index + ownership + load order
│   ├── validator.yaml                        ← P1 — token caps, severities, required sections
│   ├── codegen.yaml                          ← P1 — forbidden patterns, imports, post-process
│   ├── paths.yaml                            ← P3 — cross-cutting paths (read by both pkgs)
│   └── policies/                             ← existing ABAC (unchanged)
│       ├── 60-coding-domain.yaml
│       └── 70-task-management.yaml
│
├── packages/msp/config/                      ← LAYER 2: MSP package internals
│   ├── retrieval.defaults.yaml               ← P2 — RRF weights, timeouts, k constants
│   ├── memory.defaults.yaml                  ← P2 — session locks, regex, sentence caps
│   ├── mcp.tools.yaml                        ← P3 — tool manifest, server metadata
│   ├── hooks.defaults.yaml                   ← P3 — pre-commit/push scope patterns
│   └── identity.defaults.yaml                ← P4 — default persona, namespace, voice
│
└── packages/gks/config/                      ← LAYER 2: GKS package internals (travel-with-pkg)
    ├── embedding.defaults.yaml               ← P4 — chunk/batch/overlap, excludes
    └── database.defaults.yaml                ← P4 — pg dims, table names, validation
```

**Total: ~13 YAML files** (3 root + 5 MSP + 2 GKS + 2 policies + 1 cross-cutting paths) + shared loader + index doc.

### Why 2 layers (decided by ADR--CONFIG-TWO-LAYER-SPLIT)

| Layer | Audience | Change cadence | Naming |
|---|---|---|---|
| **Layer 1** `config/` | Operators, PMs, reviewers | Frequent | `<name>.yaml` |
| **Layer 2** `packages/<pkg>/config/` | Engineers | Rare | `<name>.defaults.yaml` |

**Loader resolution chain:**

1. `<repo>/config/<module>.yaml` — operator override
2. `packages/<pkg>/config/<name>.defaults.yaml` — package default
3. In-code constant — last-resort transition fallback (removed by Phase 5)

---

## ๔. Schema Sketches

### `config/validator.yaml`

```yaml
$schema_version: "1.0"
master_tier:
  token_ratio: 1.3              # word→token multiplier
  thresholds:
    warn: 400
    error: 600
  required_sections:
    - "## Intent"
    - "## Why"
    - "## Directives"
    - "## Apply when"
    - "## Conflicts with"

summary_validation:
  min_length: 10
  max_length: 300
  forbidden_placeholders: [TBD, TODO, FIXME, "lorem ipsum"]

phase_governance:
  phase_for_type:
    blueprint: 3
    adr: 2
    feat: 5
    audit: 6
  atom_prefix:
    concept: "CONCEPT--"
```

### `config/codegen.yaml`

```yaml
forbidden_patterns:
  - id: export-default
    pattern: '\bexport\s+default\b'
    severity: error
    message: "App Router uses named exports"
  - id: req-body-raw
    pattern: '\breq\.body\b'
    severity: error
  # ...4 more

forbidden_imports:
  absolute: [fs, child_process, net, http]
  conditional: [joi, zod, yup, axios, lodash]

post_process:
  code_keywords: [import, export, const, function, class, let, var, async, interface, type, enum]
  fence_regex:
    open: '[`~]{3,}'
    close: '[`~]{3,}'
  terminators: ['}', ';', ')']
```

### `config/retrieval.yaml`

```yaml
defaults:
  total_timeout_ms: 1500
  top_k: 10
  rrf_k: 60
  namespace: evaAI

sources:
  gks-vector:    { weight: 1.0, timeout_ms: 800 }
  obsidian-text: { weight: 0.8, timeout_ms: 400 }
  grep:          { weight: 0.6, timeout_ms: 600 }
  episodic:      { weight: 1.2, timeout_ms: 100 }
  backlinks:     { weight: 0.5, timeout_ms: 100 }
```

### `config/memory.yaml`

```yaml
sessions:
  lock_ttl_ms: 300000           # 5 min

episodic:
  default_namespace: evaAI
  summariser_heuristic:
    decision_regex: '\b(decided|chose|will|going to|let''s|let us)\b'
    min_anchor_length: 20
    max_summary_length: 240
    sentence_min: 10
    sentence_max: 200
    max_decisions: 5

backlinks:
  walk_excludes: ['00_index', 'node_modules', '.git', 'dist']
```

### `config/mcp.yaml`

```yaml
server:
  name: msp
  version: "0.1.0"
  context: mcp-stdio

tools:
  - name: validate;          enabled: true
  - name: propose;           enabled: true
  - name: run_task;          enabled: true
  - name: session_open;      enabled: true
  - name: session_append;    enabled: true
  - name: episode_write;     enabled: true
  - name: backlinks_query;   enabled: true
  # ...21 more
```

### `config/hooks.yaml`

```yaml
pre_commit:
  validate_paths:
    - '^gks/.*\.md$'
    - '^\.brain/msp/projects/[^/]+/inbound/.*\.md$'
  hotfix_check_excludes:
    - '^(gks|\.brain|\.github|examples|scripts|test|node_modules|dist)/'

pre_push:
  verify_flow_on: ['gks/feat/.*\.md']
```

### `config/paths.yaml`

```yaml
gks_root: gks
brain_root: .brain/msp
projects_dir: .brain/msp/projects
brain_skeleton: [skills, episodic, proto, params]

outputs:
  codegen_schemas: .brain/msp/schemas
  codegen_prompts: .brain/msp/prompts

index:
  atomic: gks/00_index/atomic_index.jsonl
  backlinks_template: '.brain/msp/projects/{namespace}/vector/backlinks.jsonl'
```

### `config/embedding.yaml`

```yaml
default_store: atomic
default_source: gks
file_pattern: '\.md$'
max_tokens: 512
overlap_tokens: 64
batch_size: 1
chunk_threshold_chars: 5000
exclude_patterns:
  - 'node_modules/'
  - '\.git/'
```

### `config/database.yaml`

```yaml
postgres:
  vector_dim: 1024
  vector_dim_range: { min: 1, max: 16000 }
  tables:
    vector: gks_vector
    graph: gks_graph
  name_regex: '^[a-z_][a-z0-9_]*$'
```

### `config/identity.yaml`

```yaml
defaults:
  namespace: evaAI
  persona: assistant
  voice:
    tone: neutral
    style: concise
```

---

## ๕. Shared Loader (2-layer aware)

New module: `packages/msp/src/config/loader.ts`

```ts
// Pattern proven in validator/utils/registry.ts — generalize it for 2-layer.
export function loadConfig<T = any>(
  moduleName: string,    // e.g. 'retrieval', 'validator'
  packageName: string,   // e.g. 'msp', 'gks' — restricts Layer 2 search
  root: string
): T | null

export function clearConfigCache(): void   // for tests
```

**Resolution order** (per ADR--CONFIG-TWO-LAYER-SPLIT):

1. **Layer 1:** `<root>/config/<moduleName>.yaml` — operator override (if present)
2. **Layer 2:** `<root>/packages/<packageName>/config/<moduleName>.defaults.yaml` — package default
3. Walk up 5 levels (monorepo-friendly) from `root`
4. Return `null` if not found (caller falls back to in-code constant during transition)

**Boundary enforcement:** `packageName === 'gks'` MUST NOT find a Layer 2 file under `packages/msp/config/` — enforced by passing `packageName` explicitly (no cross-package leak).

**Caching:** per `(moduleName, packageName)` pair, module-level, cleared via `clearConfigCache()` in test setup.

---

## ๖. Phase Plan

### Phase 0 — Foundation (1 PR)

- [ ] Create directories: `config/`, `packages/msp/config/`, `packages/gks/config/`
- [ ] Create `config/README.md` (index, ownership table, load order, naming convention)
- [ ] Implement `packages/msp/src/config/loader.ts` with 2-layer resolution (`loadConfig<T>(module, key, root)`)
- [ ] Unit tests for loader: Layer 1 override, Layer 2 fallback, missing-file → null, walk-up, caching, `clearConfigCache()`
- [ ] JSON Schema convention: each YAML carries `$schema_version: "1.0"` header
- [ ] Add boundary check: GKS code may NOT load from `packages/msp/config/` (enforced by lint + reviewer)

### Phase 1 — Validator + Codegen (Layer 1 — operator-facing, 1–2 PRs)

- [ ] Create `config/validator.yaml`
- [ ] Refactor `packages/msp/src/validator/proto/master-token-cap.ts` → read thresholds from config
- [ ] Refactor `packages/msp/src/validator/proto/master-body-schema.ts` → read required sections
- [ ] Refactor `packages/msp/src/validator/proto/phase-gates.ts` → read phase mapping
- [ ] Refactor `packages/msp/src/validator/rules/summary-min.ts` → read length + placeholders
- [ ] Create `config/codegen.yaml`
- [ ] Refactor `packages/msp/src/codegen/forbidden-patterns.ts` → read patterns + imports
- [ ] Refactor `packages/msp/src/codegen/post-process.ts` → read fence regex + keywords
- [ ] Update tests; verify no behavior change

### Phase 2 — Retrieval + Memory (1 PR) — Layer 2 (MSP internal)

- [ ] Create `packages/msp/config/retrieval.defaults.yaml`
- [ ] Refactor `packages/msp/src/orchestrator/retrieval/types.ts` → `DEFAULT_WEIGHTS`, timeouts, k constants
- [ ] Create `packages/msp/config/memory.defaults.yaml`
- [ ] Refactor `packages/msp/src/memory/sessions/lock.ts` → `DEFAULT_MAX_AGE_MS`
- [ ] Refactor `packages/msp/src/memory/episodic/writer.ts` → `DEFAULT_NAMESPACE`
- [ ] Refactor `packages/msp/src/memory/episodic/summarisers/heuristic.ts` → all regex + thresholds
- [ ] Refactor `packages/msp/src/memory/backlinks/walk.ts` → exclude list

### Phase 3 — Hooks + MCP + Paths (1 PR) — mixed

- [ ] Create `packages/msp/config/hooks.defaults.yaml` (Layer 2)
- [ ] Refactor `packages/msp/examples/hooks/pre-commit-validator.sh` → read regex from YAML (via `yq` or pre-baked sourced file)
- [ ] Create `packages/msp/config/mcp.tools.yaml` (Layer 2)
- [ ] Refactor `packages/msp/src/mcp/server.ts` → register tools from YAML manifest
- [ ] Create `config/paths.yaml` (Layer 1 — cross-cutting; read by both packages)
- [ ] Refactor all `.brain/msp/...` hardcodes across `memory/`, `orchestrator/`, `codegen/`

### Phase 4 — Embedding + Database + Identity (1 PR) — Layer 2

- [ ] Create `packages/gks/config/embedding.defaults.yaml`
- [ ] Refactor `scripts/msp/re-embed.ts`
- [ ] Create `packages/gks/config/database.defaults.yaml`
- [ ] Refactor `scripts/msp/pg-migrate.ts`
- [ ] Create `packages/msp/config/identity.defaults.yaml`
- [ ] Refactor `packages/msp/src/identity/profile.ts`, `voice.ts` defaults

### Phase 5 — Hardening (1 PR)

- [ ] Add `npm run config:validate` — JSON Schema check on every YAML
- [ ] Add registry-drift-style validator: each PR must `git diff config/` if it changes related code
- [ ] Add `config/README.md` examples for each file
- [ ] Add `AUDIT--CONFIG-EXTERNALIZATION.md` after Phase 4

---

## ๗. Risks

| Risk | Mitigation |
|---|---|
| YAML parse errors break boot | Loader returns null + falls back to in-code defaults during transition |
| Tests assume hardcoded values | Refactor tests in same PR; use `clearConfigCache()` in beforeEach |
| Shell script (`pre-commit-validator.sh`) can't easily read nested YAML | Use `yq` (already common) or generate a sourced `.env`-style file at install time |
| MCP server tool changes need careful import lifecycle | Keep static imports; YAML controls only `enabled` flag, not the import set |
| Config drift between branches | Treat `config/*.yaml` like atoms — required in PR description if touched |

---

## ๘. Out of scope

- Migrating one-shot migration scripts (`migrate-*.mjs`) — those are historical; leave as-is
- Externalizing trivial constants tightly coupled to logic (line delimiters `---`, protocol constants)
- Web UI config (`apps/web/`) — separate ultraplan

---

## ๙. Master Checklist

- [ ] **Phase 0 — Foundation** (loader + dir + README)
- [ ] **Phase 1 — Validator** (validator.yaml + 4 refactors)
- [ ] **Phase 1 — Codegen** (codegen.yaml + 2 refactors)
- [ ] **Phase 2 — Retrieval** (retrieval.yaml + 1 refactor)
- [ ] **Phase 2 — Memory** (memory.yaml + 4 refactors)
- [ ] **Phase 3 — Hooks** (hooks.yaml + 1 shell refactor)
- [ ] **Phase 3 — MCP** (mcp.yaml + 1 refactor)
- [ ] **Phase 3 — Paths** (paths.yaml + cross-cutting refactor)
- [ ] **Phase 4 — Embedding** (embedding.yaml + 1 refactor)
- [ ] **Phase 4 — Database** (database.yaml + 1 refactor)
- [ ] **Phase 4 — Identity** (identity.yaml + 2 refactors)
- [ ] **Phase 5 — Hardening** (validate + drift check + audit)

**Total estimated effort:** 6 PRs over ~2 weeks of focused work.

---

## ๑๐. Acceptance criteria

- [ ] All Layer 1 files exist under `config/` (`validator.yaml`, `codegen.yaml`, `paths.yaml`)
- [ ] All Layer 2 files exist under `packages/msp/config/` and `packages/gks/config/` with `.defaults.yaml` suffix
- [ ] No hardcoded thresholds/patterns/mappings remain in the modules listed in Phase 1–4
- [ ] `npm test --workspace=packages/msp` passes (same baseline as today)
- [ ] `npm run config:validate` passes for every YAML (per JSON Schema)
- [ ] `config/README.md` documents every file + ownership + which package reads it
- [ ] **Boundary check:** GKS code does NOT import from `packages/msp/config/` — verified by lint
- [ ] **Operator scenario:** changing `config/validator.yaml` → `master_tier.thresholds.warn` from 400 → 350 takes effect on next run with **zero code changes, zero recompile**
- [ ] **Package extraction scenario:** copying `packages/gks/` to a standalone repo carries its config (`packages/gks/config/*.defaults.yaml`) with it — no broken references
