/**
 * Named-project registry — YAML read/write on top of `~/.msp/projects.yaml`.
 *
 * Per `BLUEPRINT--GLOBAL-VS-WORKSPACE-MIGRATION`, the registry is YAML (reusing
 * the existing `yaml` dep), atomic-ish writes via `mkdir + writeFile`, and
 * idempotent — duplicate registers are an error rather than a silent overwrite.
 *
 * See `src/projects/types.ts` for shape and resolution semantics.
 */

import { mkdir, readFile, rename, writeFile } from 'node:fs/promises'
import { dirname } from 'node:path'

import { parse as parseYaml, stringify as stringifyYaml } from 'yaml'

import { projectsRegistryPath } from '../lib/msp-home.js'

import {
  REGISTRY_SCHEMA_VERSION,
  defaultRegistry,
  type ProjectEntry,
  type ProjectsRegistry,
} from './types.js'

/**
 * Read the projects registry from `~/.msp/projects.yaml` (or `MSP_HOME` path).
 *
 * - Missing file → returns the default empty registry. Does NOT create the
 *   file (writes happen via `writeRegistry` / `registerProject`).
 * - Invalid YAML or wrong shape → throws (refuse to silently overwrite).
 * - `schemaVersion > 1` → throws.
 */
export async function readRegistry(): Promise<ProjectsRegistry> {
  const path = projectsRegistryPath()
  let raw: string
  try {
    raw = await readFile(path, 'utf8')
  } catch (err) {
    if ((err as NodeJS.ErrnoException).code === 'ENOENT') {
      return defaultRegistry()
    }
    throw err
  }

  let parsed: unknown
  try {
    parsed = parseYaml(raw)
  } catch (err) {
    throw new Error(
      `projects.yaml at ${path} is not valid YAML: ${(err as Error).message}`,
    )
  }

  if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) {
    throw new Error(`projects.yaml at ${path} must be a YAML object`)
  }

  const obj = parsed as Record<string, unknown>
  const sv = obj.schemaVersion
  if (typeof sv !== 'number') {
    throw new Error(
      `projects.yaml at ${path} is missing required \`schemaVersion\` (expected 1)`,
    )
  }
  if (sv > REGISTRY_SCHEMA_VERSION) {
    throw new Error(
      `projects.yaml at ${path} has schemaVersion=${sv}; this build only supports up to ${REGISTRY_SCHEMA_VERSION}`,
    )
  }

  const projectsRaw = obj.projects
  const projects: Record<string, ProjectEntry> = {}
  if (projectsRaw !== undefined) {
    if (!projectsRaw || typeof projectsRaw !== 'object' || Array.isArray(projectsRaw)) {
      throw new Error(`projects.yaml at ${path}: \`projects\` must be a map`)
    }
    for (const [name, entry] of Object.entries(projectsRaw as Record<string, unknown>)) {
      if (!entry || typeof entry !== 'object' || Array.isArray(entry)) {
        throw new Error(`projects.yaml at ${path}: project \`${name}\` must be an object`)
      }
      const e = entry as Record<string, unknown>
      if (typeof e.path !== 'string' || e.path.length === 0) {
        throw new Error(
          `projects.yaml at ${path}: project \`${name}\` is missing required \`path\` field`,
        )
      }
      const cleaned: ProjectEntry = { path: e.path }
      if (typeof e.embedder === 'string') cleaned.embedder = e.embedder
      if (typeof e.description === 'string') cleaned.description = e.description
      projects[name] = cleaned
    }
  }

  const result: ProjectsRegistry = {
    schemaVersion: REGISTRY_SCHEMA_VERSION,
    projects,
  }
  if (typeof obj.default === 'string') result.default = obj.default
  return result
}

/**
 * Atomically write the registry to disk (temp file + rename). Creates the
 * `MSP_HOME` directory if missing.
 */
export async function writeRegistry(registry: ProjectsRegistry): Promise<void> {
  const path = projectsRegistryPath()
  await mkdir(dirname(path), { recursive: true })
  // Force schemaVersion on every write — same contract as identity.
  const payload: ProjectsRegistry = {
    ...registry,
    schemaVersion: REGISTRY_SCHEMA_VERSION,
  }
  const tmp = `${path}.tmp.${process.pid}.${Date.now()}`
  await writeFile(tmp, stringifyYaml(payload), 'utf8')
  await rename(tmp, path)
}

/**
 * Register a new project in the registry. Errors if a project with the same
 * name already exists (no silent overwrite — callers should call
 * `writeRegistry` directly to update an existing entry).
 */
export async function registerProject(
  name: string,
  entry: ProjectEntry,
): Promise<void> {
  if (name.length === 0) {
    throw new Error('project name must be non-empty')
  }
  if (entry.path.length === 0) {
    throw new Error(`project \`${name}\` is missing required \`path\``)
  }
  const registry = await readRegistry()
  if (registry.projects[name] !== undefined) {
    throw new Error(`project \`${name}\` is already registered`)
  }
  registry.projects[name] = entry
  await writeRegistry(registry)
}
