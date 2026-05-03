import type { ParsedAtom, ValidationContext, ValidationError } from '../types.js'

// Match claims like `src/foo.ts`, `src/foo.ts:42`, `src/foo.ts:fn`, `src/api/route.ts:handle`
// Path parts contain only [A-Za-z0-9_-]; extension is one of tsx?/jsx?/mjs/cjs/sh/mts/cts.
// Skips matches inside fenced code blocks AND inline backtick spans.
// Only matches paths starting with src/, lib/, app/, scripts/.
const PATH_RE =
  /\b(?:src|lib|app|scripts)\/(?:[A-Za-z0-9_-]+\/)*[A-Za-z0-9_-]+\.(?:tsx?|jsx?|mjs|cjs|sh|mts|cts)(?::[A-Za-z0-9_]+)?(?::\d+)?/g
const FENCE_RE = /^(\s*)(```|~~~)/

function stripInlineCode(line: string): string {
  let out = ''
  let i = 0
  while (i < line.length) {
    if (line[i] === '`') {
      let n = 0
      while (i + n < line.length && line[i + n] === '`') n++
      const opener = '`'.repeat(n)
      const closeIdx = line.indexOf(opener, i + n)
      if (closeIdx === -1) {
        out += line.slice(i)
        break
      }
      out += ' '.repeat(closeIdx + n - i)
      i = closeIdx + n
    } else {
      out += line[i]
      i++
    }
  }
  return out
}

function extractPathClaims(body: string): Array<{ path: string; line: number }> {
  const out: Array<{ path: string; line: number }> = []
  const lines = body.split('\n')
  let inFence = false
  let marker: string | null = null
  for (let i = 0; i < lines.length; i++) {
    const fenceMatch = lines[i]!.match(FENCE_RE)
    if (fenceMatch) {
      const m = fenceMatch[2]!
      if (!inFence) {
        inFence = true
        marker = m
      } else if (m === marker) {
        inFence = false
        marker = null
      }
      continue
    }
    if (inFence) continue
    const masked = stripInlineCode(lines[i]!)
    PATH_RE.lastIndex = 0
    let m: RegExpExecArray | null
    while ((m = PATH_RE.exec(masked)) !== null) {
      out.push({ path: m[0]!, line: i + 1 })
    }
  }
  return out
}

/**
 * Per ADR--ANTI-HALLUCINATION-RULES (rule #5, severity: warning): claims about
 * code (file path, line, function) require either a matching `linked_symbols`
 * entry OR `epistemic.source_type: inferred` with `confidence < 1.0`.
 *
 * The rule walks the body for path-shaped strings. For each, checks if any
 * linked_symbols[].file matches by prefix. If none match and no inferred-mark,
 * emit a warning per claim (capped at 5 to avoid spam).
 */
export function citeOrMarkInferred(
  atom: ParsedAtom,
  _ctx: ValidationContext,
): ValidationError[] {
  const claims = extractPathClaims(atom.body)
  if (claims.length === 0) return []

  // Marked as inferred → no warnings.
  const epistemic = atom.fm.epistemic
  if (epistemic && typeof epistemic === 'object' && !Array.isArray(epistemic)) {
    const ep = epistemic as Record<string, unknown>
    if (ep.source_type === 'inferred') {
      const conf = ep.confidence
      if (typeof conf === 'number' && conf < 1.0) return []
    }
  }

  // Build set of cited file paths from linked_symbols.
  const cited = new Set<string>()
  const linked = atom.fm.linked_symbols
  if (Array.isArray(linked)) {
    for (const s of linked) {
      if (s && typeof s === 'object' && !Array.isArray(s)) {
        const file = (s as { file?: unknown }).file
        if (typeof file === 'string') cited.add(file)
      }
    }
  }

  const warnings: ValidationError[] = []
  const seen = new Set<string>()
  for (const claim of claims) {
    // Strip trailing :fn or :line (NOT .ext — that's part of the path).
    const path = claim.path.replace(/(?::[A-Za-z0-9_]+)?(?::\d+)?$/, '')
    if (seen.has(claim.path)) continue
    seen.add(claim.path)
    if (cited.has(path)) continue
    // Also accept partial-prefix match (e.g. claim is src/foo.ts:bar, citation is src/foo.ts)
    let matched = false
    for (const c of cited) {
      if (path === c || path.startsWith(c + ':')) {
        matched = true
        break
      }
    }
    if (matched) continue
    warnings.push({
      rule: 'cite-or-mark-inferred',
      severity: 'warning',
      line: claim.line,
      message: `body cites '${claim.path}' without a matching linked_symbols entry — add the citation OR mark epistemic.source_type='inferred' with confidence<1.0`,
      offending: claim.path,
    })
    if (warnings.length >= 5) break
  }
  return warnings
}
