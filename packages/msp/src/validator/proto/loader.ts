import { readdir, readFile, stat } from 'node:fs/promises'
import { extname, isAbsolute, join, resolve } from 'node:path'
import { pathToFileURL } from 'node:url'

import { parse as parseYaml } from 'yaml'

import type {
  Predicate,
  PredicateContext,
  PredicateResult,
  ProtoMeta,
  ProtoRunResult,
  ProtoStatus,
  ProtoSummary,
  Severity,
} from './types.js'

const PROTO_ID_PATTERN = /^PROTO--[A-Z][A-Z0-9-]*$/

interface FrontmatterShape {
  id?: unknown
  status?: unknown
  severity?: unknown
  crosslinks?: unknown
  linked_symbols?: unknown
}

function parseFrontmatter(raw: string): FrontmatterShape | null {
  const m = raw.match(/^---\s*\n([\s\S]*?)\n---\s*(?:\n|$)/)
  if (!m) return null
  try {
    return parseYaml(m[1]!) as FrontmatterShape
  } catch {
    return null
  }
}

function extractEnforces(crosslinks: unknown): string[] {
  if (typeof crosslinks !== 'object' || crosslinks === null) return []
  const enforces = (crosslinks as { enforces?: unknown }).enforces
  if (!Array.isArray(enforces)) return []
  return enforces.filter((s): s is string => typeof s === 'string')
}

function extractImplPath(linkedSymbols: unknown): string | null {
  if (!Array.isArray(linkedSymbols) || linkedSymbols.length === 0) return null
  const first = linkedSymbols[0]
  if (typeof first !== 'object' || first === null) return null
  const file = (first as { file?: unknown }).file
  return typeof file === 'string' && file.length > 0 ? file : null
}

function normaliseStatus(s: unknown): ProtoStatus | null {
  if (s === 'draft' || s === 'stable' || s === 'superseded') return s
  return null
}

function normaliseSeverity(s: unknown): Severity {
  if (s === 'error' || s === 'warning' || s === 'info') return s
  return 'warning'
}

/**
 * Discover PROTO atoms under `gks/proto/` and parse their metadata.
 *
 * Atoms that fail validation (bad id, missing enforces, missing impl path,
 * impl file doesn't exist) are silently dropped. The validator's main rules
 * still run against them — they just don't get loaded as PROTOs.
 */
export async function discoverProtos(repoRoot: string): Promise<ProtoMeta[]> {
  const dir = resolve(repoRoot, 'gks/proto')
  let entries: string[]
  try {
    entries = await readdir(dir)
  } catch (err) {
    if ((err as NodeJS.ErrnoException).code === 'ENOENT') return []
    throw err
  }

  const out: ProtoMeta[] = []
  for (const entry of entries) {
    if (extname(entry) !== '.md') continue
    const filepath = join(dir, entry)
    let raw: string
    try {
      raw = await readFile(filepath, 'utf8')
    } catch {
      continue
    }
    const fm = parseFrontmatter(raw)
    if (!fm) continue

    const id = typeof fm.id === 'string' ? fm.id : ''
    if (!PROTO_ID_PATTERN.test(id)) continue

    const status = normaliseStatus(fm.status)
    if (status === null) continue

    const enforces = extractEnforces(fm.crosslinks)
    if (enforces.length === 0) continue

    const implPath = extractImplPath(fm.linked_symbols)
    if (implPath === null) continue

    // Verify impl file exists
    const absImpl = isAbsolute(implPath)
      ? implPath
      : resolve(repoRoot, implPath)
    try {
      await stat(absImpl)
    } catch {
      continue
    }

    out.push({
      id,
      status,
      severity: normaliseSeverity(fm.severity),
      enforces,
      implPath,
      filepath,
    })
  }

  out.sort((a, b) => a.id.localeCompare(b.id))
  return out
}

async function loadPredicate(
  meta: ProtoMeta,
  repoRoot: string,
): Promise<{ predicate?: Predicate; loadError?: string }> {
  const absImpl = isAbsolute(meta.implPath)
    ? meta.implPath
    : resolve(repoRoot, meta.implPath)
  try {
    const mod = await import(pathToFileURL(absImpl).href)
    const candidate =
      (mod as { default?: unknown }).default ??
      (mod as { predicate?: unknown }).predicate
    if (typeof candidate !== 'function') {
      return {
        loadError: `${meta.implPath}: no default export (or named export 'predicate') of type function`,
      }
    }
    return { predicate: candidate as Predicate }
  } catch (err) {
    return {
      loadError: `${meta.implPath}: ${(err as Error).message}`,
    }
  }
}

/**
 * Run all loaded PROTO predicates and collect results.
 *
 * - `superseded` PROTOs are skipped entirely.
 * - `draft` PROTOs are run; their results are reported but never fail-exit.
 * - `stable` PROTOs are run; results with severity:'error' fail-exit
 *   (caller passes the summary to `shouldFailExit()`).
 */
export async function runProtos(
  metas: ProtoMeta[],
  ctx: PredicateContext,
): Promise<ProtoSummary> {
  const results: ProtoRunResult[] = []
  let passed = 0
  let failed = 0
  const byStatus: Record<ProtoStatus, number> = {
    draft: 0,
    stable: 0,
    superseded: 0,
  }

  for (const meta of metas) {
    byStatus[meta.status] += 1
    if (meta.status === 'superseded') continue

    const { predicate, loadError } = await loadPredicate(meta, ctx.repoRoot)
    if (!predicate) {
      const result: PredicateResult = {
        ok: false,
        violations: [
          {
            message: loadError ?? 'unknown load error',
            severity: 'error',
          },
        ],
      }
      results.push({ meta, result, loadError })
      failed += 1
      continue
    }

    let result: PredicateResult
    try {
      result = await predicate(ctx)
    } catch (err) {
      result = {
        ok: false,
        violations: [
          { message: `predicate threw: ${(err as Error).message}`, severity: 'error' },
        ],
      }
    }
    results.push({ meta, result })
    if (result.ok) passed += 1
    else failed += 1
  }

  return { total: metas.length, passed, failed, byStatus, results }
}

/**
 * Decide whether the validator CLI should exit 1 based on PROTO results.
 *
 * Rule: any `stable` PROTO whose result contains a severity:'error' violation
 * causes a fail-exit. `draft` violations never fail-exit (gradual rollout).
 */
export function shouldFailExit(summary: ProtoSummary): boolean {
  for (const r of summary.results) {
    if (r.meta.status !== 'stable') continue
    for (const v of r.result.violations) {
      if (v.severity === 'error') return true
    }
  }
  return false
}
