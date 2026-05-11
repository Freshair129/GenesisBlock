import { mkdir, readFile, rename, writeFile } from 'node:fs/promises'
import { dirname, resolve } from 'node:path'

import { globalIdentityPath } from '../lib/msp-home.js'

import { migrateIfNeeded } from './migrate.js'
import {
  CURRENT_SCHEMA_VERSION,
  DEFAULT_NAMESPACE,
  defaultIdentity,
  defaultProfile,
  defaultVoice,
  type Identity,
  type IdentityOptions,
} from './types.js'

/**
 * Compute the **legacy / project** on-disk path for a namespace's identity file.
 *
 * Per `ADR--PATH-ENCODING`, this is `.brain/msp/projects/<namespace>/identity.json`.
 * Post-`ADR--GLOBAL-VS-WORKSPACE`, this path is preserved for migration reads
 * but no longer the canonical write target — see `globalIdentityPath()` for
 * the current canonical location and `projectOverridePath()` for sparse
 * per-project overrides.
 */
export function identityPath(root: string, namespace: string): string {
  return resolve(root, '.brain/msp/projects', namespace, 'identity.json')
}

/**
 * Compute the path to the **per-project** sparse override file.
 *
 * Per `ADR--GLOBAL-VS-WORKSPACE`, this lives next to the legacy identity.json
 * but holds only the fields the user explicitly overrode for this project
 * (e.g. project-specific voice). Stored sparse — fields not present in the
 * file fall through to the global identity.
 */
export function projectOverridePath(root: string, namespace: string): string {
  return resolve(
    root,
    '.brain/msp/projects',
    namespace,
    'identity.override.json',
  )
}

/**
 * Resolve `IdentityOptions` to (root, namespace, view) with defaults applied.
 * Defaults: `root = process.cwd()`, `namespace = 'evaAI'`, `view = 'merged'`.
 */
export function resolveOptions(opts?: IdentityOptions): {
  root: string
  namespace: string
  view: 'merged' | 'global' | 'project'
} {
  return {
    root: opts?.root ?? process.cwd(),
    namespace: opts?.namespace ?? DEFAULT_NAMESPACE,
    view: opts?.view ?? 'merged',
  }
}

/** Write scope discriminator for `writeIdentity`. */
export interface IdentityScope {
  scope: 'global' | 'project'
}

/**
 * Read options. Same as `IdentityOptions`; the `view` field is honoured
 * (defaults to `'merged'`). Re-exported for clarity at call sites.
 */
export type ReadIdentityOptions = IdentityOptions

interface ReadResult {
  identity: Identity
  /** True if any field was sourced from the file (vs purely default). */
  hasFile: boolean
}

async function readJsonFile(path: string): Promise<Partial<Identity> | null> {
  let raw: string
  try {
    raw = await readFile(path, 'utf8')
  } catch (err) {
    if ((err as NodeJS.ErrnoException).code === 'ENOENT') return null
    throw err
  }

  const parsed = JSON.parse(raw) as Partial<Identity> & {
    schemaVersion?: number
  }
  if (
    typeof parsed.schemaVersion === 'number' &&
    parsed.schemaVersion > CURRENT_SCHEMA_VERSION
  ) {
    throw new Error(
      `identity file at ${path} has schemaVersion=${parsed.schemaVersion}; ` +
        `this build only supports up to ${CURRENT_SCHEMA_VERSION}. Refusing to read ` +
        `(forward-compat is undefined; remove or migrate the file).`,
    )
  }
  return parsed
}

/**
 * Read the global identity file, layered with defaults. Missing file →
 * returns the default-constructed identity (no file is created).
 */
async function readGlobal(): Promise<ReadResult> {
  const parsed = await readJsonFile(globalIdentityPath())
  if (parsed === null) {
    return { identity: defaultIdentity(), hasFile: false }
  }
  const base = defaultIdentity()
  const profileBase = defaultProfile()
  const voiceBase = defaultVoice()
  return {
    identity: {
      schemaVersion: CURRENT_SCHEMA_VERSION,
      profile: { ...profileBase, ...(parsed.profile ?? {}) },
      voice: { ...voiceBase, ...(parsed.voice ?? {}) },
      preferences: { ...base.preferences, ...(parsed.preferences ?? {}) },
    },
    hasFile: true,
  }
}

/**
 * Read the workspace project override file (sparse). Missing file → null.
 * The override is stored sparsely — only fields the user explicitly set for
 * this project — so we do NOT default-fill missing fields here.
 */
async function readProjectOverride(
  root: string,
  namespace: string,
): Promise<Partial<Identity> | null> {
  return readJsonFile(projectOverridePath(root, namespace))
}

/**
 * Shallow-merge a sparse override on top of a base identity.
 * Applied at the top-level keys profile/voice/preferences; sub-fields within
 * each are themselves shallow-merged.
 */
