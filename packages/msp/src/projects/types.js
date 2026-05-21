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
/** Current registry schema version. */
export const REGISTRY_SCHEMA_VERSION = 1;
/** Construct an empty registry (used when the file is missing). */
export function defaultRegistry() {
    return {
        schemaVersion: REGISTRY_SCHEMA_VERSION,
        projects: {},
    };
}
