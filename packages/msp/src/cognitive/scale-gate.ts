/**
 * §7.7.2 — Scale-Level gate.
 *
 * Before a codegen microtask is allowed to run, the facade verifies that
 * the atoms required for its scale tier exist in `gks/<type>/` AND have
 * `status: stable | active`.
 *
 *   L1 (quick task)       → no required atoms (gate is a no-op).
 *   L2 (feature/module)   → CONCEPT + ADR + FEAT + BLUEPRINT.
 *   L3 (major/core)       → above + FRAME + FLOW.
 *
 * The check reads the L0 index (`gks/00_index/atomic_index.jsonl`) when
 * present; otherwise it falls back to walking `gks/<type>/*.md` and
 * extracting frontmatter. Either way, no PR / DB / API calls.
 */

import { readFile, readdir } from 'node:fs/promises'
import { join } from 'node:path'

import type { ScaleLevel } from './types.js'
import { ScaleLevelGateError } from './types.js'
import type { PredicateContext, PredicateResult } from '../validator/proto/types.js'

export interface ScaleGateInput {
  root: string
  /** The parent_blueprint id of the running microtask. */
  blueprintId: string
  scale: ScaleLevel
}

interface IndexEntry {
  id: string
  type?: string
  status?: string
  crosslinks?: { references?: string[] }
}

const REQUIRED_BY_SCALE: Record<ScaleLevel, string[]> = {
  L1: [],
  L2: ['concept', 'adr', 'feat', 'blueprint'],
  L3: ['concept', 'adr', 'feat', 'blueprint', 'framework', 'flow'],
}

const STABLE_STATUSES = new Set(['stable', 'active'])

export async function enforceScaleGate(input: ScaleGateInput): Promise<void> {
  const required = REQUIRED_BY_SCALE[input.scale]
  if (required.length === 0) return

  const atoms = await loadAtoms(input.root)
  const blueprint = atoms.find((a) => a.id === input.blueprintId)
  if (!blueprint) {
    throw new ScaleLevelGateError(input.scale, [`BLUEPRINT not found: ${input.blueprintId}`])
  }

  // Collect the blueprint plus everything in its references-closure (one level deep).
  // For L2/L3 we need *at least one* atom of each required type to be stable in that closure.
  const closure: IndexEntry[] = [blueprint]
  for (const refId of blueprint.crosslinks?.references ?? []) {
    const ref = atoms.find((a) => a.id === refId)
    if (ref) closure.push(ref)
  }

  const missing: string[] = []
  for (const t of required) {
    const present = closure.some(
      (a) => (a.type ?? '').toLowerCase() === t && STABLE_STATUSES.has((a.status ?? '').toLowerCase()),
    )
    if (!present) missing.push(t.toUpperCase())
  }

  if (missing.length > 0) {
    throw new ScaleLevelGateError(input.scale, missing)
  }
}

async function loadAtoms(root: string): Promise<IndexEntry[]> {
  const indexPath = join(root, 'gks', '00_index', 'atomic_index.jsonl')
  try {
    const raw = await readFile(indexPath, 'utf8')
    const out: IndexEntry[] = []
    for (const line of raw.split('\n')) {
      const t = line.trim()
      if (!t) continue
      try {
        out.push(JSON.parse(t) as IndexEntry)
      } catch {
        // skip malformed line — best-effort load
      }
    }
    if (out.length > 0) return out
  } catch {
    // fall through to directory scan
  }
  return scanGksDir(join(root, 'gks'))
}

async function scanGksDir(gksDir: string): Promise<IndexEntry[]> {
  const out: IndexEntry[] = []
  let types: string[] = []
  try {
    types = await readdir(gksDir)
  } catch {
    return out
  }
  for (const type of types) {
    if (type === '00_index') continue
    const typeDir = join(gksDir, type)
    let files: string[]
    try {
      files = await readdir(typeDir)
    } catch {
      continue
    }
    for (const file of files) {
      if (!file.endsWith('.md')) continue
      try {
        const txt = await readFile(join(typeDir, file), 'utf8')
        const fm = parseFrontmatter(txt)
        if (fm?.id) {
          out.push({
            id: fm.id,
            type,
            ...(fm.status ? { status: fm.status } : {}),
            ...(fm.crosslinksJson
              ? { crosslinks: tryParseJson(fm.crosslinksJson) ?? {} }
              : {}),
          })
        }
      } catch {
        // ignore broken atom
      }
    }
  }
  return out
}

interface ParsedFrontmatter {
  id?: string
  status?: string
  crosslinksJson?: string
}

function parseFrontmatter(text: string): ParsedFrontmatter | null {
  const m = /^---\n([\s\S]*?)\n---/.exec(text)
  if (!m) return null
  const out: ParsedFrontmatter = {}
  for (const line of m[1]!.split('\n')) {
    const eq = line.indexOf(':')
    if (eq < 0) continue
    const key = line.slice(0, eq).trim()
    const val = line.slice(eq + 1).trim()
    if (key === 'id') out.id = val
    else if (key === 'status') out.status = val
    else if (key === 'crosslinks' && val.startsWith('{')) out.crosslinksJson = val
  }
  return out
}

function tryParseJson(s: string): { references?: string[] } | null {
  try {
    return JSON.parse(s)
  } catch {
    return null
  }
}

/** Validator for PROTO--SCALE-LEVEL-GATE */
export async function predicate(ctx: PredicateContext): Promise<PredicateResult> {
  const violations: any[] = []
  const blueprints = ctx.atomicIndex.filter(
    (a) => a.type === 'blueprint' && (a.status === 'active' || a.status === 'stable'),
  )

  for (const bp of blueprints) {
    try {
      const raw = await readFile(join(ctx.repoRoot, bp.path), 'utf8')
      const fmMatch = raw.match(/^---\n([\s\S]*?)\n---/)
      if (!fmMatch) continue

      const scaleLevelMatch = fmMatch[1]!.match(/^scale_level:\s*(L[123])\s*$/m)
      if (!scaleLevelMatch) {
        violations.push({
          atomId: bp.id,
          message: `BLUEPRINT ${bp.id} is missing a valid 'scale_level' field (expected L1, L2, or L3)`,
          severity: 'warning',
        })
      }
    } catch {
      // ignore read errors
    }
  }

  return { ok: violations.length === 0, violations }
}
