/**
 * Genesis Block backend — graph benchmark runner (P3.5).
 *
 * Spec reference: `gks/blueprint/BLUEPRINT--GENESIS-GRAPH-INTEGRATION.md` §P3.5.
 *
 * Compares three GraphBackend implementations on a fixed in-memory fixture:
 *   - graph-store    : the legacy in-process `GraphStore` baseline.
 *   - genesis-ts     : pure-TypeScript `GenesisGraphBackend` (event-sourced JSONL).
 *   - genesis-native : Rust-backed `NativeGenesisGraphBackend`, loaded through the
 *                      factory. Skipped (with a warning) if the prebuilt `.node`
 *                      cannot be loaded on this host.
 *
 * Workloads measured (latency only — wall-clock per op):
 *   bulk_load       Time to ingest the full fixture (addNode N + addEdge M).
 *   query_by_from   1000 random `from` ids → `.query({from})`.
 *   query_by_rel    4 rel types → `.query({rel})` per rel.
 *   neighbors_1hop  1000 random seeds → `.neighbors(seed, {depth: 1})`.
 *   neighbors_3hop  100  random seeds → `.neighbors(seed, {depth: 3})`.
 *
 * Output:
 *   packages/gks/benchmarks/genesis-block/report.json
 *   packages/gks/benchmarks/genesis-block/report.md
 *
 * Both files are stamped with git-SHA + run timestamp + fixture size + host info.
 * The MD report flags whether the BLUEPRINT <50 ms p50 target is met by
 * `genesis-native` on each workload (✅ / ❌).
 *
 * CLI:
 *   --workdir=<path>   Working dir for backend stores (default: ./tmp-bench-<rand>)
 *   --seed=<int>       PRNG seed for reproducibility (default: 42)
 *   --quick            10× smaller fixture for smoke testing (5k / 50k)
 *
 * Run:
 *   npm run bench:graph -- --quick
 *   npm run bench:graph
 */

import { execSync } from 'node:child_process'
import { mkdir, rm, writeFile } from 'node:fs/promises'
import { parseArgs } from 'node:util'
import { join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { dirname } from 'node:path'

import { GraphStore } from '../../src/memory/graph.js'
import type {
  GraphBackend,
} from '../../src/memory/graph.js'
import {
  GenesisGraphBackend,
  createGenesisGraphBackend,
} from '../../src/memory/graph/genesis-graph.js'
import { createLogger } from '../../src/lib/logger.js'

import { percentile, round2 } from '../_harness.js'

const log = createLogger('bench:genesis-block')

// ─── CLI ──────────────────────────────────────────────────────────────────

interface BenchOptions {
  workDir: string
  seed: number
  quick: boolean
  outDir: string
}

function parseOptions(): BenchOptions {
  const { values } = parseArgs({
    args: process.argv.slice(2),
    options: {
      workdir: { type: 'string' },
      seed: { type: 'string' },
      quick: { type: 'boolean' },
      'out-dir': { type: 'string' },
    },
  })

  const __filename = fileURLToPath(import.meta.url)
  const __dirname = dirname(__filename)
  const defaultOut = __dirname

  const randSuffix = Math.random().toString(36).slice(2, 8)
  const defaultWork = resolve(process.cwd(), `tmp-bench-${randSuffix}`)

  return {
    workDir: resolve((values['workdir'] as string | undefined) ?? defaultWork),
    seed: Number(values['seed'] ?? 42),
    quick: values['quick'] === true,
    outDir: resolve((values['out-dir'] as string | undefined) ?? defaultOut),
  }
}

// ─── deterministic PRNG (mulberry32) ──────────────────────────────────────

function mulberry32(seed: number): () => number {
  let a = seed >>> 0
  return function () {
    a = (a + 0x6d2b79f5) >>> 0
    let t = a
    t = Math.imul(t ^ (t >>> 15), t | 1)
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61)
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296
  }
}

// ─── fixture ──────────────────────────────────────────────────────────────