function mergeOverride(
  base: Identity,
  override: Partial<Identity> | null,
): Identity {
  if (!override) return base
  return {
    schemaVersion: CURRENT_SCHEMA_VERSION,
    profile: { ...base.profile, ...(override.profile ?? {}) },
    voice: { ...base.voice, ...(override.voice ?? {}) },
    preferences: { ...base.preferences, ...(override.preferences ?? {}) },
  }
}

/**
 * Read the identity file for a namespace.
 *
 * View semantics (per `ADR--GLOBAL-VS-WORKSPACE`):
 *
 *   - `view: 'merged'` (default) — global identity shallow-merged with the
 *     workspace `identity.override.json`. Backwards-compatible: callers that
 *     don't pass `view` get the same shape they got pre-migration, with the
 *     global file as the source of truth post-migration.
 *   - `view: 'global'` — only `~/.msp/identity.json` (or `MSP_HOME` override).
 *   - `view: 'project'` — only the workspace override (returns
 *     `defaultIdentity()` filled with override fields where present).
 *
 * Migration: on every read, if the global file is missing but a workspace
 * legacy `.brain/msp/projects/<ns>/identity.json` exists, copy it to
 * `~/.msp/identity.json` (one-time, idempotent, never deletes the source).
 * Set `MSP_DISABLE_MIGRATION=1` to opt out.
 *
 * Schema:
 *   - Missing global + override → returns `defaultIdentity()`.
 *   - File with `schemaVersion > 1` → throws.
 *   - Missing fields are filled from defaults so callers get a complete shape.
 */
export async function readIdentity(
  opts?: ReadIdentityOptions,
): Promise<Identity> {
  const { root, namespace, view } = resolveOptions(opts)

  // Run migration before any read so first-call wires legacy → global.
  if (process.env['MSP_DISABLE_MIGRATION'] !== '1') {
    await migrateIfNeeded(root, namespace).catch(() => {
      // Migration is best-effort; if it fails (e.g. read-only home), fall
      // through to whatever the global / override read produces. The error is
      // intentionally swallowed to keep readIdentity total.
    })
  }

  if (view === 'global') {
    const { identity } = await readGlobal()
    return identity
  }

  if (view === 'project') {
    const override = await readProjectOverride(root, namespace)
    // Project view returns defaults filled with whatever the override sets,
    // so the shape is always complete.
    return mergeOverride(defaultIdentity(), override)
  }

  // merged
  const { identity: globalIdentity } = await readGlobal()
  const override = await readProjectOverride(root, namespace)
  return mergeOverride(globalIdentity, override)
}

/**
 * Atomically replace the identity file at the requested scope.
 *
 * Algorithm (per ADR):
 *   1. mkdir -p the parent directory
 *   2. write to `<path>.tmp.<pid>.<timestamp>`
 *   3. rename(tmp, path) — atomic on POSIX
 *
 * Scope semantics:
 *   - `scope: 'global'`  — writes the FULL identity to `~/.msp/identity.json`.
 *                          `schemaVersion` is forced to 1.
 *   - `scope: 'project'` — writes a SPARSE override to
 *                          `.brain/msp/projects/<ns>/identity.override.json`.
 *                          Pass only the fields you want overridden — passing
 *                          the merged identity would defeat sparse semantics.
 *
 * Backwards-compatible call form: when `opts` does not declare a `scope`,
 * defaults to `'global'` (matches ADR's default writes-go-to-global rule).
 * The full Identity must be supplied for global writes; for project writes a
 * `Partial<Identity>` is acceptable and will be persisted as-is.
 */
export async function writeIdentity(
  opts: (IdentityOptions & Partial<IdentityScope>) | undefined,
  identity: Identity | Partial<Identity>,
): Promise<void> {
  const { root, namespace } = resolveOptions(opts)
  const scope: 'global' | 'project' = opts?.scope ?? 'global'

  if (scope === 'global') {
    const path = globalIdentityPath()
    await mkdir(dirname(path), { recursive: true })
    const payload: Identity = {
      // For global writes the caller must supply a full Identity. We force
      // schemaVersion regardless to keep the on-disk contract honest.
      ...(identity as Identity),
      schemaVersion: CURRENT_SCHEMA_VERSION,
    }
    const tmp = `${path}.tmp.${process.pid}.${Date.now()}`
    await writeFile(tmp, JSON.stringify(payload, null, 2), 'utf8')
    await rename(tmp, path)
    return
  }

  // project scope — sparse override
  const path = projectOverridePath(root, namespace)
  await mkdir(dirname(path), { recursive: true })
  // Strip schemaVersion from sparse overrides — the merge layer reasserts it.
  // Callers may include it; we drop it on write to keep the file genuinely
  // sparse (only fields the user actually overrode).
  const sparse = { ...(identity as Partial<Identity>) }
  delete (sparse as { schemaVersion?: number }).schemaVersion
  const tmp = `${path}.tmp.${process.pid}.${Date.now()}`
  await writeFile(tmp, JSON.stringify(sparse, null, 2), 'utf8')
  await rename(tmp, path)
}
