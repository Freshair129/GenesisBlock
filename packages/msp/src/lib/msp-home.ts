/**
 * MSP global root resolution.
 *
 * Per `ADR--GLOBAL-VS-WORKSPACE`, MSP keeps user-level state (identity,
 * preferences, projects registry, cross-project audit) in a global root —
 * default `~/.msp/`. Workspace-scoped state stays under
 * `./.brain/msp/projects/<namespace>/` per `ADR--PATH-ENCODING`.
 *
 * The `MSP_HOME` env var overrides the default; this exists for tests, CI,
 * sandboxes, and multi-account setups (mirrors `GIT_CONFIG_GLOBAL`).
 *
 * Always use `os.homedir()` (not `~` string concat) for cross-platform
 * correctness — Windows stores under `%USERPROFILE%\.msp\`.
 */

import { homedir } from 'node:os'
import { resolve } from 'node:path'

/**
 * Resolve the MSP global root, honoring `MSP_HOME` env override.
 *
 * Default: `<os.homedir()>/.msp`.
 */
export function mspHome(): string {
  const override = process.env['MSP_HOME']
  if (override && override.length > 0) return resolve(override)
  return resolve(homedir(), '.msp')
}

/** Path to the global identity file. */
export function globalIdentityPath(): string {
  return resolve(mspHome(), 'identity.json')
}

/** Path to the global preferences file. */
export function globalPreferencesPath(): string {
  return resolve(mspHome(), 'preferences.json')
}

/** Path to the global projects registry (YAML). */
export function projectsRegistryPath(): string {
  return resolve(mspHome(), 'projects.yaml')
}

/** Path to the global authentication config (PIN hashes, etc). */
export function authConfigPath(): string {
  return resolve(mspHome(), 'auth.json')
}

/** Directory for cross-project audit logs (JSONL by date). */
export function globalAuditDir(): string {
  return resolve(mspHome(), 'audit')
}
