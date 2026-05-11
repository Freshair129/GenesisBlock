/**
 * Project name resolution chain.
 *
 * Per `CONCEPT--NAMED-PROJECT-REGISTRY` + `ADR--GLOBAL-VS-WORKSPACE`:
 *
 *   1. CLI flag (`--project=<name>`)         — highest priority
 *   2. Env var (`MSP_PROJECT=<name>`)
 *   3. `.mspconfig` walking up from cwd
 *   4. Registry's `default` field, then literal `'default'`  — lowest
 *
 * Once a name is resolved, it MUST exist in `~/.msp/projects.yaml`. If the
 * name is unknown, this module errors loudly — projects must be registered
 * before they can be resolved.
 */

import { readFile, stat } from 'node:fs/promises'
import { dirname, resolve } from 'node:path'

import { parse as parseYaml } from 'yaml'

import { readRegistry } from './registry.js'
import type { ProjectEntry, ProjectsRegistry } from './types.js'

export interface ResolveOptions {
  /** Explicit CLI flag value (e.g. argv `--project=eva`). */
  cliFlag?: string
  /** Env var value (caller passes `process.env.MSP_PROJECT`). */
  env?: string
  /** Starting directory for `.mspconfig` walk. Defaults to `process.cwd()`. */
  cwd?: string
}

export type ResolveSource = 'cli' | 'env' | 'mspconfig' | 'default'

export interface ResolvedProject {
  name: string
  entry: ProjectEntry
  source: ResolveSource
  /** When `source === 'mspconfig'`, the path to the file that decided. */
  mspconfigPath?: string
}

/** Filename of the project-declared config (no leading `.` is `.mspconfig`). */
const MSPCONFIG_FILENAME = '.mspconfig'

/**
 * Walk from `start` up to filesystem root looking for a `.mspconfig` file.
 * Returns the first hit's absolute path, or `null` if none.
 */
async function findMspconfig(start: string): Promise<string | null> {
  let dir = resolve(start)
  // Cap the walk at 64 ancestors to avoid pathological symlink loops.
  for (let i = 0; i < 64; i += 1) {
    const candidate = resolve(dir, MSPCONFIG_FILENAME)
    try {
      const st = await stat(candidate)
      if (st.isFile()) return candidate
    } catch {
      // ENOENT or other → keep walking.
    }
    const parent = dirname(dir)
    if (parent === dir) return null
    dir = parent
  }
  return null
}

/**
 * Parse a `.mspconfig` file. Two accepted forms:
 *
 *   1. Single-line shorthand: `project: <name>` (with optional whitespace).
 *      Anything before/after on the same line still parses as YAML.
 *   2. Full YAML object with at least `project: <name>`.
 *
 * Returns the project name string, or throws on malformed content.
 */
export async function parseMspconfig(path: string): Promise<string> {
  const raw = await readFile(path, 'utf8')

  // Try YAML parse first — handles both shorthand (single key) and full form.
  let parsed: unknown
  try {
    parsed = parseYaml(raw)
  } catch (err) {
    throw new Error(`${path} is not valid YAML: ${(err as Error).message}`)
  }

  if (parsed === null || parsed === undefined) {
    throw new Error(`${path} is empty or has no \`project\` key`)
  }

  // Bare string form — `project: foo` parses as a string in some yaml flavors;
  // the shorthand "project: <name>" is a single-key object in YAML 1.2.
  if (typeof parsed === 'string') {
    // Treat the whole content as the project name (rare but accept).
    return parsed.trim()
  }

  if (typeof parsed !== 'object' || Array.isArray(parsed)) {
    throw new Error(`${path}: top-level must be a YAML object`)
  }

  const obj = parsed as Record<string, unknown>
  const name = obj.project
  if (typeof name !== 'string' || name.length === 0) {
    throw new Error(`${path}: missing required \`project: <name>\` field`)
  }
  return name
}

/**
 * Resolve the active project name from the priority chain, then look it up
 * in the registry.
 *
 * Errors loudly if:
 *   - the resolved name is not in the registry, or
 *   - no source produced a name AND there is no `default` registered.
 */
export async function resolveProject(
  opts: ResolveOptions = {},
): Promise<ResolvedProject> {
  const registry = await readRegistry()

  // 1. CLI flag
  if (typeof opts.cliFlag === 'string' && opts.cliFlag.length > 0) {
    return lookup(registry, opts.cliFlag, 'cli')
  }

  // 2. Env
  if (typeof opts.env === 'string' && opts.env.length > 0) {
    return lookup(registry, opts.env, 'env')
  }

  // 3. .mspconfig walk
  const cwd = opts.cwd ?? process.cwd()
  const mspconfigPath = await findMspconfig(cwd)
  if (mspconfigPath !== null) {
    const name = await parseMspconfig(mspconfigPath)
    const result = lookup(registry, name, 'mspconfig')
    result.mspconfigPath = mspconfigPath
    return result
  }

  // 4. registry `default`, then literal 'default'
  const fallbackName = registry.default ?? 'default'
  return lookup(registry, fallbackName, 'default')
}

function lookup(
  registry: ProjectsRegistry,
  name: string,
  source: ResolveSource,
): ResolvedProject {
  const entry = registry.projects[name]
  if (!entry) {
    const known = Object.keys(registry.projects).sort()
    const knownStr = known.length > 0 ? known.join(', ') : '(none registered)'
    throw new Error(
      `project '${name}' is not registered (resolved via ${source}). Known projects: ${knownStr}. ` +
        `Register with msp_project_register or edit ~/.msp/projects.yaml.`,
    )
  }
  return { name, entry, source }
}
