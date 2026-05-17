---
id: AUDIT--PHASE-B-IMPL-COMPLETE
phase: 6
type: audit
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: AUDIT — Phase B impl shipped (global vs workspace storage + 3 new MCP tools)
tags: &a1
  - msp
  - storage
  - global
  - workspace
  - migration
  - audit
  - phase-6
crosslinks: &a2
  references:
    - BLUEPRINT--GLOBAL-VS-WORKSPACE-MIGRATION
    - ADR--GLOBAL-VS-WORKSPACE
    - CONCEPT--AGENT-AGNOSTIC
    - CONCEPT--NAMED-PROJECT-REGISTRY
    - ADR--PATH-ENCODING
created_at: 2026-05-10T07:00:00.000+07:00
aliases: &a3
  - AUDIT
  - implementation_flow
  - Test results / quality report
cluster: implementation_flow
role: Test results / quality report
attributes:
  id: AUDIT--PHASE-B-IMPL-COMPLETE
  phase: 6
  type: audit
  status: stable
  vault_id: default
  tier: process
  source_type: axiomatic
  title: AUDIT — Phase B impl shipped (global vs workspace storage + 3 new MCP tools)
  tags: *a1
  crosslinks: *a2
  created_at: 2026-05-10T07:00:00.000+07:00
  aliases: *a3
  cluster: implementation_flow
  role: Test results / quality report
  attributes:
    id: AUDIT--PHASE-B-IMPL-COMPLETE
    phase: 6
    type: audit
    status: stable
    vault_id: default
    tier: process
    source_type: axiomatic
    title: AUDIT — Phase B impl shipped (global vs workspace storage + 3 new MCP
      tools)
    tags: *a1
    crosslinks: *a2
    created_at: 2026-05-10T07:00:00.000+07:00
    aliases: *a3
    cluster: implementation_flow
    role: Test results / quality report
    attributes:
      domain: audit
    domain: audit
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: true
    secret_type: high_entropy_string
    leak_risk: high
    encryption_level: none
  domain: audit
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: true
  secret_type: high_entropy_string
  leak_risk: high
  encryption_level: none
---

# AUDIT — Phase B impl shipped

## Summary

Phase B of the architecture-doc cleanup landed on 2026-05-10. The TypeScript implementation now matches `[[ADR--GLOBAL-VS-WORKSPACE]]`: identity, preferences, projects registry, and cross-project audit live globally under `~/.msp/` (or `MSP_HOME`); sessions / episodic / candidates / vector remain in the workspace under `./.brain/msp/projects/<namespace>/`.

A one-time, idempotent migration moves any existing workspace `identity.json` to the global location on first read; the workspace file is preserved (per ADR), so rollback is a matter of deleting `~/.msp/identity.json`.

## What shipped

### New source files (8)

| File | Purpose |
|---|---|
| `src/lib/msp-home.ts` | `mspHome()`, `globalIdentityPath()`, `globalPreferencesPath()`, `projectsRegistryPath()`, `globalAuditDir()` — honours `MSP_HOME` |
| `src/projects/types.ts` | `ProjectEntry`, `ProjectsRegistry`, `defaultRegistry()` |
| `src/projects/registry.ts` | `readRegistry()`, `writeRegistry()`, `registerProject()` (YAML + atomic write) |
| `src/projects/resolve.ts` | `resolveProject()` 4-step chain (CLI → env → `.mspconfig` walk → default) |
| `src/identity/migrate.ts` | `migrateIfNeeded()` — workspace→global, idempotent, opt-out via `MSP_DISABLE_MIGRATION=1` |
| `src/mcp/tools/project-list.ts` | `msp_project_list` MCP tool |
| `src/mcp/tools/project-register.ts` | `msp_project_register` MCP tool |
| `src/mcp/tools/project-resolve.ts` | `msp_project_resolve` MCP tool |

### Modified source files (4)

| File | Change |
|---|---|
| `src/identity/types.ts` | `IdentityOptions` gains optional `view: 'merged' \| 'global' \| 'project'` |
| `src/identity/store.ts` | New `IdentityScope` write target; `readIdentity` now layers global + project override; migration runs on every read until both layers agree |
| `src/identity/index.ts` | Re-exports `migrateIfNeeded`, `projectOverridePath`, `IdentityScope`, `ReadIdentityOptions` |
| `src/mcp/server.ts` | Registers 3 new tools; `REGISTERED_TOOL_NAMES` count goes 16 → 19 |
| `src/mcp/tools/identity-get.ts` | New `view` + `explain` args |
| `src/mcp/tools/identity-set.ts` | New `scope` arg (`global` default for profile/voice; `preference` writes always go global; `project` is allowed for profile/voice only) |

### Tests (3 new files + 5 updated)

New:
- `test/identity/migrate.test.ts` (9 tests) — migrate on/off, idempotent, schemaVersion guard, corrupt JSON guard, merged-read with override
- `test/projects/registry.test.ts` (12 tests) — read/write round-trip, schema rejection, registerProject duplicate guard
- `test/projects/resolve.test.ts` (12 tests) — full priority chain + `.mspconfig` walk + error-on-unknown

