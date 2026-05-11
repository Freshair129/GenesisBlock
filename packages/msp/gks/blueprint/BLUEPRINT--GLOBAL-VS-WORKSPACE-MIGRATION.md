---
id: BLUEPRINT--GLOBAL-VS-WORKSPACE-MIGRATION
phase: 3
type: blueprint
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: BLUEPRINT — implement global vs workspace storage split (~/.msp/ + .brain/msp/projects/<ns>/)
tags:
  - msp
  - storage
  - global
  - workspace
  - migration
  - blueprint
  - implementation
crosslinks: {"references":["ADR--GLOBAL-VS-WORKSPACE","CONCEPT--AGENT-AGNOSTIC","CONCEPT--NAMED-PROJECT-REGISTRY","ADR--PATH-ENCODING"]}
linked_symbols:
  - {"file":"src/identity/store.ts"}
  - {"file":"src/identity/types.ts"}
  - {"file":"src/identity/migrate.ts"}
  - {"file":"src/identity/index.ts"}
  - {"file":"src/projects/registry.ts"}
  - {"file":"src/projects/resolve.ts"}
  - {"file":"src/projects/types.ts"}
  - {"file":"src/lib/msp-home.ts"}
  - {"file":"src/mcp/tools/identity-get.ts"}
  - {"file":"src/mcp/tools/identity-set.ts"}
  - {"file":"test/identity/store.test.ts"}
  - {"file":"test/identity/migrate.test.ts"}
  - {"file":"test/projects/registry.test.ts"}
  - {"file":"test/projects/resolve.test.ts"}
created_at: 2026-05-09T00:00:00.000Z
---

# BLUEPRINT — global vs workspace storage migration

> Implements `ADR--GLOBAL-VS-WORKSPACE`. Single-PR scope; ~3-5 days of work.

## Goal

Move identity + preferences + projects-registry to global `~/.msp/`, keep sessions/episodic/candidates/vector in workspace `./.brain/msp/projects/<ns>/`, with a one-time auto-migration from existing workspace identity files.

## Files to add

### `src/lib/msp-home.ts`

```ts
import { homedir } from 'node:os'
import { resolve } from 'node:path'

/**
 * Resolve the MSP global root, honoring MSP_HOME env override.
 * Default: <homedir>/.msp/
 */
export function mspHome(): string {
  return process.env['MSP_HOME'] ?? resolve(homedir(), '.msp')
}

export function globalIdentityPath(): string {
  return resolve(mspHome(), 'identity.json')
}

export function globalPreferencesPath(): string {
  return resolve(mspHome(), 'preferences.json')
}

export function projectsRegistryPath(): string {
  return resolve(mspHome(), 'projects.yaml')
}

export function globalAuditDir(): string {
  return resolve(mspHome(), 'audit')
}
```

### `src/projects/types.ts`

```ts
export interface ProjectEntry {
  path: string
  embedder?: string
  description?: string
}

export interface ProjectsRegistry {
  schemaVersion: 1
  projects: Record<string, ProjectEntry>
  default?: string  // name of default project; falls back to 'default' if unset
}
```

### `src/projects/registry.ts`

```ts
export async function readRegistry(): Promise<ProjectsRegistry>
export async function writeRegistry(registry: ProjectsRegistry): Promise<void>
export async function registerProject(name: string, entry: ProjectEntry): Promise<void>
```

YAML schema; reuse the `yaml` dep already in `package.json`.

### `src/projects/resolve.ts`

```ts
export interface ResolveOptions {
  cliFlag?: string       // --project=<name>
  env?: string           // process.env.MSP_PROJECT
  cwd?: string           // for .mspconfig walk
}

export interface ResolvedProject {
  name: string
  entry: ProjectEntry
  source: 'cli' | 'env' | 'mspconfig' | 'default'
}

export async function resolveProject(opts: ResolveOptions): Promise<ResolvedProject>
```

Resolution order per `CONCEPT--NAMED-PROJECT-REGISTRY`:

1. `cliFlag` if given
2. `env` if set
3. `.mspconfig` walking up from `cwd ?? process.cwd()` (single-line `project: <name>` or YAML)
4. registry's `default` field, then literal `'default'`

Errors loudly if the resolved name isn't in the registry (no silent fallback).

### `src/identity/migrate.ts`

```ts
/**
 * One-time migration: if global identity doesn't exist but a workspace
 * identity does, copy workspace → global and emit a deprecation warning.
 *
 * Idempotent. Runs on every readIdentity until both sides agree (global
 * exists). Never deletes the workspace file.
 */
export async function migrateIfNeeded(workspaceRoot: string, namespace: string): Promise<{
  migrated: boolean
  source?: string
  destination: string
}>
```

## Files to modify

### `src/identity/store.ts`

Refactor signatures:

