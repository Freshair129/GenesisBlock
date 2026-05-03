interface VitestAssertion {
  status?: string
  fullName?: string
  failureMessages?: string[]
  title?: string
}

interface VitestTestResult {
  name?: string
  assertionResults?: VitestAssertion[]
  message?: string
}

interface VitestReport {
  testResults?: VitestTestResult[]
  numFailedTests?: number
  numPassedTests?: number
}

/**
 * Parse vitest --reporter=json output. Falls back to stderr summary
 * when JSON is malformed or missing testResults.
 *
 * Returns one error string per failed assertion, formatted as:
 *   "<file> > <fullName>: <first line of failureMessages>"
 *
 * Empty array means all passed.
 */
export function parseVitestJson(stdout: string, stderr: string, exitCode: number): string[] {
  const trimmed = stdout.trim()
  if (trimmed === '') {
    if (exitCode === 0) return []
    return fallbackFromStderr(stderr)
  }

  let report: VitestReport
  try {
    // vitest may emit log lines before/after the JSON. Find the outermost {}.
    const start = trimmed.indexOf('{')
    const end = trimmed.lastIndexOf('}')
    if (start === -1 || end <= start) throw new Error('no JSON object found')
    report = JSON.parse(trimmed.slice(start, end + 1)) as VitestReport
  } catch {
    return fallbackFromStderr(stderr)
  }

  if (!Array.isArray(report.testResults)) {
    if (exitCode === 0) return []
    return fallbackFromStderr(stderr)
  }

  const errors: string[] = []
  for (const file of report.testResults) {
    const fileName = file.name ?? '<unknown file>'
    if (!Array.isArray(file.assertionResults)) {
      if (file.message && exitCode !== 0) {
        errors.push(`${fileName}: ${firstLine(file.message)}`)
      }
      continue
    }
    for (const a of file.assertionResults) {
      if (a.status === 'failed') {
        const name = a.fullName ?? a.title ?? '<unnamed test>'
        const detail = Array.isArray(a.failureMessages) && a.failureMessages.length > 0
          ? firstLine(a.failureMessages[0]!)
          : 'failed (no message)'
        errors.push(`${fileName} > ${name}: ${detail}`)
      }
    }
  }
  return errors
}

function firstLine(s: string): string {
  return s.split('\n')[0]!.trim().slice(0, 240)
}

function fallbackFromStderr(stderr: string): string[] {
  const lines = stderr.split('\n').filter((l) => l.trim() !== '')
  const tail = lines.slice(-50)
  if (tail.length === 0) return ['acceptance: vitest produced no parseable output']
  return [`acceptance: ${tail.join(' | ').slice(0, 1000)}`]
}
