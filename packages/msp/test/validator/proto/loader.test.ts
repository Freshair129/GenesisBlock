import { mkdir, mkdtemp, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { describe, expect, it } from 'vitest'

import {
  discoverProtos,
  runProtos,
  shouldFailExit,
} from '../../../src/validator/proto/loader.js'
import type {
  AtomicIndexEntry,
} from '../../../src/validator/types.js'

async function freshRoot(): Promise<string> {
  return mkdtemp(join(tmpdir(), 'msp-proto-loader-'))
}

async function writeProtoAtom(
  root: string,
  fname: string,
  body: string,
): Promise<void> {
  const dir = join(root, 'gks/proto')
  await mkdir(dir, { recursive: true })
  await writeFile(join(dir, fname), body)
}

async function writeImpl(
  root: string,
  rel: string,
  body: string,
): Promise<string> {
  const abs = join(root, rel)
  await mkdir(join(abs, '..'), { recursive: true })
  await writeFile(abs, body)
  return rel
}

const stubAtomicIndex: AtomicIndexEntry[] = [
  {
    id: 'FRAME--TEST',
    phase: 0,
    type: 'frame',
    status: 'stable',
    vault_id: 'default',
    path: 'frame/FRAME--TEST.md',
  } as AtomicIndexEntry,
]

describe('discoverProtos', () => {
  it('returns [] when gks/proto/ is missing', async () => {
    const root = await freshRoot()
    const protos = await discoverProtos(root)
    expect(protos).toEqual([])
  })

  it('returns [] when gks/proto/ is empty', async () => {
    const root = await freshRoot()
    await mkdir(join(root, 'gks/proto'), { recursive: true })
    const protos = await discoverProtos(root)
    expect(protos).toEqual([])
  })

  it('parses a valid PROTO atom and surfaces metadata', async () => {
    const root = await freshRoot()
    const implRel = await writeImpl(
      root,
      'src/validator/proto/test-rule.mjs',
      `export default function predicate() { return { ok: true, violations: [] } }`,
    )
    await writeProtoAtom(
      root,
      'PROTO--TEST-RULE.md',
      `---\nid: PROTO--TEST-RULE\nphase: 2\ntype: proto\nstatus: stable\nseverity: error\ncrosslinks:\n  enforces: [FRAME--TEST]\nlinked_symbols:\n  - {"file":"${implRel}"}\n---\n# Body\n`,
    )
    const protos = await discoverProtos(root)
    expect(protos).toHaveLength(1)
    expect(protos[0]!.id).toBe('PROTO--TEST-RULE')
    expect(protos[0]!.status).toBe('stable')
    expect(protos[0]!.severity).toBe('error')
    expect(protos[0]!.enforces).toEqual(['FRAME--TEST'])
    expect(protos[0]!.implPath).toBe('src/validator/proto/test-rule.mjs')
  })

  it('skips atoms with bad id pattern', async () => {
    const root = await freshRoot()
    await writeProtoAtom(
      root,
      'PROTO--bad.md',
      `---\nid: proto--lowercase\nphase: 2\ntype: proto\nstatus: stable\ncrosslinks:\n  enforces: [FRAME--TEST]\nlinked_symbols:\n  - {"file":"x.ts"}\n---\n`,
    )
    const protos = await discoverProtos(root)
    expect(protos).toEqual([])
  })

  it('skips atoms missing crosslinks.enforces', async () => {
    const root = await freshRoot()
    const implRel = await writeImpl(
      root,
      'src/validator/proto/no-enforces.mjs',
      `export default function () { return { ok: true, violations: [] } }`,
    )
    await writeProtoAtom(
      root,
      'PROTO--NO-ENFORCES.md',
      `---\nid: PROTO--NO-ENFORCES\nphase: 2\ntype: proto\nstatus: stable\ncrosslinks:\n  references: [FRAME--TEST]\nlinked_symbols:\n  - {"file":"${implRel}"}\n---\n`,
    )
    const protos = await discoverProtos(root)
    expect(protos).toEqual([])
  })

  it('skips atoms missing linked_symbols', async () => {
    const root = await freshRoot()
    await writeProtoAtom(
      root,
      'PROTO--NO-IMPL.md',
      `---\nid: PROTO--NO-IMPL\nphase: 2\ntype: proto\nstatus: stable\ncrosslinks:\n  enforces: [FRAME--TEST]\n---\n`,
    )
    const protos = await discoverProtos(root)
    expect(protos).toEqual([])
  })

  it('skips atoms whose impl file does not exist', async () => {
    const root = await freshRoot()
    await writeProtoAtom(
      root,
      'PROTO--BAD-IMPL.md',
      `---\nid: PROTO--BAD-IMPL\nphase: 2\ntype: proto\nstatus: stable\ncrosslinks:\n  enforces: [FRAME--TEST]\nlinked_symbols:\n  - {"file":"src/validator/proto/missing.mjs"}\n---\n`,
    )
    const protos = await discoverProtos(root)
    expect(protos).toEqual([])
  })

  it('defaults severity to warning when omitted', async () => {
    const root = await freshRoot()
    const implRel = await writeImpl(
      root,
      'src/validator/proto/default-sev.mjs',
      `export default function () { return { ok: true, violations: [] } }`,
    )
    await writeProtoAtom(
      root,
      'PROTO--DEFAULT-SEV.md',
      `---\nid: PROTO--DEFAULT-SEV\nphase: 2\ntype: proto\nstatus: draft\ncrosslinks:\n  enforces: [FRAME--TEST]\nlinked_symbols:\n  - {"file":"${implRel}"}\n---\n`,
    )
    const protos = await discoverProtos(root)
    expect(protos).toHaveLength(1)
    expect(protos[0]!.severity).toBe('warning')
  })

  it('sorts results by id', async () => {
    const root = await freshRoot()
    for (const id of ['PROTO--ZEBRA', 'PROTO--ALPHA', 'PROTO--MIDDLE']) {
      const implRel = await writeImpl(
        root,
        `src/validator/proto/${id.toLowerCase()}.mjs`,
        `export default function () { return { ok: true, violations: [] } }`,
      )
      await writeProtoAtom(
        root,
        `${id}.md`,
        `---\nid: ${id}\nphase: 2\ntype: proto\nstatus: stable\ncrosslinks:\n  enforces: [FRAME--TEST]\nlinked_symbols:\n  - {"file":"${implRel}"}\n---\n`,
      )
    }
    const protos = await discoverProtos(root)
    expect(protos.map((p) => p.id)).toEqual([
      'PROTO--ALPHA',
      'PROTO--MIDDLE',
      'PROTO--ZEBRA',
    ])
  })
})

describe('runProtos', () => {
  it('runs predicate and collects passing result', async () => {
    const root = await freshRoot()
    const implRel = await writeImpl(
      root,
      'src/validator/proto/passes.mjs',
      `export default function () { return { ok: true, violations: [] } }`,
    )
    await writeProtoAtom(
      root,
      'PROTO--PASSES.md',
      `---\nid: PROTO--PASSES\nphase: 2\ntype: proto\nstatus: stable\nseverity: error\ncrosslinks:\n  enforces: [FRAME--TEST]\nlinked_symbols:\n  - {"file":"${implRel}"}\n---\n`,
    )
    const metas = await discoverProtos(root)
    const summary = await runProtos(metas, {
      atomicIndex: stubAtomicIndex,
      repoRoot: root,
    })
    expect(summary.passed).toBe(1)
    expect(summary.failed).toBe(0)
    expect(summary.byStatus.stable).toBe(1)
  })

  it('injects symbolGraph into context', async () => {
    const root = await freshRoot()
    const implRel = await writeImpl(
      root,
      'src/validator/proto/check-ctx.mjs',
      `export default function (ctx) { return { ok: ctx.symbolGraph === null, violations: [] } }`,
    )
    await writeProtoAtom(
      root,
      'PROTO--CHECK-CTX.md',
      `---\nid: PROTO--CHECK-CTX\nphase: 2\ntype: proto\nstatus: stable\nseverity: error\ncrosslinks:\n  enforces: [FRAME--TEST]\nlinked_symbols:\n  - {"file":"${implRel}"}\n---\n`,
    )
    const metas = await discoverProtos(root)
    const summary = await runProtos(metas, {
      atomicIndex: stubAtomicIndex,
      repoRoot: root,
    })
    expect(summary.passed).toBe(1) // Passes if ctx.symbolGraph === null as expected in this mock env
  })

  it('records load error when impl has no predicate export', async () => {
    const root = await freshRoot()
    const implRel = await writeImpl(
      root,
      'src/validator/proto/no-export.mjs',
      `// no default export, no named predicate\nexport const other = 'x'`,
    )
    await writeProtoAtom(
      root,
      'PROTO--NO-EXPORT.md',
      `---\nid: PROTO--NO-EXPORT\nphase: 2\ntype: proto\nstatus: stable\nseverity: error\ncrosslinks:\n  enforces: [FRAME--TEST]\nlinked_symbols:\n  - {"file":"${implRel}"}\n---\n`,
    )
    const metas = await discoverProtos(root)
    const summary = await runProtos(metas, {
      atomicIndex: stubAtomicIndex,
      repoRoot: root,
    })
    expect(summary.failed).toBe(1)
    expect(summary.results[0]!.loadError).toMatch(/no default export/)
  })

  it('catches predicate throws gracefully', async () => {
    const root = await freshRoot()
    const implRel = await writeImpl(
      root,
      'src/validator/proto/throws.mjs',
      `export default function () { throw new Error('boom') }`,
    )
    await writeProtoAtom(
      root,
      'PROTO--THROWS.md',
      `---\nid: PROTO--THROWS\nphase: 2\ntype: proto\nstatus: stable\nseverity: error\ncrosslinks:\n  enforces: [FRAME--TEST]\nlinked_symbols:\n  - {"file":"${implRel}"}\n---\n`,
    )
    const metas = await discoverProtos(root)
    const summary = await runProtos(metas, {
      atomicIndex: stubAtomicIndex,
      repoRoot: root,
    })
    expect(summary.failed).toBe(1)
    expect(summary.results[0]!.result.violations[0]!.message).toMatch(
      /boom/,
    )
  })

  it('skips superseded PROTOs', async () => {
    const root = await freshRoot()
    const implRel = await writeImpl(
      root,
      'src/validator/proto/superseded.mjs',
      `export default function () { return { ok: false, violations: [{ message: 'should not run', severity: 'error' }] } }`,
    )
    await writeProtoAtom(
      root,
      'PROTO--SUPERSEDED.md',
      `---\nid: PROTO--SUPERSEDED\nphase: 2\ntype: proto\nstatus: superseded\nseverity: error\ncrosslinks:\n  enforces: [FRAME--TEST]\nlinked_symbols:\n  - {"file":"${implRel}"}\n---\n`,
    )
    const metas = await discoverProtos(root)
    const summary = await runProtos(metas, {
      atomicIndex: stubAtomicIndex,
      repoRoot: root,
    })
    expect(summary.passed).toBe(0)
    expect(summary.failed).toBe(0)
    expect(summary.byStatus.superseded).toBe(1)
  })
})

describe('shouldFailExit', () => {
  it('is false when nothing failed', () => {
    expect(
      shouldFailExit({
        total: 0,
        passed: 0,
        failed: 0,
        byStatus: { draft: 0, stable: 0, active: 0, superseded: 0 },
        results: [],
      }),
    ).toBe(false)
  })

  it('is true when a stable PROTO has a severity:error violation', () => {
    expect(
      shouldFailExit({
        total: 1,
        passed: 0,
        failed: 1,
        byStatus: { draft: 0, stable: 1, active: 0, superseded: 0 },
        results: [
          {
            meta: {
              id: 'PROTO--X',
              status: 'stable',
              severity: 'error',
              enforces: ['FRAME--Y'],
              implPath: 'x.ts',
              filepath: 'gks/proto/PROTO--X.md',
            },
            result: {
              ok: false,
              violations: [{ message: 'fail', severity: 'error' }],
            },
          },
        ],
      }),
    ).toBe(true)
  })

  it('is false when a draft PROTO has a severity:error violation', () => {
    expect(
      shouldFailExit({
        total: 1,
        passed: 0,
        failed: 1,
        byStatus: { draft: 1, stable: 0, active: 0, superseded: 0 },
        results: [
          {
            meta: {
              id: 'PROTO--X',
              status: 'draft',
              severity: 'error',
              enforces: ['FRAME--Y'],
              implPath: 'x.ts',
              filepath: 'gks/proto/PROTO--X.md',
            },
            result: {
              ok: false,
              violations: [{ message: 'fail', severity: 'error' }],
            },
          },
        ],
      }),
    ).toBe(false)
  })

  it('is false when stable PROTO only has warning-severity violations', () => {
    expect(
      shouldFailExit({
        total: 1,
        passed: 0,
        failed: 1,
        byStatus: { draft: 0, stable: 1, active: 0, superseded: 0 },
        results: [
          {
            meta: {
              id: 'PROTO--X',
              status: 'stable',
              severity: 'warning',
              enforces: ['FRAME--Y'],
              implPath: 'x.ts',
              filepath: 'gks/proto/PROTO--X.md',
            },
            result: {
              ok: false,
              violations: [{ message: 'soft', severity: 'warning' }],
            },
          },
        ],
      }),
    ).toBe(false)
  })
})
