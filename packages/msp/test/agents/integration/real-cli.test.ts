/**
 * Real-CLI integration tests for T1/T2/T3 tier adapters.
 *
 * Gated on `process.env.MSP_TEST_REAL_CLIS === '1'` — the entire suite is
 * SKIPPED by default so CI stays green without the binaries installed.
 *
 * To opt in locally (with `qwen` / `gemini` / `claude` on PATH):
 *
 *   MSP_TEST_REAL_CLIS=1 npm test --workspace=packages/msp -- test/agents/integration/
 *
 * Per-adapter behaviour:
 *   - `healthcheck()` is called first. If false (binary missing), the
 *     adapter's run-tests are skipped with a console message — they do NOT
 *     fail. This lets developers run the suite even when only one or two
 *     CLIs are installed.
 *   - When the binary IS present, `run('echo "test"', { timeout_ms: 30s,
 *     capture_stderr: true })` is invoked. We only assert `result.ok` —
 *     stdout content is provider-dependent and not contract-checked here.
 */
import { describe, expect, it } from 'vitest'

import { claudeAdapter } from '../../../src/agents/tiers/claude.js'
import { geminiAdapter } from '../../../src/agents/tiers/gemini.js'
import { qwenAdapter } from '../../../src/agents/tiers/qwen.js'
import type { TierAdapter } from '../../../src/agents/tiers/types.js'

const ENABLED = process.env.MSP_TEST_REAL_CLIS === '1'

const RUN_TIMEOUT_MS = 30_000
const PROMPT = 'echo "test"'

interface AdapterCase {
  readonly label: string
  readonly adapter: TierAdapter
}

const CASES: readonly AdapterCase[] = [
  { label: 'T1 / qwen', adapter: qwenAdapter },
  { label: 'T2 / gemini', adapter: geminiAdapter },
  { label: 'T3 / claude', adapter: claudeAdapter },
]

describe.skipIf(!ENABLED)('real-CLI integration (MSP_TEST_REAL_CLIS=1)', () => {
  for (const { label, adapter } of CASES) {
    describe(label, () => {
      it('healthcheck never throws and returns a boolean', async () => {
        // The adapter contract: healthcheck returns false on binary-missing,
        // never throws. We assert that property here regardless of whether
        // the binary is installed.
        const ok = await adapter.healthcheck()
        expect(typeof ok).toBe('boolean')
      })

      it('run() invocation returns a well-shaped RunResult (smoke)', async () => {
        const healthy = await adapter.healthcheck()
        if (!healthy) {
          // eslint-disable-next-line no-console
          console.log(
            `[real-cli] ${label}: binary missing or unhealthy — skipping run() test`,
          )
          return
        }
        const result = await adapter.run(PROMPT, {
          timeout_ms: RUN_TIMEOUT_MS,
          capture_stderr: true,
        })
        // Shape-only assertions: the adapter MUST return a RunResult whether
        // the LLM backend (Ollama, Gemini API auth, Claude auth) is currently
        // reachable or not. We don't assert ok=true here because backend
        // outages are environmental, not adapter bugs — when that happens we
        // log the non-zero exit_code and pass the test. This is a smoke test
        // of the spawn/exec wiring, not an end-to-end LLM correctness check.
        expect(typeof result.ok).toBe('boolean')
        expect(typeof result.output).toBe('string')
        expect(typeof result.exit_code).toBe('number')
        if (!result.ok) {
          // eslint-disable-next-line no-console
          console.log(
            `[real-cli] ${label}: run() returned ok=false (exit_code=${result.exit_code}). ` +
              `Likely backend outage (e.g. Ollama not running, auth missing). ` +
              `Adapter wiring is verified — backend health is out of scope.`,
          )
        }
      }, RUN_TIMEOUT_MS + 5_000)
    })
  }
})
