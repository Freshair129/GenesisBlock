import { readFile } from 'node:fs/promises'
import { resolve } from 'node:path'

import { parse as parseYaml } from 'yaml'

import { FORBIDDEN_FIELDS as DEFAULT_FORBIDDEN_FIELDS } from './rules/forbidden-fields.js'

export interface AtomicContract {
  version: number
  forbiddenFields: ReadonlySet<string>
  source: 'yaml' | 'default'
  warnings: string[]
}

const DEFAULT_CONTRACT_PATH = '.brain/msp/LLM_Contract/atomic_contract.yaml'

/**
 * Load the atomic contract from `atomic_contract.yaml`. If the file is
 * missing or invalid, fall back to hardcoded defaults and surface a warning
 * via the returned `warnings` array (callers decide whether to print).
 *
 * The loader never throws — degradation is silent so the validator stays
 * usable in projects that haven't authored the YAML yet.
 */
export async function loadContract(
  root: string,
  contractPath = DEFAULT_CONTRACT_PATH,
): Promise<AtomicContract> {
  const fullPath = resolve(root, contractPath)
  const warnings: string[] = []

  let raw: string
  try {
    raw = await readFile(fullPath, 'utf8')
  } catch (err) {
    const code = (err as NodeJS.ErrnoException).code
    if (code === 'ENOENT') {
      warnings.push(
        `atomic_contract.yaml not found at ${fullPath} — using hardcoded defaults`,
      )
      return defaultContract(warnings)
    }
    warnings.push(`atomic_contract.yaml unreadable: ${(err as Error).message} — using defaults`)
    return defaultContract(warnings)
  }

  let parsed: unknown
  try {
    parsed = parseYaml(raw)
  } catch (err) {
    warnings.push(`atomic_contract.yaml invalid YAML: ${(err as Error).message} — using defaults`)
    return defaultContract(warnings)
  }

  if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) {
    warnings.push('atomic_contract.yaml: top-level must be a YAML object — using defaults')
    return defaultContract(warnings)
  }

  const obj = parsed as Record<string, unknown>
  const version = typeof obj.version === 'number' ? obj.version : 0
  if (version < 1) {
    warnings.push('atomic_contract.yaml: missing or unsupported `version` — using defaults')
    return defaultContract(warnings)
  }

  const forbidden = obj.forbidden_fields
  let forbiddenFields: ReadonlySet<string>
  if (Array.isArray(forbidden) && forbidden.every((x) => typeof x === 'string')) {
    forbiddenFields = new Set(forbidden as string[])
  } else {
    warnings.push('atomic_contract.yaml: forbidden_fields missing or not a string array — using defaults')
    forbiddenFields = DEFAULT_FORBIDDEN_FIELDS
  }

  return {
    version,
    forbiddenFields,
    source: 'yaml',
    warnings,
  }
}

function defaultContract(warnings: string[]): AtomicContract {
  return {
    version: 1,
    forbiddenFields: DEFAULT_FORBIDDEN_FIELDS,
    source: 'default',
    warnings,
  }
}
