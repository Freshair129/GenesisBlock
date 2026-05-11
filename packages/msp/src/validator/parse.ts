import { readFile } from 'node:fs/promises'

import { parse as parseYaml } from 'yaml'

import { ValidatorIOError, type ParsedAtom, type Wikilink } from './types.js'

const FRONTMATTER_DELIM = '---'

export async function parseFile(filepath: string): Promise<ParsedAtom> {
  let source: string
  try {
    source = await readFile(filepath, 'utf8')
  } catch (err) {
    throw new ValidatorIOError(`cannot read ${filepath}`, err)
  }
  return parseSource(source, filepath)
}

export function parseSource(source: string, filepath: string): ParsedAtom {
  if (!source.startsWith(FRONTMATTER_DELIM)) {
    throw new ValidatorIOError(`${filepath}: missing frontmatter (no leading ---)`)
  }
  const end = source.indexOf(`\n${FRONTMATTER_DELIM}`, FRONTMATTER_DELIM.length)
  if (end === -1) {
    throw new ValidatorIOError(`${filepath}: missing frontmatter close delimiter`)
  }
  const fmText = source.slice(FRONTMATTER_DELIM.length, end).trim()

  let fm: Record<string, unknown>
  try {
    const parsed = parseYaml(fmText)
    if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) {
      throw new Error('frontmatter is not a YAML object')
    }
    fm = parsed as Record<string, unknown>
  } catch (err) {
    throw new ValidatorIOError(`${filepath}: invalid YAML frontmatter`, err)
  }

  const bodyStart = end + `\n${FRONTMATTER_DELIM}`.length
  const body = source.slice(bodyStart).replace(/^\n/, '')

  return { fm, body, source, filepath }
}

const WIKILINK_RE = /\[\[([^\]\n]+?)\]\]/g
const FENCE_RE = /^(\s*)(```|~~~)/

/**
 * Strip inline-code spans (`...`) from a line so wikilinks inside code
 * examples are not treated as real references. Backslash-escaped backticks
 * are not handled — markdown allows them but they're vanishingly rare in
 * frontmatter-style atom bodies.
 */
function stripInlineCode(line: string): string {
  let out = ''
  let i = 0
  while (i < line.length) {
    if (line[i] === '`') {
      // Find run of opening backticks (1..N) and matching closer.
      let n = 0
      while (i + n < line.length && line[i + n] === '`') n++
      const opener = '`'.repeat(n)
      const closeIdx = line.indexOf(opener, i + n)
      if (closeIdx === -1) {
        // No closer → bail, keep rest of line literal.
        out += line.slice(i)
        break
      }
      // Replace span (including backticks) with spaces so column offsets stay aligned.
      const span = closeIdx + n - i
      out += ' '.repeat(span)
      i = closeIdx + n
    } else {
      out += line[i]
      i++
    }
  }
  return out
}

export function extractWikilinks(body: string): Wikilink[] {
  const out: Wikilink[] = []
  const lines = body.split('\n')
  let inFence = false
  let fenceMarker: string | null = null

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i]!
    const fenceMatch = line.match(FENCE_RE)
    if (fenceMatch) {
      const marker = fenceMatch[2]!
      if (!inFence) {
        inFence = true
        fenceMarker = marker
      } else if (marker === fenceMarker) {
        inFence = false
        fenceMarker = null
      }
      continue
    }
    if (inFence) continue

    const masked = stripInlineCode(line)
    WIKILINK_RE.lastIndex = 0
    let m: RegExpExecArray | null
    while ((m = WIKILINK_RE.exec(masked)) !== null) {
      const id = m[1]!.trim()
      out.push({ id, line: i + 1, column: m.index + 1 })
    }
  }

  return out
}