interface Fixture {
  nodeCount: number
  edgeCount: number
  nodeIds: string[]
  edges: Array<{ from: string; to: string; rel: string }>
  /** Pre-picked workload inputs so all backends see the same input distribution. */
  queryFromIds: string[]
  rels: string[]
  neighborSeeds1: string[]
  neighborSeeds3: string[]
}

const REL_TYPES = ['references', 'implements', 'supersedes', 'derives_from'] as const
const STATUS_VALUES = ['draft', 'stable', 'deprecated', 'experimental'] as const

function buildFixture(seed: number, quick: boolean): Fixture {
  const rng = mulberry32(seed)
  const nodeCount = quick ? 5_000 : 50_000
  const edgeCount = quick ? 50_000 : 500_000

  const nodeIds: string[] = []
  for (let i = 0; i < nodeCount; i++) nodeIds.push(`N-${i.toString(36)}`)

  const edges = new Array<{ from: string; to: string; rel: string }>(edgeCount)
  for (let i = 0; i < edgeCount; i++) {
    const fromIdx = Math.floor(rng() * nodeCount)
    let toIdx = Math.floor(rng() * nodeCount)
    if (toIdx === fromIdx) toIdx = (toIdx + 1) % nodeCount
    const rel = REL_TYPES[Math.floor(rng() * REL_TYPES.length)]!
    edges[i] = { from: nodeIds[fromIdx]!, to: nodeIds[toIdx]!, rel }
  }

  // Pre-pick workload inputs deterministically.
  const queryFromIds: string[] = []
  for (let i = 0; i < 1000; i++) {
    queryFromIds.push(nodeIds[Math.floor(rng() * nodeCount)]!)
  }
  const neighborSeeds1: string[] = []
  for (let i = 0; i < 1000; i++) {
    neighborSeeds1.push(nodeIds[Math.floor(rng() * nodeCount)]!)
  }
  const neighborSeeds3: string[] = []
  for (let i = 0; i < 100; i++) {
    neighborSeeds3.push(nodeIds[Math.floor(rng() * nodeCount)]!)
  }

  return {
    nodeCount,
    edgeCount,
    nodeIds,
    edges,
    queryFromIds,
    rels: [...REL_TYPES],
    neighborSeeds1,
    neighborSeeds3,
  }
}

function statusFor(rng: () => number): string {
  return STATUS_VALUES[Math.floor(rng() * STATUS_VALUES.length)]!
}

// ─── timing ───────────────────────────────────────────────────────────────

function now(): number {
  // process.hrtime.bigint() is the cleanest sub-ms clock in node; convert ns→ms.
  return Number(process.hrtime.bigint()) / 1e6
}

interface Stats {
  ops: number
  total_ms: number
  mean_ms: number
  min_ms: number
  max_ms: number
  p50_ms: number
  p95_ms: number
  p99_ms: number
}

function summarize(samples: number[]): Stats {
  if (samples.length === 0) {
    return { ops: 0, total_ms: 0, mean_ms: 0, min_ms: 0, max_ms: 0, p50_ms: 0, p95_ms: 0, p99_ms: 0 }
  }
  let total = 0
  let min = Infinity
  let max = -Infinity
  for (const v of samples) {
    total += v
    if (v < min) min = v
    if (v > max) max = v
  }
  return {
    ops: samples.length,
    total_ms: round2(total),
    mean_ms: round2(total / samples.length),
    min_ms: round2(min),
    max_ms: round2(max),
    p50_ms: round2(percentile(samples, 50)),
    p95_ms: round2(percentile(samples, 95)),
    p99_ms: round2(percentile(samples, 99)),
  }
}

// ─── workloads ────────────────────────────────────────────────────────────

type WorkloadName =
  | 'bulk_load'
  | 'query_by_from'
  | 'query_by_rel'
  | 'neighbors_1hop'
  | 'neighbors_3hop'

const WORKLOADS: WorkloadName[] = [
  'bulk_load',
  'query_by_from',
  'query_by_rel',
  'neighbors_1hop',
  'neighbors_3hop',
]

