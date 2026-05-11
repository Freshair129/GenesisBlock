import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import { parseFile } from '../../src/symbols/parser/typescript.js'

let workDir: string

function writeFile(name: string, content: string): string {
  const path = join(workDir, name)
  writeFileSync(path, content, 'utf8')
  return path
}

beforeEach(() => {
  workDir = mkdtempSync(join(tmpdir(), 'sg-parser-'))
})

afterEach(() => {
  rmSync(workDir, { recursive: true, force: true })
})

describe('typescript parser', () => {
  it('returns just the module symbol for an empty file', () => {
    const path = writeFile('empty.ts', '')
    const { symbols, edges } = parseFile(path, workDir)
    // A truly empty TS file still gets a module symbol; no edges.
    expect(symbols).toHaveLength(1)
    expect(symbols[0].kind).toBe('module')
    expect(edges).toHaveLength(0)
  })

  it('emits module + function symbol + defines edge for a top-level function', () => {
    const path = writeFile('fn.ts', 'export function greet() { return 1 }\n')
    const { symbols, edges } = parseFile(path, workDir)

    const moduleSym = symbols.find((s) => s.kind === 'module')
    const fnSym = symbols.find((s) => s.kind === 'function')
    expect(moduleSym).toBeDefined()
    expect(fnSym).toBeDefined()
    expect(fnSym?.name).toBe('greet')
    expect(fnSym?.exported).toBe(true)

    const defines = edges.filter((e) => e.type === 'defines')
    expect(defines).toHaveLength(1)
    expect(defines[0].src_id).toBe(moduleSym?.id)
    expect(defines[0].dst_id).toBe(fnSym?.id)
  })

  it('emits class + heritage edges (extends + implements)', () => {
    const path = writeFile(
      'cls.ts',
      `interface Animal { kind: string }
class Mammal { warmBlooded = true }
export class Dog extends Mammal implements Animal {
  kind = 'dog'
}
`,
    )
    const { symbols, edges } = parseFile(path, workDir)

    const animal = symbols.find((s) => s.name === 'Animal')
    const mammal = symbols.find((s) => s.name === 'Mammal')
    const dog = symbols.find((s) => s.name === 'Dog')
    expect(animal?.kind).toBe('interface')
    expect(mammal?.kind).toBe('class')
    expect(dog?.kind).toBe('class')

    const extendsEdges = edges.filter((e) => e.type === 'extends')
    const implementsEdges = edges.filter((e) => e.type === 'implements')
    expect(extendsEdges).toHaveLength(1)
    expect(extendsEdges[0].src_id).toBe(dog?.id)
    expect(extendsEdges[0].dst_id).toBe(mammal?.id)
    expect(implementsEdges).toHaveLength(1)
    expect(implementsEdges[0].src_id).toBe(dog?.id)
    expect(implementsEdges[0].dst_id).toBe(animal?.id)
  })

  it('emits an imports edge for `import { x } from "./y"`', () => {
    const path = writeFile(
      'imp.ts',
      `import { foo, bar as renamed } from './y'
export const z = 1
`,
    )
    const { edges } = parseFile(path, workDir)

    const imports = edges.filter((e) => e.type === 'imports')
    // Two named imports → two edges (resolved=false; we don't bind cross-file in v1)
    expect(imports).toHaveLength(2)
    const dsts = imports.map((e) => e.dst_id).sort()
    expect(dsts).toEqual(['external:./y:bar', 'external:./y:foo'])
    expect(imports.every((e) => e.resolved === false)).toBe(true)
  })

  it('returns empty arrays on syntactically broken TS without throwing', () => {
    const path = writeFile('bad.ts', 'export function broken( { return\n')
    const result = parseFile(path, workDir)
    // Either empty (parse-error gate) or at least we didn't throw.
    expect(Array.isArray(result.symbols)).toBe(true)
    expect(Array.isArray(result.edges)).toBe(true)
    expect(result.symbols.length).toBe(0)
    expect(result.edges.length).toBe(0)
  })
})
