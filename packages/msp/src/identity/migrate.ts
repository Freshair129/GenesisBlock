/**
 * One-time migration: workspace identity → global identity.
 *
 * Per `ADR--GLOBAL-VS-WORKSPACE` migration section:
 *
 *   1. If `~/.msp/identity.json` exists → use as global (no-op).
 *   2. Else if `./.brain/msp/projects/<ns>/identity.json` exists →
 *      copy to `~/.msp/identity.json`, log deprecation warning to stderr.
 *      Source file is preserved (never deleted).
 *   3. Else → leave both layers absent (caller's `readIdentity` returns
 *      `defaultIdentity()`).
 *
 * Idempotent. Safe to call on every `readIdentity` — once the global file
 * exists, this fast-paths in a single `stat`.
 *
 * Concurrency: two MCP processes may both detect "global missing" and both
 * try to copy. `mkdir + writeFile + rename` keeps the global file consistent
 * (last-write-wins on identical content); no destructive interleaving.
 *
 * Opt-out: `MSP_DISABLE_MIGRATION=1` (escape hatch only; not advertised).
 */

import {
  copyFile,
  mkdir,
  readFile,
  rename,
  stat,
  writeFile,
} from 'node:fs/promises'
import { dirname, resolve } from 'node:path'

import { globalIdentityPath } from '../lib/msp-home.js'

import { CURRENT_SCHEMA_VERSION, type Identity } from './types.js'

export interface MigrationResult {
  migrated: boolean
  /** Source path when migrated; undefined when no migration was needed. */
  source?: string
  /** Destination path (always populated; the canonical global path). */
  destination: string
}

function legacyIdentityPath(root: string, namespace: string): string {
  return resolve(root, '.brain/msp/projects', namespace, 'identity.json')
}

async function fileExists(path: string): Promise<boolean> {
  try {
    const st = await stat(path)
    return st.isFile()
  } catch {
    return false
  }
}

/**
 * If the global identity is missing but a workspace legacy `identity.json`
 * exists, copy it to the global location. Returns metadata describing what
 * (if anything) was migrated.
 */
export async function migrateIfNeeded(
  workspaceRoot: string,
  namespace: string,
): Promise<MigrationResult> {
  const destination = globalIdentityPath()

  if (await fileExists(destination)) {
    return { migrated: false, destination }
  }

  const source = legacyIdentityPath(workspaceRoot, namespace)
  if (!(await fileExists(source))) {
    return { migrated: false, destination }
  }

  // Validate the source schema before copying — refuse to migrate a file we
  // cannot read back. If the schema check fails, we leave both layers as-is.
  let raw: string
  try {
    raw = await readFile(source, 'utf8')
  } catch {
    return { migrated: false, destination }
  }
  let parsed: Partial<Identity> & { schemaVersion?: number }
  try {
    parsed = JSON.parse(raw) as Partial<Identity> & { schemaVersion?: number }
  } catch {
    // Corrupt source; bail rather than copy garbage to the global.
    return { migrated: false, destination }
  }
  if (
    typeof parsed.schemaVersion === 'number' &&
    parsed.schemaVersion > CURRENT_SCHEMA_VERSION
  ) {
    // Future schema; do not silently downgrade.
    return { migrated: false, destination }
  }

  // Copy via tmp + rename for atomicity. Use `copyFile` to preserve byte-for-
  // byte equivalence (including any extra fields the reader will tolerate).
  await mkdir(dirname(destination), { recursive: true })
  const tmp = `${destination}.tmp.migrate.${process.pid}.${Date.now()}`
  try {
    await copyFile(source, tmp)
    await rename(tmp, destination)
  } catch (err) {
    // Best-effort: try to leave no .tmp.migrate.* leftovers.
    try {
      await writeFile(tmp, '', 'utf8') // touch + ignore
    } catch {
      /* swallow */
    }
    throw err
  }

  // Deprecation warning to stderr (one line; consumers can grep).
  // eslint-disable-next-line no-console
  console.warn(
    `[msp] migrated workspace identity to global: ${source} → ${destination}. ` +
      `The workspace file is preserved; future writes go to the global location.`,
  )

  return { migrated: true, source, destination }
}