async function runBulkLoad(
  backend: GraphBackend,
  fixture: Fixture,
  seed: number,
): Promise<{ bulk_load: Stats; ingestedNodes: number; ingestedEdges: number }> {
  await backend.load()
  // Per-op timing for bulk_load is dominated by tiny addNode/addEdge calls;
  // we report a single sample = the full ingest wall-clock instead of a
  // distribution (matches BLUEPRINT "time to ingest the full fixture").
  const rng = mulberry32(seed ^ 0xc0ffee)

  const t0 = now()

  for (let i = 0; i < fixture.nodeCount; i++) {
    const id = fixture.nodeIds[i]!
    await backend.addNode({
      id,
      labels: ['Atom'],
      props: { id, status: statusFor(rng) },
    })
  }

  let ingestedEdges = 0
  for (const e of fixture.edges) {
    try {
      await backend.addEdge({ from: e.from, to: e.to, rel: e.rel })
      ingestedEdges++
    } catch {
      // Skip — extremely rare on a deterministic fixture but defensive.
    }
  }

  const t1 = now()
  const total = t1 - t0
  return {
    bulk_load: {
      ops: 1,
      total_ms: round2(total),
      mean_ms: round2(total),
      min_ms: round2(total),
      max_ms: round2(total),
      p50_ms: round2(total),
      p95_ms: round2(total),
      p99_ms: round2(total),
    },
    ingestedNodes: fixture.nodeCount,
    ingestedEdges,
  }
}

async function runQueryByFrom(backend: GraphBackend, fixture: Fixture): Promise<Stats> {
  const samples: number[] = []
  for (const id of fixture.queryFromIds) {
    const t0 = now()
    await backend.query({ from: id })
    samples.push(now() - t0)
  }
  return summarize(samples)
}

async function runQueryByRel(backend: GraphBackend, fixture: Fixture): Promise<Stats> {
  const samples: number[] = []
  // 4 rels × 1 call each is too few samples for a meaningful p95/p99 —
  // repeat each rel 25× so we get 100 samples total. Same number of samples
  // for every backend so the comparison is apples-to-apples.
  for (let pass = 0; pass < 25; pass++) {
    for (const rel of fixture.rels) {
      const t0 = now()
      await backend.query({ rel })
      samples.push(now() - t0)
    }
  }
  return summarize(samples)
}

async function runNeighbors(
  backend: GraphBackend,
  seeds: string[],
  depth: number,
): Promise<Stats> {
  const samples: number[] = []
  for (const seed of seeds) {
    const t0 = now()
    await backend.neighbors(seed, { depth, direction: 'out', limit: 1000 })
    samples.push(now() - t0)
  }
  return summarize(samples)
}

// ─── per-backend run ──────────────────────────────────────────────────────

interface BackendRun {
  name: 'graph-store' | 'genesis-ts' | 'genesis-native'
  description: string
  available: boolean
  skipReason?: string
  results: Partial<Record<WorkloadName, Stats>>
  fixture: { nodes_ingested: number; edges_ingested: number }
}

