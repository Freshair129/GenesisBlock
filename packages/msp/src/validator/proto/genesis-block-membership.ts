/**
 * PROTO--GENESIS-BLOCK-MEMBERSHIP — machine-enforces the membership and
 * status-cascade contract that SPEC--GENESIS-BLOCK-MANIFEST §2.2 / §3 / §4.2
 * declares for GENESIS-- Block Manifest atoms.
 *
 * For every atom with `type: genesis`, the predicate reads the manifest
 * frontmatter and checks:
 *   1. block fields present — `members`, `manifest_version`, `daci.driver`
 *   2. five-dimension core — `members.core` lists cognitive / algo / runbook /
 *      concept / params, each a non-empty array (SPEC §3.1)
 *   3. aggregation grammar — every `members.*` id matches the canonical id
 *      regex, resolves to an existing atom, and that atom's `type:` matches
 *      the role it is listed under (SPEC §3.3)
 *   4. status cascade — the block's `status` equals min(member statuses)
 *      under `stub < raw < draft < active < stable`; a deprecated/superseded
 *      member forces the block to a terminal status (SPEC §4.2)
 *
 * Severity: error. Status: draft (runs, reports, does not fail-exit yet).
 * No GENESIS-- atoms exist in the repo yet, so on a real `--all` run this
 * predicate passes trivially — behavioural coverage lives in the unit test.
 */
import { readFile } from 'node:fs/promises'
import { isAbsolute, join, resolve } from 'node:path'

import { parse as parseYaml } from 'yaml'

import type { AtomicIndexEntry } from '../types.js'

import type {
  Predicate,
  PredicateContext,
  PredicateResult,
  PredicateViolation,
} from './types.js'

/** The five mandatory core dimensions (EVA 4.0 — SPEC §3.1). */
export const CORE_ROLES = ['cognitive', 'algo', 'runbook', 'concept', 'params'] as const
/** Conditional supplements (SPEC §3.2). */
export const OPTIONAL_ROLES = ['guard', 'safety', 'stack', 'protocol', 'mod', 'spec'] as const

export type CoreRole = (typeof CORE_ROLES)[number]
export type OptionalRole = (typeof OPTIONAL_ROLES)[number]
export type Role = CoreRole | OptionalRole

/** Role name → the `type:` an atom listed under that role must declare. */
export const ROLE_TYPE: Record<Role, string> = {
  cognitive: 'cognitive',
  algo: 'algo',
  runbook: 'runbook',
  concept: 'concept',
  params: 'params',
  guard: 'guard',
  safety: 'safety',
  stack: 'stack',
  protocol: 'protocol',
  mod: 'mod',
  spec: 'spec',
}

/** SPEC §4.2 status order, ascending. */
export const STATUS_ORDER = ['stub', 'raw', 'draft', 'active', 'stable'] as const
/** Statuses that propagate immediately to the block (SPEC §4.2). */
export const TERMINAL_STATUSES = new Set(['deprecated', 'superseded'])

/** Canonical atom-id regex (mirrors gks `atomic-id.ts`). */
export const ID_PATTERN = /^[A-Z][A-Z0-9_]*(?:-[a-zA-Z0-9-]+)?--[A-Z0-9][A-Z0-9_-]*(?:--K\d+)?$/

/** Rank of a status in STATUS_ORDER; -1 if not a graded status. */
export function statusRank(s: string): number {
  return (STATUS_ORDER as readonly string[]).indexOf(s)
}

interface ManifestFrontmatter {
  id?: unknown
  status?: unknown
  manifest_version?: unknown
  members?: unknown
  daci?: unknown
}

/** Strip the leading `---\nYAML\n---` block and parse it; null on failure. */
export function parseFrontmatter(raw: string): ManifestFrontmatter | null {
  const m = raw.match(/^---\r?\n([\s\S]*?)\r?\n---(?:\r?\n|$)/)
  if (!m) return null
  try {
    return parseYaml(m[1]!) as ManifestFrontmatter
  } catch {
    return null
  }
}

function asIdList(v: unknown): string[] {
  if (!Array.isArray(v)) return []
  return v.filter((x): x is string => typeof x === 'string')
}

/** Pull { role → id[] } out of members.core and members.optional. */
export function collectMembers(members: unknown): {
  core: Record<string, string[]>
  optional: Record<string, string[]>
} {
  const out = {
    core: {} as Record<string, string[]>,
    optional: {} as Record<string, string[]>,
  }
  if (typeof members !== 'object' || members === null) return out
  const core = (members as { core?: unknown }).core
  const optional = (members as { optional?: unknown }).optional
  if (typeof core === 'object' && core !== null) {
    for (const role of CORE_ROLES) {
      out.core[role] = asIdList((core as Record<string, unknown>)[role])
    }
  }
  if (typeof optional === 'object' && optional !== null) {
    for (const role of OPTIONAL_ROLES) {
      out.optional[role] = asIdList((optional as Record<string, unknown>)[role])
    }
  }
  return out
}

