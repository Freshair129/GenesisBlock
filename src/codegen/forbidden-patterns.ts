/**
 * Forbidden patterns per ADR--CODEGEN-FORBIDDEN-PATTERNS.
 * Each rule has a regex + severity. Hard rules (error) cause retry/escalation;
 * soft rules (warning) are surfaced in the report but don't fail the task.
 */

export interface PatternRule {
  id: string
  pattern: RegExp
  severity: 'error' | 'warning'
  message: string
}

export const FORBIDDEN_PATTERNS: ReadonlyArray<PatternRule> = [
  { id: 'export-default', pattern: /\bexport\s+default\b/, severity: 'error', message: 'export default is not allowed (App Router uses named exports)' },
  { id: 'req-body', pattern: /\breq\.body\b/, severity: 'error', message: 'req.body is Pages-Router; use `await req.json()` for App Router' },
  { id: 'req-tenant-id', pattern: /\breq\.tenantId\b/, severity: 'error', message: 'tenantId comes from withAuth context, not req' },
  { id: 'todo-comment', pattern: /\/\/\s*(TODO|FIXME|XXX)\b/, severity: 'error', message: 'no TODO/FIXME/XXX punts — retry instead' },
  { id: 'console-log', pattern: /\bconsole\.(log|debug|info|error|warn)\b/, severity: 'warning', message: 'route handlers should use Sentry/Pino, not console' },
  { id: 'process-env', pattern: /\bprocess\.env\.[A-Z_]+/, severity: 'warning', message: 'route-level env access is a smell — use @/lib/config' },
] as const

export interface PatternCheckResult {
  errors: string[]
  warnings: string[]
}

export function checkForbiddenPatterns(code: string): PatternCheckResult {
  const errors: string[] = []
  const warnings: string[] = []
  for (const rule of FORBIDDEN_PATTERNS) {
    if (rule.pattern.test(code)) {
      const line = `[${rule.id}] ${rule.message}`
      if (rule.severity === 'error') errors.push(line)
      else warnings.push(line)
    }
  }
  return { errors, warnings }
}

/**
 * Forbidden imports per ADR--CODEGEN-FORBIDDEN-PATTERNS:
 * - Conditional: zod/lodash/axios/etc — only allowed if in package.json.
 * - Absolute: fs/child_process/net/http — never allowed at the route layer.
 * - "../" relative parent paths — never; use @/ alias.
 */
export const ABSOLUTE_FORBIDDEN_IMPORTS: ReadonlySet<string> = new Set([
  'fs',
  'child_process',
  'net',
  'http',
])
export const CONDITIONAL_FORBIDDEN_IMPORTS: ReadonlySet<string> = new Set([
  'joi', 'zod', 'yup', 'ajv', 'uuid', 'lodash', 'ramda', 'axios', 'moment', 'underscore', 'bluebird', 'request',
])

const IMPORT_RE = /import\s+(?:[^'"]*?\s+from\s+)?['"]([^'"]+)['"]/g

export function checkImports(
  code: string,
  packageDeps: ReadonlySet<string>,
): string[] {
  const errors: string[] = []
  IMPORT_RE.lastIndex = 0
  let m: RegExpExecArray | null
  while ((m = IMPORT_RE.exec(code)) !== null) {
    const mod = m[1]!
    if (mod.startsWith('../')) {
      errors.push(`[forbidden-import] '${mod}' — relative parent paths not allowed; use @/ alias`)
      continue
    }
    // Absolute forbid (Node built-ins at route layer).
    if (ABSOLUTE_FORBIDDEN_IMPORTS.has(mod) || mod.startsWith('node:fs') || mod.startsWith('node:child_process')) {
      errors.push(`[forbidden-import] '${mod}' — disallowed at the route layer`)
      continue
    }
    // Conditional: not in deps → reject.
    if (CONDITIONAL_FORBIDDEN_IMPORTS.has(mod) && !packageDeps.has(mod)) {
      errors.push(`[forbidden-import] '${mod}' — not in package.json deps; install it explicitly first`)
    }
  }
  return errors
}