async function runBackend(
  name: BackendRun['name'],
  backendDir: string,
  fixture: Fixture,
  seed: number,
): Promise<BackendRun> {
  const run: BackendRun = {
    name,
    description: '',
    available: true,
    results: {},
    fixture: { nodes_ingested: 0, edges_ingested: 0 },
  }

  let backend: GraphBackend
  if (name === 'graph-store') {
    backend = new GraphStore({ path: join(backendDir, 'graph.jsonl') })
    run.description = 'GraphStore (in-process JSONL baseline)'
  } else if (name === 'genesis-ts') {
    // Force the pure-TS path by instantiating the class directly. This
    // bypasses createGenesisGraphBackend's native-first factory.
    backend = new GenesisGraphBackend({ path: join(backendDir, 'genesis-ts') })
    run.description = 'GenesisGraphBackend (pure-TS)'
  } else {
    // genesis-native: go through the factory; assert we actually got the
    // native adapter by inspecting the class name.
    const candidate = createGenesisGraphBackend({ path: join(backendDir, 'genesis-native') })
    const ctorName = candidate.constructor.name
    if (ctorName !== 'NativeGenesisGraphBackend') {
      log.warn('native backend unavailable; skipping', { ctorName })
      run.available = false
      run.skipReason = `factory returned ${ctorName}, expected NativeGenesisGraphBackend`
      run.description = 'GenesisGraphBackend (native — UNAVAILABLE)'
      return run
    }
    backend = candidate
    run.description = 'GenesisGraphBackend (native Rust via napi-rs)'
  }

  log.info('backend start', { name, description: run.description })

  const bulk = await runBulkLoad(backend, fixture, seed)
  run.results.bulk_load = bulk.bulk_load
  run.fixture.nodes_ingested = bulk.ingestedNodes
  run.fixture.edges_ingested = bulk.ingestedEdges
  log.info('bulk_load done', { name, total_ms: bulk.bulk_load.total_ms })

  run.results.query_by_from = await runQueryByFrom(backend, fixture)
  log.info('query_by_from done', { name, p50: run.results.query_by_from.p50_ms })

  run.results.query_by_rel = await runQueryByRel(backend, fixture)
  log.info('query_by_rel done', { name, p50: run.results.query_by_rel.p50_ms })

  run.results.neighbors_1hop = await runNeighbors(backend, fixture.neighborSeeds1, 1)
  log.info('neighbors_1hop done', { name, p50: run.results.neighbors_1hop.p50_ms })

  run.results.neighbors_3hop = await runNeighbors(backend, fixture.neighborSeeds3, 3)
  log.info('neighbors_3hop done', { name, p50: run.results.neighbors_3hop.p50_ms })

  return run
}

// ─── reporting ────────────────────────────────────────────────────────────

function gitSha(): string {
  try {
    return execSync('git rev-parse HEAD', { stdio: ['ignore', 'pipe', 'ignore'] })
      .toString()
      .trim()
  } catch {
    return 'unknown'
  }
}

function gitDirty(): boolean {
  try {
    const out = execSync('git status --porcelain', { stdio: ['ignore', 'pipe', 'ignore'] })
      .toString()
      .trim()
    return out.length > 0
  } catch {
    return false
  }
}

const P50_TARGET_MS = 50

interface Report {
  benchmark: 'genesis-block-graph'
  git_sha: string
  git_dirty: boolean
  generated_at: string
  host: { platform: string; arch: string; node: string }
  fixture: {
    quick: boolean
    seed: number
    nodes: number
    edges: number
    note: string
  }
  p50_target_ms: number
  runs: BackendRun[]
}

function buildJsonReport(opts: BenchOptions, fixture: Fixture, runs: BackendRun[]): Report {
  return {
    benchmark: 'genesis-block-graph',
    git_sha: gitSha(),
    git_dirty: gitDirty(),
    generated_at: new Date().toISOString(),
    host: {
      platform: process.platform,
      arch: process.arch,
      node: process.version,
    },
    fixture: {
      quick: opts.quick,
      seed: opts.seed,
      nodes: fixture.nodeCount,
      edges: fixture.edgeCount,
      note: opts.quick
        ? 'QUICK fixture (10× smaller than BLUEPRINT P3.5 target)'
        : 'FULL fixture per BLUEPRINT P3.5: 50k nodes / 500k edges',
    },
    p50_target_ms: P50_TARGET_MS,
    runs,
  }
}

