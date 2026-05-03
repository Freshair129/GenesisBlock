/**
 * Strip SLM output wrapping per ADR--CODEGEN-POST-PROCESSING.
 * Order matters: fences first, then commentary trim, then EOL normalise.
 */

const FENCE_OPEN_RE = /^\s*([`~]{3,})[a-zA-Z0-9_-]*\s*$/
const FENCE_CLOSE_RE = /^\s*([`~]{3,})\s*$/
const FIRST_CODE_RE = /^(import|export|const|function|class|let|var|async|interface|type|enum)\b/

export function stripMarkdownFences(input: string): string {
  const lines = input.split(/\r?\n/)
  // Find first opening fence.
  const openIdx = lines.findIndex((l) => FENCE_OPEN_RE.test(l))
  if (openIdx === -1) return input
  // Find matching closing fence after it.
  let closeIdx = -1
  for (let i = openIdx + 1; i < lines.length; i++) {
    if (FENCE_CLOSE_RE.test(lines[i]!)) {
      closeIdx = i
      break
    }
  }
  if (closeIdx === -1) {
    // Unterminated fence — keep everything after the opener.
    return lines.slice(openIdx + 1).join('\n')
  }
  // Take only the lines between the fences.
  return lines.slice(openIdx + 1, closeIdx).join('\n')
}

export function stripLeadingCommentary(input: string): string {
  const lines = input.split(/\r?\n/)
  let i = 0
  while (i < lines.length) {
    const trimmed = lines[i]!.trim()
    if (trimmed === '') {
      i++
      continue
    }
    if (FIRST_CODE_RE.test(trimmed)) break
    if (trimmed.startsWith('//') || trimmed.startsWith('/*')) {
      // Allow comments to lead — they belong with the code below.
      break
    }
    i++
  }
  return lines.slice(i).join('\n')
}

export function stripTrailingCommentary(input: string): string {
  const lines = input.split(/\r?\n/)
  let last = -1
  for (let i = lines.length - 1; i >= 0; i--) {
    const trimmed = lines[i]!.trim()
    // The last line that ends with } or ; is treated as the end of code.
    if (trimmed.endsWith('}') || trimmed.endsWith(';') || trimmed.endsWith(')')) {
      last = i
      break
    }
  }
  if (last === -1) return input
  return lines.slice(0, last + 1).join('\n')
}

export function normaliseLineEndings(input: string): string {
  return input.replace(/\r\n?/g, '\n')
}

export function postProcess(raw: string): string {
  let s = raw
  s = stripMarkdownFences(s)
  s = stripLeadingCommentary(s)
  s = stripTrailingCommentary(s)
  s = normaliseLineEndings(s)
  return s.trimEnd() + '\n'
}