```ts
export interface IdentityScope {
  scope: 'global' | 'project'
}

export interface ReadIdentityOptions extends IdentityOptions {
  // when 'merged' (default), reads global + project override and shallow-merges
  // when 'global', reads only ~/.msp/identity.json
  // when 'project', reads only .brain/msp/projects/<ns>/identity.override.json
  view?: 'merged' | 'global' | 'project'
}

export async function readIdentity(opts?: ReadIdentityOptions): Promise<Identity>
export async function writeIdentity(opts: IdentityOptions & IdentityScope, identity: Partial<Identity>): Promise<void>
```

`writeIdentity` with `scope: 'global'` writes to `mspHome()/identity.json`; `scope: 'project'` writes a sparse `identity.override.json` in workspace.

### `src/identity/index.ts`

Public-facing API: keep current shape but plumb the merged read through. Old callers passing only `IdentityOptions` get the merged view by default — backwards-compatible.

### `src/mcp/tools/identity-get.ts` and `identity-set.ts`

Add `scope` arg. `identity-get` gains optional `explain: true` flag returning the resolution chain (which fields came from which layer). `identity-set` requires explicit `scope` — error if missing for non-default keys.

## Tests to add

### `test/identity/migrate.test.ts`

- migrate when global missing + workspace exists → global created, workspace preserved
- noop when global exists
- noop when neither exists (fresh install) → defaults written to global
- override sparse: writing project scope creates only the changed fields
- merged read: global has `voice.formality=casual`, override has `voice.formality=formal` → merged returns `formal`

### `test/projects/registry.test.ts`

- registry round-trip (read, modify, write)
- register new project
- duplicate name → error
- missing schemaVersion → reject

### `test/projects/resolve.test.ts`

- CLI flag wins over env
- env wins over `.mspconfig`
- `.mspconfig` walked up from cwd
- unknown project name → error (not silent fallback)
- single-line `.mspconfig` shorthand
- YAML `.mspconfig` full form
- registry empty + first read → registers `default` automatically

## Migration test (integration)

```ts
// test/identity/migration-integration.test.ts
it('upgrades workspace identity to global on first read', async () => {
  // arrange: workspace .brain/msp/projects/evaAI/identity.json exists, global doesn't
  const tmp = mkdtemp()
  process.env['MSP_HOME'] = resolve(tmp, '.msp')
  await writeFile(resolve(tmp, '.brain/msp/projects/evaAI/identity.json'), JSON.stringify(legacyIdentity))

  // act
  const id = await readIdentity({ root: tmp, namespace: 'evaAI' })

  // assert: returned identity matches legacy; global file now exists
  expect(id).toEqual(legacyIdentity)
  expect(existsSync(globalIdentityPath())).toBe(true)
})
```

## MCP surface changes

| Tool | Change |
|---|---|
| `msp_identity_get` | New optional arg `view: 'merged' \| 'global' \| 'project'` (default `merged`). New optional `explain: true` returns resolution chain. |
| `msp_identity_set` | New required arg `scope: 'global' \| 'project'`. Error if missing for ambiguous keys. |
| (new) `msp_project_list` | Lists registered projects from `~/.msp/projects.yaml`. |
| (new) `msp_project_register` | Adds entry to registry. |
| (new) `msp_project_resolve` | Returns the resolved `ResolvedProject` for the current invocation. Useful for debugging "which project loaded". |

3 new tools → MCP tool count goes 16 → 19.

## Rollout

Single PR, no feature flag. Migration is opt-out via `MSP_DISABLE_MIGRATION=1` (escape hatch only; not advertised). After this PR:

1. CI passes on Node 20 + 22.
2. AUDIT atom records what shipped.
3. Update `msp_spec.md` §12 (project paths) + §16 (references) to reflect new layout.
4. Update `gks/frame/FRAME--MSP-ARCHITECTURE-V2.md` storage section.
5. Update GKS `docs/MSP_RELATIONSHIP.md` upstream (Phase D).

## Risks

- **Cross-platform path differences**. Mitigated by exclusively using `os.homedir()` + `node:path.resolve`.
- **Concurrent reads during migration** — two MCP servers booting simultaneously could both detect "global missing" and both try to create it. Mitigated by `mkdir + writeFile` being atomic-ish and migration being idempotent (last write wins on identical content).
- **User has existing `~/.msp/`** (unlikely but possible). Mitigated by reading the file and merging defaults rather than blindly overwriting.

## Out of scope (deferred)

- Multiple Identity profiles per user (work / personal). Add when needed; current shape allows one identity per `MSP_HOME`.
- Project-level secrets in registry (API keys etc). Stay external — registry stays plaintext-safe.
- Sync `~/.msp/` across machines (user's choice via dotfile manager). Not MSP's job.

## Acceptance criteria

- [ ] `~/.msp/identity.json` is the canonical identity location.
- [ ] First-run migration moves existing workspace identity transparently.
- [ ] `msp_identity_get { explain: true }` shows resolution chain.
- [ ] `msp_project_register` + `msp_project_list` work.
- [ ] `MSP_HOME` override works in tests.
- [ ] All existing identity tests pass.
- [ ] 3+ new tests for migration + project resolution.
- [ ] Documentation updated (`msp_spec.md` + FRAME-V2).
- [ ] AUDIT atom written.