function buildMarkdownReport(report: Report): string {
  const runs = report.runs
  const findRun = (name: BackendRun['name']) => runs.find((r) => r.name === name)
  const baseline = findRun('graph-store')
  const ts = findRun('genesis-ts')
  const native = findRun('genesis-native')

  const colHeader = (r: BackendRun | undefined, label: string): string => {
    if (!r) return `${label} (missing)`
    if (!r.available) return `${label} ⚠ skipped`
    return label
  }

  const cell = (r: BackendRun | undefined, w: WorkloadName, metric: keyof Stats): string => {
    if (!r || !r.available) return '—'
    const stats = r.results[w]
    if (!stats) return '—'
    const v = stats[metric] as number
    return `${v.toFixed(2)}`
  }

  const targetMark = (r: BackendRun | undefined, w: WorkloadName): string => {
    if (!r || !r.available) return '—'
    const stats = r.results[w]
    if (!stats) return '—'
    if (w === 'bulk_load') return '—' // bulk_load is total wall-clock, not a per-op target
    return stats.p50_ms < P50_TARGET_MS ? '✅' : '❌'
  }

  const lines: string[] = []
  lines.push('# Genesis Block Graph Benchmark Report')
  lines.push('')
  lines.push(`- **Generated:** ${report.generated_at}`)
  lines.push(`- **Git SHA:** \`${report.git_sha}\`${report.git_dirty ? ' *(dirty)*' : ''}`)
  lines.push(`- **Host:** ${report.host.platform}/${report.host.arch}, Node ${report.host.node}`)
  lines.push(
    `- **Fixture:** ${report.fixture.nodes.toLocaleString()} nodes / ${report.fixture.edges.toLocaleString()} edges${report.fixture.quick ? ' *(QUICK)*' : ''}`,
  )
  lines.push(`- **Seed:** ${report.fixture.seed}`)
  lines.push(`- **Note:** ${report.fixture.note}`)
  lines.push(`- **BLUEPRINT target:** p50 < ${P50_TARGET_MS} ms (CONCEPT--GENESIS-GRAPH-BACKEND)`)
  lines.push('')

  if (report.fixture.quick) {
    lines.push(
      '> ⚠ **This run used the `--quick` fixture (10× smaller than BLUEPRINT P3.5 specifies).** The pass/fail marks below are *indicative only*. The full 50k/500k fixture should be re-run before any ADR promotion.',
    )
    lines.push('')
  }

  if (native && !native.available) {
    lines.push(
      `> ⚠ **\`genesis-native\` was skipped:** ${native.skipReason ?? 'native binary unavailable'}.`,
    )
    lines.push('')
  }

  // Headline table (p50)
  lines.push('## Headline — p50 latency (ms)')
  lines.push('')
  lines.push(
    `| Workload | ${colHeader(baseline, 'graph-store')} | ${colHeader(ts, 'genesis-ts')} | ${colHeader(native, 'genesis-native')} | Target (<${P50_TARGET_MS} ms p50)? |`,
  )
  lines.push('|---|---:|---:|---:|---|')
  for (const w of WORKLOADS) {
    const baseV = cell(baseline, w, 'p50_ms')
    const tsV = cell(ts, w, 'p50_ms')
    const natV = cell(native, w, 'p50_ms')
    const mark = targetMark(native, w)
    lines.push(`| \`${w}\` | ${baseV} | ${tsV} | ${natV} | ${mark} |`)
  }
  lines.push('')

  // Detailed per-workload tables
  for (const w of WORKLOADS) {
    lines.push(`## Workload: \`${w}\``)
    lines.push('')
    const unit = w === 'bulk_load' ? 'total ms' : 'per-op ms'
    lines.push(`*All numbers in **${unit}**.*`)
    lines.push('')
    lines.push(
      `| Metric | ${colHeader(baseline, 'graph-store')} | ${colHeader(ts, 'genesis-ts')} | ${colHeader(native, 'genesis-native')} |`,
    )
    lines.push('|---|---:|---:|---:|')
    const metrics: Array<keyof Stats> = ['ops', 'mean_ms', 'min_ms', 'p50_ms', 'p95_ms', 'p99_ms', 'max_ms']
    for (const m of metrics) {
      lines.push(
        `| ${m} | ${cell(baseline, w, m)} | ${cell(ts, w, m)} | ${cell(native, w, m)} |`,
      )
    }
    lines.push('')
  }

  // Verdict summary
  lines.push('## BLUEPRINT <50 ms p50 target — summary')
  lines.push('')
  if (!native || !native.available) {
    lines.push(
      '⚠ **Inconclusive** — `genesis-native` did not run. Target verdict requires the native backend.',
    )
  } else {
    const queryWorkloads: WorkloadName[] = [
      'query_by_from',
      'query_by_rel',
      'neighbors_1hop',
      'neighbors_3hop',
    ]
    const metAll = queryWorkloads.every((w) => {
      const s = native.results[w]
      return s !== undefined && s.p50_ms < P50_TARGET_MS
    })
    if (metAll) {
      lines.push(`✅ **Target met** — all per-op p50 latencies on \`genesis-native\` are below ${P50_TARGET_MS} ms.`)
    } else {
      const failed = queryWorkloads
        .filter((w) => {
          const s = native.results[w]
          return s === undefined || s.p50_ms >= P50_TARGET_MS
        })
        .map((w) => `\`${w}\``)
      lines.push(`❌ **Target NOT met** — failing workloads: ${failed.join(', ')}.`)
    }
  }
  lines.push('')
  lines.push('---')
  lines.push('')
  lines.push(
    'Generated by `packages/gks/benchmarks/genesis-block/bench.ts` — see `BLUEPRINT--GENESIS-GRAPH-INTEGRATION.md` §P3.5.',
  )
  lines.push('')
  return lines.join('\n')
}

