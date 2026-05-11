/**
 * Named-project registry types.
 *
 * Per `CONCEPT--NAMED-PROJECT-REGISTRY` + `ADR--GLOBAL-VS-WORKSPACE`, the
 * registry lives at `~/.msp/projects.yaml` and maps short names to filesystem
 * paths + per-project settings (embedder, retrieval defaults).
 *
 * Resolution chain (`resolveProject`):
 *   1. CLI flag `--project=<name>`
 *   2. Env `MSP_PROJECT=<name>`
 *   3. `.mspconfig` walked up from cwd
 *   4. Registry's `default` field, then literal `'default'`
 *
 * If the resolved name is not in the registry, MSP errors loudly — projects
 * must be registered before use (no silent fallback).
 */

/** A single project registry entry. */
export interface ProjectEntry {
  /** Filesystem path to the project root. */
  path: string
  /**
   * Embedder name (e.g. `nomic-embed-text-v1.5`). Optional; falls back to the
   * server's default embedder when omitted.
   */
  embedder?: string
  /** Free-form human description. Not interpreted by code. */
  description?: string
}

/** Registry shape persisted to `~/.msp/projects.yaml`. */
export interface ProjectsRegistry {
  /** On-disk schema version. Currently locked to 1; reads with > 1 are rejected. */
  schemaVersion: 1
  /** Map of project short-name → entry. */
  projects: Record<string, ProjectEntry>
  /**
   * Optional name of the default project. When unset, resolution falls back to
   * the literal string `'default'` (which must itself be registered).
   */
  default?: string
}

/** Current registry schema version. */
export const REGISTRY_SCHEMA_VERSION = 1 as const

/** Construct an empty registry (used when the file is missing). */
export function defaultRegistry(): ProjectsRegistry {
  return {
    schemaVersion: REGISTRY_SCHEMA_VERSION,
    projects: {},
  }
}