Updated to use `MSP_HOME` per-test isolation (legacy workspace identity tests now exercise the global path):
- `test/identity/store.test.ts`, `profile.test.ts`, `voice.test.ts`, `preferences.test.ts`, `index.test.ts`
- `test/mcp/tools/identity-get.test.ts`, `identity-set.test.ts`
- `test/mcp/server.test.ts`, `bin.test.ts`, `tools/candidate.test.ts` (tool count assertions 16 → 19)

### Documentation atoms

- `gks/audit/[[AUDIT--PHASE-B-IMPL-COMPLETE]].md` (this atom)
- `gks/blueprint/[[BLUEPRINT--GLOBAL-VS-WORKSPACE-MIGRATION]].md` flipped from `draft` → `stable`
- `msp_spec.md` 2.0.3 changelog entry

## MCP tool surface — 16 → 19

| Tool | Status |
|---|---|
| `msp_identity_get` | extended: `view`, `explain` args |
| `msp_identity_set` | extended: `scope` arg (default `global`) |
| `msp_project_list` | **new** |
| `msp_project_register` | **new** |
| `msp_project_resolve` | **new** |

## Migration shape (the riskiest part)

```
~/.msp/identity.json missing AND ./.brain/msp/projects/<ns>/identity.json present
  → atomic copy (mkdir + tmp + rename) → ~/.msp/identity.json
  → stderr deprecation warning
  → workspace file preserved
```

After the copy, every subsequent `readIdentity` fast-paths on `stat` of the global file. The legacy file lives on as historical evidence; deleting it is a future major-version concern.

`MSP_DISABLE_MIGRATION=1` is the documented escape hatch (used by tests to assert pre-migration behaviour).

## Verification matrix

| Check | Result |
|---|---|
| `npm run typecheck` | clean |
| `npm test` | 663 passed, 1 failed (`test/mcp/bin.test.ts > uses --root=<path> argv flag when cwd is unrelated` — documented flake on slow machines, pre-existing on `main`) |
| `npm run msp:check-links` | OK (199 atoms) |
| `npx tsx src/validator/cli.ts --all` | only PROTO `PHASE-GATES` and `SCALING-LEVEL-GATE` failures, both pre-existing |
| `npx vitest run test/identity test/projects test/mcp/server.test.ts` | 113/113 |

## Deltas from BLUEPRINT

| Topic | BLUEPRINT said | Reality | Why |
|---|---|---|---|
| `kind=preference` with `scope=project` | implied possible | rejected with explicit error | Preferences are stored in the same `Identity` shape; sparse-merging individual entries with TTL would require a deeper merge strategy. Deferred until a use case appears (clinic-vs-self preferences haven't been requested). The tool returns a clear error rather than silently routing global. |
| Tests for namespace isolation | "all existing identity tests pass" | rewrote `test/identity/index.test.ts` "multi-namespace isolation" → "per-project overrides" | The original test asserted that namespace A's identity didn't leak into namespace B — but in the new global model, identity IS shared; sub-namespace divergence is now the override's job. The replacement test exercises that correctly. |
| Schema validation for `identity.override.json` | reuse same `schemaVersion` check | sparse override files do NOT carry `schemaVersion` | Per `writeIdentity` design — overrides are deliberately sparse so a stripped `schemaVersion` keeps the file genuinely small ("only what the user changed"). The merge layer reasserts version 1 on read. |
| Migration error handling | "log warning to stderr" | also: silently no-op on corrupt JSON / future schemaVersion | Better to leave the user in a state they understand (no migration, they see defaults until they fix the source) than to copy a corrupt file to the global location. |
| Default `scope` for `msp_identity_set` | "error if missing for ambiguous keys" | default `'global'` for all kinds | After implementation it became clear that `profile` and `voice` ambiguity is rare in practice — global IS the most common write target. The only escape is `scope: 'project'`, which the tool accepts explicitly. Errors only trigger when caller passes `scope: 'project'` with `kind: 'preference'` (unsupported). |

## Out of scope (deferred to follow-up)

- `~/.msp/preferences.json` — stub paths exist via `globalPreferencesPath()`, but no read/write API yet. Today, all preferences live in `~/.msp/identity.json`'s `preferences` field (matches pre-migration behaviour). Splitting into a separate file is a future PR; the layout already accommodates it.
- Cross-project audit log under `~/.msp/audit/<date>.jsonl` — `globalAuditDir()` exists but no writer hooks yet. Will land alongside cross-project recall (`msp_recall { cross_project: true }`).
- `MSP_PROJECT` env auto-pickup in MCP tools — `msp_project_resolve` reads it on demand, but other tools (`msp_recall`, `msp_remember`, etc.) still take an explicit `namespace` arg. Threading the resolver through every tool is a separate UX concern.

## Source

`[[BLUEPRINT--GLOBAL-VS-WORKSPACE-MIGRATION]]` (Phase B impl, 2026-05-10). Atoms PR was `claude/architecture-ssot-selection-D76og` (2026-05-09); this impl PR is `claude/msp-global-workspace-impl-D76og`.

## Connections
- [[CONCEPT--AGENT-AGNOSTIC]]
- [[CONCEPT--NAMED-PROJECT-REGISTRY]]
- [[ADR--PATH-ENCODING]]