// ─── main ─────────────────────────────────────────────────────────────────

async function main(): Promise<void> {
  const opts = parseOptions()
  log.info('starting bench', {
    workDir: opts.workDir,
    seed: opts.seed,
    quick: opts.quick,
    outDir: opts.outDir,
  })

  await rm(opts.workDir, { recursive: true, force: true })
  await mkdir(opts.workDir, { recursive: true })
  await mkdir(opts.outDir, { recursive: true })

  const fixture = buildFixture(opts.seed, opts.quick)
  log.info('fixture built', {
    nodes: fixture.nodeCount,
    edges: fixture.edgeCount,
  })

  const runs: BackendRun[] = []
  for (const name of ['graph-store', 'genesis-ts', 'genesis-native'] as const) {
    try {
      const run = await runBackend(name, opts.workDir, fixture, opts.seed)
      runs.push(run)
    } catch (err) {
      const reason = err instanceof Error ? `${err.message}\n${err.stack}` : String(err)
      log.error('backend run failed', { name, reason })
      runs.push({
        name,
        description: `${name} — FAILED`,
        available: false,
        skipReason: err instanceof Error ? err.message : String(err),
        results: {},
        fixture: { nodes_ingested: 0, edges_ingested: 0 },
      })
    }
  }

  const report = buildJsonReport(opts, fixture, runs)
  const jsonPath = join(opts.outDir, 'report.json')
  const mdPath = join(opts.outDir, 'report.md')

  await writeFile(jsonPath, JSON.stringify(report, null, 2) + '\n', 'utf8')
  await writeFile(mdPath, buildMarkdownReport(report), 'utf8')

  // Brief stdout summary so CI logs show headline numbers without grepping JSON.
  const headline = runs
    .map((r) => {
      if (!r.available) return `  ${r.name}: SKIPPED (${r.skipReason ?? 'n/a'})`
      const q1 = r.results.query_by_from?.p50_ms ?? -1
      const qR = r.results.query_by_rel?.p50_ms ?? -1
      const n1 = r.results.neighbors_1hop?.p50_ms ?? -1
      const n3 = r.results.neighbors_3hop?.p50_ms ?? -1
      const bulk = r.results.bulk_load?.total_ms ?? -1
      return `  ${r.name.padEnd(16)} bulk=${bulk.toFixed(0)}ms  q_from=${q1.toFixed(2)}ms  q_rel=${qR.toFixed(2)}ms  n1=${n1.toFixed(2)}ms  n3=${n3.toFixed(2)}ms`
    })
    .join('\n')

  console.log('\n── Genesis Block Graph Benchmark ──')
  console.log(`git: ${report.git_sha.slice(0, 12)}${report.git_dirty ? ' (dirty)' : ''}`)
  console.log(`fixture: ${report.fixture.nodes} nodes / ${report.fixture.edges} edges${opts.quick ? ' (quick)' : ''}`)
  console.log('headline p50 latencies:')
  console.log(headline)
  console.log(`\nreports:\n  ${jsonPath}\n  ${mdPath}\n`)

  // Cleanup workdir (it's a temp scratch dir).
  await rm(opts.workDir, { recursive: true, force: true })
}

main().catch((err) => {
  log.error('benchmark failed', {
    err: err instanceof Error ? err.message : String(err),
    stack: err instanceof Error ? err.stack : undefined,
  })
  process.exit(1)
})