/** Check a single Block Manifest's frontmatter against the SPEC contract. */
export function checkManifest(
  blockId: string,
  fm: ManifestFrontmatter,
  index: Map<string, AtomicIndexEntry>,
): PredicateViolation[] {
  const violations: PredicateViolation[] = []
  const err = (message: string): void => {
    violations.push({ atomId: blockId, message: `${blockId}: ${message}`, severity: 'error' })
  }

  // 1. block-specific fields present (SPEC §2.2)
  if (fm.members === undefined) err('missing required `members:` block')
  if (fm.manifest_version === undefined) err('missing required `manifest_version:`')
  const daci = fm.daci
  if (
    typeof daci !== 'object' ||
    daci === null ||
    typeof (daci as { driver?: unknown }).driver !== 'string'
  ) {
    err('missing required `daci.driver:` (single owner atom id)')
  }

  const { core, optional } = collectMembers(fm.members)

  // 2. five-dimension core — each role lists ≥1 atom (SPEC §3.1)
  for (const role of CORE_ROLES) {
    if (!core[role] || core[role]!.length === 0) {
      err(
        `members.core.${role} must list ≥1 atom (five-dimension core per SPEC--GENESIS-BLOCK-MANIFEST §3.1)`,
      )
    }
  }

  // 3. aggregation grammar — id format, resolution, type-vs-role (SPEC §3.3)
  const roleLists: Array<[Role, string[]]> = [
    ...CORE_ROLES.map((r): [Role, string[]] => [r, core[r] ?? []]),
    ...OPTIONAL_ROLES.map((r): [Role, string[]] => [r, optional[r] ?? []]),
  ]
  const memberStatuses: string[] = []
  for (const [role, ids] of roleLists) {
    for (const id of ids) {
      if (!ID_PATTERN.test(id)) {
        err(`members.${role} entry '${id}' is not a canonical atom id`)
        continue
      }
      const entry = index.get(id)
      if (!entry) {
        err(`members.${role} entry '${id}' does not resolve to any atom`)
        continue
      }
      if (entry.type !== ROLE_TYPE[role]) {
        err(
          `members.${role} entry '${id}' has type '${entry.type}', expected '${ROLE_TYPE[role]}'`,
        )
      }
      memberStatuses.push(entry.status)
    }
  }

  // 4. status cascade — block status = min(member statuses) (SPEC §4.2)
  const blockStatus = typeof fm.status === 'string' ? fm.status : ''
  if (memberStatuses.length > 0) {
    const terminal = memberStatuses.find((s) => TERMINAL_STATUSES.has(s))
    if (terminal !== undefined) {
      if (!TERMINAL_STATUSES.has(blockStatus)) {
        err(
          `status '${blockStatus}' contradicts member status '${terminal}' — a deprecated/superseded member forces the block to a terminal status (SPEC §4.2)`,
        )
      }
    } else {
      let minRank = Number.POSITIVE_INFINITY
      let minStatus = ''
      for (const s of memberStatuses) {
        const r = statusRank(s)
        if (r >= 0 && r < minRank) {
          minRank = r
          minStatus = s
        }
      }
      if (minStatus !== '' && blockStatus !== minStatus) {
        err(
          `status '${blockStatus}' must equal min(member statuses) = '${minStatus}' (SPEC §4.2 status cascade)`,
        )
      }
    }
  }

  return violations
}

function resolveAtomPath(repoRoot: string, relPath: string): string {
  if (isAbsolute(relPath)) return relPath
  return resolve(join(repoRoot, 'gks', relPath))
}

const predicate: Predicate = async (
  ctx: PredicateContext,
): Promise<PredicateResult> => {
  const violations: PredicateViolation[] = []
  const index = new Map<string, AtomicIndexEntry>()
  for (const entry of ctx.atomicIndex) index.set(entry.id, entry)

  for (const entry of ctx.atomicIndex) {
    if (entry.type !== 'genesis') continue
    const abs = resolveAtomPath(ctx.repoRoot, entry.path)
    let raw: string
    try {
      raw = await readFile(abs, 'utf8')
    } catch (readErr) {
      violations.push({
        atomId: entry.id,
        message: `${entry.id}: cannot read manifest at ${entry.path} (${(readErr as Error).message})`,
        severity: 'error',
      })
      continue
    }
    const fm = parseFrontmatter(raw)
    if (!fm) {
      violations.push({
        atomId: entry.id,
        message: `${entry.id}: manifest frontmatter is missing or not valid YAML`,
        severity: 'error',
      })
      continue
    }
    violations.push(...checkManifest(entry.id, fm, index))
  }

  return { ok: violations.length === 0, violations }
}

export default predicate
